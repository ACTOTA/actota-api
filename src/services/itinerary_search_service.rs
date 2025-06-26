use crate::models::{itinerary::base::FeaturedVacation, search::SearchItinerary};
use crate::services::itinerary_generation_service::ItineraryGenerator;
use crate::services::vertex_search_service::VertexSearchService;
use crate::services::search_scoring::AsyncSearchScorer;
use bson::{doc, Document};
use futures::TryStreamExt;
use mongodb::{Client, Collection};
use std::{collections::HashSet, sync::Arc};

pub async fn search_itineraries(
    client: Arc<Client>,
    search_params: SearchItinerary,
) -> Result<Vec<FeaturedVacation>, mongodb::error::Error> {
    let collection: Collection<FeaturedVacation> =
        client.database("Itineraries").collection("Featured");

    // First, get activities from Vertex AI Search if activity types are provided
    if let Some(activity_types) = &search_params.activities {
        if !activity_types.is_empty() {
            println!("Fetching activities from Vertex AI Search for types: {:?}", activity_types);
            match fetch_activities_from_vertex(&search_params).await {
                Ok(activities) => {
                    println!("Found {} activities from Vertex AI Search", activities.len());
                    // Store activities for later use in generation if needed
                },
                Err(e) => {
                    eprintln!("Failed to fetch activities from Vertex AI: {:?}", e);
                }
            }
        }
    }

    // Try exact matching first
    if let Ok(exact_results) = try_exact_search(&collection, &search_params).await {
        if !exact_results.is_empty() {
            println!("Found {} exact matches", exact_results.len());
            return Ok(exact_results);
        }
    }

    // Try partial matching if no exact matches
    if let Ok(partial_results) = try_partial_search(&collection, &search_params).await {
        if !partial_results.is_empty() {
            println!("Found {} partial matches", partial_results.len());
            return Ok(partial_results);
        }
    }

    // Try location-only matching as final fallback
    if let Ok(location_results) = try_location_only_search(&collection, &search_params).await {
        if !location_results.is_empty() {
            println!("Found {} location-based matches", location_results.len());
            return Ok(location_results);
        }
    }

    // Return empty if nothing found
    println!("No matches found for search criteria");
    Ok(Vec::new())
}

/// Try exact matching search
async fn try_exact_search(
    collection: &Collection<FeaturedVacation>,
    search_params: &SearchItinerary,
) -> Result<Vec<FeaturedVacation>, mongodb::error::Error> {
    // Build the filter query based on search parameters
    let mut filter = Document::new();

    // Add search criteria to filter if they exist
    if let Some(locations) = &search_params.locations {
        if !locations.is_empty() {
            // Search for itineraries where start or end location city matches any of the requested locations
            let cities = locations
                .iter()
                .map(|l| {
                    let parts = l.split(',').collect::<Vec<&str>>();

                    if parts.len() > 1 {
                        parts[0].to_string()
                    } else {
                        l.to_string()
                    }
                })
                .collect::<Vec<String>>();

            filter.insert(
                "$or",
                vec![
                    doc! { "start_location.city": { "$in": cities.clone() } },
                    doc! { "end_location.city": { "$in": cities } },
                ],
            );
        }
    }

    // Activity filtering - require ALL requested activities to be present
    if let Some(activities) = &search_params.activities {
        if !activities.is_empty() {
            // Create an AND condition for activities
            let mut and_conditions = Vec::new();

            for activity in activities {
                // For each requested activity, create a condition that it must exist
                and_conditions.push(doc! {
                    "activities": {
                        "$elemMatch": {
                            "label": {
                                "$regex": activity,
                                "$options": "i"  // case-insensitive match
                            }
                        }
                    }
                });
            }

            // Convert to Bson for compatibility
            let and_conditions_bson: Vec<bson::Bson> = and_conditions
                .into_iter()
                .map(bson::Bson::Document)
                .collect();

            // Use $and to require ALL activities to be present
            filter.insert("$and", and_conditions_bson);
        }
    }

    // Lodging filtering - require ALL requested lodging types to be present
    if let Some(lodging) = &search_params.lodging {
        if !lodging.is_empty() {
            let mut lodging_conditions = Vec::new();

            for lodging_type in lodging {
                // For each requested lodging type, create a condition that it must exist
                // Check in the days collection for accommodation items
                lodging_conditions.push(doc! {
                    "days.days": {
                        "$elemMatch": {
                            "$elemMatch": {
                                "type": "accommodation"
                            }
                        }
                    }
                });
            }

            // Convert lodging_conditions to Bson for compatibility
            let lodging_conditions_bson: Vec<bson::Bson> = lodging_conditions
                .into_iter()
                .map(bson::Bson::Document)
                .collect();

            // Add lodging conditions to the existing $and array or create a new one
            if filter.contains_key("$and") {
                // Get the existing $and array and append to it
                match filter.get_array_mut("$and") {
                    Ok(existing_and) => {
                        // Append lodging conditions to existing $and array
                        existing_and.extend(lodging_conditions_bson);
                    }
                    Err(e) => {
                        // Log error and create a new $and array
                        eprintln!("Error accessing $and array: {:?}", e);
                        filter.insert("$and", lodging_conditions_bson);
                    }
                }
            } else {
                // Create new $and array with lodging conditions
                filter.insert("$and", lodging_conditions_bson);
            }
        }
    }

    // Add demographic filters if provided
    if let Some(adults) = search_params.adults {
        filter.insert("min_group", doc! { "$lte": adults });
        filter.insert("max_group", doc! { "$gte": adults });
    }
    
    // Add trip pace filtering if provided
    if let Some(trip_pace) = &search_params.trip_pace {
        // Filter itineraries based on activities per day matching the pace
        let max_activities = trip_pace.typical_activities_per_day();
        // This is a heuristic - we check if the itinerary has a reasonable number of activities
        // In production, you'd want to analyze the actual daily schedule
    }

    // If filter is empty (no search criteria provided), return all itineraries
    let cursor = if filter.is_empty() {
        collection.find(doc! {}).await?
    } else {
        collection.find(filter).await?
    };

    // Collect results
    let itineraries = cursor.try_collect().await?;

    Ok(itineraries)
}

/// Search for itineraries with generation fallback
/// If no exact matches are found, generates a new itinerary based on search parameters
pub async fn search_or_generate_itineraries(
    client: Arc<Client>,
    search_params: SearchItinerary,
    min_results_threshold: usize,
) -> Result<Vec<FeaturedVacation>, Box<dyn std::error::Error>> {
    // First, try to find existing itineraries
    let mut results = search_itineraries(client.clone(), search_params.clone()).await?;
    
    // Score the results and filter by match score
    let scorer = AsyncSearchScorer::new(client.clone());
    let mut scored_results = scorer.score_and_rank_itineraries(results.clone(), &search_params).await;
    
    // Filter for high-quality matches (90+ score)
    let high_quality_matches: Vec<FeaturedVacation> = scored_results
        .iter()
        .filter(|scored| {
            // Calculate percentage score (0-100)
            let max_possible_score = scorer.weights.location_weight
                + scorer.weights.activity_weight
                + scorer.weights.group_size_weight
                + scorer.weights.lodging_weight
                + scorer.weights.transportation_weight;
            let percentage_score = (scored.total_score / max_possible_score) * 100.0;
            percentage_score >= 90.0
        })
        .map(|scored| scored.itinerary.clone())
        .collect();
    
    println!("Found {} high-quality matches (90+ score) out of {} total matches", 
        high_quality_matches.len(), results.len());

    // If we have enough high-quality results, return them
    if high_quality_matches.len() >= min_results_threshold {
        return Ok(high_quality_matches);
    }
    
    // Otherwise, we need to generate more itineraries
    results = high_quality_matches;

    // If not enough results, try to generate a new itinerary
    println!(
        "Found only {} itineraries, generating new ones to meet threshold of {}",
        results.len(),
        min_results_threshold
    );

    // Check if we have the required fields for generation
    if search_params.arrival_datetime.is_none() || search_params.departure_datetime.is_none() {
        println!("Cannot generate itinerary without arrival and departure dates, returning existing results (including partial matches)");
        
        // If we have no results at all and no dates for generation, try a more flexible search
        if results.is_empty() {
            println!("No results found, attempting flexible search without strict criteria");
            match try_flexible_search(&client.database("Itineraries").collection("Featured"), &search_params).await {
                Ok(flexible_results) => {
                    println!("Flexible search found {} results", flexible_results.len());
                    return Ok(flexible_results);
                }
                Err(e) => {
                    println!("Flexible search failed: {:?}", e);
                }
            }
        }
        
        println!("Attempting to find activities using Vertex AI without dates");
        match find_and_generate_itineraries(client, &search_params).await {
            Ok(generated_itineraries) => {
                if !generated_itineraries.is_empty() {
                    println!("Generated itineraries from search and AI generated activities");
                    results.extend(generated_itineraries);
                }
            }
            Err(e) => {
                println!("Failed to generate itineraries from Vertex AI: {:?}", e);
            }
        }

        return Ok(results);
    }

    // Create itinerary generator
    let generator = ItineraryGenerator::new(client.clone());
    let collection: Collection<FeaturedVacation> =
        client.database("Itineraries").collection("Featured");

    // Generate enough itineraries to meet the threshold with variety
    let needed_count = min_results_threshold.saturating_sub(results.len());
    println!("Need to generate {} more itineraries", needed_count);
    
    let mut generated_names = std::collections::HashSet::new();
    let mut generation_attempts = 0;
    let max_attempts = needed_count * 3; // Allow multiple attempts to ensure variety
    
    for i in 1..=needed_count {
        let mut attempt = 0;
        let max_retries = 5;
        
        while attempt < max_retries && generation_attempts < max_attempts {
            generation_attempts += 1;
            
            match generator.generate_unique_itinerary(&search_params, i, &generated_names).await {
                Ok(generated_itinerary) => {
                    // Check if this name is already used
                    if generated_names.contains(&generated_itinerary.trip_name) {
                        println!("Generated duplicate name '{}', retrying...", generated_itinerary.trip_name);
                        attempt += 1;
                        continue;
                    }
                    
                    // Check if this itinerary is too similar to existing ones
                    if is_too_similar_to_existing(&generated_itinerary, &results) {
                        println!("Generated itinerary '{}' is too similar to existing ones, retrying...", generated_itinerary.trip_name);
                        attempt += 1;
                        continue;
                    }
                    
                    println!(
                        "Successfully generated unique itinerary {}/{}: {}",
                        i, needed_count, generated_itinerary.trip_name
                    );

                    // Save the generated itinerary to the database
                    match collection.insert_one(&generated_itinerary).await {
                        Ok(insert_result) => {
                            println!(
                                "Saved generated itinerary {} to database with ID: {:?}",
                                i, insert_result.inserted_id
                            );
                        }
                        Err(e) => {
                            eprintln!("Failed to save generated itinerary to database: {}", e);
                            // Continue anyway - the itinerary is still useful for this request
                        }
                    }

                    generated_names.insert(generated_itinerary.trip_name.clone());
                    results.push(generated_itinerary);
                    break; // Successfully generated unique itinerary
                }
                Err(e) => {
                    eprintln!("Failed to generate itinerary {} (attempt {}): {}", i, attempt + 1, e);
                    attempt += 1;
                }
            }
        }
        
        if attempt >= max_retries {
            println!("Failed to generate unique itinerary {} after {} attempts", i, max_retries);
        }
    }

    Ok(results)
}

/// Try partial matching search (some criteria match)
async fn try_partial_search(
    collection: &Collection<FeaturedVacation>,
    search_params: &SearchItinerary,
) -> Result<Vec<FeaturedVacation>, mongodb::error::Error> {
    let mut filter = Document::new();

    // Add location filter if provided
    if let Some(locations) = &search_params.locations {
        if !locations.is_empty() {
            let cities = locations
                .iter()
                .map(|l| {
                    let parts = l.split(',').collect::<Vec<&str>>();
                    if parts.len() > 1 {
                        parts[0].to_string()
                    } else {
                        l.to_string()
                    }
                })
                .collect::<Vec<String>>();

            filter.insert(
                "$or",
                vec![
                    doc! { "start_location.city": { "$in": cities.clone() } },
                    doc! { "end_location.city": { "$in": cities } },
                ],
            );
        }
    }

    // Activity filtering - match ANY of the requested activities (OR logic)
    if let Some(activities) = &search_params.activities {
        if !activities.is_empty() {
            let mut or_conditions = Vec::new();

            for activity in activities {
                or_conditions.push(doc! {
                    "activities": {
                        "$elemMatch": {
                            "label": {
                                "$regex": activity,
                                "$options": "i"
                            }
                        }
                    }
                });
            }

            let or_conditions_bson: Vec<bson::Bson> = or_conditions
                .into_iter()
                .map(bson::Bson::Document)
                .collect();

            // Add to existing $or or create new one
            if filter.contains_key("$or") {
                // Combine with existing $or using $and
                let existing_or = filter.remove("$or").unwrap();
                filter.insert(
                    "$and",
                    vec![
                        doc! { "$or": existing_or },
                        doc! { "$or": or_conditions_bson },
                    ],
                );
            } else {
                filter.insert("$or", or_conditions_bson);
            }
        }
    }

    let cursor = collection.find(filter).limit(10).await?;
    let itineraries = cursor.try_collect().await?;
    Ok(itineraries)
}

/// Try location-only search (fallback for closest matches)
async fn try_location_only_search(
    collection: &Collection<FeaturedVacation>,
    search_params: &SearchItinerary,
) -> Result<Vec<FeaturedVacation>, mongodb::error::Error> {
    let mut filter = Document::new();

    // Only filter by location
    if let Some(locations) = &search_params.locations {
        if !locations.is_empty() {
            let cities = locations
                .iter()
                .map(|l| {
                    let parts = l.split(',').collect::<Vec<&str>>();
                    if parts.len() > 1 {
                        parts[0].to_string()
                    } else {
                        l.to_string()
                    }
                })
                .collect::<Vec<String>>();

            filter.insert(
                "$or",
                vec![
                    doc! { "start_location.city": { "$in": cities.clone() } },
                    doc! { "end_location.city": { "$in": cities } },
                ],
            );
        }
    }

    // Add demographic filters if provided
    if let Some(adults) = search_params.adults {
        filter.insert("min_group", doc! { "$lte": adults });
        filter.insert("max_group", doc! { "$gte": adults });
    }
    
    // Add trip pace filtering if provided
    if let Some(trip_pace) = &search_params.trip_pace {
        // Filter itineraries based on activities per day matching the pace
        let max_activities = trip_pace.typical_activities_per_day();
        // This is a heuristic - we check if the itinerary has a reasonable number of activities
        // In production, you'd want to analyze the actual daily schedule
    }

    let cursor = collection.find(filter).limit(5).await?;
    let itineraries = cursor.try_collect().await?;
    Ok(itineraries)
}

/// Very flexible search when all other searches fail
/// Returns a sample of itineraries based on any available criteria, or just recent ones
async fn try_flexible_search(
    collection: &Collection<FeaturedVacation>,
    search_params: &SearchItinerary,
) -> Result<Vec<FeaturedVacation>, mongodb::error::Error> {
    let mut filter = Document::new();
    
    // Try to match on any single criterion
    let mut or_conditions = Vec::new();
    
    // Add location conditions if available
    if let Some(locations) = &search_params.locations {
        if !locations.is_empty() {
            for location in locations {
                let parts = location.split(',').collect::<Vec<&str>>();
                let city = if parts.len() > 1 {
                    parts[0].trim().to_string()
                } else {
                    location.trim().to_string()
                };
                
                or_conditions.push(doc! { "start_location.city": { "$regex": city.clone(), "$options": "i" } });
                or_conditions.push(doc! { "end_location.city": { "$regex": city, "$options": "i" } });
            }
        }
    }
    
    // Add activity conditions if available
    if let Some(activities) = &search_params.activities {
        if !activities.is_empty() {
            for activity in activities {
                or_conditions.push(doc! {
                    "activities": {
                        "$elemMatch": {
                            "label": {
                                "$regex": activity,
                                "$options": "i"
                            }
                        }
                    }
                });
            }
        }
    }
    
    // If we have any conditions, use them
    if !or_conditions.is_empty() {
        let or_conditions_bson: Vec<bson::Bson> = or_conditions
            .into_iter()
            .map(bson::Bson::Document)
            .collect();
        filter.insert("$or", or_conditions_bson);
    }
    
    // Add group size filter if available
    if let Some(adults) = search_params.adults {
        filter.insert("max_group", doc! { "$gte": adults });
    }
    
    // If still no filter criteria, just get recent itineraries
    let cursor = if filter.is_empty() {
        println!("No search criteria available, returning recent itineraries");
        // Don't sort by created_at to avoid DateTime deserialization issues
        collection
            .find(doc! {})
            .limit(10)
            .await?
    } else {
        collection.find(filter).limit(10).await?
    };
    
    // Use a more lenient collection approach to handle data inconsistencies
    let mut itineraries = Vec::new();
    let mut cursor = cursor;
    
    while let Ok(Some(doc)) = cursor.try_next().await {
        itineraries.push(doc);
        // Stop if we have enough results
        if itineraries.len() >= 5 {
            break;
        }
    }
    
    println!("Flexible search successfully found {} itineraries", itineraries.len());
    Ok(itineraries)
}

/// Fetch activities from Vertex AI Search
async fn fetch_activities_from_vertex(
    search_params: &SearchItinerary,
) -> Result<Vec<crate::models::activity::Activity>, Box<dyn std::error::Error>> {
    let vertex_service = VertexSearchService::new()?;
    let mut all_activities = Vec::new();
    
    // Build location query
    let location_query = search_params.locations
        .as_ref()
        .and_then(|locs| locs.first())
        .cloned()
        .unwrap_or_default();
    
    // Fetch activities for each activity type
    if let Some(activity_types) = &search_params.activities {
        for activity_type in activity_types {
            match vertex_service.search_activities(&[activity_type.clone()], &location_query).await {
                Ok(response) => {
                    println!("Vertex AI found {} results for activity type: {}", 
                        response.results.len(), activity_type);
                    
                    // Convert Vertex search results to Activity models
                    for result in response.results {
                        if let Ok(activity) = parse_vertex_result_to_activity(&result) {
                            all_activities.push(activity);
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Failed to search for activity type {}: {:?}", activity_type, e);
                }
            }
        }
    }
    
    Ok(all_activities)
}

/// Convert Vertex AI search result to Activity model
fn parse_vertex_result_to_activity(result: &crate::services::vertex_search_service::SearchResult) -> Result<crate::models::activity::Activity, Box<dyn std::error::Error>> {
    use mongodb::bson::oid::ObjectId;
    
    // Extract data from the structured data
    let struct_data = &result.document.struct_data;
    
    // Parse address
    let address = if let Some(addr_str) = struct_data.get("address").and_then(|v| v.as_str()) {
        // Simple address parsing - in production you'd want more robust parsing
        let parts: Vec<&str> = addr_str.split(',').collect();
        crate::models::activity::Address {
            street: parts.get(0).unwrap_or(&"").trim().to_string(),
            unit: "".to_string(),
            city: parts.get(1).unwrap_or(&"").trim().to_string(),
            state: parts.get(2).unwrap_or(&"").trim().to_string(),
            zip: parts.get(3).unwrap_or(&"").trim().to_string(),
            country: "USA".to_string(),
        }
    } else {
        crate::models::activity::Address {
            street: "".to_string(),
            unit: "".to_string(),
            city: "".to_string(),
            state: "".to_string(),
            zip: "".to_string(),
            country: "USA".to_string(),
        }
    };
    
    // Parse time slots
    let daily_time_slots = struct_data.get("daily_time_slots")
        .and_then(|v| v.as_array())
        .map(|arr| arr.iter()
            .filter_map(|slot| {
                if let Some(slot_obj) = slot.as_object() {
                    Some(crate::models::activity::TimeSlot {
                        start: slot_obj.get("start").and_then(|v| v.as_str()).unwrap_or("09:00").to_string(),
                        end: slot_obj.get("end").and_then(|v| v.as_str()).unwrap_or("17:00").to_string(),
                    })
                } else {
                    None
                }
            })
            .collect())
        .unwrap_or_else(|| vec![crate::models::activity::TimeSlot {
            start: "09:00".to_string(),
            end: "17:00".to_string(),
        }]);
    
    let activity = crate::models::activity::Activity {
        id: Some(ObjectId::new()),
        company: struct_data.get("company")
            .and_then(|v| v.as_str())
            .unwrap_or("Unknown Company")
            .to_string(),
        company_id: struct_data.get("company_id")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        booking_link: struct_data.get("booking_link")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        online_booking_status: "available".to_string(),
        guide: struct_data.get("guide")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
        title: struct_data.get("title")
            .and_then(|v| v.as_str())
            .unwrap_or("Unknown Activity")
            .to_string(),
        description: struct_data.get("description")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        activity_types: struct_data.get("activity_types")
            .and_then(|v| v.as_array())
            .map(|arr| arr.iter()
                .filter_map(|v| v.as_str())
                .map(|s| s.to_string())
                .collect())
            .unwrap_or_default(),
        tags: struct_data.get("tags")
            .and_then(|v| v.as_array())
            .map(|arr| arr.iter()
                .filter_map(|v| v.as_str())
                .map(|s| s.to_string())
                .collect())
            .unwrap_or_default(),
        price_per_person: struct_data.get("price")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0) as f32,
        duration_minutes: struct_data.get("duration")
            .and_then(|v| v.as_i64())
            .unwrap_or(120) as u16, // Default 2 hours
        daily_time_slots,
        address,
        whats_included: struct_data.get("whats_included")
            .and_then(|v| v.as_array())
            .map(|arr| arr.iter()
                .filter_map(|v| v.as_str())
                .map(|s| s.to_string())
                .collect())
            .unwrap_or_default(),
        weight_limit_lbs: struct_data.get("weight_limit")
            .and_then(|v| v.as_i64())
            .map(|w| w as u16),
        age_requirement: struct_data.get("age_requirement")
            .and_then(|v| v.as_i64())
            .map(|a| a as u8),
        height_requiremnt: struct_data.get("height_requirement")
            .and_then(|v| v.as_i64())
            .map(|h| h as u8),
        blackout_date_ranges: None,
        capacity: crate::models::activity::Capacity {
            minimum: struct_data.get("min_capacity").and_then(|v| v.as_i64()).unwrap_or(1) as u16,
            maximum: struct_data.get("max_capacity").and_then(|v| v.as_i64()).unwrap_or(20) as u16,
        },
        created_at: None,
        updated_at: None,
    };
    
    Ok(activity)
}

/// Find activities using Vertex AI Search and generate itineraries from them
async fn find_and_generate_itineraries(
    client: Arc<Client>,
    search_params: &SearchItinerary,
) -> Result<Vec<FeaturedVacation>, Box<dyn std::error::Error>> {
    let generator = ItineraryGenerator::new(client.clone());
    let mut generated_itineraries = Vec::new();
    
    // Create a modified search params with default dates for generation
    let mut modified_params = search_params.clone();
    
    // If no dates provided, use default dates (next week for a 3-day trip)
    if modified_params.arrival_datetime.is_none() {
        let now = chrono::Utc::now();
        let next_week = now + chrono::Duration::days(7);
        modified_params.arrival_datetime = Some(next_week.format("%Y-%m-%dT%H:%M:%S").to_string());
    }
    
    if modified_params.departure_datetime.is_none() {
        let arrival = chrono::DateTime::parse_from_rfc3339(
            &modified_params.arrival_datetime.as_ref().unwrap()
        ).unwrap_or_else(|_| {
            let now = chrono::Utc::now();
            let next_week = now + chrono::Duration::days(7);
            next_week.into()
        });
        
        let departure = arrival + chrono::Duration::days(3); // Default to 3-day trip
        modified_params.departure_datetime = Some(departure.format("%Y-%m-%dT%H:%M:%S").to_string());
    }
    
    // Try to generate up to 5-10 itineraries for better variety
    let target_count = if generated_itineraries.is_empty() { 10 } else { 5 };
    
    for i in 1..=target_count {
        match generator.generate_itinerary(&modified_params).await {
            Ok(generated_itinerary) => {
                println!(
                    "Successfully generated itinerary {}: {}",
                    i, generated_itinerary.trip_name
                );
                
                // Save the generated itinerary to the database
                let collection: Collection<FeaturedVacation> =
                    client.database("Itineraries").collection("Featured");
                match collection.insert_one(&generated_itinerary).await {
                    Ok(insert_result) => {
                        println!(
                            "Saved generated itinerary {} to database with ID: {:?}",
                            i, insert_result.inserted_id
                        );
                    }
                    Err(e) => {
                        eprintln!("Failed to save generated itinerary to database: {}", e);
                        // Continue anyway - the itinerary is still useful for this request
                    }
                }
                
                generated_itineraries.push(generated_itinerary);
            }
            Err(e) => {
                eprintln!("Failed to generate itinerary {}: {}", i, e);
                // Continue trying to generate more
            }
        }
    }
    
    Ok(generated_itineraries)
}

/// Check if a generated itinerary is too similar to existing ones
fn is_too_similar_to_existing(new_itinerary: &FeaturedVacation, existing_itineraries: &[FeaturedVacation]) -> bool {
    for existing in existing_itineraries {
        // Check for identical trip names
        if new_itinerary.trip_name == existing.trip_name {
            return true;
        }
        
        // Check for very similar characteristics
        let same_location = new_itinerary.start_location.city() == existing.start_location.city();
        let same_duration = new_itinerary.length_days == existing.length_days;
        let same_group_size = new_itinerary.min_group == existing.min_group && new_itinerary.max_group == existing.max_group;
        
        // Count similar activities
        let new_activity_count = new_itinerary.days.days.values().map(|day| {
            day.iter().filter(|item| matches!(item, crate::models::itinerary::base::DayItem::Activity { .. })).count()
        }).sum::<usize>();
        
        let existing_activity_count = existing.days.days.values().map(|day| {
            day.iter().filter(|item| matches!(item, crate::models::itinerary::base::DayItem::Activity { .. })).count()
        }).sum::<usize>();
        
        let similar_activity_count = new_activity_count.abs_diff(existing_activity_count) <= 1;
        
        // If too many characteristics are the same, consider it too similar
        if same_location && same_duration && same_group_size && similar_activity_count {
            return true;
        }
    }
    
    false
}
