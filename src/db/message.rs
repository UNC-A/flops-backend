use crate::collect;
use crate::db::Data;
use crate::structures::models::Message;
use futures_util::future::join_all;
use futures_util::StreamExt;
use indexmap::IndexSet;
use mongodb::bson::doc;
impl Data {

    pub async fn message_insert(&self, message: Message) -> crate::Result<()> {
        Ok(self.messages.insert_one(message, None).await.map(|_|{})?)
    }
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

    /// get many messages from a channel
    pub async fn message_get_many_channel(&self, id: impl Into<String>) -> crate::Result<Vec<Message>>{
        Ok(collect!(self.messages.find(doc!("channel": id.into()), None).await?))
    }
    /// get many messages from many channels
    pub async fn message_get_many_channel_unsafe(&self, ids: IndexSet<String>) -> Vec<Message> {
        let mut future_vec = Vec::new();
        for id in ids {
            future_vec.push(self.message_get_many_channel(id));
        }
        Data::flatten_vec(join_all(future_vec).await)
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
