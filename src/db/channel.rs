use crate::db::Data;
use crate::structures::models::Channel;
use mongodb::bson::doc;

impl Data {
    pub async fn get_channel_one(
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
}
