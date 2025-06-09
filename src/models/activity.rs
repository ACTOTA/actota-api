use mongodb::bson::{oid::ObjectId, DateTime};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct TimeSlot {
    start: String,
    end: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Address {
    street: String,
    unit: String,
    city: String,
    state: String,
    zip: String,
    country: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Capacity {
    pub minimum: u16,
    pub maximum: u16,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct BlackoutDateRange {
    start: i64,
    end: i64,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Activity {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,
    pub company: String,
    pub company_id: String,
    pub booking_link: String,
    pub online_booking_status: String,
    pub guide: Option<String>,
    pub title: String,
    pub description: String,
    pub activity_types: Vec<String>,
    pub tags: Vec<String>,
    pub price_per_person: f32,
    pub duration_minutes: u16,
    pub daily_time_slots: Vec<TimeSlot>,
    pub address: Address,
    pub whats_included: Vec<String>,
    pub weight_limit_lbs: Option<u16>,
    pub age_requirement: Option<u8>,
    pub height_requiremnt: Option<u8>,
    pub blackout_date_ranges: Option<Vec<BlackoutDateRange>>,
    pub capacity: Capacity,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub created_at: Option<DateTime>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<DateTime>,
}
