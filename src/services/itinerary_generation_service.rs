use crate::models::{
    activity::Activity,
    itinerary::base::{DayItem, FeaturedVacation, ItemLocation, Location},
    search::SearchItinerary,
};
use crate::services::distance_service::{DistanceService, TravelMode};
use crate::services::route_optimization_service::RouteOptimizationService;
use chrono::{Duration, NaiveTime};
use mongodb::{bson::oid::ObjectId, Client, Collection};
use std::{collections::HashMap, sync::Arc};

const DEFAULT_TRAVEL_TIME_BUFFER: f32 = 0.05; // 5% buffer
const MINUTES_PER_MILE: f32 = 2.0; // Rough estimate for driving time
const MIN_TIME_BETWEEN_ACTIVITIES_MINUTES: i64 = 30;
const MAX_ACTIVITIES_PER_DAY: usize = 4;

#[derive(Clone)]
pub struct ItineraryGenerationConfig {
    pub travel_time_buffer: f32,
    pub minutes_per_mile: f32,
    pub min_time_between_activities: i64,
    pub max_activities_per_day: usize,
}

impl Default for ItineraryGenerationConfig {
    fn default() -> Self {
        Self {
            travel_time_buffer: DEFAULT_TRAVEL_TIME_BUFFER,
            minutes_per_mile: MINUTES_PER_MILE,
            min_time_between_activities: MIN_TIME_BETWEEN_ACTIVITIES_MINUTES,
            max_activities_per_day: MAX_ACTIVITIES_PER_DAY,
        }
    }
}

pub struct ItineraryGenerator {
    client: Arc<Client>,
    config: ItineraryGenerationConfig,
    distance_service: Option<DistanceService>,
    route_optimizer: RouteOptimizationService,
}

impl ItineraryGenerator {
    pub fn new(client: Arc<Client>) -> Self {
        // Try to initialize distance service, but don't fail if Google Maps API key is missing
        let distance_service = match DistanceService::new(client.clone()) {
            Ok(service) => {
                println!("DistanceService initialized with Google Maps API");
                Some(service)
            }
            Err(e) => {
                println!("DistanceService not available: {}. Using fallback distance calculation.", e);
                None
            }
        };

        // Create a new distance service for the route optimizer if available
        let optimizer_distance_service = if distance_service.is_some() {
            DistanceService::new(client.clone()).ok()
        } else {
            None
        };
        let route_optimizer = RouteOptimizationService::new(optimizer_distance_service);

        Self {
            client,
            config: ItineraryGenerationConfig::default(),
            distance_service,
            route_optimizer,
        }
    }

    pub fn with_config(client: Arc<Client>, config: ItineraryGenerationConfig) -> Self {
        let distance_service = match DistanceService::new(client.clone()) {
            Ok(service) => Some(service),
            Err(_) => None,
        };

        // Create a new distance service for the route optimizer if available
        let optimizer_distance_service = if distance_service.is_some() {
            DistanceService::new(client.clone()).ok()
        } else {
            None
        };
        let route_optimizer = RouteOptimizationService::new(optimizer_distance_service);

        Self { 
            client, 
            config,
            distance_service,
            route_optimizer,
        }
    }

    /// Generate a new itinerary based on search parameters
    pub async fn generate_itinerary(
        &self,
        search_params: &SearchItinerary,
    ) -> Result<FeaturedVacation, Box<dyn std::error::Error>> {
        // Get available activities and locations
        let activities = self.fetch_matching_activities(search_params).await?;
        let locations = self.fetch_matching_locations(search_params).await?;

        if activities.is_empty() {
            return Err("No matching activities found".into());
        }

        if locations.is_empty() {
            return Err("No matching locations found".into());
        }

        // Calculate trip duration
        let arrival_str = search_params
            .arrival_datetime
            .as_ref()
            .ok_or("Arrival datetime required")?;
        let departure_str = search_params
            .departure_datetime
            .as_ref()
            .ok_or("Departure datetime required")?;

        // Parse date strings to calculate duration
        let arrival_date = chrono::NaiveDateTime::parse_from_str(arrival_str, "%Y-%m-%d %H:%M:%S")
            .or_else(|_| chrono::NaiveDateTime::parse_from_str(arrival_str, "%Y-%m-%dT%H:%M:%S"))
            .map_err(|e| format!("Failed to parse arrival date: {}", e))?;
        let departure_date = chrono::NaiveDateTime::parse_from_str(departure_str, "%Y-%m-%d %H:%M:%S")
            .or_else(|_| chrono::NaiveDateTime::parse_from_str(departure_str, "%Y-%m-%dT%H:%M:%S"))
            .map_err(|e| format!("Failed to parse departure date: {}", e))?;

        let trip_duration_days = (departure_date - arrival_date).num_days() as u32;

        // Select start and end locations
        let (start_location, end_location) = self.select_locations(&locations, search_params);

        // Generate day-by-day itinerary
        let days = self.generate_daily_schedules(
            &activities,
            &start_location,
            &end_location,
            trip_duration_days,
            search_params,
        ).await?;

        // Calculate total cost
        let person_cost = self.calculate_total_cost(&days, &activities);

        // Create the featured vacation with targeted naming for high match scores
        let trip_name = self.generate_targeted_trip_name(&start_location, &end_location, search_params);
        let description = self.generate_targeted_description(&activities, &start_location, &end_location, search_params);
        
        let generated_itinerary = FeaturedVacation {
            id: Some(ObjectId::new()),
            fareharbor_id: None,
            trip_name,
            min_age: None,
            min_group: search_params.adults.unwrap_or(1),
            max_group: search_params.adults.unwrap_or(1)
                + search_params.children.unwrap_or(0)
                + search_params.infants.unwrap_or(0),
            length_days: trip_duration_days,
            length_hours: trip_duration_days * 24,
            start_location: start_location.clone(),
            end_location: end_location.clone(),
            description,
            days: crate::models::itinerary::base::Days { days },
            images: None,
            arrival_datetime: Some(mongodb::bson::DateTime::from_millis(arrival_date.and_utc().timestamp_millis())),
            departure_datetime: Some(mongodb::bson::DateTime::from_millis(departure_date.and_utc().timestamp_millis())),
            adults: search_params.adults,
            children: search_params.children,
            infants: search_params.infants,
            pets: Some(0), // Default to 0 as pets field not in SearchItinerary
            lodging: search_params.lodging.clone(),
            transportation: search_params.transportation.clone(),
            person_cost: Some(person_cost as f64),
            created_at: Some(mongodb::bson::DateTime::now()),
            updated_at: Some(mongodb::bson::DateTime::now()),
            tag: Some("generated".to_string()),
        };

        Ok(generated_itinerary)
    }

    /// Fetch activities that match the search criteria with smart matching and fallbacks
    async fn fetch_matching_activities(
        &self,
        search_params: &SearchItinerary,
    ) -> Result<Vec<Activity>, mongodb::error::Error> {
        let collection: Collection<Activity> = self
            .client
            .database("Itineraries")
            .collection("Activities");

        // First try: exact and synonym matching
        if let Some(exact_matches) = self.try_exact_activity_matching(&collection, search_params).await? {
            if !exact_matches.is_empty() {
                println!("Found {} exact/synonym matches for generation", exact_matches.len());
                return Ok(exact_matches);
            }
        }

        // Second try: partial matching (any of the search terms)
        if let Some(partial_matches) = self.try_partial_activity_matching(&collection, search_params).await? {
            if !partial_matches.is_empty() {
                println!("Found {} partial matches for generation", partial_matches.len());
                return Ok(partial_matches);
            }
        }

        // Third try: location-based activities (closest matches)
        if let Some(location_matches) = self.try_location_based_matching(&collection, search_params).await? {
            if !location_matches.is_empty() {
                println!("Found {} location-based matches for generation", location_matches.len());
                return Ok(location_matches);
            }
        }

        // Final fallback: any activities for generation
        let fallback_filter = mongodb::bson::doc! {};
        let cursor = collection.find(fallback_filter).limit(10).await?;
        let fallback_activities: Vec<Activity> = cursor.try_collect().await?;
        
        println!("Using {} fallback activities for generation", fallback_activities.len());
        Ok(fallback_activities)
    }

    /// Try exact matching with synonyms (strictest matching)
    async fn try_exact_activity_matching(
        &self,
        collection: &Collection<Activity>,
        search_params: &SearchItinerary,
    ) -> Result<Option<Vec<Activity>>, mongodb::error::Error> {
        let mut filter = mongodb::bson::doc! {};

        // Filter by location if provided
        if let Some(locations) = &search_params.locations {
            if !locations.is_empty() {
                let cities: Vec<String> = locations
                    .iter()
                    .map(|l| l.split(',').next().unwrap_or("").trim().to_string())
                    .collect();

                filter.insert("address.city", mongodb::bson::doc! { "$in": cities });
            }
        }

        // Enhanced activity filtering with synonyms and partial matching
        if let Some(search_activities) = &search_params.activities {
            if !search_activities.is_empty() {
                let mut activity_filters = Vec::new();
                
                for search_activity in search_activities {
                    let search_term = search_activity.to_lowercase();
                    let synonyms = self.get_activity_synonyms(&search_term);
                    
                    // Create regex patterns for partial matching
                    let mut terms = vec![search_term.clone()];
                    terms.extend(synonyms);
                    
                    let regex_patterns: Vec<mongodb::bson::Regex> = terms
                        .iter()
                        .map(|term| mongodb::bson::Regex {
                            pattern: format!(".*{}.*", term.replace(" ", ".*")),
                            options: "i".to_string(), // case insensitive
                        })
                        .collect();
                    
                    // Match against activity_types, tags, title, or description
                    activity_filters.push(mongodb::bson::doc! {
                        "$or": [
                            { "activity_types": { "$in": regex_patterns.clone() } },
                            { "tags": { "$in": regex_patterns.clone() } },
                            { "title": { "$in": regex_patterns.clone() } },
                            { "description": { "$in": regex_patterns } }
                        ]
                    });
                }
                
                // All search activities should match (AND logic)
                filter.insert("$and", activity_filters);
            }
        }

        let cursor = collection.find(filter).await?;
        let activities: Vec<Activity> = cursor.try_collect().await?;
        
        Ok(if activities.is_empty() { None } else { Some(activities) })
    }

    /// Try partial matching (any of the search terms match)
    async fn try_partial_activity_matching(
        &self,
        collection: &Collection<Activity>,
        search_params: &SearchItinerary,
    ) -> Result<Option<Vec<Activity>>, mongodb::error::Error> {
        let mut filter = mongodb::bson::doc! {};

        // Filter by location if provided
        if let Some(locations) = &search_params.locations {
            if !locations.is_empty() {
                let cities: Vec<String> = locations
                    .iter()
                    .map(|l| l.split(',').next().unwrap_or("").trim().to_string())
                    .collect();

                filter.insert("address.city", mongodb::bson::doc! { "$in": cities });
            }
        }

        // Partial activity matching - any search term matches (OR logic)
        if let Some(search_activities) = &search_params.activities {
            if !search_activities.is_empty() {
                let mut all_terms = Vec::new();
                
                for search_activity in search_activities {
                    let search_term = search_activity.to_lowercase();
                    let synonyms = self.get_activity_synonyms(&search_term);
                    
                    all_terms.push(search_term);
                    all_terms.extend(synonyms);
                }
                
                let regex_patterns: Vec<mongodb::bson::Regex> = all_terms
                    .iter()
                    .map(|term| mongodb::bson::Regex {
                        pattern: format!(".*{}.*", term.replace(" ", ".*")),
                        options: "i".to_string(),
                    })
                    .collect();
                
                // Match any of the terms (OR logic)
                filter.insert("$or", vec![
                    mongodb::bson::doc! { "activity_types": { "$in": regex_patterns.clone() } },
                    mongodb::bson::doc! { "tags": { "$in": regex_patterns.clone() } },
                    mongodb::bson::doc! { "title": { "$in": regex_patterns.clone() } },
                    mongodb::bson::doc! { "description": { "$in": regex_patterns } }
                ]);
            }
        }

        let cursor = collection.find(filter).limit(15).await?;
        let activities: Vec<Activity> = cursor.try_collect().await?;
        
        Ok(if activities.is_empty() { None } else { Some(activities) })
    }

    /// Try location-based matching (activities in the same area)
    async fn try_location_based_matching(
        &self,
        collection: &Collection<Activity>,
        search_params: &SearchItinerary,
    ) -> Result<Option<Vec<Activity>>, mongodb::error::Error> {
        let mut filter = mongodb::bson::doc! {};

        // Filter by location if provided
        if let Some(locations) = &search_params.locations {
            if !locations.is_empty() {
                let cities: Vec<String> = locations
                    .iter()
                    .map(|l| l.split(',').next().unwrap_or("").trim().to_string())
                    .collect();

                filter.insert("address.city", mongodb::bson::doc! { "$in": cities });
            }
        }

        let cursor = collection.find(filter).limit(10).await?;
        let activities: Vec<Activity> = cursor.try_collect().await?;
        
        Ok(if activities.is_empty() { None } else { Some(activities) })
    }

    /// Get activity synonyms for better matching
    fn get_activity_synonyms(&self, search_term: &str) -> Vec<String> {
        match search_term {
            "atving" | "atv" | "atvs" => vec![
                "quad".to_string(),
                "four wheeler".to_string(),
                "off road".to_string(),
                "off-road".to_string(),
                "4x4".to_string(),
                "all terrain vehicle".to_string(),
                "dirt bike".to_string(),
                "trail riding".to_string(),
            ],
            "hotsprings" | "hot springs" | "hot spring" => vec![
                "thermal".to_string(), 
                "spa".to_string(), 
                "mineral springs".to_string(), 
                "geothermal".to_string(), 
                "springs".to_string(),
                "natural springs".to_string(),
                "thermal baths".to_string(),
            ],
            "goldminetours" | "gold mine tours" | "gold mine" | "goldmine" => vec![
                "mining".to_string(),
                "mine tour".to_string(),
                "mining tour".to_string(),
                "historical mine".to_string(),
                "gold rush".to_string(),
                "underground tour".to_string(),
                "mine exploration".to_string(),
                "mining history".to_string(),
            ],
            "hiking" | "hike" | "hikes" => vec![
                "trail".to_string(), 
                "trek".to_string(), 
                "walking".to_string(), 
                "nature walk".to_string(), 
                "mountain".to_string(), 
                "wilderness".to_string()
            ],
            "skiing" | "ski" => vec![
                "slope".to_string(), 
                "mountain resort".to_string(), 
                "powder".to_string(), 
                "alpine".to_string()
            ],
            "rafting" | "raft" => vec![
                "river".to_string(), 
                "whitewater".to_string(), 
                "rapids".to_string(), 
                "float".to_string()
            ],
            "climbing" | "climb" => vec![
                "rock climbing".to_string(), 
                "bouldering".to_string(), 
                "mountaineering".to_string()
            ],
            "fishing" | "fish" => vec![
                "angling".to_string(), 
                "fly fishing".to_string(), 
                "catch".to_string()
            ],
            "biking" | "bike" | "cycling" => vec![
                "bicycle".to_string(), 
                "mountain bike".to_string(), 
                "trail ride".to_string()
            ],
            "kayaking" | "kayak" => vec![
                "paddle".to_string(), 
                "paddling".to_string(), 
                "water sports".to_string()
            ],
            "camping" | "camp" => vec![
                "campground".to_string(), 
                "outdoor".to_string(), 
                "tent".to_string(), 
                "rv".to_string()
            ],
            "wildlife" => vec![
                "animals".to_string(), 
                "safari".to_string(), 
                "nature viewing".to_string(), 
                "bird watching".to_string()
            ],
            _ => vec![],
        }
    }

    /// Fetch locations that match the search criteria
    async fn fetch_matching_locations(
        &self,
        search_params: &SearchItinerary,
    ) -> Result<Vec<Location>, mongodb::error::Error> {
        // For now, create default locations based on search parameters
        // In a real implementation, you would fetch from database
        let mut locations = Vec::new();
        
        if let Some(search_locations) = &search_params.locations {
            for location_str in search_locations {
                let parts: Vec<&str> = location_str.split(',').map(|s| s.trim()).collect();
                if !parts.is_empty() {
                    // Create a default Location - this is a workaround
                    // In production, you'd fetch actual Location objects from DB
                    let default_location = Location::default();
                    locations.push(default_location);
                }
            }
        }
        
        // If no locations specified, add a default one
        if locations.is_empty() {
            locations.push(Location::default());
        }
        
        Ok(locations)
    }

    /// Select start and end locations based on search parameters
    fn select_locations(
        &self,
        locations: &[Location],
        _search_params: &SearchItinerary,
    ) -> (Location, Location) {
        // For now, use the first location as both start and end
        // In a more sophisticated implementation, you might:
        // - Use different locations for multi-city trips
        // - Consider user preferences
        // - Optimize based on activity locations
        let default_location = locations.first().cloned().unwrap_or_default();
        (default_location.clone(), default_location)
    }

    /// Generate daily schedules with activities
    async fn generate_daily_schedules(
        &self,
        activities: &[Activity],
        start_location: &Location,
        end_location: &Location,
        trip_duration_days: u32,
        _search_params: &SearchItinerary,
    ) -> Result<HashMap<String, Vec<DayItem>>, Box<dyn std::error::Error>> {
        let mut days = HashMap::new();

        for day_num in 1..=trip_duration_days {
            let day_key = format!("day{}", day_num);
            let mut day_items = Vec::new();

            // Add morning transportation/check-in for first day
            if day_num == 1 {
                day_items.push(DayItem::Transportation {
                    time: "09:00:00".to_string(),
                    location: ItemLocation {
                        name: format!("{}, {}", start_location.city(), start_location.state()),
                        coordinates: vec![start_location.coordinates().0 as f64, start_location.coordinates().1 as f64],
                    },
                    name: "Arrival and Check-in".to_string(),
                });
            }

            // Use route optimization for activity scheduling
            let day_activities: Vec<Activity> = activities.iter()
                .take(self.config.max_activities_per_day)
                .cloned()
                .collect();

            if !day_activities.is_empty() {
                let starting_location = (
                    start_location.coordinates().0 as f64,
                    start_location.coordinates().1 as f64,
                );

                match self.route_optimizer.optimize_daily_route(
                    day_activities.clone(),
                    starting_location,
                    day_num == 1,
                    day_num == trip_duration_days,
                ).await {
                    Ok(optimized_activities) => {
                        // Convert optimized activities to DayItems
                        for optimized in optimized_activities {
                            day_items.push(DayItem::Activity {
                                time: optimized.scheduled_time.format("%H:%M:%S").to_string(),
                                activity_id: optimized.activity.id.unwrap_or_else(ObjectId::new),
                            });
                        }
                        
                        println!("Day {} optimized route: {} activities scheduled", 
                            day_num, day_items.len() - if day_num == 1 { 1 } else { 0 });
                    }
                    Err(e) => {
                        eprintln!("Route optimization failed for day {}: {}. Using fallback.", day_num, e);
                        // Fallback to simple scheduling if optimization fails
                        self.fallback_schedule_activities(&day_activities, &mut day_items).await?;
                    }
                }
            }

            // Add evening transportation/check-out for last day
            if day_num == trip_duration_days {
                day_items.push(DayItem::Transportation {
                    time: "17:00:00".to_string(),
                    location: ItemLocation {
                        name: format!("{}, {}", end_location.city(), end_location.state()),
                        coordinates: vec![end_location.coordinates().0 as f64, end_location.coordinates().1 as f64],
                    },
                    name: "Check-out and Departure".to_string(),
                });
            }

            days.insert(day_key, day_items);
        }

        Ok(days)
    }

    /// Fallback activity scheduling when route optimization fails
    async fn fallback_schedule_activities(
        &self,
        activities: &[Activity],
        day_items: &mut Vec<DayItem>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        println!("Using fallback scheduling for {} activities", activities.len());
        
        let current_time = NaiveTime::from_hms_opt(10, 0, 0).unwrap();
        
        // Simple scheduling: just add activities with 2-hour spacing
        for (i, activity) in activities.iter().take(3).enumerate() {
            let scheduled_time = current_time + Duration::hours(i as i64 * 2);
            
            day_items.push(DayItem::Activity {
                time: scheduled_time.format("%H:%M:%S").to_string(),
                activity_id: activity.id.unwrap_or_else(ObjectId::new),
            });
        }
        
        Ok(())
    }

    /// Find the best available time slot for an activity
    fn find_best_time_slot(&self, _activity: &Activity, preferred_start: NaiveTime) -> Option<String> {
        // For now, return a default time based on preferred start
        // In a real implementation, you would parse the activity.daily_time_slots
        // and find the best match
        Some(preferred_start.format("%H:%M:%S").to_string())
    }

    /// Calculate the total cost per person for the itinerary
    fn calculate_total_cost(
        &self,
        days: &HashMap<String, Vec<DayItem>>,
        activities: &[Activity],
    ) -> f32 {
        let mut total_cost = 0.0;

        // Create a map of activity IDs to costs for quick lookup
        let activity_costs: HashMap<ObjectId, f32> = activities
            .iter()
            .filter_map(|a| a.id.map(|id| (id, a.price_per_person)))
            .collect();

        // Sum up all activity costs
        for day_items in days.values() {
            for item in day_items {
                if let DayItem::Activity { activity_id, .. } = item {
                    if let Some(cost) = activity_costs.get(activity_id) {
                        total_cost += cost;
                    }
                }
            }
        }

        total_cost
    }

    /// Generate a targeted trip name that includes search terms for high match scores
    fn generate_targeted_trip_name(
        &self,
        start_location: &Location,
        end_location: &Location,
        search_params: &SearchItinerary,
    ) -> String {
        let mut name_parts = Vec::new();
        
        // Add location
        if start_location.city() == end_location.city() {
            name_parts.push(start_location.city().to_string());
        } else {
            name_parts.push(format!("{} to {}", start_location.city(), end_location.city()));
        }
        
        // Add search activities for better matching
        if let Some(activities) = &search_params.activities {
            if !activities.is_empty() {
                let activity_str = if activities.len() == 1 {
                    activities[0].clone()
                } else if activities.len() == 2 {
                    format!("{} and {}", activities[0], activities[1])
                } else {
                    format!("{}, {}, and more", activities[0], activities[1])
                };
                name_parts.push(activity_str);
            }
        }
        
        name_parts.push("Adventure".to_string());
        name_parts.join(" ")
    }

    /// Generate a targeted description that includes search terms for high match scores
    fn generate_targeted_description(
        &self,
        activities: &[Activity],
        start_location: &Location,
        end_location: &Location,
        search_params: &SearchItinerary,
    ) -> String {
        let mut description_parts = Vec::new();
        
        // Start with location
        if start_location.city() == end_location.city() {
            description_parts.push(format!("Discover the best of {} with this expertly crafted itinerary", start_location.city()));
        } else {
            description_parts.push(format!("Experience an unforgettable journey from {} to {}", start_location.city(), end_location.city()));
        }
        
        // Highlight search activities
        if let Some(search_activities) = &search_params.activities {
            if !search_activities.is_empty() {
                description_parts.push(format!("featuring {} activities", search_activities.join(", ")));
            }
        }
        
        // Add actual activity types from found activities
        let activity_types: Vec<String> = activities
            .iter()
            .flat_map(|a| a.activity_types.clone())
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .take(3)
            .collect();
        
        if !activity_types.is_empty() {
            description_parts.push(format!("including {}", activity_types.join(", ")));
        }
        
        description_parts.push("This carefully crafted itinerary combines adventure, relaxation, and local experiences to create memories that will last a lifetime.".to_string());
        
        description_parts.join(". ")
    }

    /// Generate a description for the itinerary (legacy method)
    fn generate_description(
        &self,
        activities: &[Activity],
        start_location: &Location,
        end_location: &Location,
    ) -> String {
        let activity_types: Vec<String> = activities
            .iter()
            .flat_map(|a| a.activity_types.clone())
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .take(3)
            .collect();

        format!(
            "Experience an unforgettable journey from {} to {}, featuring exciting activities including {}. This carefully crafted itinerary combines adventure, relaxation, and local experiences to create memories that will last a lifetime.",
            start_location.city(),
            end_location.city(),
            activity_types.join(", ")
        )
    }

    /// Get coordinates for an activity (placeholder implementation)
    fn get_activity_coordinates(&self, activity: &Activity) -> (f64, f64) {
        // In a real implementation, you would:
        // 1. Use a geocoding service to convert address to coordinates
        // 2. Store coordinates in the database with activities
        // 3. Cache geocoding results
        
        // For now, return placeholder coordinates based on activity ID hash
        // This ensures consistent but varied coordinates for different activities
        let id_string = activity.id.map(|id| id.to_string()).unwrap_or_default();
        let hash = id_string.chars().map(|c| c as u32).sum::<u32>();
        
        // Generate coordinates within a reasonable range (e.g., continental US)
        let lat = 25.0 + ((hash % 2000) as f64 / 100.0); // 25.0 to 45.0
        let lon = -125.0 + ((hash % 5000) as f64 / 100.0); // -125.0 to -75.0
        
        (lat, lon)
    }

    /// Calculate travel time between two coordinates with buffer (fallback method)
    fn calculate_travel_time_fallback(&self, from: (f64, f64), to: (f64, f64)) -> i64 {
        // Calculate distance using Haversine formula
        let distance_miles = self.haversine_distance(from, to);
        
        // Calculate base travel time
        let base_time_minutes = (distance_miles * self.config.minutes_per_mile as f64) as i64;
        
        // Add buffer
        let buffer_minutes = (base_time_minutes as f32 * self.config.travel_time_buffer) as i64;
        
        // Ensure minimum time between activities
        let total_time = base_time_minutes + buffer_minutes;
        total_time.max(self.config.min_time_between_activities)
    }

    /// Get travel time using Google Maps API (public method for external use)
    pub async fn calculate_travel_time(
        &self,
        from: (f64, f64),
        to: (f64, f64),
        with_traffic: bool,
    ) -> Result<i64, Box<dyn std::error::Error>> {
        if let Some(ref distance_service) = self.distance_service {
            let result = distance_service.get_distance(
                from,
                to,
                TravelMode::Driving,
                with_traffic,
            ).await?;
            
            let base_time = result.duration_in_traffic_minutes.unwrap_or(result.duration_minutes) as i64;
            let buffer_time = (base_time as f32 * self.config.travel_time_buffer) as i64;
            Ok((base_time + buffer_time).max(self.config.min_time_between_activities))
        } else {
            Ok(self.calculate_travel_time_fallback(from, to))
        }
    }

    /// Calculate distance between two points using Haversine formula
    fn haversine_distance(&self, from: (f64, f64), to: (f64, f64)) -> f64 {
        const EARTH_RADIUS_MILES: f64 = 3959.0;
        
        let lat1_rad = from.0.to_radians();
        let lat2_rad = to.0.to_radians();
        let delta_lat = (to.0 - from.0).to_radians();
        let delta_lon = (to.1 - from.1).to_radians();
        
        let a = (delta_lat / 2.0).sin().powi(2)
            + lat1_rad.cos() * lat2_rad.cos() * (delta_lon / 2.0).sin().powi(2);
        let c = 2.0 * a.sqrt().atan2((1.0 - a).sqrt());
        
        EARTH_RADIUS_MILES * c
    }
}

use futures::TryStreamExt;