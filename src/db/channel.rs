use crate::collect;
use crate::db::Data;
use crate::structures::models::Channel;
use futures_util::StreamExt;
use mongodb::bson::doc;
impl Data {
    /// get one channel based on an ID
    pub async fn channel_get_one(
        &self,
        channel_id: impl Into<String>,
    ) -> crate::Result<Option<Channel>> {
        Ok(self
            .channels
            .find_one(doc! ( "_id": channel_id.into()), None)
            .await?)
    }
    /// get one channel based on id, will return None if user cannot access
    pub async fn channel_get_one_accessible(
        &self,
        user_id: impl Into<String>,
        channel_id: impl Into<String>,
    ) -> crate::Result<Option<Channel>> {
        Ok(self
            .channels
            .find_one(
                doc! {"members": user_id.into(), "_id": channel_id.into()},
                None,
            )
            .await?)
    }
    /// get all channels in which a user is a member in
    pub async fn channel_get_many_accessible(
        &self,
        user_id: impl Into<String>,
    ) -> crate::Result<Vec<Channel>> {
        Ok(collect!(
            self.channels
                .find(doc! {"members": user_id.into()}, None)
                .await?
        ))
    }
}
