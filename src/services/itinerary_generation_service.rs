use crate::models::{
    activity::Activity,
    itinerary::base::{DayItem, FeaturedVacation},
    search::{SearchItinerary, TripPace},
};
use crate::services::vertex_search_service::VertexSearchService;
use chrono::{Datelike, Duration, NaiveDate, NaiveDateTime, NaiveTime, Utc};
use mongodb::{bson::oid::ObjectId, Client, Collection};
use std::{collections::HashMap, sync::Arc};

#[derive(Clone)]
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
                println!(
                    "VertexSearchService not available: {}. Using MongoDB only.",
                    e
                );
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

        println!("🔍 Found {} activities total for itinerary generation", activities.len());
        for (i, activity) in activities.iter().enumerate() {
            println!("   Activity {}: ID={:?}, Title={}, Duration={}min", 
                i+1, activity.id, activity.title, activity.duration_minutes);
        }

        if activities.is_empty() {
            println!("❌ No activities found - cannot generate itinerary");
            return Err("No matching activities found".into());
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

        let arrival_date = self.parse_datetime(arrival_str)?;
        let departure_date = self.parse_datetime(departure_str)?;
        let trip_duration_days = (departure_date - arrival_date).num_days() as u32;

        // Generate daily schedules based on trip pace
        let trip_pace = search_params.trip_pace.as_ref().unwrap_or(&TripPace::Moderate);
        let days = self.generate_daily_schedules_with_pace(&activities, trip_duration_days, trip_pace)?;
        
        println!("🔄 Generated {} days with total items: {}", 
            days.len(), 
            days.values().map(|v| v.len()).sum::<usize>());

        // Calculate cost
        let person_cost = self.calculate_cost(&days, &activities);

        // Create itinerary
        let trip_name = format!("{} Adventure", locations.0.city());
        let description = format!(
            "Discover {} with exciting activities and experiences.",
            locations.0.city()
        );

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
            images: Some(vec![]), // Initialize as empty array instead of None
            arrival_datetime: Some(mongodb::bson::DateTime::from_millis(
                arrival_date.and_utc().timestamp_millis(),
            )),
            departure_datetime: Some(mongodb::bson::DateTime::from_millis(
                departure_date.and_utc().timestamp_millis(),
            )),
            adults: search_params.adults,
            children: search_params.children,
            infants: search_params.infants,
            pets: Some(0),
            lodging: Some(search_params.lodging.clone().unwrap_or_default()),
            transportation: search_params.transportation.clone().or_else(|| Some("Private Vehicle".to_string())),
            created_at: Some(mongodb::bson::DateTime::now()),
            updated_at: Some(mongodb::bson::DateTime::now()),
            tag: Some("generated".to_string()),
            activities: Some(
                activities
                    .iter()
                    .map(|activity| crate::models::itinerary::base::Activity {
                        label: activity.title.clone(),
                        description: activity.description.clone(),
                        tags: activity.tags.clone(),
                    })
                    .collect(),
            ),
            match_score: None, // Will be set during search scoring
            score_breakdown: None, // Will be set during search scoring
        };

        Ok(generated_itinerary)
    }

    /// Generate a unique itinerary with variety to avoid duplicates
    pub async fn generate_unique_itinerary(
        &self,
        search_params: &SearchItinerary,
        variation_index: usize,
        existing_names: &std::collections::HashSet<String>,
    ) -> Result<FeaturedVacation, String> {
        // Get activities and locations
        let activities = self.fetch_activities(search_params).await.map_err(|e| e.to_string())?;
        let locations = self.get_locations(search_params);

        if activities.is_empty() {
            return Err("No matching activities found".to_string());
        }

        // Calculate trip duration
        let arrival_str = search_params
            .arrival_datetime
            .as_ref()
            .ok_or("Arrival datetime required".to_string())?;
        let departure_str = search_params
            .departure_datetime
            .as_ref()
            .ok_or("Departure datetime required".to_string())?;

        let arrival_date = self.parse_datetime(arrival_str).map_err(|e| e.to_string())?;
        let departure_date = self.parse_datetime(departure_str).map_err(|e| e.to_string())?;

        let trip_duration_days = (departure_date - arrival_date).num_days() as u32;

        // Create unique trip name based on variation
        let trip_name = self.generate_unique_trip_name(&locations.0, search_params, variation_index, existing_names);

        // Generate varied daily schedules
        let days = self.generate_varied_daily_schedules_with_pace(
            &activities,
            trip_duration_days,
            search_params.trip_pace.as_ref(),
            variation_index,
        ).map_err(|e| e.to_string())?;

        // Calculate cost with some variation
        let base_cost = self.calculate_cost(&days, &activities);
        let cost_variation = (variation_index % 3) as f32 * 10.0; // Small cost variations
        let person_cost = base_cost + cost_variation;

        // Create description with variation
        let description = self.generate_varied_description(&locations.0, search_params, variation_index);

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
            images: Some(vec![]), // Initialize as empty array instead of None
            arrival_datetime: Some(mongodb::bson::DateTime::from_millis(
                arrival_date.and_utc().timestamp_millis(),
            )),
            departure_datetime: Some(mongodb::bson::DateTime::from_millis(
                departure_date.and_utc().timestamp_millis(),
            )),
            adults: search_params.adults,
            children: search_params.children,
            infants: search_params.infants,
            pets: Some(0),
            lodging: Some(search_params.lodging.clone().unwrap_or_default()),
            transportation: search_params.transportation.clone().or_else(|| Some("Private Vehicle".to_string())),
            created_at: Some(mongodb::bson::DateTime::now()),
            updated_at: Some(mongodb::bson::DateTime::now()),
            tag: Some("generated".to_string()),
            activities: Some(
                activities
                    .iter()
                    .map(|activity| crate::models::itinerary::base::Activity {
                        label: activity.title.clone(),
                        description: activity.description.clone(),
                        tags: activity.tags.clone(),
                    })
                    .collect(),
            ),
            match_score: None,
            score_breakdown: None,
        };

        Ok(generated_itinerary)
    }

    /// Generate unique trip names with different themes
    fn generate_unique_trip_name(
        &self,
        location: &crate::models::itinerary::base::Location,
        search_params: &SearchItinerary,
        variation_index: usize,
        existing_names: &std::collections::HashSet<String>,
    ) -> String {
        let city = location.city();
        let default_activities = vec![];
        let activities = search_params.activities.as_ref().unwrap_or(&default_activities);
        let default_activity = "adventure".to_string();
        let primary_activity = activities.first().unwrap_or(&default_activity);
        
        // Different name templates based on variation
        let activity_title = Self::to_title_case(primary_activity);
        let name_templates = vec![
            format!("{} {} Adventure", city, activity_title),
            format!("Discover {} - {} Experience", city, activity_title),
            format!("{} {} Getaway", city, activity_title),
            format!("Ultimate {} {} Tour", city, activity_title),
            format!("{} Explorer - {}", city, activity_title),
            format!("{} {} Expedition", city, activity_title),
            format!("Wild {} - {} Journey", city, activity_title),
            format!("{} {} Quest", city, activity_title),
        ];
        
        // Try different templates until we find a unique name
        for (i, template) in name_templates.iter().enumerate() {
            let candidate_name = if i == variation_index % name_templates.len() {
                template.clone()
            } else {
                continue;
            };
            
            if !existing_names.contains(&candidate_name) {
                return candidate_name;
            }
        }
        
        // Fallback with timestamp if all templates are used
        format!("{} Adventure {}", city, Utc::now().timestamp() % 10000)
    }

    /// Generate varied descriptions
    fn generate_varied_description(
        &self,
        location: &crate::models::itinerary::base::Location,
        search_params: &SearchItinerary,
        variation_index: usize,
    ) -> String {
        let city = location.city();
        let default_activities = vec![];
        let activities = search_params.activities.as_ref().unwrap_or(&default_activities);
        let default_activity = "adventure".to_string();
        let primary_activity = activities.first().unwrap_or(&default_activity);
        
        let descriptions = vec![
            format!("Discover {} with exciting {} activities and unforgettable experiences.", city, primary_activity),
            format!("Explore the best of {} through thrilling {} adventures and local attractions.", city, primary_activity),
            format!("Immerse yourself in {}'s culture while enjoying amazing {} activities.", city, primary_activity),
            format!("Experience {} like never before with our curated {} itinerary.", city, primary_activity),
            format!("Journey through {} with perfectly planned {} experiences and hidden gems.", city, primary_activity),
        ];
        
        descriptions[variation_index % descriptions.len()].clone()
    }

    /// Generate varied daily schedules to create different itineraries
    fn generate_varied_daily_schedules_with_pace(
        &self,
        activities: &[Activity],
        trip_duration_days: u32,
        trip_pace: Option<&TripPace>,
        variation_index: usize,
    ) -> Result<HashMap<String, Vec<DayItem>>, String> {
        let pace = trip_pace.unwrap_or(&TripPace::Moderate);
        let max_hours_per_day = pace.max_activity_hours_per_day();
        let activities_per_day = pace.typical_activities_per_day();

        let mut daily_schedules = HashMap::new();
        let mut used_activity_ids = std::collections::HashSet::new(); // Track used activities

        // Create shuffled activity list for variation
        let mut available_activities = activities.to_vec();
        
        // Shuffle based on variation_index for different orderings
        for i in 0..available_activities.len() {
            let swap_index = (i + variation_index * 7) % available_activities.len();
            available_activities.swap(i, swap_index);
        }
        
        let mut global_activity_index = 0; // Track position in shuffled list

        for day in 1..=trip_duration_days {
            let mut day_schedule = Vec::new();
            let mut day_hours = 0.0;
            let mut activities_added = 0;

            // Start times vary by variation to create different schedules
            let base_start_hour = match variation_index % 3 {
                0 => 9,  // Early start
                1 => 10, // Regular start  
                2 => 11, // Late start
                _ => 9,
            };

            let mut current_hour = base_start_hour;
            
            while activities_added < activities_per_day && day_hours < max_hours_per_day {
                // Find next unused activity
                let mut found_activity = false;
                let start_search_index = global_activity_index;
                
                loop {
                    if global_activity_index >= available_activities.len() {
                        // If we've gone through all activities, we're done
                        break;
                    }
                    
                    let activity = &available_activities[global_activity_index];
                    
                    // Check if this activity is already used
                    if let Some(activity_id) = activity.id {
                        if !used_activity_ids.contains(&activity_id) {
                            let activity_duration_hours = activity.duration_minutes as f32 / 60.0;
                            
                            if day_hours + activity_duration_hours <= max_hours_per_day {
                                let time = format!("{:02}:00:00", current_hour);
                                
                                day_schedule.push(DayItem::Activity {
                                    activity_id,
                                    time,
                                });
                                
                                used_activity_ids.insert(activity_id); // Mark as used
                                day_hours += activity_duration_hours;
                                activities_added += 1;
                                current_hour += activity_duration_hours.ceil() as u32;
                                
                                // Add buffer time between activities (varies by variation)
                                let buffer_hours = match variation_index % 3 {
                                    0 => 1, // Tight schedule
                                    1 => 2, // Normal schedule
                                    2 => 3, // Relaxed schedule
                                    _ => 2,
                                };
                                current_hour += buffer_hours;
                                
                                global_activity_index += 1;
                                found_activity = true;
                                break;
                            }
                        }
                    }
                    
                    global_activity_index += 1;
                    
                    // If we've cycled back to where we started, stop looking
                    if global_activity_index >= available_activities.len() {
                        break;
                    }
                }
                
                if !found_activity {
                    // No more suitable unused activities available
                    break;
                }
            }

            if !day_schedule.is_empty() {
                daily_schedules.insert(day.to_string(), day_schedule);
            }
        }

        Ok(daily_schedules)
    }

    /// Fetch activities using Vertex AI first, MongoDB as fallback
    async fn fetch_activities(
        &self,
        search_params: &SearchItinerary,
    ) -> Result<Vec<Activity>, mongodb::error::Error> {
        // Always try Vertex AI first - even with minimal search criteria
        if let Some(ref vertex_service) = self.vertex_search_service {
            // Build query from available search parameters
            let activities_query = search_params
                .activities
                .as_ref()
                .unwrap_or(&vec!["outdoor".to_string(), "adventure".to_string(), "sightseeing".to_string()])
                .clone();
            
            let location_query = search_params
                .locations
                .as_ref()
                .map(|locs| locs.join(" "))
                .unwrap_or_default();

            println!("Trying Vertex AI search with activities: {:?}, location: {}", activities_query, location_query);

            if let Ok(vertex_response) =
                vertex_service.search_activities(&activities_query, &location_query).await
            {
                if !vertex_response.results.is_empty() {
                    println!("Vertex AI returned {} activity results", vertex_response.results.len());
                    let mut vertex_activities = Vec::new();
                    let _collection: Collection<Activity> =
                        self.client.database("Options").collection("Activity");

                    for result in vertex_response.results.iter() {
                            // Transform Vertex AI document to match Activity struct format
                            let mut vertex_data = result.document.struct_data.clone();
                            
                            // Transform daily_time_slots from strings to TimeSlot objects
                            if let Some(time_slots) = vertex_data.get("daily_time_slots").and_then(|v| v.as_array()) {
                                let transformed_slots: Vec<serde_json::Value> = time_slots.iter()
                                    .filter_map(|slot| {
                                        if let Some(slot_str) = slot.as_str() {
                                            let parts: Vec<&str> = slot_str.split('-').collect();
                                            if parts.len() == 2 {
                                                Some(serde_json::json!({
                                                    "start": parts[0],
                                                    "end": parts[1]
                                                }))
                                            } else {
                                                None
                                            }
                                        } else {
                                            None
                                        }
                                    })
                                    .collect();
                                
                                if let Some(data_obj) = vertex_data.as_object_mut() {
                                    data_obj.insert(
                                        "daily_time_slots".to_string(),
                                        serde_json::Value::Array(transformed_slots)
                                    );
                                }
                            }

                            // Add missing required fields with defaults
                            if let Some(data_obj) = vertex_data.as_object_mut() {
                                if !data_obj.contains_key("description") {
                                    if let Some(title) = data_obj.get("title").and_then(|v| v.as_str()) {
                                        data_obj.insert("description".to_string(), serde_json::Value::String(title.to_string()));
                                    } else {
                                        data_obj.insert("description".to_string(), serde_json::Value::String("Activity description".to_string()));
                                    }
                                }

                                // Ensure address field exists with proper structure
                                if !data_obj.contains_key("address") {
                                    let address = serde_json::json!({
                                        "street": data_obj.get("full_address").and_then(|v| v.as_str()).unwrap_or(""),
                                        "unit": "",
                                        "city": data_obj.get("city").and_then(|v| v.as_str()).unwrap_or(""),
                                        "state": data_obj.get("state").and_then(|v| v.as_str()).unwrap_or(""),
                                        "zip": data_obj.get("zip").and_then(|v| v.as_str()).unwrap_or(""),
                                        "country": data_obj.get("country").and_then(|v| v.as_str()).unwrap_or("USA")
                                    });
                                    data_obj.insert("address".to_string(), address);
                                }

                                // Ensure capacity field exists
                                if !data_obj.contains_key("capacity") {
                                    let capacity = serde_json::json!({
                                        "minimum": data_obj.get("min_capacity").and_then(|v| v.as_u64()).unwrap_or(1),
                                        "maximum": data_obj.get("max_capacity").and_then(|v| v.as_u64()).unwrap_or(100)
                                    });
                                    data_obj.insert("capacity".to_string(), capacity);
                                }

                                // Handle ID field - Vertex AI returns 'id' as string, convert to ObjectId
                                if let Some(id_val) = data_obj.get("id") {
                                    if let Some(id_str) = id_val.as_str() {
                                        // Try to parse the string as an ObjectId
                                        if let Ok(object_id) = ObjectId::parse_str(id_str) {
                                            // Insert as proper ObjectId for _id field
                                            data_obj.insert("_id".to_string(), serde_json::json!({
                                                "$oid": id_str
                                            }));
                                        } else {
                                            println!("Warning: Invalid ObjectId format from Vertex AI: {}", id_str);
                                        }
                                    }
                                }
                            }

                            // Try to parse the transformed document as an Activity
                            match serde_json::from_value::<Activity>(vertex_data) {
                                Ok(mut activity) => {
                                    // If the ID wasn't properly parsed, try to extract it manually
                                    if activity.id.is_none() {
                                        if let Some(id_val) = result.document.struct_data.get("id") {
                                            if let Some(id_str) = id_val.as_str() {
                                                if let Ok(object_id) = ObjectId::parse_str(id_str) {
                                                    activity.id = Some(object_id);
                                                    println!("Manually set VERTEX ID: {:?}", object_id);
                                                }
                                            }
                                        }
                                    }
                                    println!("FINAL VERTEX ID: {:?}", activity.id);
                                    vertex_activities.push(activity);
                                }
                                Err(e) => {
                                    println!("Failed to parse Vertex AI document as Activity: {}", e);
                                    println!("Document data: {:?}", result.document.struct_data);
                                }
                            }
                        }

                    if !vertex_activities.is_empty() {
                        println!(
                            "Found {} activities using Vertex AI",
                            vertex_activities.len()
                        );
                        return Ok(vertex_activities);
                    }
                }
            }
        }

        // Fallback to MongoDB
        self.fetch_activities_from_mongodb(search_params).await
    }

    /// Fallback MongoDB search
    async fn fetch_activities_from_mongodb(
        &self,
        search_params: &SearchItinerary,
    ) -> Result<Vec<Activity>, mongodb::error::Error> {
        let collection: Collection<Activity> =
            self.client.database("Options").collection("Activity");
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

                filter.insert(
                    "$or",
                    vec![
                        mongodb::bson::doc! { "activity_types": { "$in": regex_patterns.clone() } },
                        mongodb::bson::doc! { "title": { "$in": regex_patterns } },
                    ],
                );
            }
        }

        let cursor = collection.find(filter).limit(10).await?;
        let activities: Vec<Activity> = cursor.try_collect().await?;

        println!("Found {} activities from MongoDB", activities.len());
        Ok(activities)
    }

    /// Get locations from search params or use default
    fn get_locations(
        &self,
        search_params: &SearchItinerary,
    ) -> (
        crate::models::itinerary::base::Location,
        crate::models::itinerary::base::Location,
    ) {
        if let Some(locations) = &search_params.locations {
            if let Some(location_str) = locations.first() {
                let parts: Vec<&str> = location_str.split(',').map(|s| s.trim()).collect();
                if parts.len() >= 2 {
                    let city = parts[0];
                    let state = parts[1];
                    let coords = self.get_coordinates(city, state);

                    let location: crate::models::itinerary::base::Location =
                        serde_json::from_value(serde_json::json!({
                            "city": city,
                            "state": state,
                            "coordinates": [coords.1, coords.0] // [longitude, latitude]
                        }))
                        .unwrap();
                    return (location.clone(), location);
                }
            }
        }

        // Default to Denver
        let default_location: crate::models::itinerary::base::Location =
            serde_json::from_value(serde_json::json!({
                "city": "Denver",
                "state": "CO",
                "coordinates": [-104.9903, 39.7392]
            }))
            .unwrap();
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

    /// Generate daily schedules based on trip pace
    fn generate_daily_schedules_with_pace(
        &self,
        activities: &[Activity],
        trip_duration_days: u32,
        trip_pace: &TripPace,
    ) -> Result<HashMap<String, Vec<DayItem>>, Box<dyn std::error::Error>> {
        println!("📅 Generating schedules for {} activities:", activities.len());
        for (i, activity) in activities.iter().enumerate() {
            println!("   Activity {}: ID={:?}, Title={}", i+1, activity.id, activity.title);
        }
        
        let mut days = HashMap::new();
        let mut used_activity_ids = std::collections::HashSet::new(); // Track used activities
        
        // Determine activities per day based on trip pace
        let activities_per_day = trip_pace.typical_activities_per_day();
        let max_hours_per_day = trip_pace.max_activity_hours_per_day();
        
        println!("Trip pace: {:?}, activities per day: {}, max hours: {}", 
            trip_pace, activities_per_day, max_hours_per_day);

        // Create a shuffled copy of activities for variety
        let mut available_activities = activities.to_vec();
        let mut global_activity_index = 0;

        for day_num in 1..=trip_duration_days {
            let day_key = day_num.to_string();
            let mut day_items = Vec::new();
            let mut day_hours = 0.0;
            
            // Start time based on trip pace
            let mut current_time = match trip_pace {
                TripPace::Relaxed => NaiveTime::from_hms_opt(10, 0, 0).unwrap(),   // Later start
                TripPace::Moderate => NaiveTime::from_hms_opt(9, 0, 0).unwrap(),   // Moderate start
                TripPace::Adventure => NaiveTime::from_hms_opt(8, 0, 0).unwrap(),  // Early start
            };

            let mut activities_added = 0;
            
            // Add activities until we reach the pace limit or run out of hours
            while activities_added < activities_per_day && day_hours < max_hours_per_day {
                // Find next unused activity
                let mut found_activity = false;
                let start_search_index = global_activity_index;
                
                // Search for unused activity starting from current index
                for search_offset in 0..available_activities.len() {
                    let idx = (global_activity_index + search_offset) % available_activities.len();
                    let activity = &available_activities[idx];
                    
                    if let Some(activity_id) = activity.id {
                        // Check if this activity is already used
                        if !used_activity_ids.contains(&activity_id) {
                            let activity_duration_hours = activity.duration_minutes as f32 / 60.0;
                            
                            // Check if adding this activity would exceed daily hour limit
                            if day_hours + activity_duration_hours <= max_hours_per_day {
                                println!("   📍 Day {}: Adding activity '{}' (ID: {:?}) at {}", 
                                    day_num, activity.title, activity_id, current_time.format("%H:%M:%S"));
                                
                                day_items.push(DayItem::Activity {
                                    time: current_time.format("%H:%M:%S").to_string(),
                                    activity_id,
                                });
                                
                                used_activity_ids.insert(activity_id); // Mark as used
                                day_hours += activity_duration_hours;
                                activities_added += 1;
                                
                                // Add break time between activities based on pace
                                let break_time = match trip_pace {
                                    TripPace::Relaxed => Duration::minutes(90),   // Longer breaks
                                    TripPace::Moderate => Duration::minutes(60),  // Moderate breaks
                                    TripPace::Adventure => Duration::minutes(30), // Short breaks
                                };
                                
                                current_time = current_time + Duration::minutes(activity.duration_minutes as i64) + break_time;
                                global_activity_index = (idx + 1) % available_activities.len();
                                found_activity = true;
                                break;
                            } else {
                                println!("   ⚠️  Day {}: Skipping activity '{}' - would exceed daily hour limit ({} + {} > {})", 
                                    day_num, activity.title, day_hours, activity_duration_hours, max_hours_per_day);
                            }
                        }
                    }
                }
                
                if !found_activity {
                    println!("   ⚠️  Day {}: No more suitable unused activities available", day_num);
                    break;
                }
            }
            
            println!("   ✅ Day {}: Added {} activities, total hours: {:.1}", 
                day_num, activities_added, day_hours);

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

    /// Enhanced datetime parsing that handles various formats
    fn parse_datetime(
        &self,
        datetime_str: &str,
    ) -> Result<chrono::NaiveDateTime, Box<dyn std::error::Error>> {
        let trimmed = datetime_str.trim();

        println!("Attempting to parse datetime: '{}'", trimmed);

        // Try common formats
        let formats = vec![
            "%Y-%m-%d %H:%M:%S",
            "%Y-%m-%dT%H:%M:%S",
            "%Y-%m-%d",
            "%m/%d/%Y",
            "%m/%d/%Y %H:%M:%S",
            "%Y/%m/%d",
            "%Y/%m/%d %H:%M:%S",
        ];

        // Try standard formats first
        for format in formats {
            if let Ok(datetime) = NaiveDateTime::parse_from_str(trimmed, format) {
                println!("Successfully parsed with format: {}", format);
                return Ok(datetime);
            }
            if let Ok(date) = NaiveDate::parse_from_str(trimmed, format) {
                println!("Successfully parsed date with format: {}", format);
                return Ok(date.and_hms_opt(0, 0, 0).unwrap());
            }
        }

        // Handle formats without year (assume current year)
        let current_year = chrono::Utc::now().year();

        // Special handling for "Jul 22T09:00:00" format
        // Check if string contains 'T' as a separator
        if trimmed.contains('T') && !trimmed.starts_with(|c: char| c.is_numeric()) {
            // Try to parse month abbreviation format
            let parts: Vec<&str> = trimmed.split('T').collect();
            if parts.len() == 2 {
                let date_part = parts[0];
                let time_part = parts[1];

                // Try to parse the date part with current year
                let date_with_year = format!("{} {}", date_part, current_year);
                if let Ok(date) = NaiveDate::parse_from_str(&date_with_year, "%b %d %Y") {
                    // Now parse the time part
                    if let Ok(time) = NaiveTime::parse_from_str(time_part, "%H:%M:%S") {
                        let datetime = date.and_time(time);
                        println!(
                            "Successfully parsed split format: {} T {}",
                            date_part, time_part
                        );
                        return Ok(datetime);
                    }
                }

                // Try with single digit day (e.g., "Jul 8" instead of "Jul 08")
                if let Ok(date) = NaiveDate::parse_from_str(&date_with_year, "%b %e %Y") {
                    // Now parse the time part
                    if let Ok(time) = NaiveTime::parse_from_str(time_part, "%H:%M:%S") {
                        let datetime = date.and_time(time);
                        println!(
                            "Successfully parsed split format with single digit day: {} T {}",
                            date_part, time_part
                        );
                        return Ok(datetime);
                    }
                }
            }
        }

        // Try to parse the abbreviated month format with year appended
        let datetime_with_year = format!("{} {}", trimmed, current_year);
        println!(
            "Trying to parse with year appended: '{}'",
            datetime_with_year
        );

        if let Ok(datetime) =
            NaiveDateTime::parse_from_str(&datetime_with_year, "%b %dT%H:%M:%S %Y")
        {
            println!("Successfully parsed with format: %b %dT%H:%M:%S %Y");
            return Ok(datetime);
        }

        // Try without the T separator (e.g., "Jul 22 09:00:00")
        if let Ok(datetime) =
            NaiveDateTime::parse_from_str(&datetime_with_year, "%b %d %H:%M:%S %Y")
        {
            println!("Successfully parsed with format: %b %d %H:%M:%S %Y");
            return Ok(datetime);
        }

        // Try with just date part if time is missing
        if let Ok(date) =
            NaiveDate::parse_from_str(&format!("{} {}", trimmed, current_year), "%b %d %Y")
        {
            println!("Successfully parsed date with format: %b %d %Y");
            return Ok(date.and_hms_opt(0, 0, 0).unwrap());
        }

        Err(format!("Unable to parse datetime '{}'. Supported formats include: YYYY-MM-DD, MM/DD/YYYY, Jul 22T09:00:00, etc.", trimmed).into())
    }

    /// Simple Vertex AI activity parsing
    fn parse_vertex_activity(
        &self,
        result: &crate::services::vertex_search_service::SearchResult,
    ) -> Result<Activity, Box<dyn std::error::Error>> {
        let struct_data = result
            .document
            .struct_data
            .as_object()
            .ok_or("struct_data is not an object")?;

        // Extract basic fields with defaults
        let title = struct_data
            .get("title")
            .and_then(|v| v.as_str())
            .unwrap_or("Activity")
            .to_string();
        let description = struct_data
            .get("description")
            .and_then(|v| v.as_str())
            .unwrap_or(&title)
            .to_string();
        let price = struct_data
            .get("price_per_person")
            .and_then(|v| v.as_f64())
            .unwrap_or(50.0) as f32;
        let duration = struct_data
            .get("duration_minutes")
            .and_then(|v| v.as_u64())
            .unwrap_or(120) as u16;

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
                city: struct_data
                    .get("city")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Denver")
                    .to_string(),
                state: "CO".to_string(),
                zip: "".to_string(),
                country: "USA".to_string(),
            },
            whats_included: vec![],
            weight_limit_lbs: None,
            age_requirement: None,
            height_requiremnt: None,
            blackout_date_ranges: None,
            capacity: crate::models::activity::Capacity {
                minimum: 1,
                maximum: 100,
            },
            created_at: None,
            updated_at: None,
        };

        Ok(activity)
    }

    /// Simple title case conversion
    fn to_title_case(s: &str) -> String {
        s.split_whitespace()
            .map(|word| {
                if word.is_empty() {
                    String::new()
                } else {
                    let mut chars = word.chars();
                    match chars.next() {
                        None => String::new(),
                        Some(first) => first.to_uppercase().collect::<String>() + &chars.as_str().to_lowercase(),
                    }
                }
            })
            .collect::<Vec<String>>()
            .join(" ")
    }
}

use futures::TryStreamExt;
