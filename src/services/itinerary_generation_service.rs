use crate::models::{
    activity::Activity,
    itinerary::base::{DayItem, FeaturedVacation},
    search::SearchItinerary,
};
use crate::services::vertex_search_service::VertexSearchService;
use chrono::{Duration, NaiveTime};
use mongodb::{bson::oid::ObjectId, Client, Collection};
use std::{collections::HashMap, sync::Arc};

pub struct ItineraryGenerator {
    client: Arc<Client>,
    vertex_search_service: Option<VertexSearchService>,
}

impl ItineraryGenerator {
    pub fn new(client: Arc<Client>) -> Self {
        let vertex_search_service = match VertexSearchService::new() {
            Ok(service) => {
                println!("VertexSearchService initialized successfully");
                Some(service)
            }
            Err(e) => {
                println!("VertexSearchService not available: {}. Using MongoDB only.", e);
                None
            }
        };

        Self {
            client,
            vertex_search_service,
        }
    }

    /// Generate a new itinerary based on search parameters
    pub async fn generate_itinerary(
        &self,
        search_params: &SearchItinerary,
    ) -> Result<FeaturedVacation, Box<dyn std::error::Error>> {
        // Get activities and locations
        let activities = self.fetch_activities(search_params).await?;
        let locations = self.get_locations(search_params);

        if activities.is_empty() {
            return Err("No matching activities found".into());
        }

        // Calculate trip duration
        let arrival_str = search_params.arrival_datetime.as_ref().ok_or("Arrival datetime required")?;
        let departure_str = search_params.departure_datetime.as_ref().ok_or("Departure datetime required")?;
        
        let arrival_date = self.parse_datetime(arrival_str)?;
        let departure_date = self.parse_datetime(departure_str)?;
        let trip_duration_days = (departure_date - arrival_date).num_days() as u32;

        // Generate daily schedules
        let days = self.generate_daily_schedules(&activities, trip_duration_days)?;

        // Calculate cost
        let person_cost = self.calculate_cost(&days, &activities);

        // Create itinerary
        let trip_name = format!("{} Adventure", locations.0.city());
        let description = format!("Discover {} with exciting activities and experiences.", locations.0.city());
        
        let generated_itinerary = FeaturedVacation {
            id: None,
            fareharbor_id: None,
            trip_name,
            min_age: None,
            min_group: search_params.adults.unwrap_or(1),
            max_group: search_params.adults.unwrap_or(1) + search_params.children.unwrap_or(0),
            length_days: trip_duration_days,
            length_hours: trip_duration_days * 24,
            start_location: locations.0.clone(),
            end_location: locations.1.clone(),
            description,
            days: crate::models::itinerary::base::Days { days },
            images: None,
            arrival_datetime: Some(mongodb::bson::DateTime::from_millis(arrival_date.and_utc().timestamp_millis())),
            departure_datetime: Some(mongodb::bson::DateTime::from_millis(departure_date.and_utc().timestamp_millis())),
            adults: search_params.adults,
            children: search_params.children,
            infants: search_params.infants,
            pets: Some(0),
            lodging: search_params.lodging.clone(),
            transportation: search_params.transportation.clone(),
            person_cost: Some(person_cost as f64),
            created_at: Some(mongodb::bson::DateTime::now()),
            updated_at: Some(mongodb::bson::DateTime::now()),
            tag: Some("generated".to_string()),
            activities: Some(
                activities.iter().map(|activity| crate::models::itinerary::base::Activity {
                    label: activity.title.clone(),
                    description: activity.description.clone(),
                    tags: activity.tags.clone(),
                }).collect()
            ),
        };

        Ok(generated_itinerary)
    }

    /// Fetch activities using Vertex AI first, MongoDB as fallback
    async fn fetch_activities(&self, search_params: &SearchItinerary) -> Result<Vec<Activity>, mongodb::error::Error> {
        // Try Vertex AI first
        if let Some(ref vertex_service) = self.vertex_search_service {
            if let Some(activities) = &search_params.activities {
                let query = search_params.locations.as_ref().map(|locs| locs.join(" ")).unwrap_or_default();
                
                if let Ok(vertex_response) = vertex_service.search_activities(activities, &query).await {
                    if !vertex_response.results.is_empty() {
                        let mut vertex_activities = Vec::new();
                        let collection: Collection<Activity> = self.client.database("Itineraries").collection("Activities");
                        
                        for result in vertex_response.results.iter() {
                            // Try to find existing activity in MongoDB
                            if let Ok(object_id) = mongodb::bson::oid::ObjectId::parse_str(&result.id) {
                                if let Ok(Some(activity)) = collection.find_one(mongodb::bson::doc! { "_id": object_id }).await {
                                    vertex_activities.push(activity);
                                    continue;
                                }
                            }
                            
                            // Parse from Vertex AI data if not found in MongoDB
                            if let Ok(activity) = self.parse_vertex_activity(&result) {
                                vertex_activities.push(activity);
                            }
                        }
                        
                        if !vertex_activities.is_empty() {
                            println!("Found {} activities using Vertex AI", vertex_activities.len());
                            return Ok(vertex_activities);
                        }
                    }
                }
            }
        }

        // Fallback to MongoDB
        self.fetch_activities_from_mongodb(search_params).await
    }

    /// Fallback MongoDB search
    async fn fetch_activities_from_mongodb(&self, search_params: &SearchItinerary) -> Result<Vec<Activity>, mongodb::error::Error> {
        let collection: Collection<Activity> = self.client.database("Itineraries").collection("Activities");
        let mut filter = mongodb::bson::doc! {};

        // Add activity filter if provided
        if let Some(activities) = &search_params.activities {
            if !activities.is_empty() {
                let regex_patterns: Vec<mongodb::bson::Regex> = activities
                    .iter()
                    .map(|activity| mongodb::bson::Regex {
                        pattern: format!(".*{}.*", activity.to_lowercase()),
                        options: "i".to_string(),
                    })
                    .collect();
                
                filter.insert("$or", vec![
                    mongodb::bson::doc! { "activity_types": { "$in": regex_patterns.clone() } },
                    mongodb::bson::doc! { "title": { "$in": regex_patterns } }
                ]);
            }
        }

        let cursor = collection.find(filter).limit(10).await?;
        let activities: Vec<Activity> = cursor.try_collect().await?;
        
        println!("Found {} activities from MongoDB", activities.len());
        Ok(activities)
    }

    /// Get locations from search params or use default
    fn get_locations(&self, search_params: &SearchItinerary) -> (crate::models::itinerary::base::Location, crate::models::itinerary::base::Location) {
        if let Some(locations) = &search_params.locations {
            if let Some(location_str) = locations.first() {
                let parts: Vec<&str> = location_str.split(',').map(|s| s.trim()).collect();
                if parts.len() >= 2 {
                    let city = parts[0];
                    let state = parts[1];
                    let coords = self.get_coordinates(city, state);
                    
                    let location: crate::models::itinerary::base::Location = serde_json::from_value(
                        serde_json::json!({
                            "city": city,
                            "state": state,
                            "coordinates": [coords.1, coords.0] // [longitude, latitude]
                        })
                    ).unwrap();
                    return (location.clone(), location);
                }
            }
        }
        
        // Default to Denver
        let default_location: crate::models::itinerary::base::Location = serde_json::from_value(
            serde_json::json!({
                "city": "Denver",
                "state": "CO",
                "coordinates": [-104.9903, 39.7392]
            })
        ).unwrap();
        (default_location.clone(), default_location)
    }

    /// Simple coordinate lookup
    fn get_coordinates(&self, city: &str, state: &str) -> (f64, f64) {
        match (city.to_lowercase().as_str(), state.to_lowercase().as_str()) {
            ("denver", "co") => (39.7392, -104.9903),
            ("boulder", "co") => (40.0150, -105.2705),
            ("aspen", "co") => (39.1911, -106.8175),
            ("vail", "co") => (39.6403, -106.3742),
            _ => (39.7392, -104.9903), // Default to Denver
        }
    }

    /// Generate simple daily schedules
    fn generate_daily_schedules(&self, activities: &[Activity], trip_duration_days: u32) -> Result<HashMap<String, Vec<DayItem>>, Box<dyn std::error::Error>> {
        let mut days = HashMap::new();
        let activities_per_day = std::cmp::min(3, activities.len());

        for day_num in 1..=trip_duration_days {
            let day_key = day_num.to_string();
            let mut day_items = Vec::new();

            // Add activities for this day
            let start_idx = ((day_num - 1) as usize * activities_per_day) % activities.len();
            let end_idx = std::cmp::min(start_idx + activities_per_day, activities.len());
            
            let mut current_time = NaiveTime::from_hms_opt(10, 0, 0).unwrap();
            
            for activity in activities[start_idx..end_idx].iter() {
                let activity_id = activity.id.unwrap_or_else(|| ObjectId::new());
                
                day_items.push(DayItem::Activity {
                    time: current_time.format("%H:%M:%S").to_string(),
                    activity_id,
                });
                
                // Add 2 hours between activities
                current_time = current_time + Duration::hours(2);
            }

            days.insert(day_key, day_items);
        }

        Ok(days)
    }

    /// Calculate total cost
    fn calculate_cost(&self, days: &HashMap<String, Vec<DayItem>>, activities: &[Activity]) -> f32 {
        let activity_costs: HashMap<ObjectId, f32> = activities
            .iter()
            .filter_map(|a| a.id.map(|id| (id, a.price_per_person)))
            .collect();

        let mut total_cost = 0.0;
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

    /// Simple datetime parsing
    fn parse_datetime(&self, datetime_str: &str) -> Result<chrono::NaiveDateTime, Box<dyn std::error::Error>> {
        // Try common formats
        let formats = vec![
            "%Y-%m-%d %H:%M:%S",
            "%Y-%m-%dT%H:%M:%S",
            "%Y-%m-%d",
            "%m/%d/%Y",
        ];
        
        for format in formats {
            if let Ok(datetime) = chrono::NaiveDateTime::parse_from_str(datetime_str.trim(), format) {
                return Ok(datetime);
            }
            if let Ok(date) = chrono::NaiveDate::parse_from_str(datetime_str.trim(), format) {
                return Ok(date.and_hms_opt(0, 0, 0).unwrap());
            }
        }
        
        Err(format!("Unable to parse datetime '{}'", datetime_str).into())
    }

    /// Simple Vertex AI activity parsing
    fn parse_vertex_activity(&self, result: &crate::services::vertex_search_service::SearchResult) -> Result<Activity, Box<dyn std::error::Error>> {
        let struct_data = result.document.struct_data.as_object()
            .ok_or("struct_data is not an object")?;

        // Extract basic fields with defaults
        let title = struct_data.get("title").and_then(|v| v.as_str()).unwrap_or("Activity").to_string();
        let description = struct_data.get("description").and_then(|v| v.as_str()).unwrap_or(&title).to_string();
        let price = struct_data.get("price_per_person").and_then(|v| v.as_f64()).unwrap_or(50.0) as f32;
        let duration = struct_data.get("duration_minutes").and_then(|v| v.as_u64()).unwrap_or(120) as u16;

        // Create simple activity
        let activity = Activity {
            id: None, // Will be assigned when needed
            company: "Generated".to_string(),
            company_id: "generated".to_string(),
            booking_link: "".to_string(),
            online_booking_status: "available".to_string(),
            guide: None,
            title,
            description,
            activity_types: vec![],
            tags: vec![],
            price_per_person: price,
            duration_minutes: duration,
            daily_time_slots: vec![],
            address: crate::models::activity::Address {
                street: "".to_string(),
                unit: "".to_string(),
                city: struct_data.get("city").and_then(|v| v.as_str()).unwrap_or("Denver").to_string(),
                state: "CO".to_string(),
                zip: "".to_string(),
                country: "USA".to_string(),
            },
            whats_included: vec![],
            weight_limit_lbs: None,
            age_requirement: None,
            height_requiremnt: None,
            blackout_date_ranges: None,
            capacity: crate::models::activity::Capacity { minimum: 1, maximum: 100 },
            created_at: None,
            updated_at: None,
        };

        Ok(activity)
    }
}

use futures::TryStreamExt;