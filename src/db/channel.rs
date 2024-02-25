use crate::collect;
use crate::db::Data;
use crate::structures::models::Channel;
use futures_util::StreamExt;
use mongodb::bson::doc;
impl Data {
    /// Get one channel based on a channel ID.
    pub async fn channel_get_one(
        &self,
        channel_id: impl Into<String>,
    ) -> crate::Result<Option<Channel>> {
        Ok(self
            .channels
            .find_one(doc! ( "_id": channel_id.into()), None)
            .await?)
    }
    /// Get one Channel based on Channel ID.
    /// Returns nothing if inaccessible.
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
    /// Get all channels accessible to user.
    /// Takes a User ID.
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
