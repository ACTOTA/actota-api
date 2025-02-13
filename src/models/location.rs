use mongodb::bson::{oid::ObjectId, DateTime};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct Location {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,
    pub city: String,
    pub state: String,
    pub coordinates: (f64, f64),
    pub created_at: Option<DateTime>,
    pub updated_at: Option<DateTime>,
}
