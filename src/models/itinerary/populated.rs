use mongodb::bson::DateTime;
use mongodb::bson::oid::ObjectId;
use serde::ser::SerializeStruct;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::collections::HashMap;

use super::base::{FeaturedVacation, ItemLocation};
use crate::services::search_scoring::ScoreBreakdown;

// Custom deserializer to handle floating point to u16 conversion
fn deserialize_rounded_u16<'de, D>(deserializer: D) -> Result<u16, D::Error>
where
    D: Deserializer<'de>,
{
    let value: serde_json::Value = serde_json::Value::deserialize(deserializer)?;
    match value {
        serde_json::Value::Number(n) => {
            if let Some(f) = n.as_f64() {
                Ok(f.ceil() as u16)
            } else if let Some(i) = n.as_u64() {
                Ok(i as u16)
            } else {
                Ok(0)
            }
        }
        _ => Ok(0),
    }
}

// Custom deserializer for optional u16 fields
fn deserialize_optional_rounded_u16<'de, D>(deserializer: D) -> Result<Option<u16>, D::Error>
where
    D: Deserializer<'de>,
{
    let value: Option<serde_json::Value> = Option::deserialize(deserializer)?;
    match value {
        None => Ok(None),
        Some(serde_json::Value::Number(n)) => {
            if let Some(f) = n.as_f64() {
                Ok(Some(f.ceil() as u16))
            } else if let Some(i) = n.as_u64() {
                Ok(Some(i as u16))
            } else {
                Ok(None)
            }
        }
        Some(serde_json::Value::Null) => Ok(None),
        _ => Ok(None),
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct TimeSlot {
    pub start: String,
    pub end: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Address {
    pub street: String,
    pub unit: Option<String>,
    pub city: String,
    pub state: String,
    pub zip: String,
    pub country: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Capacity {
    #[serde(deserialize_with = "deserialize_rounded_u16")]
    pub minimum: u16,
    #[serde(deserialize_with = "deserialize_rounded_u16")]
    pub maximum: u16,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ActivityModel {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,
    pub company: String,
    pub company_id: String, // The ID for the booking platform, NOT an ObjectId in MongoDB
    pub booking_link: String,
    pub online_booking_status: String,
    pub title: String,
    pub description: String,
    pub activity_types: Vec<String>,
    pub tags: Vec<String>,
    pub price_per_person: f32,
    pub duration_minutes: u16,
    pub daily_time_slots: Vec<TimeSlot>,
    pub address: Address,
    pub whats_included: Vec<String>,
    #[serde(deserialize_with = "deserialize_optional_rounded_u16", default)]
    pub weight_limit_lbs: Option<u16>,
    #[serde(deserialize_with = "deserialize_optional_rounded_u16", default)]
    pub age_requirement: Option<u16>,
    #[serde(deserialize_with = "deserialize_optional_rounded_u16", default)]
    pub height_requirement: Option<u16>,
    // pub blackout_date_ranges: Option<Vec<String>>, // Update later
    pub capacity: Capacity,
    // For the frontend
    pub activities: Option<Vec<ActivitySummary>>,
    pub primary_image: Option<String>,
    pub images: Option<Vec<String>>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ActivitySummary {
    pub time: String,
    pub label: String,
    pub tags: Vec<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct AccommodationModel {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,
    pub name: String,
    pub address: Option<String>,
    pub location: Option<ItemLocation>,
    pub price_per_night: Option<f32>,
    pub amenities: Option<Vec<String>>,
    pub primary_image: Option<String>,
    pub images: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub created_at: Option<DateTime>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<DateTime>,
}

// Populated version of the DayItem enum
#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(tag = "type")]
pub enum PopulatedDayItem {
    #[serde(rename = "transportation")]
    Transportation {
        time: String,
        location: ItemLocation,
        name: String,
    },

    #[serde(rename = "activity")]
    Activity {
        time: String,
        // Keep activity_id for backward compatibility with existing database records
        activity_id: Option<ObjectId>,
        #[serde(flatten)]
        activity: ActivityModel,
    },

    #[serde(rename = "accommodation")]
    Accommodation {
        time: String,
        #[serde(flatten)]
        accommodation: AccommodationModel,
    },
}

// Populated version using composition for minimal maintenance
#[derive(Debug, Clone)]
pub struct PopulatedFeaturedVacation {
    // Reuse the original struct rather than duplicating all fields
    pub base: FeaturedVacation,
    pub person_cost: f32,
    pub populated_days: HashMap<String, Vec<PopulatedDayItem>>,
    pub activities: Vec<ActivitySummary>,
    pub match_score: Option<u8>, // Score from 0-100
    pub score_breakdown: Option<ScoreBreakdown>, // Detailed score breakdown
    pub activity_cost: Option<f32>, // Total activity costs
    pub lodging_cost: Option<f32>, // Total lodging costs
    pub transport_cost: Option<f32>, // Total transport costs
    pub service_fee: Option<f32>, // Service fee
}

// Custom serialization to handle the composition
impl Serialize for PopulatedFeaturedVacation {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        // Create a serialization struct with all the fields
        let mut field_count = 18;
        if self.match_score.is_some() { field_count += 1; }
        if self.score_breakdown.is_some() { field_count += 1; }
        if self.activity_cost.is_some() { field_count += 1; }
        if self.lodging_cost.is_some() { field_count += 1; }
        if self.transport_cost.is_some() { field_count += 1; }
        if self.service_fee.is_some() { field_count += 1; }
        let mut state = serializer.serialize_struct("PopulatedFeaturedVacation", field_count)?;

        // Serialize all base fields
        state.serialize_field("_id", &self.base.id)?;
        state.serialize_field("fareharbor_id", &self.base.fareharbor_id)?;
        state.serialize_field("trip_name", &self.base.trip_name)?;
        state.serialize_field("min_age", &self.base.min_age)?;
        state.serialize_field("min_group", &self.base.min_group)?;
        state.serialize_field("max_group", &self.base.max_group)?;
        state.serialize_field("length_days", &self.base.length_days)?;
        state.serialize_field("length_hours", &self.base.length_hours)?;
        state.serialize_field("start_location", &self.base.start_location)?;
        state.serialize_field("end_location", &self.base.end_location)?;
        state.serialize_field("description", &self.base.description)?;
        state.serialize_field("images", &self.base.images)?;
        state.serialize_field("created_at", &self.base.created_at)?;
        state.serialize_field("updated_at", &self.base.updated_at)?;

        // Serialize the person_cost field
        state.serialize_field("person_cost", &self.person_cost)?;

        // Serialize the populated days
        state.serialize_field("days", &self.populated_days)?;

        // Serialize the activities summary
        state.serialize_field("activities", &self.activities)?;
        
        // Serialize the match score if present
        if let Some(score) = self.match_score {
            state.serialize_field("match_score", &score)?;
        }
        
        // Serialize the score breakdown if present
        if let Some(breakdown) = &self.score_breakdown {
            state.serialize_field("score_breakdown", breakdown)?;
        }
        
        // Serialize the cost fields if present
        if let Some(activity_cost) = self.activity_cost {
            state.serialize_field("activity_cost", &activity_cost)?;
        }
        if let Some(lodging_cost) = self.lodging_cost {
            state.serialize_field("lodging_cost", &lodging_cost)?;
        }
        if let Some(transport_cost) = self.transport_cost {
            state.serialize_field("transport_cost", &transport_cost)?;
        }
        if let Some(service_fee) = self.service_fee {
            state.serialize_field("service_fee", &service_fee)?;
        }

        state.end()
    }
}

// Helper method to convert from base to populated version
impl PopulatedFeaturedVacation {
    pub fn from_base(
        base: FeaturedVacation,
        person_cost: f32,
        populated_days: HashMap<String, Vec<PopulatedDayItem>>,
        activities: Vec<ActivitySummary>,
    ) -> Self {
        Self {
            base,
            person_cost,
            populated_days,
            activities,
            match_score: None,
            score_breakdown: None,
            activity_cost: None,
            lodging_cost: None,
            transport_cost: None,
            service_fee: None,
        }
    }

    pub fn id(&self) -> Option<ObjectId> {
        self.base.id
    }

    pub fn trip_name(&self) -> &str {
        &self.base.trip_name
    }

    pub fn person_cost(&self) -> f32 {
        self.person_cost
    }
    
    pub fn set_match_score(&mut self, score: u8) {
        self.match_score = Some(score.min(100)); // Ensure score doesn't exceed 100
    }
    
    pub fn set_score_breakdown(&mut self, breakdown: ScoreBreakdown) {
        self.score_breakdown = Some(breakdown);
    }
    
    pub fn set_activity_cost(&mut self, cost: f32) {
        self.activity_cost = Some(cost);
    }
    
    pub fn set_lodging_cost(&mut self, cost: f32) {
        self.lodging_cost = Some(cost);
    }
    
    pub fn set_transport_cost(&mut self, cost: f32) {
        self.transport_cost = Some(cost);
    }
    
    pub fn set_service_fee(&mut self, fee: f32) {
        self.service_fee = Some(fee);
    }
}
