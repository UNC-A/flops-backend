use serde::{Deserialize, Serialize};

/// # Action.
/// Action data is sent from Client to Server.
#[derive(Serialize, Debug, Clone, Deserialize)]
#[serde(tag = "action")]
pub enum ActionEnum {
    Establish,
    Ping {
        data: Option<usize>,
    },
    MessageSend {
        content: String,
        // todo pending implementation, a part of 0.1.1
        //reply: Option<String>,
        channel: String,
    },
    // todo pending implementation, a part of 0.1.1
    // MessageEdit {
    //     message: String,
    //     channel: String,
    //     content: String,
    // },
    // MessageDelete {
    //     message: String,
    //     channel: String,
    // },
    TypeStatus {
        typing: Option<bool>,
        channel: String,
    },
}
