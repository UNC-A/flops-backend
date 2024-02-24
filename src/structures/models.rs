use indexmap::IndexSet;
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Channel {
    #[serde(rename = "_id")]
    pub id: String,
    // todo not yet implemented, a part of 0.1.1
    //#[serde(skip_serializing_if = "is_false", default)]
    //pub is_self: bool,
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
  // todo not yet implemented, a part of 0.1.1
  //  #[serde(skip_serializing_if = "Option::is_none", default)]
  //  pub reply: Option<String>,
    pub channel: String,
}
