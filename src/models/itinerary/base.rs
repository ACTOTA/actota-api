use std::collections::HashMap;

use chrono::NaiveTime;
use mongodb::bson::oid::ObjectId;
use mongodb::bson::DateTime;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, Clone)]
pub enum DayItemType {
    #[serde(rename = "transportation")]
    Transportation,
    #[serde(rename = "activity")]
    Activity,
    #[serde(rename = "accomodation")]
    Accomodation,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Activity {
    pub label: String,
    pub description: String,
    pub tags: Vec<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ItinerarySubmission {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,
    pub user_id: Option<ObjectId>,
    pub location_start: String,
    pub location_end: String,
    pub arrival_datetime: DateTime,
    pub departure_datetime: DateTime,
    pub adults: u32,
    pub children: u32,
    pub infants: u32,
    pub pets: u32,
    pub activities: Vec<Activity>,
    pub lodging: Vec<String>,
    pub transportation: String,
    pub budger_per_person: Option<f32>,
    pub interests: Option<Vec<String>>,
    pub created_at: Option<DateTime>,
    pub updated_at: Option<DateTime>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct FeaturedVacation {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,
    pub fareharbor_id: Option<u32>,
    pub trip_name: String,
    pub person_cost: f32,
    pub min_age: Option<u32>,
    pub min_group: u32,
    pub max_group: u32,
    pub length_days: u32,
    pub length_hours: u32,
    pub start_location: Location,
    pub end_location: Location,
    pub description: String,
    #[serde(flatten)]
    pub days: Days,
    pub images: Option<Vec<String>>,
    pub created_at: Option<DateTime>,
    pub updated_at: Option<DateTime>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Location {
    city: String,
    state: String,
    coordinates: (f32, f32),
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Days {
    pub days: HashMap<String, Vec<DayItem>>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(tag = "type")] // Use the "type" field to determine which variant to use
pub enum DayItem {
    #[serde(rename = "transportation")]
    Transportation {
        time: NaiveTime,
        location: ItemLocation,
        name: String,
    },

    #[serde(rename = "activity")]
    Activity {
        time: NaiveTime,
        activity_id: ObjectId,
    },

    #[serde(rename = "accommodation")]
    Accommodation {
        time: NaiveTime,
        accommodation_id: ObjectId,
    },
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ItemLocation {
    name: String,
    coordinates: (f32, f32),
}
