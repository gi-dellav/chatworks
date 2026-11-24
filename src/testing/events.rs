use super::*;
use crate::subscription::EventStream;

#[derive(Default)]
pub(super) struct EventHub {
    streams: Mutex<Vec<Weak<Mutex<StreamState>>>>,
}

impl EventHub {
    pub(super) fn subscribe(&self, filter: EventFilter) -> MockEventStream {
        let state = Arc::new(Mutex::new(StreamState {
            filter,
            events: VecDeque::new(),
            closed: false,
            waker: None,
        }));
        let mut streams = lock(&self.streams);
        streams.retain(|stream| stream.upgrade().is_some());
        streams.push(Arc::downgrade(&state));
        drop(streams);
        MockEventStream { state }
    }

    pub(super) fn publish(&self, envelope: EventEnvelope) {
        let mut wakers = Vec::new();
        {
            let mut streams = lock(&self.streams);
            streams.retain(|stream| {
                let Some(state) = stream.upgrade() else {
                    return false;
                };
                let mut state = lock(&state);
                if state.closed {
                    return false;
                }
                if matches_filter(&state.filter, &envelope) {
                    state.events.push_back(envelope.clone());
                    if let Some(waker) = state.waker.take() {
                        wakers.push(waker);
                    }
                }
                true
            });
        }
        for waker in wakers {
            waker.wake();
        }
    }
}

pub(super) struct MockEventStream {
    state: Arc<Mutex<StreamState>>,
}

struct StreamState {
    filter: EventFilter,
    events: VecDeque<EventEnvelope>,
    closed: bool,
    waker: Option<Waker>,
}

impl EventStream for MockEventStream {
    fn poll_next(
        self: std::pin::Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Result<EventEnvelope>>> {
        let this = self.get_mut();
        let mut state = lock(&this.state);
        if state.closed {
            return Poll::Ready(None);
        }
        match state.events.pop_front() {
            Some(envelope) => Poll::Ready(Some(Ok(envelope))),
            None => {
                state.waker = Some(cx.waker().clone());
                Poll::Pending
            }
        }
    }

    fn close(self: std::pin::Pin<&mut Self>) {
        let this = self.get_mut();
        let mut state = lock(&this.state);
        state.closed = true;
        state.events.clear();
        if let Some(waker) = state.waker.take() {
            waker.wake();
        }
    }
}

fn matches_filter(filter: &EventFilter, envelope: &EventEnvelope) -> bool {
    if let Some(types) = &filter.types
        && !types.contains(&event_type(&envelope.event))
    {
        return false;
    }
    if let Some(conversations) = &filter.conversations {
        match event_conversation(&envelope.event) {
            Some(conversation) if conversations.contains(conversation) => {}
            _ => return false,
        }
    }
    if let Some(threads) = &filter.threads {
        match event_thread(&envelope.event) {
            Some(thread) if threads.contains(thread) => {}
            _ => return false,
        }
    }
    if let (Some(since), Some(sequence)) = (&filter.since, &envelope.sequence)
        && !cursor_is_after(sequence, since)
    {
        return false;
    }
    true
}

fn cursor_is_after(sequence: &Cursor, since: &Cursor) -> bool {
    match (
        sequence.as_str().parse::<u64>(),
        since.as_str().parse::<u64>(),
    ) {
        (Ok(sequence), Ok(since)) => sequence > since,
        _ => sequence > since,
    }
}

fn event_type(event: &Event) -> EventType {
    match event {
        Event::MessageCreated(_) => EventType::MessageCreated,
        Event::MessageUpdated(_) => EventType::MessageUpdated,
        Event::MessageDeleted { .. } => EventType::MessageDeleted,
        Event::ConversationCreated(_) => EventType::ConversationCreated,
        Event::ConversationUpdated(_) => EventType::ConversationUpdated,
        Event::ConversationDeleted { .. } => EventType::ConversationDeleted,
        Event::ThreadCreated(_) => EventType::ThreadCreated,
        Event::ThreadUpdated(_) => EventType::ThreadUpdated,
        Event::ThreadDeleted { .. } => EventType::ThreadDeleted,
        Event::MembershipChanged { .. } => EventType::MembershipChanged,
        Event::ReactionChanged { .. } => EventType::ReactionChanged,
        Event::TypingChanged { .. } => EventType::TypingChanged,
        Event::PresenceChanged { .. } => EventType::PresenceChanged,
        Event::ReadMarkerChanged { .. } => EventType::ReadMarkerChanged,
        Event::ConnectionChanged(_) => EventType::ConnectionChanged,
        Event::Lagged { .. } => EventType::Lagged,
        Event::Unknown { .. } => EventType::Unknown,
    }
}

fn event_conversation(event: &Event) -> Option<&ConversationId> {
    match event {
        Event::MessageCreated(message) | Event::MessageUpdated(message) => {
            Some(&message.conversation)
        }
        Event::MessageDeleted { conversation, .. } => Some(conversation),
        Event::ConversationCreated(conversation) | Event::ConversationUpdated(conversation) => {
            Some(&conversation.id)
        }
        Event::ConversationDeleted { conversation } => Some(conversation),
        Event::ThreadCreated(thread) | Event::ThreadUpdated(thread) => Some(&thread.conversation),
        Event::ThreadDeleted { conversation, .. } => Some(conversation),
        Event::MembershipChanged { conversation, .. } => Some(conversation),
        Event::ReactionChanged { conversation, .. } => Some(conversation),
        Event::TypingChanged { conversation, .. } => Some(conversation),
        Event::ReadMarkerChanged { conversation, .. } => Some(conversation),
        Event::PresenceChanged { .. }
        | Event::ConnectionChanged(_)
        | Event::Lagged { .. }
        | Event::Unknown { .. } => None,
    }
}

fn event_thread(event: &Event) -> Option<&ThreadId> {
    match event {
        Event::MessageCreated(message) | Event::MessageUpdated(message) => message.thread.as_ref(),
        Event::MessageDeleted { thread, .. } => thread.as_ref(),
        Event::ThreadCreated(thread) | Event::ThreadUpdated(thread) => Some(&thread.id),
        Event::ThreadDeleted { thread, .. } => Some(thread),
        Event::ReactionChanged { thread, .. } => thread.as_ref(),
        Event::TypingChanged { thread, .. } => thread.as_ref(),
        Event::ConversationCreated(_)
        | Event::ConversationUpdated(_)
        | Event::ConversationDeleted { .. }
        | Event::MembershipChanged { .. }
        | Event::PresenceChanged { .. }
        | Event::ReadMarkerChanged { .. }
        | Event::ConnectionChanged(_)
        | Event::Lagged { .. }
        | Event::Unknown { .. } => None,
    }
}
