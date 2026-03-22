use crate::domain::SessionRepository;
use crate::domain::models::Session;
use crate::error::AppResult;
use async_trait::async_trait;
use mongodb::bson::doc;
use mongodb::{Client, Collection};

const SESSIONS_COLLECTION: &str = "session";

pub struct MongoSessionRepository {
    collection: Collection<bson::Document>,
}

impl MongoSessionRepository {
    pub fn new(client: &Client, database: &str) -> Self {
        let db = client.database(database);
        Self {
            collection: db.collection(SESSIONS_COLLECTION),
        }
    }
}

#[async_trait]
impl SessionRepository for MongoSessionRepository {
    async fn find_by_id(&self, id: &str) -> AppResult<Option<Session>> {
        let filter = doc! {
            "$or": [
                { "id": id },
                { "_id": id },
                { "token": id }
            ]
        };

        let result = self.collection.find_one(filter).await?;

        match result {
            Some(doc) => {
                let session = Session::from_bson_document(doc)?;
                Ok(Some(session))
            }
            None => Ok(None),
        }
    }
}
