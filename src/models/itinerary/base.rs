use std::collections::HashMap;

use mongodb::bson::oid::ObjectId;
use mongodb::bson::DateTime;
use serde::{Deserialize, Serialize};

fn default_datetime() -> DateTime {
    DateTime::now()
}

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
    #[serde(default = "default_datetime")]
    pub arrival_datetime: DateTime,
    #[serde(default = "default_datetime")]
    pub departure_datetime: DateTime,
    pub adults: u32,
    pub children: u32,
    pub infants: u32,
    pub pets: u32,
    pub activities: Vec<Activity>,
    pub lodging: Vec<String>,
    pub transportation: String,
    pub budget_per_person: Option<f32>,
    pub interests: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub created_at: Option<DateTime>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<DateTime>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct FeaturedVacation {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,
    pub fareharbor_id: Option<String>,
    pub trip_name: String,
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub arrival_datetime: Option<DateTime>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub departure_datetime: Option<DateTime>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub adults: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub children: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub infants: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pets: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lodging: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub transportation: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub created_at: Option<DateTime>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<DateTime>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tag: Option<String>,
    #[serde(default, skip_serializing)]
    pub activities: Option<Vec<Activity>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub match_score: Option<u8>, // Score from 0-100
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub score_breakdown: Option<crate::services::search_scoring::ScoreBreakdown>, // Detailed score breakdown
}

impl Default for FeaturedVacation {
    fn default() -> Self {
        Self {
            id: None,
            fareharbor_id: None,
            trip_name: String::new(),
            min_age: None,
            min_group: 1,
            max_group: 1,
            length_days: 1,
            length_hours: 24,
            start_location: Location::default(),
            end_location: Location::default(),
            description: String::new(),
            days: Days::default(),
            images: None,
            arrival_datetime: None,
            departure_datetime: None,
            adults: None,
            children: None,
            infants: None,
            pets: None,
            lodging: None,
            transportation: None,
            created_at: None,
            updated_at: None,
            tag: None,
            activities: None,
            match_score: None,
            score_breakdown: None,
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct Location {
    city: String,
    state: String,
    coordinates: Vec<f64>,  // MongoDB stores as array of doubles
}

impl Location {
    pub fn city(&self) -> &str {
        &self.city
    }

    pub fn state(&self) -> &str {
        &self.state
    }

    pub fn coordinates(&self) -> (f32, f32) {
        // Convert from Vec<f64> to tuple, with defaults if missing
        let x = self.coordinates.get(0).copied().unwrap_or(0.0) as f32;
        let y = self.coordinates.get(1).copied().unwrap_or(0.0) as f32;
        (x, y)
    }
}


#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct Days {
    pub days: HashMap<String, Vec<DayItem>>,
}


#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(tag = "type")] // Use the "type" field to determine which variant to use
pub enum DayItem {
    #[serde(rename = "transportation")]
    Transportation {
        time: String,  // Changed from NaiveTime to String
        location: ItemLocation,
        name: String,
    },

    #[serde(rename = "activity")]
    Activity {
        time: String,  // Changed from NaiveTime to String
        activity_id: ObjectId,
    },

    #[serde(rename = "accommodation")]
    Accommodation {
        time: String,  // Changed from NaiveTime to String
        accommodation_id: ObjectId,
    },
}

impl Default for DayItem {
    fn default() -> Self {
        DayItem::Transportation {
            time: "00:00:00".to_string(),
            location: ItemLocation::default(),
            name: String::new(),
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct ItemLocation {
    pub name: String,
    pub coordinates: Vec<f64>,  // MongoDB stores as array of doubles
}
