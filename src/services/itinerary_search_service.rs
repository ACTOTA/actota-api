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

    if let Some(activities) = &search_params.activities {
        if !activities.is_empty() {
            // Look for activities matching any of the requested ones
            filter.insert(
                "activities.label",
                doc! { "$in": activities
                .iter()
                .map(|s| s.to_lowercase())
                .collect::<Vec<String>>() },
            );
        }
    }

    if let Some(lodging) = &search_params.lodging {
        if !lodging.is_empty() {
            // This would need to be adjusted based on how lodging is stored in the itinerary
            // For now, assuming there might be lodging tags in activities
            filter.insert(
                "activities.tags",
                doc! { "$in": lodging.iter().map(|s| s.to_lowercase()).collect::<Vec<String>>() },
            );
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
