use std::collections::HashMap;

use chrono::{DateTime, NaiveTime, Utc};
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

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct FeaturedVacation {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,
    pub fareharbor_id: u32,
    pub trip_name: String,
    pub person_cost: f32,
    pub length_days: u32,
    pub length_hours: u32,
    pub start_location: Location,
    pub end_location: Location,
    pub description: String,
    #[serde(flatten)]
    pub days: Days,
    pub activities: Option<Vec<Activity>>,
    pub images: Option<Vec<String>>,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Location {
    name: String,
    coordinates: (f32, f32),
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Days {
    days: HashMap<String, Vec<Day>>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Day {
    time: NaiveTime,
    location: Location,
    name: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Activity {
    label: String,
    description: String,
    tags: Vec<String>,
}
