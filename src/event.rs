use crate::content::RawPayload;
use crate::ids::{BackendId, ConversationId, Cursor, MessageId, ThreadId};
use crate::model::{
    Conversation, Message, PresenceState, ReactionKey, Thread, Timestamp, TypingState, UserSummary,
};

#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MembershipChange {
    Joined,
    Left,
    Added,
    Removed,
}

#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ReactionChange {
    Added,
    Removed,
}

#[non_exhaustive]
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum ConnectionState {
    Connected,
    Disconnected { reason: Option<String> },
    Reconnecting,
}

#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum EventType {
    MessageCreated,
    MessageUpdated,
    MessageDeleted,
    ConversationCreated,
    ConversationUpdated,
    ConversationDeleted,
    ThreadCreated,
    ThreadUpdated,
    ThreadDeleted,
    MembershipChanged,
    ReactionChanged,
    TypingChanged,
    PresenceChanged,
    ReadMarkerChanged,
    ConnectionChanged,
    Lagged,
    Unknown,
}

#[non_exhaustive]
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Event {
    MessageCreated(Message),
    MessageUpdated(Message),
    MessageDeleted {
        conversation: ConversationId,
        thread: Option<ThreadId>,
        message: MessageId,
    },
    ConversationCreated(Conversation),
    ConversationUpdated(Conversation),
    ConversationDeleted {
        conversation: ConversationId,
    },
    ThreadCreated(Thread),
    ThreadUpdated(Thread),
    ThreadDeleted {
        conversation: ConversationId,
        thread: ThreadId,
    },
    MembershipChanged {
        conversation: ConversationId,
        user: UserSummary,
        change: MembershipChange,
    },
    ReactionChanged {
        conversation: ConversationId,
        thread: Option<ThreadId>,
        message: MessageId,
        key: ReactionKey,
        user: UserSummary,
        change: ReactionChange,
    },
    TypingChanged {
        conversation: ConversationId,
        thread: Option<ThreadId>,
        user: UserSummary,
        state: TypingState,
    },
    PresenceChanged {
        user: UserSummary,
        state: PresenceState,
        last_active: Option<Timestamp>,
    },
    ReadMarkerChanged {
        conversation: ConversationId,
        user: UserSummary,
        last_read: Option<MessageId>,
    },
    ConnectionChanged(ConnectionState),
    Lagged {
        dropped: u64,
    },
    Unknown {
        kind: String,
        raw: RawPayload,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EventEnvelope {
    pub backend: BackendId,
    pub sequence: Option<Cursor>,
    pub timestamp: Timestamp,
    pub event: Event,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct EventFilter {
    pub conversations: Option<Vec<ConversationId>>,
    pub threads: Option<Vec<ThreadId>>,
    pub types: Option<Vec<EventType>>,
    pub since: Option<Cursor>,
}

impl EventFilter {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn for_conversation(mut self, conversation: ConversationId) -> Self {
        self.conversations
            .get_or_insert_with(Vec::new)
            .push(conversation);
        self
    }

    pub fn for_thread(mut self, thread: ThreadId) -> Self {
        self.threads.get_or_insert_with(Vec::new).push(thread);
        self
    }

    pub fn of_types(mut self, types: Vec<EventType>) -> Self {
        self.types = Some(types);
        self
    }

    pub fn since(mut self, cursor: Cursor) -> Self {
        self.since = Some(cursor);
        self
    }
}
