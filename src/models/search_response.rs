use crate::models::activity::Activity;
use crate::models::itinerary::base::Location;
use mongodb::bson::{oid::ObjectId, DateTime};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Custom response format for search results with populated activities
#[derive(Debug, Serialize, Deserialize)]
pub struct SearchResponseItem {
    #[serde(rename = "_id")]
    pub id: ObjectId,
    pub fareharbor_id: Option<u32>,
    pub trip_name: String,
    pub min_age: Option<u32>,
    pub min_group: u32,
    pub max_group: u32,
    pub length_days: u32,
    pub length_hours: u32,
    pub start_location: Location,
    pub end_location: Location,
    pub description: String,
    pub images: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<DateTime>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<DateTime>,
    pub person_cost: f64,
    pub days: HashMap<String, Vec<PopulatedDayItem>>,
    pub activities: Vec<ActivitySummary>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub match_score: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub score_breakdown: Option<serde_json::Value>,
}

/// Day item with simplified activity data
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum PopulatedDayItem {
    #[serde(rename = "activity")]
    Activity {
        time: String,
        activity_id: ObjectId,
    },
    #[serde(rename = "transportation")]
    Transportation {
        time: String,
        location: serde_json::Value,
        name: String,
    },
    #[serde(rename = "accommodation")]
    Accommodation {
        time: String,
        accommodation_id: ObjectId,
    },
}

/// Summary of activities for the itinerary
#[derive(Debug, Serialize, Deserialize)]
pub struct ActivitySummary {
    pub time: String,
    pub label: String,
    pub tags: Vec<String>,
}

impl PopulatedDayItem {
    pub fn from_activity(time: String, activity_id: ObjectId, _activity: Activity) -> Self {
        PopulatedDayItem::Activity {
            time,
            activity_id,
        }
    }
}