use mongodb::bson::{oid::ObjectId, DateTime};
use serde::{Deserialize, Deserializer, Serialize};

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct TimeSlot {
    pub start: String,
    pub end: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Address {
    pub street: String,
    pub unit: String,
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
pub struct BlackoutDateRange {
    start: i64,
    end: i64,
}

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

// Custom deserializer for optional u8 fields
fn deserialize_optional_rounded_u8<'de, D>(deserializer: D) -> Result<Option<u8>, D::Error>
where
    D: Deserializer<'de>,
{
    let value: Option<serde_json::Value> = Option::deserialize(deserializer)?;
    match value {
        None => Ok(None),
        Some(serde_json::Value::Number(n)) => {
            if let Some(f) = n.as_f64() {
                Ok(Some(f.ceil() as u8))
            } else if let Some(i) = n.as_u64() {
                Ok(Some(i as u8))
            } else {
                Ok(None)
            }
        }
        Some(serde_json::Value::Null) => Ok(None),
        _ => Ok(None),
    }
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
    #[serde(deserialize_with = "deserialize_optional_rounded_u16", default)]
    pub weight_limit_lbs: Option<u16>,
    #[serde(deserialize_with = "deserialize_optional_rounded_u8", default)]
    pub age_requirement: Option<u8>,
    #[serde(deserialize_with = "deserialize_optional_rounded_u8", default)]
    pub height_requiremnt: Option<u8>,
    pub blackout_date_ranges: Option<Vec<BlackoutDateRange>>,
    pub capacity: Capacity,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub created_at: Option<DateTime>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<DateTime>,
}
