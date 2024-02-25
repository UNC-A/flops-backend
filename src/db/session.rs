use crate::collect;
use crate::db::Data;
use crate::structures::models::{Channel, Message, User, UserSafe};
use futures_util::StreamExt;
use indexmap::IndexSet;
use mongodb::bson::doc;

impl Data {
    /// Authenticate takes the raw header data and finds the token.
    /// Deserializes session token and authenticates the User.
    pub async fn authenticate(&self, session: Option<String>) -> crate::Result<Option<User>> {
        let Some(session) = session else {
            return Ok(None);
        };
        let session = session.split('=').collect::<Vec<&str>>();
        let Some(session) = session.get(1) else {
            return Ok(None);
        };

        let doc = doc!("sessions": session);
        // check if user exists
        if self.users.find_one(doc.clone(), None).await?.is_none() {
            return Ok(None);
        };

        // while this is an unwrap, its practically infallible as the user is polled earlier
        // assumed that session is valid
        Ok(Some(self.users.find_one(doc, None).await?.unwrap()))
    }

    /// Provides history and context for websocket client
    // todo getting messages may be inefficient
    pub async fn establish(
        &self,
        user_id: impl Into<String>,
    ) -> crate::Result<(Vec<Channel>, Vec<UserSafe>, Vec<Message>)> {
        let user_id = user_id.into();

        let channels: Vec<Channel> =
            collect!(self.channels.find(doc!("members": &user_id), None).await?);

        let member_ids: IndexSet<String> = channels
            .iter()
            .flat_map(|channel| channel.members.iter())
            .cloned()
            .collect();
        let channel_ids: IndexSet<String> = channels.iter().map(|a| a.id.clone()).collect();

        let users: Vec<UserSafe> = self
            .get_user_many(member_ids)
            .await
            .into_iter()
            .flat_map(|a| match a {
                Ok(Some(a)) => Some(a.into()),
                _ => None,
            })
            .collect();

        let messages = self.message_get_many_channel_unsafe(channel_ids).await;

        Ok((channels, users, messages))
    }
}
