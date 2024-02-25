use crate::structures::websocket::events::EventEnum;
use indexmap::IndexSet;
use std::sync::Arc;
use tokio::sync::RwLock;
#[derive(Debug, Default, Clone)]
pub struct State {
    pub pending_messages: Arc<RwLock<Vec<EventMessage>>>,
    pub online_users: Arc<RwLock<IndexSet<String>>>,
}
#[derive(Debug, Default, Clone)]
pub struct EventMessage {
    pub author: String,
    pub targets: IndexSet<String>,
    pub item: EventEnum,
}

/// abstract methods
impl State {
    /// two part function: update database that the user is online, and reject simultaneous account
    /// connections
    /// if false, user is already present
    pub async fn user_online(&self, user_id: impl Into<String>) -> bool {
        self.remove_dead_messages().await;
        self.online_users.write().await.insert(user_id.into())
    }
    /// set users as offline
    pub async fn user_offline(&self, user_id: impl Into<String>) {
        let user_id = user_id.into();
        self.online_users.write().await.shift_remove(&user_id);
        self.user_remove_message(&user_id).await;
        self.remove_dead_messages().await;
    }

    /// get all messages applicable for a user
    pub async fn get_message(&self, user_id: impl Into<String>) -> Option<Vec<EventMessage>> {
        let user_id = user_id.into();
        // if user offline, return
        if self.online_users.read().await.get(&user_id).is_none() {
            self.user_remove_message(&user_id).await;
            return None;
        };
        // get all items applicable to user
        let item = self
            .pending_messages
            .read()
            .await
            .clone()
            .into_iter()
            .filter(|a| a.targets.get(&user_id).is_some())
            .collect::<Vec<EventMessage>>();

        // if no items, return none
        if item.is_empty() {
            return None;
        };
        // since all messages have been forwarded, delete old from db
        // copy to avoid thread locks
        self.user_remove_message(user_id).await;
        self.remove_dead_messages().await;
        Some(item)
    }

    /// safely adds a message to the top of the message queue
    pub async fn message_add_vdb(&self, mut event_message: EventMessage) {
        let users = &self.online_users.read().await;

        for x in event_message.targets.clone() {
            if users.get(&x).is_none() {
                event_message.targets.shift_remove(&x);
            };
        }
        if !event_message.targets.is_empty() {
            self.pending_messages.write().await.push(event_message);
        };
    }
}
/// internal methods
impl State {
    /// removes all messages that do not have targets
    async fn remove_dead_messages(&self) {
        let data = self.pending_messages.read().await.clone();
        *self.pending_messages.write().await =
            data.into_iter().filter(|a| !a.targets.is_empty()).collect();
    }
    /// removes user_id from all references in message queue
    async fn user_remove_message(&self, user_id: impl Into<String>) {
        let user_id = user_id.into();
        let data = self
            .pending_messages
            .read()
            .await
            .clone()
            .into_iter()
            .map(|mut a| {
                a.targets.shift_remove(&user_id);
                a
            })
            .collect();
        *self.pending_messages.write().await = data;
    }
}
