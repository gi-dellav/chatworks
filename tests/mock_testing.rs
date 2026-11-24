use std::future::Future;
use std::task::{Context, Poll, Waker};

use chatworks::prelude::*;
use chatworks::testing::{Call, MockMessenger};

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
fn mock_backend_can_be_driven_through_the_public_api() {
    let mock = MockMessenger::builder().backend("test").build();
    let conversation = mock.conversation_id("general");
    let mut subscription =
        run(mock.subscribe(EventFilter::new().for_conversation(conversation.clone())))
            .expect("subscribe works");

    let sent = run(mock.send_message(MessageDraft::new(conversation, Content::plain("hello"))))
        .expect("send works");

    assert_eq!(sent.id.backend().as_str(), "test");
    assert!(matches!(
        mock.last_call(),
        Some(Call::SendMessage(draft)) if draft.content.plain == "hello"
    ));

    let events = ready_events(&mut subscription);
    assert_eq!(events.len(), 1);
    assert!(matches!(
        &events[0].event,
        Event::MessageCreated(message) if message.id == sent.id
    ));
}

#[test]
fn mock_backend_reports_failures_and_services() {
    let mock = MockMessenger::new();
    assert!(mock.threads().is_some());
    assert!(mock.reactions().is_some());
    assert!(mock.search().is_some());

    mock.fail_next(Error::Unsupported {
        feature: Feature::RichContent,
    });

    let result = run(mock.current_user());
    assert!(matches!(
        result,
        Err(Error::Unsupported {
            feature: Feature::RichContent
        })
    ));
    assert!(run(mock.current_user()).is_ok());
}
