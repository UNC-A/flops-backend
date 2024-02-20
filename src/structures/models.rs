use crate::structures::is_false;
use indexmap::IndexSet;
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Channel {
    #[serde(rename = "_id")]
    pub id: String,
    #[serde(skip_serializing_if = "is_false", default)]
    pub is_self: bool,
    #[serde(skip_serializing_if = "IndexSet::is_empty", default)]
    pub members: IndexSet<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct User {
    #[serde(rename = "_id")]
    pub id: String,
    pub username: String,
    #[serde(skip_serializing_if = "IndexSet::is_empty", default)]
    pub sessions: IndexSet<String>,
    // stateful tracker for current sessions
    #[serde(skip_serializing_if = "IndexSet::is_empty", default)]
    pub connections: IndexSet<String>,
}

impl From<User> for UserSafe {
    fn from(value: User) -> Self {
        Self {
            id: value.id,
            username: value.username,
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UserSafe {
    #[serde(rename = "_id")]
    pub id: String,
    pub username: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Message {
    #[serde(rename = "_id")]
    pub id: String,
    pub author: String,
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub reply: Option<String>,
    pub channel: String,
}
