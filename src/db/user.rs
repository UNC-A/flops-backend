use crate::db::Data;
use crate::structures::models::User;
use futures_util::future::join_all;
use indexmap::IndexSet;
use mongodb::bson::doc;
impl Data {
    /// get a user based on id
    async fn get_user(&self, user_id: impl Into<String>) -> crate::Result<Option<User>> {
        Ok(self
            .users
            .find_one(doc!("_id": user_id.into()), None)
            .await?)
    }

    /// get many users based on a set of user IDs
    /// todo a non brute force method is preferred
    pub async fn get_user_many(&self, users: IndexSet<String>) -> Vec<crate::Result<Option<User>>> {
        let mut user_list = Vec::new();
        for user in users {
            user_list.push(self.get_user(user));
        }
        join_all(user_list).await
    }
}
