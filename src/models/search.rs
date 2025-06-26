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
    pub trip_pace: Option<TripPace>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "lowercase")]
pub enum TripPace {
    Relaxed,
    Moderate,
    Adventure,
}

impl TripPace {
    /// Get the maximum number of activity hours per day for this pace
    pub fn max_activity_hours_per_day(&self) -> f32 {
        match self {
            TripPace::Relaxed => 4.0,    // 4 hours of activities per day
            TripPace::Moderate => 6.0,   // 6 hours of activities per day
            TripPace::Adventure => 10.0, // 10 hours of activities per day
        }
    }
    
    /// Get the typical number of activities per day for this pace
    pub fn typical_activities_per_day(&self) -> usize {
        match self {
            TripPace::Relaxed => 2,    // 1-2 activities per day
            TripPace::Moderate => 3,   // 2-3 activities per day
            TripPace::Adventure => 5,  // 4-5 activities per day
        }
    }
}
