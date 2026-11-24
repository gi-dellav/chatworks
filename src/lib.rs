#![forbid(unsafe_code)]

pub mod capabilities;
pub mod content;
pub mod error;
pub mod event;
pub mod future;
pub mod ids;
pub mod messenger;
pub mod model;
pub mod prelude;
pub mod services;
pub mod subscription;
pub mod testing;

pub use capabilities::{
    BackendInfo, Capabilities, ConversationKind, ConversationKinds, ThreadModel,
};
pub use content::{Block, Content, RawPayload};
pub use error::{EntityRef, Error, Feature, Result};
pub use event::{
    ConnectionState, Event, EventEnvelope, EventFilter, EventType, MembershipChange, ReactionChange,
};
pub use future::BoxFuture;
pub use ids::{AttachmentId, BackendId, ConversationId, Cursor, Id, MessageId, ThreadId, UserId};
pub use messenger::Messenger;
pub use model::*;
pub use services::*;
pub use subscription::{EventStream, Subscription};
