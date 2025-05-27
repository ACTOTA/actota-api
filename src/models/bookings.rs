use bson::DateTime;
use chrono::{TimeZone, Utc};
use mongodb::bson::oid::ObjectId;
use serde::{Deserialize, Deserializer, Serialize};
use std::str::FromStr;

// A flexible date parser that attempts to parse various date formats
fn flexible_date_parser<'de, D>(deserializer: D) -> Result<DateTime, D::Error>
where
    D: Deserializer<'de>,
{
    use serde::de::Error;

    // First, try to deserialize as a string
    let date_str = String::deserialize(deserializer)?;

    // Try to parse as a standard DateTime first (MongoDB/BSON expected format)
    if let Ok(datetime) = DateTime::parse_rfc3339_str(&date_str) {
        return Ok(datetime);
    }

    // If that fails, try parsing with chrono and convert to DateTime
    if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(&date_str) {
        return Ok(DateTime::from_millis(dt.timestamp_millis()));
    }

    // Try with different formats using chrono
    let formats = [
        "%Y-%m-%dT%H:%M:%S%.fZ",   // ISO 8601 with fractional seconds
        "%Y-%m-%dT%H:%M:%SZ",      // ISO 8601 without fractional seconds
        "%Y-%m-%dT%H:%M:%S%.f%:z", // ISO 8601 with timezone offset
        "%Y-%m-%dT%H:%M:%S%:z",    // ISO 8601 with timezone offset, no fractional seconds
        "%Y-%m-%d %H:%M:%S",       // Simple datetime format
        "%Y-%m-%d",                // Just date
    ];

    for format in formats {
        if let Ok(dt) = chrono::NaiveDateTime::parse_from_str(&date_str, format) {
            let utc_dt = Utc.from_utc_datetime(&dt);
            return Ok(DateTime::from_millis(utc_dt.timestamp_millis()));
        }
    }

    // If we still can't parse it, try just the date format
    if let Ok(date) = chrono::NaiveDate::parse_from_str(&date_str, "%Y-%m-%d") {
        let datetime = date.and_hms_opt(0, 0, 0).unwrap(); // Default to midnight
        let utc_dt = Utc.from_utc_datetime(&datetime);
        return Ok(DateTime::from_millis(utc_dt.timestamp_millis()));
    }

    // If all parsing attempts fail
    Err(Error::custom(format!("Could not parse date: {}", date_str)))
}

#[derive(Serialize, Deserialize, Debug)]
pub struct BookingInput {
    #[serde(deserialize_with = "flexible_date_parser")]
    pub arrival_datetime: DateTime,
    
    #[serde(deserialize_with = "flexible_date_parser")]
    pub departure_datetime: DateTime,
    
    pub customer_id: Option<String>,
    pub transaction_id: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct BookingWithPaymentInput {
    // Booking fields
    #[serde(deserialize_with = "flexible_date_parser")]
    pub arrival_datetime: DateTime,

    #[serde(deserialize_with = "flexible_date_parser")]
    pub departure_datetime: DateTime,

    pub customer_id: String,
    // Payment fields
    pub payment_intent_id: String,
    pub amount: Option<i64>,
    pub description: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct BookingDetails {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,
    pub user_id: ObjectId,
    pub itinerary_id: ObjectId,
    pub customer_id: Option<String>,
    pub transaction_id: Option<String>,
    pub arrival_datetime: DateTime,
    pub departure_datetime: DateTime,
    pub status: String,
    pub bookings: Option<Vec<SingleBooking>>,
    pub created_at: Option<DateTime>,
    pub updated_at: Option<DateTime>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct SingleBooking {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,
    pub platform: String, // Booking platform/API
    pub item_id: String,
    pub status: String,
    pub created_at: Option<DateTime>,
    pub updated_at: Option<DateTime>,
}
