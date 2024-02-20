use crate::structures::is_false;
use crate::structures::is_none;
use crate::structures::models::{Channel, UserSafe};
use axum::extract::ws::Message;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
#[serde(tag = "action")]
pub enum EventEnum {
    Establish {
        channels: Vec<Channel>,
        users: Vec<UserSafe>,
        version: String,
    },
    MessageSend {
        id: String,
        author: String,
        content: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        reply: Option<String>,
        channel: String,
    },
    MessageEdit {
        id: String,
        content: String,
    },
    MessageDelete {
        id: String,
    },
    TypeStatus {
        #[serde(skip_serializing_if = "is_false")]
        typing: bool,
        channel: String,
        user: String,
    },
    Pong {
        #[serde(skip_serializing_if = "is_none")]
        data: Option<usize>,
    },
    #[default]
    InvalidEvent,
}
impl From<EventEnum> for Message {
    fn from(value: EventEnum) -> Self {
        Message::from(serde_json::to_string(&value).unwrap())
    }
}
