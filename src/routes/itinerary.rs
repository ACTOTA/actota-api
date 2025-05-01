use crate::models::itinerary::base::{Activity, ItinerarySubmission};
use crate::models::{itinerary::base::FeaturedVacation, search::SearchItinerary};
use crate::services::itinerary_search_service::search_itineraries;
use crate::services::itinerary_service::get_images;
use actix_web::{web, HttpResponse, Responder};
use bson::doc;
use futures::TryStreamExt;
use mongodb::{bson::oid::ObjectId, Client};
use std::sync::Arc;

/*
    /api/itineraries/{id}
*/
pub async fn get_by_id(path: web::Path<String>, data: web::Data<Arc<Client>>) -> impl Responder {
    let client = data.into_inner();
    let collection: mongodb::Collection<FeaturedVacation> =
        client.database("Itineraries").collection("Featured");
    let id: ObjectId = match ObjectId::parse_str(path.into_inner().as_str()) {
        Ok(id) => id,
        Err(_) => return HttpResponse::BadRequest().body("Invalid ID"),
    };

    let filter = doc! { "_id": id };

    println!("Filter: {:?}", filter);

    match collection.find_one(filter).await {
        Ok(Some(doc)) => {
            let processed_doc = get_images(vec![doc.clone()]).await;

            // Add await here to resolve the future
            match processed_doc[0].clone().populate(&client).await {
                Ok(populated) => HttpResponse::Ok().json(populated),
                Err(err) => {
                    eprintln!("Failed to populate data: {:?}", err);
                    HttpResponse::InternalServerError().body("Failed to populate itinerary data")
                }
            }
        }
        Ok(None) => HttpResponse::NotFound().body("Itinerary not found"),
        Err(err) => {
            eprintln!("Failed to retrieve itinerary: {:?}", err);
            HttpResponse::InternalServerError().body("Failed to retrieve itinerary")
        }
    }
}

/*
    /api/itineraries (Get all itineraries or filter by search criteria - public endpoint)
*/
pub async fn get_all(
    data: web::Data<Arc<Client>>,
    search_data: Option<web::Json<SearchItinerary>>,
) -> impl Responder {
    println!("Handling request for /api/itineraries");

    // Log cloud storage environment variables (without values)
    if let Ok(_) = std::env::var("CLOUD_STORAGE_URL") {
        println!("CLOUD_STORAGE_URL is set");
    } else {
        println!("CLOUD_STORAGE_URL is NOT set");
    }

    if let Ok(_) = std::env::var("ITINERARY_BUCKET") {
        println!("ITINERARY_BUCKET is set");
    } else {
        println!("ITINERARY_BUCKET is NOT set");
    }

    if let Ok(_) = std::env::var("GOOGLE_APPLICATION_CREDENTIALS") {
        println!("GOOGLE_APPLICATION_CREDENTIALS is set");
    } else {
        println!("GOOGLE_APPLICATION_CREDENTIALS is NOT set");
    }

    let client = data.into_inner();

    // If search criteria are provided, use them to filter results
    if let Some(search_params) = search_data {
        println!("Search criteria provided, filtering itineraries");

        // Log the search query to the Travelers.Submission collection
        let search_query = search_params.clone();
        let submission_collection: mongodb::Collection<ItinerarySubmission> =
            client.database("Travelers").collection("Submission");

        // Convert SearchItinerary to ItinerarySubmission for logging
        // Only attempt this if we have enough data to make a meaningful log
        if search_query
            .locations
            .as_ref()
            .map_or(false, |locs| !locs.is_empty())
        {
            // Create a minimal submission record from the search parameters
            let search_log = ItinerarySubmission {
                id: None,
                user_id: None, // Anonymous search
                location_start: search_query
                    .locations
                    .as_ref()
                    .and_then(|l| l.first().cloned())
                    .unwrap_or_default(),
                location_end: search_query
                    .locations
                    .as_ref()
                    .and_then(|l| l.last().cloned())
                    .unwrap_or_default(),
                arrival_datetime: chrono::Utc::now(), // Default to current time
                departure_datetime: chrono::Utc::now() + chrono::Duration::days(7), // Default to a week later
                adults: search_query.adults.unwrap_or(1),
                children: search_query.children.unwrap_or(0),
                infants: search_query.infants.unwrap_or(0),
                pets: 0,
                activities: search_query
                    .activities
                    .unwrap_or_default()
                    .into_iter()
                    .map(|label| Activity {
                        label,
                        description: String::new(),
                        tags: Vec::new(),
                    })
                    .collect(),
                lodging: search_query.lodging.unwrap_or_default(),
                transportation: search_query.transportation.unwrap_or_default(),
                budger_per_person: None,
                interests: None,
                created_at: Some(chrono::Utc::now()),
                updated_at: Some(chrono::Utc::now()),
            };

            // Log the search asynchronously (don't wait for result)
            let _log_client = Arc::clone(&client);
            tokio::spawn(async move {
                match submission_collection.insert_one(&search_log).await {
                    Ok(_) => println!("Search query logged successfully"),
                    Err(e) => eprintln!("Failed to log search query: {:?}", e),
                }
            });
        }

        // Search for itineraries matching the criteria
        match search_itineraries(Arc::clone(&client), search_params.into_inner()).await {
            Ok(itineraries) => {
                if itineraries.is_empty() {
                    return HttpResponse::Ok().json(Vec::<FeaturedVacation>::new());
                }

                println!("Found {} matching itineraries", itineraries.len());

                // Process images for the found itineraries
                let processed_itineraries = get_images(itineraries).await;

                HttpResponse::Ok().json(processed_itineraries)
            }
            Err(err) => {
                eprintln!("Error searching for itineraries: {:?}", err);
                HttpResponse::InternalServerError().body("Failed to search for itineraries")
            }
        }
    } else {
        // No search criteria provided, return all itineraries
        let collection = client
            .database("Itineraries")
            .collection::<FeaturedVacation>("Featured");

        // Get all itineraries
        let sort_options = doc! { "createdAt": -1 };
        let cursor = collection.find(doc! {}).sort(sort_options).limit(100).await;

        match cursor {
            Ok(cursor) => match cursor.try_collect::<Vec<FeaturedVacation>>().await {
                Ok(itineraries) => {
                    if itineraries.is_empty() {
                        return HttpResponse::Ok().json(Vec::<FeaturedVacation>::new());
                    }

                    println!("Found {} itineraries in database", itineraries.len());

                    // Process images for all itineraries
                    let processed_itineraries = get_images(itineraries).await;
                    println!(
                        "Processed {} itineraries with images",
                        processed_itineraries.len()
                    );

                    HttpResponse::Ok().json(processed_itineraries)
                }
                Err(err) => {
                    eprintln!("Failed to collect itineraries: {:?}", err);
                    return HttpResponse::InternalServerError()
                        .body("Failed to process itineraries");
                }
            },
            Err(err) => {
                eprintln!("Failed to retrieve itineraries: {:?}", err);
                HttpResponse::InternalServerError().body("Failed to retrieve itineraries")
            }
        }
    }
}
