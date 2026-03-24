use crate::domain::UserRepository;
use crate::domain::models::User;
use crate::error::AppResult;
use async_trait::async_trait;
use mongodb::bson::doc;
use mongodb::{Client, Collection};
use uuid::Uuid;

const USERS_COLLECTION: &str = "users";

pub struct MongoUserRepository {
    collection: Collection<bson::Document>,
}

impl MongoUserRepository {
    pub fn new(client: &Client, database: &str) -> Self {
        let db = client.database(database);
        Self {
            collection: db.collection(USERS_COLLECTION),
        }
    }
}

#[async_trait]
impl UserRepository for MongoUserRepository {
    async fn find_by_id(&self, id: Uuid) -> AppResult<Option<User>> {
        let filter = doc! {
            "$or": [
                { "id": id.to_string() },
                { "_id": id.to_string() }
            ]
        };

        let result = self.collection.find_one(filter).await?;

        match result {
            Some(doc) => {
                let user = User::from_bson_document(doc)?;
                Ok(Some(user))
            }
            None => Ok(None),
        }
    }

    async fn find_by_email(&self, email: &str) -> AppResult<Option<User>> {
        let filter = doc! { "email": email };
        let result = self.collection.find_one(filter).await?;

        match result {
            Some(doc) => {
                let user = User::from_bson_document(doc)?;
                Ok(Some(user))
            }
            None => Ok(None),
        }
    }

    async fn create(&self, user: &User) -> AppResult<()> {
        let doc = user.to_bson()?;
        self.collection.insert_one(doc).await?;
        Ok(())
    }
}
