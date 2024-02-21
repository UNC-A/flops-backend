use serde::{Deserialize, Serialize};
use tokio_tungstenite::tungstenite::Message;
use crate::structures::websocket::if_false;
use crate::structures::websocket::is_none;

#[derive(Serialize, Deserialize)]
#[serde(tag = "action")]
pub enum EventEnum {
    Establish {
        messages: Vec<String>,
    },
    MessageSend {
        id: String,
        message: String,
        timestamp: u128,
    },
    MessageEdit {
        id: String,
        content: String,
    },
    MessageDelete {
        id: String,
    },
    TypeStatus {
        #[serde(skip_serializing_if = "if_false")]
        typing: bool,
        channel: String,
        user: String,
    },
    Pong {
        #[serde(skip_serializing_if = "is_none")]
        data: Option<usize>,
    },
}
impl From<EventEnum> for Message {
    fn from(value: EventEnum) -> Self {
        Message::from(serde_json::to_string(&value).unwrap())
    }
}
