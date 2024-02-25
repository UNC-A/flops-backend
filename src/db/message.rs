use crate::collect;
use crate::db::Data;
use crate::structures::models::Message;
use futures_util::future::join_all;
use futures_util::StreamExt;
use indexmap::IndexSet;
use mongodb::bson::doc;
impl Data {
    /// get one message
    pub async fn message_get_one(
        &self,
        message_id: impl Into<String>,
    ) -> crate::Result<Option<Message>> {
        Ok(self
            .messages
            .find_one(doc!("_id": message_id.into()), None)
            .await?)
    }
    /// get one message sent by a user ID
    pub async fn message_get_one_sent(
        &self,
        message_id: impl Into<String>,
        user_id: impl Into<String>,
    ) -> crate::Result<Option<Message>> {
        Ok(self
            .messages
            .find_one(
                doc! {"_id": message_id.into(), "author": user_id.into()},
                None,
            )
            .await?)
    }
    /// todo bad time complexity, non 'brute force' method preferred
    /// get messages based on a set of IDs
    pub async fn message_get_many(&self, ids: IndexSet<String>) -> Vec<Message> {
        let mut future_vec = Vec::new();
        for id in ids {
            future_vec.push(self.message_get_one(id));
        }
        Data::flatten(join_all(future_vec).await)
    }

    /// get all messages sent by a user ID
    pub async fn message_get_sent(
        &self,
        user_id: impl Into<String>,
    ) -> crate::Result<Vec<Message>> {
        Ok(collect!(
            self.messages
                .find(doc!("author": user_id.into()), None)
                .await?
        ))
    }
}
