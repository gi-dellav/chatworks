use crate::capabilities::BackendInfo;
use crate::error::Result;
use crate::event::EventFilter;
use crate::future::BoxFuture;
use crate::ids::{ConversationId, MessageId};
use crate::model::{
    Conversation, ConversationQuery, Message, MessageDraft, MessageQuery, Page, User,
};
use crate::services::{
    AttachmentService, ConversationAdminService, DirectoryService, MessageDeletionService,
    MessageEditingService, PresenceService, ReactionService, ReadMarkerService, SearchService,
    ThreadService, TypingService,
};
use crate::subscription::Subscription;

pub trait Messenger: Send + Sync + 'static {
    fn info(&self) -> &BackendInfo;

    fn current_user(&self) -> BoxFuture<'_, Result<User>>;

    fn list_conversations(
        &self,
        query: ConversationQuery,
    ) -> BoxFuture<'_, Result<Page<Conversation>>>;

    fn get_conversation(&self, id: ConversationId) -> BoxFuture<'_, Result<Conversation>>;

    fn list_messages(&self, query: MessageQuery) -> BoxFuture<'_, Result<Page<Message>>>;

    fn get_message(&self, id: MessageId) -> BoxFuture<'_, Result<Message>>;

    fn send_message(&self, draft: MessageDraft) -> BoxFuture<'_, Result<Message>>;

    fn subscribe(&self, filter: EventFilter) -> BoxFuture<'_, Result<Subscription>>;

    fn threads(&self) -> Option<&dyn ThreadService> {
        None
    }

    fn message_editing(&self) -> Option<&dyn MessageEditingService> {
        None
    }

    fn message_deletion(&self) -> Option<&dyn MessageDeletionService> {
        None
    }

    fn reactions(&self) -> Option<&dyn ReactionService> {
        None
    }

    fn attachments(&self) -> Option<&dyn AttachmentService> {
        None
    }

    fn search(&self) -> Option<&dyn SearchService> {
        None
    }

    fn directory(&self) -> Option<&dyn DirectoryService> {
        None
    }

    fn conversation_admin(&self) -> Option<&dyn ConversationAdminService> {
        None
    }

    fn typing(&self) -> Option<&dyn TypingService> {
        None
    }

    fn presence(&self) -> Option<&dyn PresenceService> {
        None
    }

    fn read_markers(&self) -> Option<&dyn ReadMarkerService> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn messenger_is_object_safe() {
        fn accepts(_: &dyn Messenger) {}
        let _ = accepts;
    }
}
