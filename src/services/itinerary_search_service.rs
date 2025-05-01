use crate::models::{itinerary::base::FeaturedVacation, search::SearchItinerary};
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
                lodging_conditions.push(doc! {
                    "activities": {
                        "$elemMatch": {
                            "tags": {
                                "$regex": lodging_type,
                                "$options": "i"  // case-insensitive match
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
