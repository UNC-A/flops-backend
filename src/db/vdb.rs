use crate::structures::websocket::events::EventEnum;
use indexmap::IndexSet;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
/// # State
/// The Volatile Database, data is stored in memory and very quickly read / written to.
/// This DB stores pending events and online users.
#[derive(Debug, Default, Clone)]
pub struct State {
    pub pending_messages: Arc<RwLock<Vec<EventMessage>>>,
    pub online_users: Arc<RwLock<IndexSet<String>>>,
    /// User ID and channel ID
    pub type_status: Arc<RwLock<HashMap<String, String>>>,
}
/// ## Event Message
/// Contains a list of applicable targets, the original author and the message.
#[derive(Debug, Default, Clone)]
pub struct EventMessage {
    pub author: String,
    pub targets: IndexSet<String>,
    pub item: EventEnum,
}

/// # Abstract Methods.
/// Intended for use within websocket system.
impl State {
    /// Two part function: update database that the user is online, and reject simultaneous account
    /// connections.
    /// If false, user is already present.
    pub async fn user_online(&self, user_id: impl Into<String>) -> bool {
        self.remove_dead_messages().await;
        self.online_users.write().await.insert(user_id.into())
    }
    /// Set a User as offline
    pub async fn user_offline(&self, user_id: impl Into<String>) {
        let user_id = user_id.into();
        self.online_users.write().await.shift_remove(&user_id);
        self.user_remove_message(&user_id).await;
        self.remove_dead_messages().await;
        self.type_status_remove(user_id).await;
    }
    /// Get all messages applicable for a user based on User ID.
    /// Removes User ID from 'target' so that messages are not double-sent.
    /// Removes 'dead' messages (those without any targets).
    pub async fn get_message(&self, user_id: impl Into<String>) -> Option<Vec<EventMessage>> {
        let user_id = user_id.into();
        // If user offline: return
        // This condition should be impossible
        if self.online_users.read().await.get(&user_id).is_none() {
            //self.user_remove_message(&user_id).await;
            //return None;
            unreachable!()
        };
        // Get all items applicable to the User.
        let item = self
            .pending_messages
            .read()
            .await
            .clone()
            .into_iter()
            .filter(|a| a.targets.get(&user_id).is_some())
            .collect::<Vec<EventMessage>>();

        // if there are  no items: return None.
        if item.is_empty() {
            return None;
        };
        // Since all messages have been forwarded to User: remove User from targets.
        self.user_remove_message(user_id).await;
        self.remove_dead_messages().await;
        Some(item)
    }

    /// Safely adds a message to the top of the message queue.
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
    pub async fn type_status_add(&self, user_id: impl Into<String>, channel_id: impl Into<String>) {
        self.type_status
            .write()
            .await
            .insert(user_id.into(), channel_id.into());
    }
    /// If there is no current TypeStatus by User: return None and insert new TypeStatus.
    /// If there is a current one: replace it and send that existing Channel as Some.
    pub async fn type_status_replace(
        &self,
        user_id: impl Into<String>,
        channel_id: impl Into<String>,
    ) -> Option<String> {
        let user_id = user_id.into();
        let data = self.type_status_get(&user_id).await;
        self.type_status_add(&user_id, channel_id).await;
        data
    }
}
/// # Internal Methods.
/// NOT intended for use outside vdb internal processes.
impl State {
    /// Removes all messages that do not have targets.
    async fn remove_dead_messages(&self) {
        let data = self.pending_messages.read().await.clone();
        *self.pending_messages.write().await =
            data.into_iter().filter(|a| !a.targets.is_empty()).collect();
    }
    /// Removes User ID from all references in message queue.
    /// Intended for taking a User offline, or marking all messages as already sent.
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
    async fn type_status_remove(&self, user_id: impl Into<String>) {
        self.type_status.write().await.remove(&user_id.into());
    }

    async fn type_status_get(&self, user_id: impl Into<String>) -> Option<String> {
        self.type_status.read().await.get(&user_id.into()).cloned()
    }
}
