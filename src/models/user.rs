use serde::{Deserialize, Serialize};
use mongodb::bson::oid::ObjectId;
use chrono::{DateTime, Utc};

#[derive(Debug, Deserialize, Serialize)]
pub struct UserTraveler {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,
    pub email: String,
    pub password: String, // Always hashed
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    // We always want these fields, but have them optional so we can set them in the code
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>
}
