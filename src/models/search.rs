use bson::oid::ObjectId;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SearchItinerary {
    pub id: Option<ObjectId>,
    pub user_id: Option<ObjectId>,
    pub locations: Option<Vec<String>>,
    pub arrival_datetime: Option<String>,
    pub departure_datetime: Option<String>,
    pub adults: Option<u32>,
    pub children: Option<u32>,
    pub infants: Option<u32>,
    pub activities: Option<Vec<String>>,
    pub lodging: Option<Vec<String>>,
    pub transportation: Option<String>,
}
