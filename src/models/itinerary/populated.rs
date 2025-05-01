use chrono::{DateTime, NaiveTime, Utc};
use mongodb::bson::oid::ObjectId;
use serde::ser::SerializeStruct;
use serde::{Deserialize, Serialize, Serializer};
use std::collections::HashMap;

use super::base::{FeaturedVacation, ItemLocation};

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
    pub minimum: i32,
    pub maximum: i32,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ActivityModel {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,
    pub company: String,
    pub company_id: String,
    pub booking_link: String,
    pub online_booking_status: String,
    pub title: String,
    pub description: String,
    pub activity_types: Vec<String>,
    pub tags: Vec<String>,
    pub price_per_person: i32,
    pub duration_minutes: i32,
    pub daily_time_slots: Vec<TimeSlot>,
    pub address: Address,
    pub whats_included: Vec<String>,
    pub weight_limit_lbs: Option<i32>,
    pub age_requirement: Option<i32>,
    pub height_requirement: Option<i32>,
    pub blackout_date_ranges: Vec<String>, // Update later
    pub capacity: Capacity,
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
    pub images: Option<Vec<String>>,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
}

// Populated version of the DayItem enum
#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(tag = "type")]
pub enum PopulatedDayItem {
    #[serde(rename = "transportation")]
    Transportation {
        time: NaiveTime,
        location: ItemLocation,
        name: String,
    },

    #[serde(rename = "activity")]
    Activity {
        time: NaiveTime,
        #[serde(flatten)]
        activity: ActivityModel,
    },

    #[serde(rename = "accommodation")]
    Accommodation {
        time: NaiveTime,
        #[serde(flatten)]
        accommodation: AccommodationModel,
    },
}

// Populated version using composition for minimal maintenance
#[derive(Debug, Clone)]
pub struct PopulatedFeaturedVacation {
    // Reuse the original struct rather than duplicating all fields
    pub base: FeaturedVacation,
    pub populated_days: HashMap<String, Vec<PopulatedDayItem>>,
}

// Custom serialization to handle the composition
impl Serialize for PopulatedFeaturedVacation {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        // Create a serialization struct with all the fields
        let mut state = serializer.serialize_struct("PopulatedFeaturedVacation", 16)?;

        // Serialize all base fields
        state.serialize_field("_id", &self.base.id)?;
        state.serialize_field("fareharbor_id", &self.base.fareharbor_id)?;
        state.serialize_field("trip_name", &self.base.trip_name)?;
        state.serialize_field("person_cost", &self.base.person_cost)?;
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

        // Serialize the populated days
        state.serialize_field("days", &self.populated_days)?;

        state.end()
    }
}

// Helper method to convert from base to populated version
impl PopulatedFeaturedVacation {
    pub fn from_base(
        base: FeaturedVacation,
        populated_days: HashMap<String, Vec<PopulatedDayItem>>,
    ) -> Self {
        Self {
            base,
            populated_days,
        }
    }
    pub fn id(&self) -> Option<ObjectId> {
        self.base.id
    }

    pub fn trip_name(&self) -> &str {
        &self.base.trip_name
    }
}
