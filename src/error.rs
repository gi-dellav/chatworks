use std::error::Error as StdError;
use std::fmt;
use std::time::Duration;

use crate::ids::{AttachmentId, BackendId, ConversationId, MessageId, ThreadId, UserId};

pub type Result<T> = std::result::Result<T, Error>;

#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Feature {
    Threads,
    ThreadCreation,
    MessageEditing,
    MessageDeletion,
    Reactions,
    Typing,
    Presence,
    ReadMarkers,
    AttachmentUpload,
    AttachmentDownload,
    MessageSearch,
    ConversationSearch,
    UserDirectory,
    ConversationCreation,
    ConversationUpdating,
    MembershipManagement,
    RichContent,
    EventReplay,
    IdempotentSend,
}

impl fmt::Display for Feature {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self {
            Self::Threads => "threads",
            Self::ThreadCreation => "thread creation",
            Self::MessageEditing => "message editing",
            Self::MessageDeletion => "message deletion",
            Self::Reactions => "reactions",
            Self::Typing => "typing indicators",
            Self::Presence => "presence",
            Self::ReadMarkers => "read markers",
            Self::AttachmentUpload => "attachment upload",
            Self::AttachmentDownload => "attachment download",
            Self::MessageSearch => "message search",
            Self::ConversationSearch => "conversation search",
            Self::UserDirectory => "user directory",
            Self::ConversationCreation => "conversation creation",
            Self::ConversationUpdating => "conversation updating",
            Self::MembershipManagement => "membership management",
            Self::RichContent => "rich content",
            Self::EventReplay => "event replay",
            Self::IdempotentSend => "idempotent send",
        };
        f.write_str(name)
    }
}

#[non_exhaustive]
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum EntityRef {
    User(UserId),
    Conversation(ConversationId),
    Thread(ThreadId),
    Message(MessageId),
    Attachment(AttachmentId),
}

impl fmt::Display for EntityRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::User(id) => write!(f, "user {id}"),
            Self::Conversation(id) => write!(f, "conversation {id}"),
            Self::Thread(id) => write!(f, "thread {id}"),
            Self::Message(id) => write!(f, "message {id}"),
            Self::Attachment(id) => write!(f, "attachment {id}"),
        }
    }
}

#[non_exhaustive]
#[derive(Debug)]
pub enum Error {
    Unsupported {
        feature: Feature,
    },
    NotFound {
        entity: EntityRef,
    },
    InvalidInput {
        reason: String,
    },
    Unauthorized,
    Forbidden,
    RateLimited {
        retry_after: Option<Duration>,
    },
    Conflict {
        reason: String,
    },
    Transport {
        reason: String,
    },
    Protocol {
        reason: String,
    },
    Backend {
        backend: BackendId,
        source: Box<dyn StdError + Send + Sync>,
    },
}

impl Error {
    pub fn unsupported(feature: Feature) -> Self {
        Self::Unsupported { feature }
    }

    pub fn invalid_input(reason: impl Into<String>) -> Self {
        Self::InvalidInput {
            reason: reason.into(),
        }
    }

    pub fn is_retryable(&self) -> bool {
        matches!(self, Self::RateLimited { .. } | Self::Transport { .. })
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unsupported { feature } => write!(f, "unsupported feature: {feature}"),
            Self::NotFound { entity } => write!(f, "not found: {entity}"),
            Self::InvalidInput { reason } => write!(f, "invalid input: {reason}"),
            Self::Unauthorized => f.write_str("unauthorized"),
            Self::Forbidden => f.write_str("forbidden"),
            Self::RateLimited { retry_after } => match retry_after {
                Some(delay) => write!(f, "rate limited; retry after {delay:?}"),
                None => f.write_str("rate limited"),
            },
            Self::Conflict { reason } => write!(f, "conflict: {reason}"),
            Self::Transport { reason } => write!(f, "transport error: {reason}"),
            Self::Protocol { reason } => write!(f, "protocol error: {reason}"),
            Self::Backend { backend, source } => {
                write!(f, "backend {backend} error: {source}")
            }
        }
    }
}

impl StdError for Error {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        match self {
            Self::Backend { source, .. } => Some(source.as_ref()),
            _ => None,
        }
    }
}
