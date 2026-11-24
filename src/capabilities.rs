use std::ops::{BitOr, BitOrAssign};

use crate::ids::BackendId;

#[non_exhaustive]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ThreadModel {
    #[default]
    Flat,
    MessageAnchored,
    TopicAnchored,
}

#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ConversationKind {
    Direct,
    Group,
    Channel,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct ConversationKinds(u8);

impl ConversationKinds {
    pub const DIRECT: Self = Self(1 << 0);
    pub const GROUP: Self = Self(1 << 1);
    pub const CHANNEL: Self = Self(1 << 2);
    pub const ALL: Self = Self(Self::DIRECT.0 | Self::GROUP.0 | Self::CHANNEL.0);

    pub const fn empty() -> Self {
        Self(0)
    }

    pub const fn contains(self, other: Self) -> bool {
        self.0 & other.0 == other.0
    }

    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }

    pub const fn supports(self, kind: ConversationKind) -> bool {
        let bit = match kind {
            ConversationKind::Direct => Self::DIRECT,
            ConversationKind::Group => Self::GROUP,
            ConversationKind::Channel => Self::CHANNEL,
        };
        self.contains(bit)
    }
}

impl BitOr for ConversationKinds {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}

impl BitOrAssign for ConversationKinds {
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0;
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Capabilities {
    pub thread_model: ThreadModel,
    pub conversation_kinds: ConversationKinds,
    pub thread_creation: bool,
    pub message_editing: bool,
    pub message_deletion: bool,
    pub reactions: bool,
    pub typing: bool,
    pub presence: bool,
    pub read_markers: bool,
    pub attachment_upload: bool,
    pub attachment_download: bool,
    pub message_search: bool,
    pub conversation_search: bool,
    pub user_directory: bool,
    pub conversation_creation: bool,
    pub conversation_updating: bool,
    pub membership_management: bool,
    pub rich_content: bool,
    pub max_message_bytes: Option<usize>,
    pub max_attachment_bytes: Option<u64>,
    pub event_replay: bool,
    pub idempotent_send: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BackendInfo {
    pub backend: BackendId,
    pub capabilities: Capabilities,
}

impl BackendInfo {
    pub fn new(backend: BackendId, capabilities: Capabilities) -> Self {
        Self {
            backend,
            capabilities,
        }
    }

    pub fn backend(&self) -> &BackendId {
        &self.backend
    }

    pub fn capabilities(&self) -> &Capabilities {
        &self.capabilities
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn conversation_kinds_are_composable() {
        let kinds = ConversationKinds::DIRECT | ConversationKinds::GROUP;

        assert!(kinds.supports(ConversationKind::Direct));
        assert!(kinds.supports(ConversationKind::Group));
        assert!(!kinds.supports(ConversationKind::Channel));
    }
}
