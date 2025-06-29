use chrono::{DateTime, NaiveDate, Utc};
use mongodb::bson::oid::ObjectId;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct Favorite {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,
    pub user_id: ObjectId,
    pub itinerary_id: ObjectId,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct PersonalInformation {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,
    pub email: Option<String>,
    pub password: Option<String>, // Always hashed
    pub customer_id: Option<String>,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub phone_number: Option<String>,
    pub birth_date: Option<NaiveDate>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Notification {
    pub account_activities: bool,
    pub reminders: bool,
    pub travel_tips: bool,
    pub special_offers: bool,
    pub newsletter: bool,
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum UserRole {
    #[serde(rename = "user")]
    User,
    #[serde(rename = "admin")]
    Admin,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct User {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,
    pub email: String,
    pub password: String, // Always hashed
    pub customer_id: Option<String>,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub phone_number: Option<String>,
    pub birth_date: Option<NaiveDate>,
    pub profile_picture: Option<String>, // URL to the profile picture in Google Cloud Storage
    // Security related fields
    pub last_signin: Option<DateTime<Utc>>,
    pub last_signin_ip: Option<String>,
    pub failed_signins: Option<i32>,
    // Permission field
    pub role: Option<UserRole>,
    // We always want these fields, but have them optional so we can set them in the code
    pub notification: Option<Notification>,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
}
