use crate::models::{itinerary::base::FeaturedVacation, search::SearchItinerary};
use crate::services::itinerary_generation_service::ItineraryGenerator;
use bson::{doc, Document};
use futures::TryStreamExt;
use mongodb::{Client, Collection};
use std::sync::Arc;

pub async fn search_itineraries(
    client: Arc<Client>,
    search_params: SearchItinerary,
) -> Result<Vec<FeaturedVacation>, mongodb::error::Error> {
    let collection: Collection<FeaturedVacation> =
        client.database("Itineraries").collection("Featured");

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

    // If we have enough results, return them
    if results.len() >= min_results_threshold {
        return Ok(results);
    }

    // If not enough results, try to generate a new itinerary
    println!(
        "Found only {} itineraries, generating new ones to meet threshold of {}",
        results.len(),
        min_results_threshold
    );

    // Check if we have the required fields for generation
    if search_params.arrival_datetime.is_none() || search_params.departure_datetime.is_none() {
        println!("Cannot generate itinerary without arrival and departure dates, returning existing results (including partial matches)");
        return Ok(results);
    }

    // Create itinerary generator
    let generator = ItineraryGenerator::new(client.clone());

    // Try to generate a new itinerary
    match generator.generate_itinerary(&search_params).await {
        Ok(generated_itinerary) => {
            println!(
                "Successfully generated new itinerary: {}",
                generated_itinerary.trip_name
            );

            // Save the generated itinerary to the database
            let collection: Collection<FeaturedVacation> =
                client.database("Itineraries").collection("Featured");
            match collection.insert_one(&generated_itinerary).await {
                Ok(insert_result) => {
                    println!(
                        "Saved generated itinerary to database with ID: {:?}",
                        insert_result.inserted_id
                    );
                }
                Err(e) => {
                    eprintln!("Failed to save generated itinerary to database: {}", e);
                    // Continue anyway - the itinerary is still useful for this request
                }
            }

            results.push(generated_itinerary);
        }
        Err(e) => {
            eprintln!("Failed to generate itinerary: {}", e);
            // Return existing results even if generation failed
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

    let cursor = collection.find(filter).limit(5).await?;
    let itineraries = cursor.try_collect().await?;
    Ok(itineraries)
}
