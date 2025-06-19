//! Route Optimization Service
//!
//! This service optimizes the order of activities within a day to minimize travel time
//! and avoid inefficient back-and-forth routing. It considers traffic patterns and
//! uses the Google Maps Distance Service for accurate travel time calculations.
//!
//! ## Features
//! - Optimizes activity order to minimize total travel time
//! - Considers traffic patterns for different times of day
//! - Prevents inefficient back-and-forth routing
//! - Uses Google Maps real driving times with traffic
//! - Respects activity time constraints and availability
//! - Configurable optimization strategies

use crate::models::activity::Activity;
use crate::services::distance_service::{DistanceService, TravelMode};
use chrono::{Duration, NaiveTime};
use mongodb::bson::oid::ObjectId;

#[derive(Debug, Clone)]
pub struct OptimizedActivity {
    pub activity: Activity,
    pub scheduled_time: NaiveTime,
    pub travel_time_from_previous: Option<i64>, // minutes
    pub coordinates: (f64, f64),
}

#[derive(Debug, Clone)]
pub struct OptimizationConfig {
    pub max_activities_per_day: usize,
    pub min_time_between_activities: i64, // minutes
    pub travel_time_buffer: f32, // percentage (e.g., 0.05 for 5%)
    pub day_start_time: NaiveTime,
    pub day_end_time: NaiveTime,
    pub consider_traffic: bool,
    pub optimization_strategy: OptimizationStrategy,
}

#[derive(Debug, Clone)]
pub enum OptimizationStrategy {
    /// Minimize total travel time (good for most cases)
    MinimizeTotalTime,
    /// Start with activities closest to starting location
    NearestFirst,
    /// Optimize for specific time preferences (e.g., outdoor activities in morning)
    TimePreference,
}

impl Default for OptimizationConfig {
    fn default() -> Self {
        Self {
            max_activities_per_day: 4,
            min_time_between_activities: 30,
            travel_time_buffer: 0.05,
            day_start_time: NaiveTime::from_hms_opt(9, 0, 0).unwrap(),
            day_end_time: NaiveTime::from_hms_opt(17, 0, 0).unwrap(),
            consider_traffic: true,
            optimization_strategy: OptimizationStrategy::MinimizeTotalTime,
        }
    }
}

pub struct RouteOptimizationService {
    distance_service: Option<DistanceService>,
    config: OptimizationConfig,
}

impl RouteOptimizationService {
    pub fn new(distance_service: Option<DistanceService>) -> Self {
        Self {
            distance_service,
            config: OptimizationConfig::default(),
        }
    }

    pub fn with_config(distance_service: Option<DistanceService>, config: OptimizationConfig) -> Self {
        Self {
            distance_service,
            config,
        }
    }

    /// Optimize the order of activities for a single day
    pub async fn optimize_daily_route(
        &self,
        activities: Vec<Activity>,
        starting_location: (f64, f64),
        is_first_day: bool,
        is_last_day: bool,
    ) -> Result<Vec<OptimizedActivity>, Box<dyn std::error::Error>> {
        if activities.is_empty() {
            return Ok(Vec::new());
        }

        println!("Optimizing route for {} activities", activities.len());

        // Adjust day timing for first/last days
        let day_start = if is_first_day {
            NaiveTime::from_hms_opt(10, 0, 0).unwrap() // Later start on arrival day
        } else {
            self.config.day_start_time
        };

        let day_end = if is_last_day {
            NaiveTime::from_hms_opt(15, 0, 0).unwrap() // Earlier end on departure day
        } else {
            self.config.day_end_time
        };

        // Get coordinates for all activities
        let mut activity_coords: Vec<(Activity, (f64, f64))> = activities
            .into_iter()
            .map(|activity| {
                let coords = self.get_activity_coordinates(&activity);
                (activity, coords)
            })
            .collect();

        // Limit to max activities per day
        activity_coords.truncate(self.config.max_activities_per_day);

        // Choose optimization strategy
        let optimized_order = match self.config.optimization_strategy {
            OptimizationStrategy::MinimizeTotalTime => {
                self.optimize_for_minimal_travel_time(activity_coords, starting_location).await?
            }
            OptimizationStrategy::NearestFirst => {
                self.optimize_nearest_first(activity_coords, starting_location).await?
            }
            OptimizationStrategy::TimePreference => {
                self.optimize_for_time_preferences(activity_coords, starting_location).await?
            }
        };

        // Schedule the optimized activities within the day
        self.schedule_optimized_activities(optimized_order, starting_location, day_start, day_end).await
    }

    /// Traveling Salesman Problem (TSP) approach - minimize total travel time
    async fn optimize_for_minimal_travel_time(
        &self,
        activities: Vec<(Activity, (f64, f64))>,
        starting_location: (f64, f64),
    ) -> Result<Vec<(Activity, (f64, f64))>, Box<dyn std::error::Error>> {
        if activities.len() <= 1 {
            return Ok(activities);
        }

        println!("Using TSP optimization for {} activities", activities.len());

        // For small numbers of activities, use brute force or nearest neighbor
        if activities.len() <= 6 {
            self.tsp_brute_force(activities, starting_location).await
        } else {
            // For larger sets, use nearest neighbor heuristic
            self.tsp_nearest_neighbor(activities, starting_location).await
        }
    }

    /// Brute force TSP for small activity sets (≤6 activities)
    async fn tsp_brute_force(
        &self,
        activities: Vec<(Activity, (f64, f64))>,
        starting_location: (f64, f64),
    ) -> Result<Vec<(Activity, (f64, f64))>, Box<dyn std::error::Error>> {
        let n = activities.len();
        if n == 0 {
            return Ok(activities);
        }

        let mut best_order = activities.clone();
        let mut best_total_time = i64::MAX;

        // Generate all permutations and find the one with minimum travel time
        let indices: Vec<usize> = (0..n).collect();
        let permutations = self.generate_permutations(indices);

        for perm in permutations.iter().take(120) { // Limit to reasonable number for performance
            let mut total_time = 0i64;
            let mut current_location = starting_location;

            for &idx in perm {
                let activity_location = activities[idx].1;
                
                if let Some(travel_time) = self.get_travel_time(current_location, activity_location).await {
                    total_time += travel_time;
                    total_time += activities[idx].0.duration_minutes as i64;
                    current_location = activity_location;
                } else {
                    // Skip this permutation if we can't get travel time
                    total_time = i64::MAX;
                    break;
                }
            }

            if total_time < best_total_time {
                best_total_time = total_time;
                best_order = perm.iter().map(|&i| activities[i].clone()).collect();
            }
        }

        println!("Best route total time: {} minutes", best_total_time);
        Ok(best_order)
    }

    /// Nearest neighbor TSP heuristic for larger activity sets
    async fn tsp_nearest_neighbor(
        &self,
        activities: Vec<(Activity, (f64, f64))>,
        starting_location: (f64, f64),
    ) -> Result<Vec<(Activity, (f64, f64))>, Box<dyn std::error::Error>> {
        let mut unvisited = activities;
        let mut route = Vec::new();
        let mut current_location = starting_location;

        while !unvisited.is_empty() {
            let mut nearest_idx = 0;
            let mut nearest_time = i64::MAX;

            // Find nearest unvisited activity
            for (idx, (_, coords)) in unvisited.iter().enumerate() {
                if let Some(travel_time) = self.get_travel_time(current_location, *coords).await {
                    if travel_time < nearest_time {
                        nearest_time = travel_time;
                        nearest_idx = idx;
                    }
                }
            }

            // Add nearest activity to route
            let nearest_activity = unvisited.remove(nearest_idx);
            current_location = nearest_activity.1;
            route.push(nearest_activity);
        }

        println!("Nearest neighbor route completed with {} activities", route.len());
        Ok(route)
    }

    /// Simple nearest-first optimization
    async fn optimize_nearest_first(
        &self,
        activities: Vec<(Activity, (f64, f64))>,
        starting_location: (f64, f64),
    ) -> Result<Vec<(Activity, (f64, f64))>, Box<dyn std::error::Error>> {
        // This is the same as nearest neighbor for now
        self.tsp_nearest_neighbor(activities, starting_location).await
    }

    /// Optimize considering time preferences (morning outdoor activities, etc.)
    async fn optimize_for_time_preferences(
        &self,
        mut activities: Vec<(Activity, (f64, f64))>,
        starting_location: (f64, f64),
    ) -> Result<Vec<(Activity, (f64, f64))>, Box<dyn std::error::Error>> {
        // Sort by activity type preferences (outdoor activities first, indoor later)
        activities.sort_by(|a, b| {
            let a_outdoor = self.is_outdoor_activity(&a.0);
            let b_outdoor = self.is_outdoor_activity(&b.0);
            
            match (a_outdoor, b_outdoor) {
                (true, false) => std::cmp::Ordering::Less,  // Outdoor first
                (false, true) => std::cmp::Ordering::Greater, // Indoor later
                _ => std::cmp::Ordering::Equal,
            }
        });

        // Then apply nearest neighbor within those groups
        self.tsp_nearest_neighbor(activities, starting_location).await
    }

    /// Schedule optimized activities with proper timing
    async fn schedule_optimized_activities(
        &self,
        optimized_activities: Vec<(Activity, (f64, f64))>,
        starting_location: (f64, f64),
        day_start: NaiveTime,
        day_end: NaiveTime,
    ) -> Result<Vec<OptimizedActivity>, Box<dyn std::error::Error>> {
        let mut scheduled_activities = Vec::new();
        let mut current_time = day_start;
        let mut current_location = starting_location;

        for (activity, coords) in optimized_activities {
            // Calculate travel time to this activity
            let travel_time = if let Some(time) = self.get_travel_time(current_location, coords).await {
                time
            } else {
                self.config.min_time_between_activities // Fallback
            };

            // Apply buffer
            let buffered_travel_time = (travel_time as f32 * (1.0 + self.config.travel_time_buffer)) as i64;
            let final_travel_time = buffered_travel_time.max(self.config.min_time_between_activities);

            // Calculate when this activity would start
            let activity_start_time = current_time + Duration::minutes(final_travel_time);
            let activity_end_time = activity_start_time + Duration::minutes(activity.duration_minutes as i64);

            // Check if activity fits within the day
            if activity_end_time.signed_duration_since(day_end) <= Duration::zero() {
                scheduled_activities.push(OptimizedActivity {
                    activity: activity.clone(),
                    scheduled_time: activity_start_time,
                    travel_time_from_previous: Some(final_travel_time),
                    coordinates: coords,
                });

                current_time = activity_end_time;
                current_location = coords;
                
                println!("Scheduled activity '{}' at {} (travel: {}min)", 
                    activity.title, activity_start_time.format("%H:%M"), final_travel_time);
            } else {
                println!("Activity '{}' doesn't fit in remaining day time", activity.title);
                break;
            }
        }

        Ok(scheduled_activities)
    }

    /// Get travel time between two coordinates
    async fn get_travel_time(&self, from: (f64, f64), to: (f64, f64)) -> Option<i64> {
        if let Some(ref distance_service) = self.distance_service {
            match distance_service.get_distance(from, to, TravelMode::Driving, self.config.consider_traffic).await {
                Ok(result) => {
                    let time = if self.config.consider_traffic {
                        result.duration_in_traffic_minutes.unwrap_or(result.duration_minutes)
                    } else {
                        result.duration_minutes
                    };
                    Some(time as i64)
                }
                Err(e) => {
                    eprintln!("Error getting travel time: {}", e);
                    None
                }
            }
        } else {
            // Fallback to straight-line distance calculation
            Some(self.calculate_fallback_travel_time(from, to))
        }
    }

    /// Fallback travel time calculation using Haversine distance
    fn calculate_fallback_travel_time(&self, from: (f64, f64), to: (f64, f64)) -> i64 {
        const EARTH_RADIUS_MILES: f64 = 3959.0;
        const MINUTES_PER_MILE: f64 = 2.0;

        let lat1_rad = from.0.to_radians();
        let lat2_rad = to.0.to_radians();
        let delta_lat = (to.0 - from.0).to_radians();
        let delta_lon = (to.1 - from.1).to_radians();

        let a = (delta_lat / 2.0).sin().powi(2)
            + lat1_rad.cos() * lat2_rad.cos() * (delta_lon / 2.0).sin().powi(2);
        let c = 2.0 * a.sqrt().atan2((1.0 - a).sqrt());
        let distance_miles = EARTH_RADIUS_MILES * c;

        (distance_miles * MINUTES_PER_MILE) as i64
    }

    /// Get coordinates for an activity based on its address
    fn get_activity_coordinates(&self, activity: &Activity) -> (f64, f64) {
        // Try to geocode based on the activity's address
        let city = activity.address.city.to_lowercase();
        let state = activity.address.state.to_lowercase();
        
        // If we have a valid city and state, use known coordinates
        if !city.is_empty() && !state.is_empty() {
            // Common city coordinates (you can expand this)
            match (city.as_str(), state.as_str()) {
                ("denver", "co") | ("denver", "colorado") => (39.7392, -104.9903),
                ("colorado springs", "co") => (38.8339, -104.8214),
                ("boulder", "co") => (40.0150, -105.2705),
                ("aspen", "co") => (39.1911, -106.8175),
                ("vail", "co") => (39.6403, -106.3742),
                ("fort collins", "co") => (40.5853, -105.0844),
                ("grand junction", "co") => (39.0639, -108.5506),
                ("durango", "co") => (37.2753, -107.8801),
                ("steamboat springs", "co") => (40.4850, -106.8317),
                ("breckenridge", "co") => (39.4817, -106.0384),
                ("keystone", "co") => (39.5791, -105.9347),
                ("telluride", "co") => (37.9375, -107.8123),
                ("winter park", "co") => (39.8911, -105.7631),
                ("crested butte", "co") => (38.8697, -106.9878),
                ("estes park", "co") => (40.3772, -105.5217),
                ("glenwood springs", "co") => (39.5505, -107.3248),
                ("pagosa springs", "co") => (37.2694, -107.0098),
                ("salida", "co") => (38.5347, -106.0001),
                ("buena vista", "co") => (38.8422, -106.1312),
                ("leadville", "co") => (39.2508, -106.2925),
                _ => {
                    println!("Warning: Unknown city '{}', '{}' - using default Colorado coordinates", city, state);
                    (39.5501, -105.7821) // Central Colorado
                }
            }
        } else {
            // If no valid city/state, parse the full address
            let full_address = format!("{} {} {} {} {}", 
                activity.address.street, 
                activity.address.city, 
                activity.address.state, 
                activity.address.zip,
                activity.address.country
            ).trim().to_lowercase();
            
            // Try to extract city from the full address if individual fields are empty
            if full_address.contains("denver") { (39.7392, -104.9903) }
            else if full_address.contains("colorado springs") { (38.8339, -104.8214) }
            else if full_address.contains("boulder") { (40.0150, -105.2705) }
            else if full_address.contains("aspen") { (39.1911, -106.8175) }
            else if full_address.contains("vail") { (39.6403, -106.3742) }
            else if full_address.contains("breckenridge") { (39.4817, -106.0384) }
            else if full_address.contains("keystone") { (39.5791, -105.9347) }
            else if full_address.contains("telluride") { (37.9375, -107.8123) }
            else if full_address.contains("winter park") { (39.8911, -105.7631) }
            else if full_address.contains("crested butte") { (38.8697, -106.9878) }
            else if full_address.contains("estes park") { (40.3772, -105.5217) }
            else if full_address.contains("glenwood springs") { (39.5505, -107.3248) }
            else if full_address.contains("pagosa springs") { (37.2694, -107.0098) }
            else if full_address.contains("steamboat springs") { (40.4850, -106.8317) }
            else {
                println!("Warning: Could not determine coordinates from address '{}' - using default Colorado coordinates", full_address);
                (39.5501, -105.7821) // Central Colorado
            }
        }
    }

    /// Check if activity is likely outdoor (heuristic based on activity types/tags)
    fn is_outdoor_activity(&self, activity: &Activity) -> bool {
        let outdoor_keywords = ["outdoor", "hiking", "beach", "park", "nature", "scenic", "wildlife", "fishing", "camping"];
        
        activity.activity_types.iter().any(|t| 
            outdoor_keywords.iter().any(|keyword| 
                t.to_lowercase().contains(keyword)
            )
        ) || activity.tags.iter().any(|tag| 
            outdoor_keywords.iter().any(|keyword| 
                tag.to_lowercase().contains(keyword)
            )
        )
    }

    /// Generate permutations for small TSP instances
    fn generate_permutations(&self, items: Vec<usize>) -> Vec<Vec<usize>> {
        if items.len() <= 1 {
            return vec![items];
        }

        let mut result = Vec::new();
        for i in 0..items.len() {
            let mut remaining = items.clone();
            let current = remaining.remove(i);
            
            for mut perm in self.generate_permutations(remaining) {
                perm.insert(0, current);
                result.push(perm);
            }
        }
        
        result
    }

    /// Get optimization statistics for the route
    pub fn get_route_stats(&self, optimized_activities: &[OptimizedActivity]) -> RouteStats {
        let total_travel_time: i64 = optimized_activities
            .iter()
            .filter_map(|a| a.travel_time_from_previous)
            .sum();

        let total_activity_time: i64 = optimized_activities
            .iter()
            .map(|a| a.activity.duration_minutes as i64)
            .sum();

        let start_time = optimized_activities.first().map(|a| a.scheduled_time);
        let end_time = optimized_activities.last().map(|a| 
            a.scheduled_time + Duration::minutes(a.activity.duration_minutes as i64)
        );

        RouteStats {
            total_activities: optimized_activities.len(),
            total_travel_time_minutes: total_travel_time,
            total_activity_time_minutes: total_activity_time,
            total_day_time_minutes: total_travel_time + total_activity_time,
            start_time,
            end_time,
            efficiency_ratio: if total_travel_time + total_activity_time > 0 {
                total_activity_time as f32 / (total_travel_time + total_activity_time) as f32
            } else {
                0.0
            },
        }
    }
}

#[derive(Debug)]
pub struct RouteStats {
    pub total_activities: usize,
    pub total_travel_time_minutes: i64,
    pub total_activity_time_minutes: i64,
    pub total_day_time_minutes: i64,
    pub start_time: Option<NaiveTime>,
    pub end_time: Option<NaiveTime>,
    pub efficiency_ratio: f32, // Activity time / Total time
}