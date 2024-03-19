#[cfg(feature = "server")]
use axum::extract::ws::Message;
use crate::structures::{
    is_none_bool,
    models::{Channel, UserSafe},
};
use serde::{Deserialize, Serialize};
/// # Event.
/// Event data is sent from Server to Client.
#[derive(Serialize, Debug, Clone, Deserialize)]
#[serde(tag = "action")]
pub enum EventEnum {
    Establish {
        channels: Vec<Channel>,
        users: Vec<UserSafe>,
        messages: Vec<crate::structures::models::Message>,
        you: String,
        version: String,
    },
    MessageSend {
        id: String,
        author: String,
        content: String,
        // todo pending implementation, a part of 0.1.1
        // #[serde(skip_serializing_if = "Option::is_none")]
        // reply: Option<String>,
        channel: String,
    },
    // todo pending implementation
    // MessageEdit {
    //     id: String,
    //     content: String,
    // },
    // MessageDelete {
    //     id: String,
    // },
    TypeStatus {
        #[serde(skip_serializing_if = "is_none_bool")]
        typing: Option<bool>,
        channel: String,
        author: String,
    },
    Pong {
        #[serde(skip_serializing_if = "Option::is_none")]
        data: Option<usize>,
    },

}

impl EventEnum {
    pub fn message_send(self) -> Option<crate::structures::models::Message> {
        match self {
            EventEnum::MessageSend {
                id,
                author,
                content,
                channel,
            } => Some(crate::structures::models::Message {
                id,
                author,
                content,
                channel,
            }),
            _ => None,
        }
    }
}
#[cfg(feature = "server")]
impl From<EventEnum> for Message {
    fn from(value: EventEnum) -> Self {
        Message::from(serde_json::to_string(&value).unwrap())
    }
}
