use crate::structures::{
    models::UserSafe,
    models::{Channel, Message, User},
    websocket::events::EventEnum,
};
use futures_util::{future::join_all, StreamExt};
use indexmap::IndexSet;
use mongodb::{bson::doc, Client, Collection};
use std::{env, sync::Arc};
use tokio::sync::RwLock;
macro_rules! collect {
    ($cursor:expr) => {{
        $cursor
            .filter_map(|a| async move { a.ok() })
            .collect()
            .await
    }};
}
pub type Result<T> = std::result::Result<T, anyhow::Error>;
#[derive(Debug, Clone)]
pub struct Data {
    pub channels: Arc<Collection<Channel>>,
    pub users: Arc<Collection<User>>,
    pub messages: Arc<Collection<Message>>,
    //db: Database,
    // high speed volatile  DB
    pub state: Arc<State>,
}

/// abstract methods
impl State {
    /// two part function: update database that the user is online, and reject simultaneous account
    /// connections
    /// if false, user is already present
    pub async fn user_online(&self, user_id: impl Into<String>) -> bool {
        self.remove_dead_messages().await;
        self.online_users.write().await.insert(user_id.into())
    }
    /// set users as offline
    pub async fn user_offline(&self, user_id: impl Into<String>) {
        let user_id = user_id.into();
        self.online_users.write().await.shift_remove(&user_id);
        self.user_remove_message(&user_id).await;
        self.remove_dead_messages().await;
    }

    /// get all messages applicable for a user
    pub async fn get_message(&self, user_id: impl Into<String>) -> Option<Vec<EventMessage>> {
        let user_id = user_id.into();
        // if user offline, return
        if self.online_users.read().await.get(&user_id).is_none() {
            self.user_remove_message(&user_id).await;
            return None;
        };
        // get all items applicable to user
        let item = self
            .pending_messages
            .read()
            .await
            .clone()
            .into_iter()
            .filter(|a| a.targets.get(&user_id).is_some())
            .collect::<Vec<EventMessage>>();

        // if no items, return none
        if item.is_empty() {
            return None;
        };
        // since all messages have been forwarded, delete old from db
        // copy to avoid thread locks
        self.user_remove_message(user_id).await;
        self.remove_dead_messages().await;
        Some(item)
    }

    /// safely adds a message to the top of the message queue
    pub async fn message_add_vdb(&self, mut event_message: EventMessage) {
        let users = &self.online_users.read().await;

        for x in event_message.targets.clone() {
            if users.get(&x).is_none() {
                event_message.targets.shift_remove(&x);
            };
        }
        if !event_message.targets.is_empty() {
            self.pending_messages.write().await.push(event_message);
        };
    }
}
/// internal methods
impl State {
    /// removes all messages that do not have targets
    async fn remove_dead_messages(&self) {
        let data = self.pending_messages.read().await.clone();
        *self.pending_messages.write().await =
            data.into_iter().filter(|a| !a.targets.is_empty()).collect();
    }
    /// removes user_id from all references in message queue
    async fn user_remove_message(&self, user_id: impl Into<String>) {
        let user_id = user_id.into();
        let data = self
            .pending_messages
            .read()
            .await
            .clone()
            .into_iter()
            .map(|mut a| {
                a.targets.shift_remove(&user_id);
                a
            })
            .collect();
        *self.pending_messages.write().await = data;
    }
}
#[derive(Debug, Default, Clone)]
pub struct State {
    pub pending_messages: Arc<RwLock<Vec<EventMessage>>>,
    pub online_users: Arc<RwLock<IndexSet<String>>>,
}
#[derive(Debug, Default, Clone)]
pub struct EventMessage {
    pub author: String,
    pub targets: IndexSet<String>,
    pub item: EventEnum,
}


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

    async fn get_user_many(&self, users: IndexSet<String>) -> Vec<Result<Option<User>>> {
        let mut user_list = Vec::new();
        for user in users {
            user_list.push(self.get_user(user));
        }
        join_all(user_list).await
    }

    pub async fn establish(
        &self,
        user_id: impl Into<String>,
    ) -> Result<(Vec<Channel>, Vec<UserSafe>)> {
        let user_id = user_id.into();

        let channels: Vec<Channel> =
            collect!(self.channels.find(doc!("members": &user_id), None).await?);

        let member_ids: IndexSet<String> = channels
            .iter()
            .flat_map(|channel| channel.members.iter())
            .cloned()
            .collect();

        let users: Vec<UserSafe> = self
            .get_user_many(member_ids)
            .await
            .into_iter()
            .flat_map(|a| match a {
                Ok(Some(a)) => Some(a.into()),
                _ => None,
            })
            .collect();

        Ok((channels, users))
    }

    pub async fn inject_content(&mut self) -> Result<Self> {
        self.delete_all().await?;
        self.users
            .insert_many(
                serde_json::from_slice::<Vec<User>>(&std::fs::read("example.users.json")?)?,
                None,
            )
            .await?;
        self.channels
            .insert_many(
                serde_json::from_slice::<Vec<Channel>>(&std::fs::read("example.channels.json")?)?,
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
    pub async fn authenticate(&self, session: Option<String>) -> Result<Option<User>> {
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
    use crate::structures::rand;
    use dotenv::dotenv;

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
                        members: [user_id.clone(), rand()].into(),
                    },
                    Channel {
                        id: c2_id.clone(),
                        members: [user_id.clone(), rand()].into(),
                    },
                    Channel {
                        id: c3_id.clone(),
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
                        channel: c1_id.to_string(),
                    },
                    Message {
                        author: rand(),
                        id: m2_id.clone(),
                        content: "hello stranger".to_string(),
                        channel: c1_id.to_string(),
                    },
                    Message {
                        author: rand(),
                        id: m3_id.clone(),
                        content: "owo".to_string(),
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
