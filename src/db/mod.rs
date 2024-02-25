use crate::db::vdb::State;
use crate::structures::models::{Channel, Message, User};
use mongodb::bson::doc;
use mongodb::{Client, Collection};
use std::env;
use std::sync::Arc;

/// Collection and methods for the Channel collection.
pub mod channel;
/// Collection and methods for the Message collection.
pub mod message;
/// Method for authentication and session management.
pub mod session;
/// Collection and methods for the User collection.
pub mod user;
/// Methods for volatile DB.
pub mod vdb;

/// # Data
/// Database structure for the backend.
/// All items besides 'state' are MongoDB collections.
/// State is a volatile DB used for quickly managing pending messages
#[derive(Debug, Clone)]
pub struct Data {
    /// Channel Collection
    pub channels: Arc<Collection<Channel>>,
    /// User Collection
    pub users: Arc<Collection<User>>,
    /// Message Collection
    pub messages: Arc<Collection<Message>>,
    /// Volatile DB
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
    /// Takes a vector of results of options, returns a vec.
    pub fn flatten<T>(input: Vec<crate::Result<Option<T>>>) -> Vec<T> {
        input
            .into_iter()
            .flat_map(|a| match a {
                Ok(Some(a)) => Some(a),
                _ => None,
            })
            .collect()
    }
    /// Takes a vector of results of vectors, returns a vector.
    pub fn flatten_vec<T>(input: Vec<crate::Result<Vec<T>>>) -> Vec<T> {
        input
            .into_iter()
            .flatten()
            .fold(Vec::new(), |mut vec, vec_new| {
                vec.extend(vec_new);
                vec
            })
    }
    // todo temporary development function
    /// Takes several JSON files and injects them in the DB.
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

    // todo temporary development function
    /// Deletes everything in data collections
    pub async fn delete_all(&self) -> crate::Result<Self> {
        self.users.delete_many(doc!(), None).await?;
        self.channels.delete_many(doc!(), None).await?;
        self.messages.delete_many(doc!(), None).await?;
        Ok(self.clone())
    }
    /// Initialisation function for Database.
    /// The 'test' field determines the collection used.
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
