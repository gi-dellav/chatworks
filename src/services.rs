use crate::error::Result;
use crate::future::BoxFuture;
use crate::ids::{AttachmentId, ConversationId, MessageId, ThreadId, UserId};
use crate::model::{
    Attachment, Conversation, ConversationPatch, ConversationSearchQuery, DownloadedAttachment,
    Message, MessageEdit, MessageSearchQuery, NewConversation, NewThread, Page, PageRequest,
    Presence, Reaction, ReactionKey, ReadMarker, Thread, TypingState, Upload, User, UserQuery,
};

pub trait ThreadService: Send + Sync {
    fn list_threads(
        &self,
        conversation: ConversationId,
        page: PageRequest,
    ) -> BoxFuture<'_, Result<Page<Thread>>>;

    fn get_thread(&self, id: ThreadId) -> BoxFuture<'_, Result<Thread>>;

    fn create_thread(&self, draft: NewThread) -> BoxFuture<'_, Result<Thread>>;
}

pub trait MessageEditingService: Send + Sync {
    fn edit_message(&self, id: MessageId, edit: MessageEdit) -> BoxFuture<'_, Result<Message>>;
}

pub trait MessageDeletionService: Send + Sync {
    fn delete_message(&self, id: MessageId) -> BoxFuture<'_, Result<()>>;
}

pub trait ReactionService: Send + Sync {
    fn add_reaction(&self, message: MessageId, key: ReactionKey) -> BoxFuture<'_, Result<()>>;

    fn remove_reaction(&self, message: MessageId, key: ReactionKey) -> BoxFuture<'_, Result<()>>;

    fn list_reactions(
        &self,
        message: MessageId,
        page: PageRequest,
    ) -> BoxFuture<'_, Result<Page<Reaction>>>;
}

pub trait AttachmentService: Send + Sync {
    fn upload(&self, upload: Upload) -> BoxFuture<'_, Result<Attachment>>;

    fn get_attachment(&self, id: AttachmentId) -> BoxFuture<'_, Result<Attachment>>;

    fn download(&self, id: AttachmentId) -> BoxFuture<'_, Result<DownloadedAttachment>>;
}

pub trait SearchService: Send + Sync {
    fn search_messages(&self, query: MessageSearchQuery) -> BoxFuture<'_, Result<Page<Message>>>;

    fn search_conversations(
        &self,
        query: ConversationSearchQuery,
    ) -> BoxFuture<'_, Result<Page<Conversation>>>;
}

pub trait DirectoryService: Send + Sync {
    fn list_users(&self, query: UserQuery) -> BoxFuture<'_, Result<Page<User>>>;

    fn get_user(&self, id: UserId) -> BoxFuture<'_, Result<User>>;
}

pub trait ConversationAdminService: Send + Sync {
    fn create_conversation(&self, draft: NewConversation) -> BoxFuture<'_, Result<Conversation>>;

    fn update_conversation(
        &self,
        id: ConversationId,
        patch: ConversationPatch,
    ) -> BoxFuture<'_, Result<Conversation>>;

    fn add_members(
        &self,
        conversation: ConversationId,
        members: Vec<UserId>,
    ) -> BoxFuture<'_, Result<()>>;

    fn remove_members(
        &self,
        conversation: ConversationId,
        members: Vec<UserId>,
    ) -> BoxFuture<'_, Result<()>>;

    fn join_conversation(&self, id: ConversationId) -> BoxFuture<'_, Result<()>>;

    fn leave_conversation(&self, id: ConversationId) -> BoxFuture<'_, Result<()>>;
}

pub trait TypingService: Send + Sync {
    fn set_typing(
        &self,
        conversation: ConversationId,
        thread: Option<ThreadId>,
        state: TypingState,
    ) -> BoxFuture<'_, Result<()>>;
}

pub trait PresenceService: Send + Sync {
    fn get_presence(&self, users: Vec<UserId>) -> BoxFuture<'_, Result<Vec<Presence>>>;
}

pub trait ReadMarkerService: Send + Sync {
    fn mark_read(
        &self,
        conversation: ConversationId,
        through: Option<MessageId>,
    ) -> BoxFuture<'_, Result<()>>;

    fn get_read_marker(
        &self,
        conversation: ConversationId,
        user: UserId,
    ) -> BoxFuture<'_, Result<Option<ReadMarker>>>;
}
