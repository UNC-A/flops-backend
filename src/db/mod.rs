use crate::db::vdb::State;
use crate::structures::models::{Channel, Message, User};
use mongodb::bson::doc;
use mongodb::{Client, Collection};
use std::env;
use std::sync::Arc;

pub mod channel;
pub mod message;
pub mod session;
pub mod user;
pub mod vdb;

#[derive(Debug, Clone)]
pub struct Data {
    pub channels: Arc<Collection<Channel>>,
    pub users: Arc<Collection<User>>,
    pub messages: Arc<Collection<Message>>,
    //db: Database,
    // high speed volatile  DB
    pub state: Arc<State>,
}
#[macro_export]
macro_rules! collect {
    ($cursor:expr) => {{
        $cursor
            .filter_map(|a| async move { a.ok() })
            .collect()
            .await
    }};
}

impl Data {
    pub fn flatten<T>(input: Vec<crate::Result<Option<T>>>) -> Vec<T> {
        input
            .into_iter()
            .flat_map(|a| match a {
                Ok(Some(a)) => Some(a.into()),
                _ => None,
            })
            .collect()
    }
    pub async fn inject_content(&mut self) -> crate::Result<Self> {
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

    pub async fn delete_all(&self) -> crate::Result<Self> {
        self.users.delete_many(doc!(), None).await?;
        self.channels.delete_many(doc!(), None).await?;
        self.messages.delete_many(doc!(), None).await?;
        Ok(self.clone())
    }

    pub async fn start(test: bool) -> crate::Result<Self> {
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
}
