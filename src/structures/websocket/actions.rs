use serde::{Deserialize};

#[derive(Deserialize)]
#[serde(tag = "action")]
pub enum ActionEnum {
    Establish,
    Ping {
        data: Option<usize>,
    },
    MessageSend {
        content: String,
        reply: Option<String>,
    },
    MessageEdit {
        message: String,
    },
    MessageDelete {
        message: String,
    },
    TypeStatus {
        typing: bool,
        channel: String,
    },
}