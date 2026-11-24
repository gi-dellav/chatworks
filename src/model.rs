use std::fmt;
use std::num::NonZeroU16;
use std::time::SystemTime;

use crate::capabilities::ConversationKind;
use crate::content::{Content, RawPayload};
use crate::ids::{AttachmentId, ConversationId, Cursor, MessageId, ThreadId, UserId};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Timestamp(pub SystemTime);

impl Timestamp {
    pub fn now() -> Self {
        Self(SystemTime::now())
    }

    pub fn as_system_time(self) -> SystemTime {
        self.0
    }
}

impl From<SystemTime> for Timestamp {
    fn from(value: SystemTime) -> Self {
        Self(value)
    }
}

impl From<Timestamp> for SystemTime {
    fn from(value: Timestamp) -> Self {
        value.0
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct UserSummary {
    pub id: UserId,
    pub handle: Option<String>,
    pub display_name: String,
    pub avatar_url: Option<String>,
    pub is_bot: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct User {
    pub id: UserId,
    pub handle: Option<String>,
    pub display_name: String,
    pub avatar_url: Option<String>,
    pub is_bot: bool,
    pub raw: Option<RawPayload>,
}

impl User {
    pub fn summary(&self) -> UserSummary {
        UserSummary {
            id: self.id.clone(),
            handle: self.handle.clone(),
            display_name: self.display_name.clone(),
            avatar_url: self.avatar_url.clone(),
            is_bot: self.is_bot,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct UnreadState {
    pub messages: u32,
    pub mentions: u32,
    pub last_read: Option<MessageId>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Conversation {
    pub id: ConversationId,
    pub kind: ConversationKind,
    pub title: Option<String>,
    pub members: Option<Vec<UserSummary>>,
    pub member_count: Option<u32>,
    pub archived: bool,
    pub created_at: Option<Timestamp>,
    pub last_activity: Option<Timestamp>,
    pub unread: Option<UnreadState>,
    pub raw: Option<RawPayload>,
}

impl Conversation {
    pub fn is_direct(&self) -> bool {
        matches!(self.kind, ConversationKind::Direct)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Thread {
    pub id: ThreadId,
    pub conversation: ConversationId,
    pub title: Option<String>,
    pub anchor: Option<MessageId>,
    pub reply_count: Option<u32>,
    pub created_at: Option<Timestamp>,
    pub last_activity: Option<Timestamp>,
    pub raw: Option<RawPayload>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Message {
    pub id: MessageId,
    pub conversation: ConversationId,
    pub thread: Option<ThreadId>,
    pub author: UserSummary,
    pub content: Content,
    pub attachments: Vec<Attachment>,
    pub mentions: Vec<UserId>,
    pub reply_to: Option<MessageId>,
    pub sent_at: Timestamp,
    pub edited_at: Option<Timestamp>,
    pub deleted: bool,
    pub raw: Option<RawPayload>,
}

impl Message {
    pub fn is_thread_reply(&self) -> bool {
        self.thread.is_some()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MessageDraft {
    pub conversation: ConversationId,
    pub thread: Option<ThreadId>,
    pub content: Content,
    pub attachments: Vec<AttachmentDraft>,
    pub mentions: Vec<UserId>,
    pub reply_to: Option<MessageId>,
    pub client_token: Option<String>,
    pub raw: Option<RawPayload>,
}

impl MessageDraft {
    pub fn new(conversation: ConversationId, content: Content) -> Self {
        Self {
            conversation,
            thread: None,
            content,
            attachments: Vec::new(),
            mentions: Vec::new(),
            reply_to: None,
            client_token: None,
            raw: None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MessageEdit {
    pub content: Content,
    pub mentions: Vec<UserId>,
}

impl MessageEdit {
    pub fn new(content: Content) -> Self {
        Self {
            content,
            mentions: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Attachment {
    pub id: AttachmentId,
    pub filename: String,
    pub media_type: String,
    pub size: u64,
    pub url: Option<String>,
    pub expires_at: Option<Timestamp>,
    pub raw: Option<RawPayload>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Upload {
    pub filename: String,
    pub media_type: String,
    pub data: Vec<u8>,
}

impl Upload {
    pub fn new(
        filename: impl Into<String>,
        media_type: impl Into<String>,
        data: impl Into<Vec<u8>>,
    ) -> Self {
        Self {
            filename: filename.into(),
            media_type: media_type.into(),
            data: data.into(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DownloadedAttachment {
    pub attachment: Attachment,
    pub data: Vec<u8>,
}

#[non_exhaustive]
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AttachmentDraft {
    Existing(AttachmentId),
    Upload(Upload),
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ReactionKey(String);

impl ReactionKey {
    pub fn new(key: impl Into<String>) -> Self {
        Self(key.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn into_string(self) -> String {
        self.0
    }
}

impl AsRef<str> for ReactionKey {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl fmt::Display for ReactionKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl From<String> for ReactionKey {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl From<&str> for ReactionKey {
    fn from(value: &str) -> Self {
        Self(value.to_owned())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Reaction {
    pub key: ReactionKey,
    pub count: u32,
    pub users: Option<Vec<UserSummary>>,
    pub reacted_by_current_user: bool,
}

#[non_exhaustive]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum PresenceState {
    Online,
    Away,
    Busy,
    Offline,
    #[default]
    Unknown,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Presence {
    pub user: UserSummary,
    pub state: PresenceState,
    pub last_active: Option<Timestamp>,
    pub status_text: Option<String>,
}

#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TypingState {
    Started,
    Stopped,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReadMarker {
    pub user: UserSummary,
    pub last_read: Option<MessageId>,
    pub updated_at: Option<Timestamp>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PageRequest {
    pub cursor: Option<Cursor>,
    pub limit: Option<NonZeroU16>,
}

impl PageRequest {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_cursor(mut self, cursor: Cursor) -> Self {
        self.cursor = Some(cursor);
        self
    }

    pub fn with_limit(mut self, limit: NonZeroU16) -> Self {
        self.limit = Some(limit);
        self
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Page<T> {
    pub items: Vec<T>,
    pub next: Option<Cursor>,
}

impl<T> Page<T> {
    pub fn new(items: Vec<T>, next: Option<Cursor>) -> Self {
        Self { items, next }
    }

    pub fn empty() -> Self {
        Self {
            items: Vec::new(),
            next: None,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    pub fn has_more(&self) -> bool {
        self.next.is_some()
    }
}

impl<T> Default for Page<T> {
    fn default() -> Self {
        Self::empty()
    }
}

#[non_exhaustive]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Order {
    #[default]
    NewestFirst,
    OldestFirst,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ConversationQuery {
    pub kinds: Option<Vec<ConversationKind>>,
    pub include_archived: bool,
    pub page: PageRequest,
}

#[non_exhaustive]
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub enum MessageScope {
    #[default]
    Root,
    Timeline,
    Thread(ThreadId),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MessageQuery {
    pub conversation: ConversationId,
    pub scope: MessageScope,
    pub before: Option<Timestamp>,
    pub after: Option<Timestamp>,
    pub order: Order,
    pub page: PageRequest,
}

impl MessageQuery {
    pub fn new(conversation: ConversationId) -> Self {
        Self {
            conversation,
            scope: MessageScope::Root,
            before: None,
            after: None,
            order: Order::NewestFirst,
            page: PageRequest::default(),
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct UserQuery {
    pub text: Option<String>,
    pub page: PageRequest,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MessageSearchQuery {
    pub text: String,
    pub conversations: Option<Vec<ConversationId>>,
    pub thread: Option<ThreadId>,
    pub authors: Option<Vec<UserId>>,
    pub before: Option<Timestamp>,
    pub after: Option<Timestamp>,
    pub order: Order,
    pub page: PageRequest,
}

impl MessageSearchQuery {
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            conversations: None,
            thread: None,
            authors: None,
            before: None,
            after: None,
            order: Order::NewestFirst,
            page: PageRequest::default(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConversationSearchQuery {
    pub text: String,
    pub kinds: Option<Vec<ConversationKind>>,
    pub include_archived: bool,
    pub page: PageRequest,
}

impl ConversationSearchQuery {
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            kinds: None,
            include_archived: false,
            page: PageRequest::default(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NewConversation {
    pub kind: ConversationKind,
    pub title: Option<String>,
    pub members: Vec<UserId>,
    pub raw: Option<RawPayload>,
}

impl NewConversation {
    pub fn direct(user: UserId) -> Self {
        Self {
            kind: ConversationKind::Direct,
            title: None,
            members: vec![user],
            raw: None,
        }
    }

    pub fn group(title: impl Into<String>, members: Vec<UserId>) -> Self {
        Self {
            kind: ConversationKind::Group,
            title: Some(title.into()),
            members,
            raw: None,
        }
    }

    pub fn channel(title: impl Into<String>) -> Self {
        Self {
            kind: ConversationKind::Channel,
            title: Some(title.into()),
            members: Vec::new(),
            raw: None,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ConversationPatch {
    pub title: Option<String>,
    pub archived: Option<bool>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NewThread {
    pub conversation: ConversationId,
    pub anchor: Option<MessageId>,
    pub title: Option<String>,
}

impl NewThread {
    pub fn new(conversation: ConversationId) -> Self {
        Self {
            conversation,
            anchor: None,
            title: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::BackendId;

    #[test]
    fn pages_report_more_results() {
        let cursor = Cursor::new(BackendId::new("backend"), "next");
        let page = Page::new(vec![1, 2, 3], Some(cursor));

        assert!(!page.is_empty());
        assert!(page.has_more());
        assert_eq!(page.items.len(), 3);
    }

    #[test]
    fn message_drafts_start_at_root_scope() {
        let backend = BackendId::new("backend");
        let conversation = ConversationId::new(backend, "conversation");
        let draft = MessageDraft::new(conversation, Content::plain("hello"));

        assert!(draft.thread.is_none());
        assert!(draft.attachments.is_empty());
        assert!(draft.mentions.is_empty());
    }
}
