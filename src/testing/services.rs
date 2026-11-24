use super::*;

impl ThreadService for MockMessenger {
    fn list_threads(
        &self,
        conversation: ConversationId,
        page: PageRequest,
    ) -> BoxFuture<'_, Result<Page<Thread>>> {
        self.record(Call::ListThreads {
            conversation: conversation.clone(),
            page: page.clone(),
        });
        Box::pin(async move {
            self.check_failure()?;
            let items = {
                let state = lock(&self.state);
                let mut items: Vec<Thread> = state
                    .threads
                    .iter()
                    .filter(|thread| thread.conversation == conversation)
                    .cloned()
                    .collect();
                items.sort_by_key(|item| std::cmp::Reverse(item.last_activity));
                items
            };
            Ok(self.paginate(items, &page))
        })
    }

    fn get_thread(&self, id: ThreadId) -> BoxFuture<'_, Result<Thread>> {
        self.record(Call::GetThread(id.clone()));
        Box::pin(async move {
            self.check_failure()?;
            let thread = lock(&self.state)
                .threads
                .iter()
                .find(|thread| thread.id == id)
                .cloned();
            thread.ok_or_else(|| Error::NotFound {
                entity: EntityRef::Thread(id),
            })
        })
    }

    fn create_thread(&self, draft: NewThread) -> BoxFuture<'_, Result<Thread>> {
        self.record(Call::CreateThread(draft.clone()));
        Box::pin(async move {
            self.check_failure()?;
            let thread = {
                let mut state = lock(&self.state);
                state.counters.thread += 1;
                let now = Timestamp::now();
                let thread = Thread {
                    id: ThreadId::new(
                        self.info.backend.clone(),
                        format!("thread-{}", state.counters.thread),
                    ),
                    conversation: draft.conversation.clone(),
                    title: draft.title.clone(),
                    anchor: draft.anchor.clone(),
                    reply_count: Some(0),
                    created_at: Some(now),
                    last_activity: Some(now),
                    raw: None,
                };
                state.threads.push(thread.clone());
                thread
            };
            self.publish(Event::ThreadCreated(thread.clone()));
            Ok(thread)
        })
    }
}

impl MessageEditingService for MockMessenger {
    fn edit_message(&self, id: MessageId, edit: MessageEdit) -> BoxFuture<'_, Result<Message>> {
        self.record(Call::EditMessage {
            id: id.clone(),
            edit: edit.clone(),
        });
        Box::pin(async move {
            self.check_failure()?;
            let message = {
                let mut state = lock(&self.state);
                let message = state
                    .messages
                    .iter_mut()
                    .find(|message| message.id == id)
                    .ok_or_else(|| Error::NotFound {
                        entity: EntityRef::Message(id.clone()),
                    })?;
                message.content = edit.content.clone();
                message.mentions = edit.mentions.clone();
                message.edited_at = Some(Timestamp::now());
                message.clone()
            };
            self.publish(Event::MessageUpdated(message.clone()));
            Ok(message)
        })
    }
}

impl MessageDeletionService for MockMessenger {
    fn delete_message(&self, id: MessageId) -> BoxFuture<'_, Result<()>> {
        self.record(Call::DeleteMessage(id.clone()));
        Box::pin(async move {
            self.check_failure()?;
            let event = {
                let mut state = lock(&self.state);
                let message = state
                    .messages
                    .iter_mut()
                    .find(|message| message.id == id)
                    .ok_or_else(|| Error::NotFound {
                        entity: EntityRef::Message(id.clone()),
                    })?;
                message.deleted = true;
                Event::MessageDeleted {
                    conversation: message.conversation.clone(),
                    thread: message.thread.clone(),
                    message: message.id.clone(),
                }
            };
            self.publish(event);
            Ok(())
        })
    }
}

impl ReactionService for MockMessenger {
    fn add_reaction(&self, message: MessageId, key: ReactionKey) -> BoxFuture<'_, Result<()>> {
        self.record(Call::AddReaction {
            message: message.clone(),
            key: key.clone(),
        });
        Box::pin(async move {
            self.check_failure()?;
            let event = {
                let mut state = lock(&self.state);
                let target = state
                    .messages
                    .iter()
                    .find(|target| target.id == message)
                    .ok_or_else(|| Error::NotFound {
                        entity: EntityRef::Message(message.clone()),
                    })?;
                let conversation = target.conversation.clone();
                let thread = target.thread.clone();
                let user = state.current_user.summary();
                if let Some(record) = state
                    .reactions
                    .iter_mut()
                    .find(|record| record.message == message && record.reaction.key == key)
                {
                    record.reaction.count += 1;
                    record.reaction.reacted_by_current_user = true;
                    if let Some(users) = &mut record.reaction.users
                        && !users.iter().any(|existing| existing.id == user.id)
                    {
                        users.push(user.clone());
                    }
                } else {
                    state.reactions.push(StoredReaction {
                        message: message.clone(),
                        reaction: Reaction {
                            key: key.clone(),
                            count: 1,
                            users: None,
                            reacted_by_current_user: true,
                        },
                    });
                }
                Event::ReactionChanged {
                    conversation,
                    thread,
                    message: message.clone(),
                    key: key.clone(),
                    user,
                    change: ReactionChange::Added,
                }
            };
            self.publish(event);
            Ok(())
        })
    }

    fn remove_reaction(&self, message: MessageId, key: ReactionKey) -> BoxFuture<'_, Result<()>> {
        self.record(Call::RemoveReaction {
            message: message.clone(),
            key: key.clone(),
        });
        Box::pin(async move {
            self.check_failure()?;
            let event = {
                let mut state = lock(&self.state);
                let target = state
                    .messages
                    .iter()
                    .find(|target| target.id == message)
                    .ok_or_else(|| Error::NotFound {
                        entity: EntityRef::Message(message.clone()),
                    })?;
                let conversation = target.conversation.clone();
                let thread = target.thread.clone();
                let user = state.current_user.summary();
                let position = state
                    .reactions
                    .iter()
                    .position(|record| record.message == message && record.reaction.key == key);
                let Some(position) = position else {
                    return Ok(());
                };
                let record = &mut state.reactions[position];
                record.reaction.count = record.reaction.count.saturating_sub(1);
                record.reaction.reacted_by_current_user = false;
                if let Some(users) = &mut record.reaction.users {
                    users.retain(|existing| existing.id != user.id);
                }
                if record.reaction.count == 0 {
                    state.reactions.remove(position);
                }
                Event::ReactionChanged {
                    conversation,
                    thread,
                    message,
                    key,
                    user,
                    change: ReactionChange::Removed,
                }
            };
            self.publish(event);
            Ok(())
        })
    }

    fn list_reactions(
        &self,
        message: MessageId,
        page: PageRequest,
    ) -> BoxFuture<'_, Result<Page<Reaction>>> {
        self.record(Call::ListReactions {
            message: message.clone(),
            page: page.clone(),
        });
        Box::pin(async move {
            self.check_failure()?;
            let items = {
                let state = lock(&self.state);
                state
                    .reactions
                    .iter()
                    .filter(|record| record.message == message)
                    .map(|record| record.reaction.clone())
                    .collect()
            };
            Ok(self.paginate(items, &page))
        })
    }
}

impl AttachmentService for MockMessenger {
    fn upload(&self, upload: Upload) -> BoxFuture<'_, Result<Attachment>> {
        self.record(Call::Upload(upload.clone()));
        Box::pin(async move {
            self.check_failure()?;
            let attachment = {
                let mut state = lock(&self.state);
                self.store_upload(&mut state, upload)
            };
            Ok(attachment)
        })
    }

    fn get_attachment(&self, id: AttachmentId) -> BoxFuture<'_, Result<Attachment>> {
        self.record(Call::GetAttachment(id.clone()));
        Box::pin(async move {
            self.check_failure()?;
            let attachment = lock(&self.state)
                .attachments
                .iter()
                .find(|stored| stored.attachment.id == id)
                .map(|stored| stored.attachment.clone());
            attachment.ok_or_else(|| Error::NotFound {
                entity: EntityRef::Attachment(id),
            })
        })
    }

    fn download(&self, id: AttachmentId) -> BoxFuture<'_, Result<DownloadedAttachment>> {
        self.record(Call::Download(id.clone()));
        Box::pin(async move {
            self.check_failure()?;
            let attachment = lock(&self.state)
                .attachments
                .iter()
                .find(|stored| stored.attachment.id == id)
                .map(|stored| DownloadedAttachment {
                    attachment: stored.attachment.clone(),
                    data: stored.data.clone(),
                });
            attachment.ok_or_else(|| Error::NotFound {
                entity: EntityRef::Attachment(id),
            })
        })
    }
}

impl SearchService for MockMessenger {
    fn search_messages(&self, query: MessageSearchQuery) -> BoxFuture<'_, Result<Page<Message>>> {
        self.record(Call::SearchMessages(query.clone()));
        Box::pin(async move {
            self.check_failure()?;
            let needle = query.text.to_lowercase();
            let items = {
                let state = lock(&self.state);
                let mut items: Vec<Message> = state
                    .messages
                    .iter()
                    .filter(|message| !message.deleted)
                    .filter(|message| message.content.plain.to_lowercase().contains(&needle))
                    .filter(|message| {
                        query.conversations.as_ref().is_none_or(|conversations| {
                            conversations.contains(&message.conversation)
                        })
                    })
                    .filter(|message| {
                        query
                            .thread
                            .as_ref()
                            .is_none_or(|thread| message.thread.as_ref() == Some(thread))
                    })
                    .filter(|message| {
                        query
                            .authors
                            .as_ref()
                            .is_none_or(|authors| authors.contains(&message.author.id))
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

    fn search_conversations(
        &self,
        query: ConversationSearchQuery,
    ) -> BoxFuture<'_, Result<Page<Conversation>>> {
        self.record(Call::SearchConversations(query.clone()));
        Box::pin(async move {
            self.check_failure()?;
            let needle = query.text.to_lowercase();
            let items = {
                let state = lock(&self.state);
                state
                    .conversations
                    .iter()
                    .filter(|conversation| {
                        conversation
                            .title
                            .as_deref()
                            .unwrap_or("")
                            .to_lowercase()
                            .contains(&needle)
                    })
                    .filter(|conversation| query.include_archived || !conversation.archived)
                    .filter(|conversation| {
                        query
                            .kinds
                            .as_ref()
                            .is_none_or(|kinds| kinds.contains(&conversation.kind))
                    })
                    .cloned()
                    .collect()
            };
            Ok(self.paginate(items, &query.page))
        })
    }
}

impl DirectoryService for MockMessenger {
    fn list_users(&self, query: UserQuery) -> BoxFuture<'_, Result<Page<User>>> {
        self.record(Call::ListUsers(query.clone()));
        Box::pin(async move {
            self.check_failure()?;
            let needle = query.text.as_deref().map(str::to_lowercase);
            let items = {
                let state = lock(&self.state);
                let mut items: Vec<User> = state
                    .users
                    .iter()
                    .filter(|user| {
                        needle.as_ref().is_none_or(|needle| {
                            user.display_name.to_lowercase().contains(needle)
                                || user
                                    .handle
                                    .as_deref()
                                    .is_some_and(|handle| handle.to_lowercase().contains(needle))
                        })
                    })
                    .cloned()
                    .collect();
                items.sort_by(|a, b| a.display_name.cmp(&b.display_name));
                items
            };
            Ok(self.paginate(items, &query.page))
        })
    }

    fn get_user(&self, id: UserId) -> BoxFuture<'_, Result<User>> {
        self.record(Call::GetUser(id.clone()));
        Box::pin(async move {
            self.check_failure()?;
            let state = lock(&self.state);
            let user = state
                .users
                .iter()
                .find(|user| user.id == id)
                .cloned()
                .or_else(|| (state.current_user.id == id).then(|| state.current_user.clone()));
            user.ok_or_else(|| Error::NotFound {
                entity: EntityRef::User(id),
            })
        })
    }
}

impl ConversationAdminService for MockMessenger {
    fn create_conversation(&self, draft: NewConversation) -> BoxFuture<'_, Result<Conversation>> {
        self.record(Call::CreateConversation(draft.clone()));
        Box::pin(async move {
            self.check_failure()?;
            let conversation = {
                let mut state = lock(&self.state);
                let members: Vec<UserSummary> = draft
                    .members
                    .iter()
                    .map(|id| summary_for(&state, id))
                    .collect();
                state.counters.conversation += 1;
                let now = Timestamp::now();
                let conversation = Conversation {
                    id: ConversationId::new(
                        self.info.backend.clone(),
                        format!("conversation-{}", state.counters.conversation),
                    ),
                    kind: draft.kind,
                    title: draft.title.clone(),
                    members: Some(members),
                    member_count: Some(draft.members.len() as u32),
                    archived: false,
                    created_at: Some(now),
                    last_activity: Some(now),
                    unread: None,
                    raw: draft.raw.clone(),
                };
                state.conversations.push(conversation.clone());
                conversation
            };
            self.publish(Event::ConversationCreated(conversation.clone()));
            Ok(conversation)
        })
    }

    fn update_conversation(
        &self,
        id: ConversationId,
        patch: ConversationPatch,
    ) -> BoxFuture<'_, Result<Conversation>> {
        self.record(Call::UpdateConversation {
            id: id.clone(),
            patch: patch.clone(),
        });
        Box::pin(async move {
            self.check_failure()?;
            let conversation = {
                let mut state = lock(&self.state);
                let conversation = state
                    .conversations
                    .iter_mut()
                    .find(|conversation| conversation.id == id)
                    .ok_or_else(|| Error::NotFound {
                        entity: EntityRef::Conversation(id.clone()),
                    })?;
                if let Some(title) = patch.title {
                    conversation.title = Some(title);
                }
                if let Some(archived) = patch.archived {
                    conversation.archived = archived;
                }
                conversation.clone()
            };
            self.publish(Event::ConversationUpdated(conversation.clone()));
            Ok(conversation)
        })
    }

    fn add_members(
        &self,
        conversation: ConversationId,
        members: Vec<UserId>,
    ) -> BoxFuture<'_, Result<()>> {
        self.record(Call::AddMembers {
            conversation: conversation.clone(),
            members: members.clone(),
        });
        Box::pin(async move {
            self.check_failure()?;
            let events = {
                let mut state = lock(&self.state);
                let summaries: Vec<UserSummary> =
                    members.iter().map(|id| summary_for(&state, id)).collect();
                let conversation = state
                    .conversations
                    .iter_mut()
                    .find(|existing| existing.id == conversation)
                    .ok_or_else(|| Error::NotFound {
                        entity: EntityRef::Conversation(conversation.clone()),
                    })?;
                let conversation_id = conversation.id.clone();
                let member_list = conversation.members.get_or_insert_with(Vec::new);
                let mut events = Vec::new();
                for user in summaries {
                    if member_list.iter().any(|existing| existing.id == user.id) {
                        continue;
                    }
                    member_list.push(user.clone());
                    events.push(Event::MembershipChanged {
                        conversation: conversation_id.clone(),
                        user,
                        change: MembershipChange::Added,
                    });
                }
                conversation.member_count = Some(member_list.len() as u32);
                events
            };
            for event in events {
                self.publish(event);
            }
            Ok(())
        })
    }

    fn remove_members(
        &self,
        conversation: ConversationId,
        members: Vec<UserId>,
    ) -> BoxFuture<'_, Result<()>> {
        self.record(Call::RemoveMembers {
            conversation: conversation.clone(),
            members: members.clone(),
        });
        Box::pin(async move {
            self.check_failure()?;
            let events = {
                let mut state = lock(&self.state);
                let summaries: Vec<UserSummary> =
                    members.iter().map(|id| summary_for(&state, id)).collect();
                let conversation = state
                    .conversations
                    .iter_mut()
                    .find(|existing| existing.id == conversation)
                    .ok_or_else(|| Error::NotFound {
                        entity: EntityRef::Conversation(conversation.clone()),
                    })?;
                let conversation_id = conversation.id.clone();
                let mut events = Vec::new();
                if let Some(member_list) = &mut conversation.members {
                    for user in summaries {
                        let before = member_list.len();
                        member_list.retain(|member| member.id != user.id);
                        if member_list.len() != before {
                            events.push(Event::MembershipChanged {
                                conversation: conversation_id.clone(),
                                user,
                                change: MembershipChange::Removed,
                            });
                        }
                    }
                    conversation.member_count = Some(member_list.len() as u32);
                }
                events
            };
            for event in events {
                self.publish(event);
            }
            Ok(())
        })
    }

    fn join_conversation(&self, id: ConversationId) -> BoxFuture<'_, Result<()>> {
        self.record(Call::JoinConversation(id.clone()));
        Box::pin(async move {
            self.check_failure()?;
            let event = {
                let mut state = lock(&self.state);
                let user = state.current_user.summary();
                let conversation = state
                    .conversations
                    .iter_mut()
                    .find(|conversation| conversation.id == id)
                    .ok_or_else(|| Error::NotFound {
                        entity: EntityRef::Conversation(id.clone()),
                    })?;
                let conversation_id = conversation.id.clone();
                let member_list = conversation.members.get_or_insert_with(Vec::new);
                if !member_list.iter().any(|member| member.id == user.id) {
                    member_list.push(user.clone());
                }
                conversation.member_count = Some(member_list.len() as u32);
                Event::MembershipChanged {
                    conversation: conversation_id,
                    user,
                    change: MembershipChange::Joined,
                }
            };
            self.publish(event);
            Ok(())
        })
    }

    fn leave_conversation(&self, id: ConversationId) -> BoxFuture<'_, Result<()>> {
        self.record(Call::LeaveConversation(id.clone()));
        Box::pin(async move {
            self.check_failure()?;
            let event = {
                let mut state = lock(&self.state);
                let user = state.current_user.summary();
                let conversation = state
                    .conversations
                    .iter_mut()
                    .find(|conversation| conversation.id == id)
                    .ok_or_else(|| Error::NotFound {
                        entity: EntityRef::Conversation(id.clone()),
                    })?;
                let conversation_id = conversation.id.clone();
                if let Some(member_list) = &mut conversation.members {
                    member_list.retain(|member| member.id != user.id);
                }
                conversation.member_count = conversation
                    .members
                    .as_ref()
                    .map(|members| members.len() as u32);
                Event::MembershipChanged {
                    conversation: conversation_id,
                    user,
                    change: MembershipChange::Left,
                }
            };
            self.publish(event);
            Ok(())
        })
    }
}

impl TypingService for MockMessenger {
    fn set_typing(
        &self,
        conversation: ConversationId,
        thread: Option<ThreadId>,
        state: TypingState,
    ) -> BoxFuture<'_, Result<()>> {
        self.record(Call::SetTyping {
            conversation: conversation.clone(),
            thread: thread.clone(),
            state,
        });
        Box::pin(async move {
            self.check_failure()?;
            let event = {
                let state_lock = lock(&self.state);
                Event::TypingChanged {
                    conversation,
                    thread,
                    user: state_lock.current_user.summary(),
                    state,
                }
            };
            self.publish(event);
            Ok(())
        })
    }
}

impl PresenceService for MockMessenger {
    fn get_presence(&self, users: Vec<UserId>) -> BoxFuture<'_, Result<Vec<Presence>>> {
        self.record(Call::GetPresence(users.clone()));
        Box::pin(async move {
            self.check_failure()?;
            let state = lock(&self.state);
            let presence = users
                .into_iter()
                .map(|id| {
                    state
                        .presence
                        .iter()
                        .find(|presence| presence.user.id == id)
                        .cloned()
                        .unwrap_or_else(|| Presence {
                            user: summary_for(&state, &id),
                            state: PresenceState::Unknown,
                            last_active: None,
                            status_text: None,
                        })
                })
                .collect();
            Ok(presence)
        })
    }
}

impl ReadMarkerService for MockMessenger {
    fn mark_read(
        &self,
        conversation: ConversationId,
        through: Option<MessageId>,
    ) -> BoxFuture<'_, Result<()>> {
        self.record(Call::MarkRead {
            conversation: conversation.clone(),
            through: through.clone(),
        });
        Box::pin(async move {
            self.check_failure()?;
            let event = {
                let mut state = lock(&self.state);
                let user = state.current_user.summary();
                let marker = ReadMarker {
                    user: user.clone(),
                    last_read: through.clone(),
                    updated_at: Some(Timestamp::now()),
                };
                let position = state.read_markers.iter().position(|record| {
                    record.conversation == conversation && record.marker.user.id == user.id
                });
                match position {
                    Some(position) => state.read_markers[position].marker = marker,
                    None => state.read_markers.push(ReadMarkerRecord {
                        conversation: conversation.clone(),
                        marker,
                    }),
                }
                Event::ReadMarkerChanged {
                    conversation,
                    user,
                    last_read: through,
                }
            };
            self.publish(event);
            Ok(())
        })
    }

    fn get_read_marker(
        &self,
        conversation: ConversationId,
        user: UserId,
    ) -> BoxFuture<'_, Result<Option<ReadMarker>>> {
        self.record(Call::GetReadMarker {
            conversation: conversation.clone(),
            user: user.clone(),
        });
        Box::pin(async move {
            self.check_failure()?;
            let marker = lock(&self.state)
                .read_markers
                .iter()
                .find(|record| record.conversation == conversation && record.marker.user.id == user)
                .map(|record| record.marker.clone());
            Ok(marker)
        })
    }
}
