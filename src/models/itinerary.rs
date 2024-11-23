use chrono::{DateTime, Utc};
use mongodb::bson::oid::ObjectId;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct ItinerarySubmission {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,
    pub user_id: Option<ObjectId>,
    pub location_start: String,
    pub location_end: String,
    pub arrival_datetime: DateTime<Utc>,
    pub departure_datetime: DateTime<Utc>,
    pub adults: u32,
    pub children: u32,
    pub infants: u32,
    pub pets: u32,
    pub activities: Vec<String>,
    pub lodging: Vec<String>,
    pub transportation: String,
    pub budger_per_person: Option<f32>,
    pub interests: Option<Vec<String>>,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
}
