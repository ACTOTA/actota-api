use serde::{Deserialize, Serialize};
use mongodb::bson::oid::ObjectId;
use chrono::{DateTime, NaiveDate, Utc};

#[derive(Debug, Deserialize, Serialize)]
pub struct UserTraveler {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,
    pub email: String,
    pub password: String, // Always hashed
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub phone_number: Option<String>,
    pub birth_date: Option<NaiveDate>,
    // Security related fields
    pub last_signin: Option<DateTime<Utc>>,
    pub last_signin_ip: Option<String>,
    pub failed_signins: Option<i32>,
    // We always want these fields, but have them optional so we can set them in the code
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>
}
