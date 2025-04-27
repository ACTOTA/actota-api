use core::fmt;

use bson::Timestamp;
use chrono::{DateTime, Utc};
use mongodb::bson::oid::ObjectId;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
pub enum BookingStatus {
    #[serde(rename = "pending")]
    Pending, // Initial state: booking requested but not yet confirmed

    #[serde(rename = "confirmed")]
    Confirmed, // Booking is confirmed by suppliers but not paid

    #[serde(rename = "paid")]
    Paid, // Payment has been successfully processed

    #[serde(rename = "ongoing")]
    Ongoing, // Trip is currently in progress

    #[serde(rename = "completed")]
    Completed, // Trip has successfully concluded

    #[serde(rename = "cancelled")]
    Cancelled, // Booking was cancelled (NOT refunded)

    #[serde(rename = "refunded")]
    Refunded, // Booking was cancelled and payment refunded
}

impl fmt::Display for BookingStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BookingStatus::Pending => write!(f, "pending"),
            BookingStatus::Confirmed => write!(f, "confirmed"),
            BookingStatus::Paid => write!(f, "paid"),
            BookingStatus::Ongoing => write!(f, "ongoing"),
            BookingStatus::Completed => write!(f, "completed"),
            BookingStatus::Cancelled => write!(f, "cancelled"),
            BookingStatus::Refunded => write!(f, "refunded"),
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Booking {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,
    pub user_id: ObjectId,
    pub itinerary_id: ObjectId,
    pub start_datetime: Option<DateTime<Utc>>,
    pub status: String,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
}
