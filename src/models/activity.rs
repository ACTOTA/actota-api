use mongodb::bson::{oid::ObjectId, DateTime};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct Activity {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,
    pub label: String,
    pub description: Option<String>,
    pub tags: Option<Vec<String>>,
    pub created_at: Option<DateTime>,
    pub updated_at: Option<DateTime>,
}
