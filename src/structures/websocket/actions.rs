use serde::{Deserialize};

#[derive(Deserialize)]
#[serde(tag = "action")]
pub enum ActionEnum {
    Establish,
    Ping {
        data: Option<usize>,
    },
    MessageSend {
        message: String,
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