//! Test doubles for backend implementations.
//!
//! [`MockMessenger`] is an in-memory [`Messenger`] that implements every
//! optional service, records method calls, supports queued failures, and
//! publishes events to subscriptions.

use std::collections::VecDeque;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError, Weak};
use std::task::{Context, Poll, Waker};

use crate::capabilities::{BackendInfo, Capabilities, ConversationKinds, ThreadModel};
use crate::error::{EntityRef, Error, Result};
use crate::event::{
    Event, EventEnvelope, EventFilter, EventType, MembershipChange, ReactionChange,
};
use crate::future::BoxFuture;
use crate::ids::{AttachmentId, BackendId, ConversationId, Cursor, MessageId, ThreadId, UserId};
use crate::messenger::Messenger;
use crate::model::{
    Attachment, AttachmentDraft, Conversation, ConversationPatch, ConversationQuery,
    ConversationSearchQuery, DownloadedAttachment, Message, MessageDraft, MessageEdit,
    MessageQuery, MessageScope, MessageSearchQuery, NewConversation, NewThread, Order, Page,
    PageRequest, Presence, PresenceState, Reaction, ReactionKey, ReadMarker, Thread, Timestamp,
    TypingState, Upload, User, UserQuery, UserSummary,
};
use crate::services::{
    AttachmentService, ConversationAdminService, DirectoryService, MessageDeletionService,
    MessageEditingService, PresenceService, ReactionService, ReadMarkerService, SearchService,
    ThreadService, TypingService,
};
use crate::subscription::Subscription;

mod events;
mod services;

#[cfg(test)]
mod tests;

use events::EventHub;

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

fn full_capabilities() -> Capabilities {
    Capabilities {
        thread_model: ThreadModel::Flat,
        conversation_kinds: ConversationKinds::ALL,
        thread_creation: true,
        message_editing: true,
        message_deletion: true,
        reactions: true,
        typing: true,
        presence: true,
        read_markers: true,
        attachment_upload: true,
        attachment_download: true,
        message_search: true,
        conversation_search: true,
        user_directory: true,
        conversation_creation: true,
        conversation_updating: true,
        membership_management: true,
        rich_content: true,
        max_message_bytes: None,
        max_attachment_bytes: None,
        event_replay: true,
        idempotent_send: true,
    }
}

fn default_user(backend: &BackendId) -> User {
    User {
        id: UserId::new(backend.clone(), "me"),
        handle: Some("me".to_owned()),
        display_name: "Mock User".to_owned(),
        avatar_url: None,
        is_bot: false,
        raw: None,
    }
}

#[non_exhaustive]
#[derive(Clone, Debug, PartialEq, Eq)]
#[allow(clippy::large_enum_variant)]
pub enum Call {
    CurrentUser,
    ListConversations(ConversationQuery),
    GetConversation(ConversationId),
    ListMessages(MessageQuery),
    GetMessage(MessageId),
    SendMessage(MessageDraft),
    Subscribe(EventFilter),
    ListThreads {
        conversation: ConversationId,
        page: PageRequest,
    },
    GetThread(ThreadId),
    CreateThread(NewThread),
    EditMessage {
        id: MessageId,
        edit: MessageEdit,
    },
    DeleteMessage(MessageId),
    AddReaction {
        message: MessageId,
        key: ReactionKey,
    },
    RemoveReaction {
        message: MessageId,
        key: ReactionKey,
    },
    ListReactions {
        message: MessageId,
        page: PageRequest,
    },
    Upload(Upload),
    GetAttachment(AttachmentId),
    Download(AttachmentId),
    SearchMessages(MessageSearchQuery),
    SearchConversations(ConversationSearchQuery),
    ListUsers(UserQuery),
    GetUser(UserId),
    CreateConversation(NewConversation),
    UpdateConversation {
        id: ConversationId,
        patch: ConversationPatch,
    },
    AddMembers {
        conversation: ConversationId,
        members: Vec<UserId>,
    },
    RemoveMembers {
        conversation: ConversationId,
        members: Vec<UserId>,
    },
    JoinConversation(ConversationId),
    LeaveConversation(ConversationId),
    SetTyping {
        conversation: ConversationId,
        thread: Option<ThreadId>,
        state: TypingState,
    },
    GetPresence(Vec<UserId>),
    MarkRead {
        conversation: ConversationId,
        through: Option<MessageId>,
    },
    GetReadMarker {
        conversation: ConversationId,
        user: UserId,
    },
}

#[derive(Debug)]
pub struct MockMessengerBuilder {
    backend: BackendId,
    capabilities: Capabilities,
    current_user: Option<User>,
}

impl Default for MockMessengerBuilder {
    fn default() -> Self {
        Self {
            backend: BackendId::new("mock"),
            capabilities: full_capabilities(),
            current_user: None,
        }
    }
}

impl MockMessengerBuilder {
    pub fn backend(mut self, backend: impl Into<BackendId>) -> Self {
        self.backend = backend.into();
        self
    }

    pub fn capabilities(mut self, capabilities: Capabilities) -> Self {
        self.capabilities = capabilities;
        self
    }

    pub fn current_user(mut self, user: User) -> Self {
        self.current_user = Some(user);
        self
    }

    pub fn build(self) -> MockMessenger {
        let current_user = self
            .current_user
            .unwrap_or_else(|| default_user(&self.backend));
        MockMessenger {
            info: BackendInfo::new(self.backend.clone(), self.capabilities),
            state: Mutex::new(MockState {
                current_user,
                counters: Counters::default(),
                users: Vec::new(),
                conversations: Vec::new(),
                threads: Vec::new(),
                messages: Vec::new(),
                attachments: Vec::new(),
                reactions: Vec::new(),
                presence: Vec::new(),
                read_markers: Vec::new(),
            }),
            calls: Mutex::new(Vec::new()),
            failures: Mutex::new(VecDeque::new()),
            sequence: Mutex::new(0),
            events: EventHub::default(),
        }
    }
}

#[derive(Debug, Default)]
struct Counters {
    conversation: u64,
    thread: u64,
    message: u64,
    attachment: u64,
}

#[derive(Debug)]
struct MockState {
    current_user: User,
    counters: Counters,
    users: Vec<User>,
    conversations: Vec<Conversation>,
    threads: Vec<Thread>,
    messages: Vec<Message>,
    attachments: Vec<StoredAttachment>,
    reactions: Vec<StoredReaction>,
    presence: Vec<Presence>,
    read_markers: Vec<ReadMarkerRecord>,
}

#[derive(Debug)]
struct StoredAttachment {
    attachment: Attachment,
    data: Vec<u8>,
}

#[derive(Debug)]
struct StoredReaction {
    message: MessageId,
    reaction: Reaction,
}

#[derive(Debug)]
struct ReadMarkerRecord {
    conversation: ConversationId,
    marker: ReadMarker,
}

pub struct MockMessenger {
    info: BackendInfo,
    state: Mutex<MockState>,
    calls: Mutex<Vec<Call>>,
    failures: Mutex<VecDeque<Error>>,
    sequence: Mutex<u64>,
    events: EventHub,
}

impl Default for MockMessenger {
    fn default() -> Self {
        Self::new()
    }
}

impl MockMessenger {
    pub fn new() -> Self {
        MockMessengerBuilder::default().build()
    }

    pub fn builder() -> MockMessengerBuilder {
        MockMessengerBuilder::default()
    }

    pub fn backend(&self) -> &BackendId {
        self.info.backend()
    }

    pub fn user_id(&self, raw: impl Into<Arc<str>>) -> UserId {
        UserId::new(self.info.backend.clone(), raw)
    }

    pub fn conversation_id(&self, raw: impl Into<Arc<str>>) -> ConversationId {
        ConversationId::new(self.info.backend.clone(), raw)
    }

    pub fn thread_id(&self, raw: impl Into<Arc<str>>) -> ThreadId {
        ThreadId::new(self.info.backend.clone(), raw)
    }

    pub fn message_id(&self, raw: impl Into<Arc<str>>) -> MessageId {
        MessageId::new(self.info.backend.clone(), raw)
    }

    pub fn attachment_id(&self, raw: impl Into<Arc<str>>) -> AttachmentId {
        AttachmentId::new(self.info.backend.clone(), raw)
    }

    pub fn cursor(&self, raw: impl Into<Arc<str>>) -> Cursor {
        Cursor::new(self.info.backend.clone(), raw)
    }

    pub fn set_current_user(&self, user: User) {
        lock(&self.state).current_user = user;
    }

    pub fn add_user(&self, user: User) -> UserId {
        let id = user.id.clone();
        let mut state = lock(&self.state);
        state.users.retain(|existing| existing.id != user.id);
        state.users.push(user);
        id
    }

    pub fn add_conversation(&self, conversation: Conversation) -> ConversationId {
        let id = conversation.id.clone();
        let mut state = lock(&self.state);
        state
            .conversations
            .retain(|existing| existing.id != conversation.id);
        state.conversations.push(conversation);
        id
    }

    pub fn add_thread(&self, thread: Thread) -> ThreadId {
        let id = thread.id.clone();
        let mut state = lock(&self.state);
        state.threads.retain(|existing| existing.id != thread.id);
        state.threads.push(thread);
        id
    }

    pub fn add_message(&self, message: Message) -> MessageId {
        let id = message.id.clone();
        let mut state = lock(&self.state);
        state.messages.retain(|existing| existing.id != message.id);
        state.messages.push(message);
        id
    }

    pub fn set_presence(&self, presence: Presence) {
        let mut state = lock(&self.state);
        state
            .presence
            .retain(|existing| existing.user.id != presence.user.id);
        state.presence.push(presence);
    }

    pub fn calls(&self) -> Vec<Call> {
        lock(&self.calls).clone()
    }

    pub fn last_call(&self) -> Option<Call> {
        lock(&self.calls).last().cloned()
    }

    pub fn call_count(&self) -> usize {
        lock(&self.calls).len()
    }

    pub fn clear_calls(&self) {
        lock(&self.calls).clear();
    }

    pub fn fail_next(&self, error: Error) {
        lock(&self.failures).push_back(error);
    }

    pub fn clear_failures(&self) {
        lock(&self.failures).clear();
    }

    pub fn publish(&self, event: Event) -> EventEnvelope {
        let mut sequence = lock(&self.sequence);
        *sequence += 1;
        let envelope = EventEnvelope {
            backend: self.info.backend.clone(),
            sequence: Some(Cursor::new(self.info.backend.clone(), sequence.to_string())),
            timestamp: Timestamp::now(),
            event,
        };
        drop(sequence);
        self.events.publish(envelope.clone());
        envelope
    }

    pub fn publish_envelope(&self, envelope: EventEnvelope) {
        self.events.publish(envelope);
    }

    fn record(&self, call: Call) {
        lock(&self.calls).push(call);
    }

    fn check_failure(&self) -> Result<()> {
        match lock(&self.failures).pop_front() {
            Some(error) => Err(error),
            None => Ok(()),
        }
    }

    fn paginate<T>(&self, items: Vec<T>, request: &PageRequest) -> Page<T> {
        let total = items.len();
        let offset = request
            .cursor
            .as_ref()
            .and_then(|cursor| cursor.as_str().parse::<usize>().ok())
            .unwrap_or(0);
        let limit = request
            .limit
            .map(|limit| usize::from(limit.get()))
            .unwrap_or(usize::MAX);
        let items: Vec<T> = items.into_iter().skip(offset).take(limit).collect();
        let next_offset = offset.saturating_add(items.len());
        let next = (next_offset < total)
            .then(|| Cursor::new(self.info.backend.clone(), next_offset.to_string()));
        Page::new(items, next)
    }

    fn store_upload(&self, state: &mut MockState, upload: Upload) -> Attachment {
        state.counters.attachment += 1;
        let id = AttachmentId::new(
            self.info.backend.clone(),
            format!("attachment-{}", state.counters.attachment),
        );
        let attachment = Attachment {
            id: id.clone(),
            filename: upload.filename,
            media_type: upload.media_type,
            size: upload.data.len() as u64,
            url: Some(format!("mock://attachments/{}", id.as_str())),
            expires_at: None,
            raw: None,
        };
        state.attachments.push(StoredAttachment {
            attachment: attachment.clone(),
            data: upload.data,
        });
        attachment
    }
}

fn summary_for(state: &MockState, id: &UserId) -> UserSummary {
    if let Some(user) = state.users.iter().find(|user| &user.id == id) {
        return user.summary();
    }
    if &state.current_user.id == id {
        return state.current_user.summary();
    }
    UserSummary {
        id: id.clone(),
        handle: None,
        display_name: id.as_str().to_owned(),
        avatar_url: None,
        is_bot: false,
    }
}

impl Messenger for MockMessenger {
    fn info(&self) -> &BackendInfo {
        &self.info
    }

    fn current_user(&self) -> BoxFuture<'_, Result<User>> {
        self.record(Call::CurrentUser);
        Box::pin(async move {
            self.check_failure()?;
            Ok(lock(&self.state).current_user.clone())
        })
    }

    fn list_conversations(
        &self,
        query: ConversationQuery,
    ) -> BoxFuture<'_, Result<Page<Conversation>>> {
        self.record(Call::ListConversations(query.clone()));
        Box::pin(async move {
            self.check_failure()?;
            let items = {
                let state = lock(&self.state);
                let mut items: Vec<Conversation> = state
                    .conversations
                    .iter()
                    .filter(|conversation| query.include_archived || !conversation.archived)
                    .filter(|conversation| {
                        query
                            .kinds
                            .as_ref()
                            .is_none_or(|kinds| kinds.contains(&conversation.kind))
                    })
                    .cloned()
                    .collect();
                items.sort_by_key(|item| std::cmp::Reverse(item.last_activity));
                items
            };
            Ok(self.paginate(items, &query.page))
        })
    }

    fn get_conversation(&self, id: ConversationId) -> BoxFuture<'_, Result<Conversation>> {
        self.record(Call::GetConversation(id.clone()));
        Box::pin(async move {
            self.check_failure()?;
            let conversation = lock(&self.state)
                .conversations
                .iter()
                .find(|conversation| conversation.id == id)
                .cloned();
            conversation.ok_or_else(|| Error::NotFound {
                entity: EntityRef::Conversation(id),
            })
        })
    }

    fn list_messages(&self, query: MessageQuery) -> BoxFuture<'_, Result<Page<Message>>> {
        self.record(Call::ListMessages(query.clone()));
        Box::pin(async move {
            self.check_failure()?;
            let items = {
                let state = lock(&self.state);
                let mut items: Vec<Message> = state
                    .messages
                    .iter()
                    .filter(|message| message.conversation == query.conversation)
                    .filter(|message| match &query.scope {
                        MessageScope::Root => message.thread.is_none(),
                        MessageScope::Timeline => true,
                        MessageScope::Thread(thread) => message.thread.as_ref() == Some(thread),
                    })
                    .filter(|message| query.before.is_none_or(|before| message.sent_at < before))
                    .filter(|message| query.after.is_none_or(|after| message.sent_at > after))
                    .cloned()
                    .collect();
                items.sort_by(|a, b| match query.order {
                    Order::NewestFirst => b.sent_at.cmp(&a.sent_at),
                    Order::OldestFirst => a.sent_at.cmp(&b.sent_at),
                });
                items
            };
            Ok(self.paginate(items, &query.page))
        })
    }

    fn get_message(&self, id: MessageId) -> BoxFuture<'_, Result<Message>> {
        self.record(Call::GetMessage(id.clone()));
        Box::pin(async move {
            self.check_failure()?;
            let message = lock(&self.state)
                .messages
                .iter()
                .find(|message| message.id == id)
                .cloned();
            message.ok_or_else(|| Error::NotFound {
                entity: EntityRef::Message(id),
            })
        })
    }

    fn send_message(&self, draft: MessageDraft) -> BoxFuture<'_, Result<Message>> {
        self.record(Call::SendMessage(draft.clone()));
        Box::pin(async move {
            self.check_failure()?;
            let message = {
                let mut state = lock(&self.state);
                let mut attachments = Vec::with_capacity(draft.attachments.len());
                for attachment in &draft.attachments {
                    match attachment {
                        AttachmentDraft::Existing(id) => {
                            let found = state
                                .attachments
                                .iter()
                                .find(|stored| &stored.attachment.id == id)
                                .map(|stored| stored.attachment.clone());
                            match found {
                                Some(attachment) => attachments.push(attachment),
                                None => {
                                    return Err(Error::NotFound {
                                        entity: EntityRef::Attachment(id.clone()),
                                    });
                                }
                            }
                        }
                        AttachmentDraft::Upload(upload) => {
                            attachments.push(self.store_upload(&mut state, upload.clone()));
                        }
                    }
                }
                state.counters.message += 1;
                let message = Message {
                    id: MessageId::new(
                        self.info.backend.clone(),
                        format!("message-{}", state.counters.message),
                    ),
                    conversation: draft.conversation.clone(),
                    thread: draft.thread.clone(),
                    author: state.current_user.summary(),
                    content: draft.content.clone(),
                    attachments,
                    mentions: draft.mentions.clone(),
                    reply_to: draft.reply_to.clone(),
                    sent_at: Timestamp::now(),
                    edited_at: None,
                    deleted: false,
                    raw: draft.raw.clone(),
                };
                state.messages.push(message.clone());
                message
            };
            self.publish(Event::MessageCreated(message.clone()));
            Ok(message)
        })
    }

    fn subscribe(&self, filter: EventFilter) -> BoxFuture<'_, Result<Subscription>> {
        self.record(Call::Subscribe(filter.clone()));
        Box::pin(async move {
            self.check_failure()?;
            Ok(Subscription::new(self.events.subscribe(filter)))
        })
    }

    fn threads(&self) -> Option<&dyn ThreadService> {
        Some(self)
    }

    fn message_editing(&self) -> Option<&dyn MessageEditingService> {
        Some(self)
    }

    fn message_deletion(&self) -> Option<&dyn MessageDeletionService> {
        Some(self)
    }

    fn reactions(&self) -> Option<&dyn ReactionService> {
        Some(self)
    }

    fn attachments(&self) -> Option<&dyn AttachmentService> {
        Some(self)
    }

    fn search(&self) -> Option<&dyn SearchService> {
        Some(self)
    }

    fn directory(&self) -> Option<&dyn DirectoryService> {
        Some(self)
    }

    fn conversation_admin(&self) -> Option<&dyn ConversationAdminService> {
        Some(self)
    }

    fn typing(&self) -> Option<&dyn TypingService> {
        Some(self)
    }

    fn presence(&self) -> Option<&dyn PresenceService> {
        Some(self)
    }

    fn read_markers(&self) -> Option<&dyn ReadMarkerService> {
        Some(self)
    }
}
