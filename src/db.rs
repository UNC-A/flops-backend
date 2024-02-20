use crate::structures::models::UserSafe;
use crate::structures::{
    models::{Channel, Message, User},
    rand,
    websocket::events::EventEnum,
};
use futures_util::{future::join_all, StreamExt};
use indexmap::IndexSet;
use mongodb::{bson::doc, Client, Collection};
use std::{env, sync::Arc};
use tokio::sync::RwLock;

macro_rules! cursor_get {
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
pub struct Data {
    pub channels: Arc<Collection<Channel>>,
    pub users: Arc<Collection<User>>,
    pub messages: Arc<Collection<Message>>,
    //db: Database,
    // high speed volatile  DB
    pub state: Arc<State>,
}

#[derive(Debug, Default, Clone)]
pub struct State {
    pub pending_messages: Arc<RwLock<Vec<EventMessage>>>,
}
#[derive(Debug, Default, Clone)]
pub struct EventMessage {
    pub author: String,
    pub targets: IndexSet<String>,
    pub item: EventEnum,
}

pub type Result<T> = std::result::Result<T, anyhow::Error>;
impl Data {
    pub async fn start(test: bool) -> Result<Self> {
        let db = Client::with_uri_str(&env::var("MONGO_URI")?)
            .await?
            .database(match test {
                true => "unca_test",
                false => "unca",
            });

        Ok(Self {
            channels: Arc::new(db.collection("channels")),
            users: Arc::new(db.collection("users")),
            messages: Arc::new(db.collection("messages")),
            state: Default::default(),
        })
    }

    async fn get_user(&self, user_id: impl Into<String>) -> Result<Option<User>> {
        Ok(self
            .users
            .find_one(doc!("_id": user_id.into()), None)
            .await?)
    }

    pub async fn establish(
        &self,
        user_id: impl Into<String>,
    ) -> Result<(Vec<Channel>, Vec<UserSafe>)> {
        let user_id = user_id.into();

        let channels: Vec<Channel> =
            collect!(self.channels.find(doc!("members": &user_id), None).await?);

        let mut user_list = Vec::new();
        for channel in &channels {
            for member in &channel.members {
                if member != &user_id {
                    user_list.push(self.get_user(member));
                }
            }
        }
        let users: Vec<UserSafe> = join_all(user_list)
            .await
            .into_iter()
            .filter_map(|a| a.ok())
            .flat_map(|inner_vec| inner_vec.into_iter())
            .map(|a| a.into())
            .collect();
        Ok((channels, users))
    }
    async fn get_messages(&self, channel_id: impl Into<String>) -> Result<Vec<Message>> {
        Ok(collect!(
            self.messages
                .find(doc!("channel": channel_id.into()), None)
                .await?
        ))
    }
    async fn get_channels(&self, user_id: impl Into<String>) -> Result<Vec<Channel>> {
        Ok(collect!(
            self.channels
                .find(doc!("members": user_id.into()), None)
                .await?
        ))
    }

    async fn get_accessible_messages(&self, user_id: impl Into<String>) -> Result<Vec<Message>> {
        let user_id = user_id.into();
        let channels: Vec<Channel> = self.get_channels(&user_id).await?.into_iter().collect();

        let mut message_list = Vec::new();

        for channel in channels {
            if channel.members.contains(&user_id) {
                message_list.push(self.get_messages(channel.id));
            }
        }

        let messages: Vec<Message> = join_all(message_list)
            .await
            .into_iter()
            .filter_map(|a| a.ok())
            .flat_map(|inner_vec| inner_vec.into_iter())
            .collect();

        Ok(messages)
    }

    pub async fn inject_content(&mut self) -> Result<Self> {
        self.delete_all().await?;
        self.users
            .insert_many(
                serde_json::from_slice::<Vec<User>>(&std::fs::read("users.json")?)?,
                None,
            )
            .await?;
        self.channels
            .insert_many(
                serde_json::from_slice::<Vec<Channel>>(&std::fs::read("channels.json")?)?,
                None,
            )
            .await?;

        Ok(self.clone())
    }

    pub async fn delete_all(&self) -> Result<Self> {
        self.users.delete_many(doc!(), None).await?;
        self.channels.delete_many(doc!(), None).await?;
        self.messages.delete_many(doc!(), None).await?;
        Ok(self.clone())
    }

    /// authenticate takes the raw header data and finds the token
    /// additionally it will add connection ID to the database
    pub async fn authenticate(&self, session: Option<String>) -> Result<Option<(User, String)>> {
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
        let connection = rand();
        self.users
            .update_one(
                doc.clone(),
                doc! {
                    "$push": {
                        "connections": &connection,
                    }
                },
                None,
            )
            .await?;

        // while this is an unwrap, its practically infallible as the user is polled earlier
        // assumed that session is valid
        Ok(Some((
            self.users.find_one(doc, None).await?.unwrap(),
            connection,
        )))
    }
    pub async fn logout(
        &self,
        user_id: impl Into<String>,
        connection: impl Into<String>,
    ) -> Result<()> {
        self.users
            .update_one(
                doc!("_id": user_id.into()),
                doc! {
                    "$pop": {
                        "connections": connection.into(),
                    }
                },
                None,
            )
            .await?;
        Ok(())
    }

    pub async fn get_channel_one(
        &self,
        user_id: impl Into<String>,
        channel_id: impl Into<String>,
    ) -> Result<Option<Channel>> {
        Ok(self
            .channels
            .find_one(
                doc! {"members": user_id.into(), "_id": channel_id.into()},
                None,
            )
            .await?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dotenv::dotenv;

    #[tokio::test]
    async fn db_establish_test() {
        // db connection

        dotenv().ok();
        let db = Data::start(true).await.unwrap();
        let fake_data = fake_data(&db).await.unwrap();

        // test begin

        let (channels, users) = db.establish(&fake_data.user_id).await.unwrap();

        // verifying
        for channel in channels {
            assert_ne!(channel.id, fake_data.c3_id);
        }
        db.delete_all().await.unwrap();
    }
    #[tokio::test]
    async fn db_message_get() {
        dotenv().ok();
        let db = Data::start(true).await.unwrap();
        let fake_data = fake_data(&db).await.unwrap();

        let a = db.get_accessible_messages(fake_data.user_id).await.unwrap();

        assert_eq!(a.len(), 2);
        db.delete_all().await.unwrap();
    }
}

/// this needs an explanation
/// channel one and two are accessible to the user
/// channel three is not
/// message one and two are sent in channel one
/// message three is sent in channel three (non-accessible)
///
/// the purpose of these tests is to see if disability is working correctly
async fn fake_data(db: &Data) -> Result<TestData> {
    let user_id = rand();
    // fake data
    let user = User {
        id: user_id.to_string(),
        username: "Person".to_string(),
        ..Default::default()
    };
    db.users.insert_one(user, None).await?;

    // c1-2 contain user, c3 does not
    let (c1_id, c2_id, c3_id): (String, String, String) = (rand(), rand(), rand());

    // m1-2 contain user, m3 does not
    let (m1_id, m2_id, m3_id): (String, String, String) = (rand(), rand(), rand());
    db.channels
        .insert_many(
            vec![
                Channel {
                    id: c1_id.clone(),
                    is_self: false,
                    members: [user_id.clone(), rand()].into(),
                },
                Channel {
                    id: c2_id.clone(),
                    is_self: false,
                    members: [user_id.clone(), rand()].into(),
                },
                Channel {
                    id: c3_id.clone(),
                    is_self: false,
                    members: [rand(), rand()].into(),
                },
            ],
            None,
        )
        .await?;

    db.messages
        .insert_many(
            vec![
                Message {
                    id: m1_id.clone(),
                    author: rand(),
                    content: "hello world".to_string(),
                    reply: None,
                    channel: c1_id.to_string(),
                },
                Message {
                    author: rand(),
                    id: m2_id.clone(),
                    content: "hello stranger".to_string(),
                    reply: None,
                    channel: c1_id.to_string(),
                },
                Message {
                    author: rand(),
                    id: m3_id.clone(),
                    content: "owo".to_string(),
                    reply: None,
                    channel: c3_id.to_string(),
                },
            ],
            None,
        )
        .await?;

    Ok(TestData {
        user_id,
        c1_id,
        c2_id,
        c3_id,
        m1_id,
        m2_id,
        m3_id,
    })
}

pub struct TestData {
    pub user_id: String,
    pub c1_id: String,
    pub c2_id: String,
    pub c3_id: String,
    pub m1_id: String,
    pub m2_id: String,
    pub m3_id: String,
}
