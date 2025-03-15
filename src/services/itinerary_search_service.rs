use crate::models::{itinerary::FeaturedVacation, search::SearchItinerary};
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
            filter.insert(
                "$or",
                vec![
                    doc! { "start_location.city": { "$in": locations } },
                    doc! { "end_location.city": { "$in": locations } },
                ],
            );
        }
    }

    // Handle activity and lodging filters
    let mut or_conditions = Vec::new();
    
    // Activity filtering
    if let Some(activities) = &search_params.activities {
        if !activities.is_empty() {
            let activities_lowercase: Vec<String> = activities
                .iter()
                .map(|s| s.to_lowercase())
                .collect();
            
            // Create a condition that matches itineraries where any activity label matches any of the requested activities
            // We're using case-insensitive matching by converting both sides to lowercase
            let activity_filter = doc! {
                "$or": activities_lowercase.iter().map(|activity| {
                    doc! {
                        "activities": {
                            "$elemMatch": {
                                "label": { 
                                    "$regex": activity, 
                                    "$options": "i"  // case-insensitive match
                                }
                            }
                        }
                    }
                }).collect::<Vec<_>>()
            };
            
            or_conditions.push(activity_filter);
        }
    }

    // Lodging filtering
    if let Some(lodging) = &search_params.lodging {
        if !lodging.is_empty() {
            let lodging_lowercase: Vec<String> = lodging
                .iter()
                .map(|s| s.to_lowercase())
                .collect();
            
            // Create a condition that matches itineraries where any activity has tags matching any of the requested lodging types
            let lodging_filter = doc! {
                "$or": lodging_lowercase.iter().map(|lodging_type| {
                    doc! {
                        "activities": {
                            "$elemMatch": {
                                "tags": { 
                                    "$regex": lodging_type, 
                                    "$options": "i"  // case-insensitive match
                                }
                            }
                        }
                    }
                }).collect::<Vec<_>>()
            };
            
            or_conditions.push(lodging_filter);
        }
    }
    
    // Add conditions to the filter
    if !or_conditions.is_empty() {
        if or_conditions.len() == 1 {
            // If only one condition type was specified (either activities or lodging),
            // add it directly to the filter
            filter.extend(or_conditions[0].clone());
        } else {
            // If both activities and lodging were specified, use $or to match either
            filter.insert("$or", or_conditions);
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
