use bson::DateTime;
use mongodb::bson::oid::ObjectId;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct BookingInput {
    pub arrival_datetime: DateTime,
    pub departure_datetime: DateTime,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct BookingDetails {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,
    pub user_id: ObjectId,
    pub itinerary_id: ObjectId,
    pub arrival_datetime: DateTime,
    pub departure_datetime: DateTime,
    pub status: String,
    pub bookings: Option<Vec<SingleBooking>>,
    pub created_at: Option<DateTime>,
    pub updated_at: Option<DateTime>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct SingleBooking {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,
    pub platform: String, // Booking platform/API
    pub item_id: String,
    pub created_at: Option<DateTime>,
    pub updated_at: Option<DateTime>,
}
