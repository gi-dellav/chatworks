use std::future::Future;
use std::num::NonZeroU16;

use super::*;
use crate::content::Content;

fn run<F: Future>(future: F) -> F::Output {
    let waker = Waker::noop();
    let mut context = Context::from_waker(waker);
    let mut future = std::pin::pin!(future);
    loop {
        match Future::poll(future.as_mut(), &mut context) {
            Poll::Ready(output) => return output,
            Poll::Pending => std::thread::yield_now(),
        }
    }
}

fn ready_events(subscription: &mut Subscription) -> Vec<EventEnvelope> {
    let waker = Waker::noop();
    let mut context = Context::from_waker(waker);
    let mut events = Vec::new();
    loop {
        match subscription.poll_next(&mut context) {
            Poll::Ready(Some(Ok(envelope))) => events.push(envelope),
            Poll::Ready(Some(Err(error))) => panic!("unexpected stream error: {error}"),
            Poll::Ready(None) | Poll::Pending => return events,
        }
    }
}

#[test]
fn send_message_round_trips() {
    let mock = MockMessenger::new();
    let conversation = mock.conversation_id("general");
    let draft = MessageDraft::new(conversation.clone(), Content::plain("hello"));

    let sent = run(mock.send_message(draft)).expect("send succeeds");
    let current_user = run(mock.current_user()).expect("current user loads");

    assert_eq!(sent.content.plain, "hello");
    assert_eq!(sent.conversation, conversation);
    assert_eq!(sent.author.id, current_user.id);

    let fetched = run(mock.get_message(sent.id.clone())).expect("message loads");
    assert_eq!(fetched, sent);

    let page = run(mock.list_messages(MessageQuery::new(conversation))).expect("listing works");
    assert_eq!(page.items, vec![sent]);
    assert!(matches!(mock.last_call(), Some(Call::ListMessages(_))));
}

#[test]
fn queued_failures_surface_once() {
    let mock = MockMessenger::new();
    let conversation = mock.conversation_id("general");
    mock.fail_next(Error::invalid_input("nope"));

    let first = run(mock.send_message(MessageDraft::new(
        conversation.clone(),
        Content::plain("first"),
    )));
    assert!(matches!(first, Err(Error::InvalidInput { .. })));

    let second = run(mock.send_message(MessageDraft::new(
        conversation.clone(),
        Content::plain("second"),
    )));
    assert!(second.is_ok());

    let page = run(mock.list_messages(MessageQuery::new(conversation))).expect("listing works");
    assert_eq!(page.items.len(), 1);
    assert_eq!(page.items[0].content.plain, "second");
}

#[test]
fn subscriptions_receive_matching_events() {
    let mock = MockMessenger::new();
    let conversation = mock.conversation_id("general");
    let other = mock.conversation_id("other");
    let mut subscription =
        run(mock.subscribe(EventFilter::new().for_conversation(conversation.clone())))
            .expect("subscribe works");

    let sent = run(mock.send_message(MessageDraft::new(conversation, Content::plain("hello"))))
        .expect("send works");
    mock.publish(Event::MessageDeleted {
        conversation: other,
        thread: None,
        message: mock.message_id("gone"),
    });

    let events = ready_events(&mut subscription);
    assert_eq!(events.len(), 1);
    assert!(matches!(
        &events[0].event,
        Event::MessageCreated(message) if message.id == sent.id
    ));
}

#[test]
fn subscriptions_filter_by_type_and_cursor() {
    let mock = MockMessenger::new();
    let conversation = mock.conversation_id("general");
    let mut subscription = run(mock.subscribe(
        EventFilter::new()
            .for_conversation(conversation.clone())
            .of_types(vec![EventType::TypingChanged])
            .since(mock.cursor("0")),
    ))
    .expect("subscribe works");

    run(mock.send_message(MessageDraft::new(
        conversation.clone(),
        Content::plain("hello"),
    )))
    .expect("send works");
    run(mock.set_typing(conversation, None, TypingState::Started)).expect("typing works");

    let events = ready_events(&mut subscription);
    assert_eq!(events.len(), 1);
    assert!(matches!(events[0].event, Event::TypingChanged { .. }));
}

#[test]
fn services_operate_on_in_memory_state() {
    let mock = MockMessenger::new();
    let conversation = mock.conversation_id("general");
    let sent = run(mock.send_message(MessageDraft::new(conversation, Content::plain("hello"))))
        .expect("send works");

    assert!(mock.threads().is_some());
    assert!(mock.message_editing().is_some());
    assert!(mock.message_deletion().is_some());
    assert!(mock.reactions().is_some());
    assert!(mock.attachments().is_some());
    assert!(mock.search().is_some());
    assert!(mock.directory().is_some());
    assert!(mock.conversation_admin().is_some());
    assert!(mock.typing().is_some());
    assert!(mock.presence().is_some());
    assert!(mock.read_markers().is_some());

    run(mock.add_reaction(sent.id.clone(), ReactionKey::new("+1"))).expect("reaction works");
    let reactions =
        run(mock.list_reactions(sent.id.clone(), PageRequest::new())).expect("listing works");
    assert_eq!(reactions.items.len(), 1);
    assert_eq!(reactions.items[0].count, 1);
    assert!(reactions.items[0].reacted_by_current_user);

    let edited = run(mock.edit_message(sent.id.clone(), MessageEdit::new(Content::plain("hi"))))
        .expect("edit works");
    assert_eq!(edited.content.plain, "hi");
    assert!(edited.edited_at.is_some());
}

#[test]
fn attachment_upload_and_download_round_trip() {
    let mock = MockMessenger::new();
    let upload = Upload::new("notes.txt", "text/plain", b"hello".to_vec());

    let attachment = run(mock.upload(upload)).expect("upload works");
    assert_eq!(attachment.filename, "notes.txt");
    assert_eq!(attachment.size, 5);

    let downloaded = run(mock.download(attachment.id.clone())).expect("download works");
    assert_eq!(downloaded.attachment, attachment);
    assert_eq!(downloaded.data, b"hello");
}

#[test]
fn pages_use_cursors() {
    let mock = MockMessenger::new();
    let conversation = mock.conversation_id("general");
    for text in ["one", "two", "three"] {
        run(mock.send_message(MessageDraft::new(
            conversation.clone(),
            Content::plain(text),
        )))
        .expect("send works");
    }

    let limit = NonZeroU16::new(2).expect("nonzero limit");
    let mut query = MessageQuery::new(conversation);
    query.page = PageRequest::new().with_limit(limit);

    let first = run(mock.list_messages(query.clone())).expect("first page works");
    assert_eq!(first.items.len(), 2);
    assert_eq!(first.items[0].content.plain, "three");
    assert!(first.has_more());

    query.page = PageRequest::new()
        .with_limit(limit)
        .with_cursor(first.next.clone().expect("next cursor is present"));
    let second = run(mock.list_messages(query)).expect("second page works");
    assert_eq!(second.items.len(), 1);
    assert_eq!(second.items[0].content.plain, "one");
    assert!(!second.has_more());
}
