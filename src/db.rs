use futures_util::StreamExt;
use mongodb::bson::{doc, Document};
use mongodb::results::UpdateResult;
use mongodb::Collection;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

macro_rules! find_one {
    ($cursor:expr, $filter:expr) => {{
        $cursor
            .find($filter, None)
            .await?
            .filter_map(|a| async move { a.ok() })
            .collect()
            .await
    }};
}
macro_rules! collect {
    ($cursor:expr) => {{
        $cursor
            .filter_map(|a| async move { a.ok() })
            .collect()
            .await
    }};
}

#[derive(Debug, Clone)]
struct Data {
    channels: Arc<Collection<Channel>>,
    users: Arc<Collection<User>>,
    messages: Arc<Collection<Message>>,
}

impl Data {
    async fn establish(
        &self,
        user_id: String,
    ) -> Result<(Vec<Channel>, Vec<Message>), anyhow::Error> {
        let channels = self.channels.find(doc!("members": user_id), None).await?;


        let mut channel_ids = Vec::new();
        while let Ok(Channel {
            is_self: false,
            _id,
            ..
        }) = channels.deserialize_current()
        {
            channel_ids.push(doc! { "$eq": _id })
        }

        let messages = find_one!(
            self.messages,
            doc! {
                "channel": doc! {
                    "$or": channel_ids,
                }
            }
        );
        Ok((collect!(channels), messages))
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
struct Channel {
    _id: String,
    is_self: bool,
    members: Vec<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
struct User {
    _id: String,
    username: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
struct Message {
    _id: String,
    content: String,
    reply: Option<String>,
    channel: String,
}
