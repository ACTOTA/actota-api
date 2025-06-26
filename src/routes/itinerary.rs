use crate::models::itinerary::base::{Activity, ItinerarySubmission};
use crate::models::itinerary::populated::PopulatedFeaturedVacation;
use crate::models::{itinerary::base::FeaturedVacation, search::SearchItinerary};
use crate::services::itinerary_search_service::search_or_generate_itineraries;
use crate::services::itinerary_service::get_images;
use crate::services::search_scoring::AsyncSearchScorer;
use actix_web::{web, HttpResponse, Responder};
use bson::{doc, DateTime};
use futures::TryStreamExt;
use mongodb::{bson::oid::ObjectId, Client};
use serde::Deserialize;
use std::sync::Arc;

#[derive(Deserialize)]
pub struct PaginationQuery {
    pub limit: Option<i64>,
    pub page: Option<i64>,
}

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
    // Before attempting to deserialize
    //
    println!(
        "\n\nDocument structure: {}",
        bson::to_document(&filter).unwrap()
    );

    // Try deserializing just the problematic field
    if let Ok(cost) = filter.get_i32("min_group") {
        println!("Successfully extracted person_cost: {}", cost);
    } else {
        println!(
            "Failed to extract person_cost: {:?}",
            filter.get("person_cost")
        );
    }

    match collection.find_one(filter).await {
        Ok(Some(doc)) => {
            println!("=== FULL DOCUMENT STRUCTURE ===");

            // Get the raw BSON document
            let raw_doc = match bson::to_raw_document_buf(&doc) {
                Ok(raw) => raw,
                Err(e) => {
                    println!("Error converting to raw document: {:?}", e);
                    return HttpResponse::InternalServerError().body("Conversion error");
                }
            };

            println!("{:#?}", raw_doc);

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
    /api/itineraries (Get all itineraries - public endpoint)
*/
pub async fn get_all(
    data: web::Data<Arc<Client>>,
    query: web::Query<PaginationQuery>,
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

    // Return all itineraries
    let collection = client
        .database("Itineraries")
        .collection::<FeaturedVacation>("Featured");

    // Extract pagination parameters with defaults
    let limit = query.limit.unwrap_or(10); // Default to 10 items per page
    let page = query.page.unwrap_or(1); // Default to page 1
    let skip = (page - 1) * limit;

    println!(
        "Pagination - page: {}, limit: {}, skip: {}",
        page, limit, skip
    );

    // Get itineraries with pagination
    let sort_options = doc! { "created_at": -1 };
    let cursor = collection
        .find(doc! {})
        .sort(sort_options)
        .skip(skip as u64)
        .limit(limit)
        .await;

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

                // Populate all itineraries concurrently
                let populate_futures: Vec<_> = processed_itineraries
                    .iter()
                    .map(|itinerary| {
                        let client_clone = client.clone();
                        let itinerary_clone = itinerary.clone();
                        async move { itinerary_clone.populate(&client_clone).await }
                    })
                    .collect();

                let populate_results = futures::future::join_all(populate_futures).await;

                let mut populated_itineraries = Vec::new();
                for result in populate_results {
                    match result {
                        Ok(populated) => populated_itineraries.push(populated),
                        Err(err) => {
                            eprintln!("Failed to populate itinerary: {:?}", err);
                            // Skip this itinerary if population fails
                        }
                    }
                }

                if !populated_itineraries.is_empty() {
                    HttpResponse::Ok().json(populated_itineraries)
                } else {
                    // Fallback to original itineraries if population failed
                    HttpResponse::Ok().json(processed_itineraries)
                }
            }
            Err(err) => {
                eprintln!("Failed to collect itineraries: {:?}", err);
                return HttpResponse::InternalServerError().body("Failed to process itineraries");
            }
        },
        Err(err) => {
            eprintln!("Failed to retrieve itineraries: {:?}", err);
            HttpResponse::InternalServerError().body("Failed to retrieve itineraries")
        }
    }
}

/*
    /api/itineraries/search (Search itineraries with intelligent generation fallback)

    This endpoint now uses smart search-or-generate functionality:
    1. Searches existing itineraries first
    2. If insufficient results found (< MIN_SEARCH_RESULTS), generates new optimized itineraries
    3. Uses route optimization with Google Maps for realistic travel times
    4. Returns mix of existing + generated itineraries for better user experience

    Environment variables:
    - MIN_SEARCH_RESULTS: Minimum results before triggering generation (default: 3)
    - GOOGLE_MAPS_API_KEY: For real driving distances and traffic-aware routing
*/
pub async fn search_itineraries_endpoint(
    data: web::Data<Arc<Client>>,
    search_params: web::Json<SearchItinerary>,
) -> impl Responder {
    println!("Handling search request for /api/itineraries/search");
    println!("Search params: {:?}", search_params);

    let client = data.into_inner();
    let search_query = search_params.into_inner();

    // Log the search query to the Travelers.Submission collection
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
        let now = DateTime::now();
        let chrono_date = chrono::Utc::now() + chrono::Duration::days(7);
        let week_out = bson::DateTime::from_millis(chrono_date.timestamp_millis());

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
            arrival_datetime: now,        // Default to current time
            departure_datetime: week_out, // Default to a week later
            adults: search_query.adults.unwrap_or(1),
            children: search_query.children.unwrap_or(0),
            infants: search_query.infants.unwrap_or(0),
            pets: 0,
            activities: search_query
                .activities
                .as_ref()
                .unwrap_or(&Vec::new())
                .iter()
                .map(|label| Activity {
                    label: label.clone(),
                    description: String::new(),
                    tags: Vec::new(),
                })
                .collect(),
            lodging: search_query.lodging.as_ref().unwrap_or(&Vec::new()).clone(),
            transportation: search_query
                .transportation
                .as_ref()
                .unwrap_or(&String::new())
                .clone(),
            budget_per_person: None,
            interests: None,
            created_at: Some(now),
            updated_at: Some(now),
        };

        // Log the search asynchronously (don't wait for result)
        let log_client = Arc::clone(&client);
        tokio::spawn(async move {
            match submission_collection.insert_one(&search_log).await {
                Ok(_) => println!("Search query logged successfully"),
                Err(e) => eprintln!("Failed to log search query: {:?}", e),
            }
        });
    }

    // Use search-or-generate functionality for better user experience
    // Define minimum results threshold (configurable via env var)
    let min_results_threshold = std::env::var("MIN_SEARCH_RESULTS")
        .ok()
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(5); // Default to 5 minimum results to ensure generation

    println!(
        "Using search-or-generate with threshold: {}",
        min_results_threshold
    );

    // Use search_or_generate_itineraries which includes route optimization
    match search_or_generate_itineraries(
        client.as_ref().clone(),
        search_query.clone(),
        min_results_threshold,
    )
    .await
    {
        Ok(itineraries) => {
            if itineraries.is_empty() {
                return HttpResponse::Ok().json(Vec::<PopulatedFeaturedVacation>::new());
            }

            println!(
                "Found/generated {} itineraries for frontend search",
                itineraries.len()
            );

            // Process images for all itineraries
            let processed_itineraries = get_images(itineraries).await;

            // Initialize the async search scorer for better activity matching
            let scorer = AsyncSearchScorer::new(client.as_ref().clone());

            // Score all itineraries (existing and generated) with database lookup
            let scored_results = scorer
                .score_and_rank_itineraries(processed_itineraries.clone(), &search_query)
                .await;

            // Calculate max possible score once
            let max_possible_score = scorer.weights.location_weight
                + scorer.weights.activity_weight
                + scorer.weights.group_size_weight
                + scorer.weights.lodging_weight
                + scorer.weights.transportation_weight
                + scorer.weights.trip_pace_weight;

            // Populate all itineraries concurrently with scores
            let populate_futures: Vec<_> = processed_itineraries
                .iter()
                .map(|itinerary| {
                    let client_clone = client.clone();
                    let itinerary_clone = itinerary.clone();
                    let scored_result = scored_results
                        .iter()
                        .find(|s| s.itinerary.id == itinerary.id)
                        .cloned();

                    async move {
                        match itinerary_clone.populate(&client_clone).await {
                            Ok(mut populated) => {
                                // Apply scores if found
                                if let Some(scored) = scored_result {
                                    // Normalize total score to 0-100 scale
                                    let normalized_score = if max_possible_score > 0.0 {
                                        ((scored.total_score / max_possible_score) * 100.0).min(100.0).max(0.0) as u8
                                    } else {
                                        0
                                    };
                                    populated.set_match_score(normalized_score);

                                    // Normalize individual score components to 0-100 range
                                    let mut normalized_breakdown = scored.score_breakdown.clone();
                                    
                                    // Normalize each component based on its maximum possible weight
                                    normalized_breakdown.location_score = if scorer.weights.location_weight > 0.0 {
                                        ((normalized_breakdown.location_score / scorer.weights.location_weight) * 100.0).min(100.0).max(0.0)
                                    } else { 0.0 };
                                    
                                    normalized_breakdown.activity_score = if scorer.weights.activity_weight > 0.0 {
                                        ((normalized_breakdown.activity_score / scorer.weights.activity_weight) * 100.0).min(100.0).max(0.0)
                                    } else { 0.0 };
                                    
                                    normalized_breakdown.group_size_score = if scorer.weights.group_size_weight > 0.0 {
                                        ((normalized_breakdown.group_size_score / scorer.weights.group_size_weight) * 100.0).min(100.0).max(0.0)
                                    } else { 0.0 };
                                    
                                    normalized_breakdown.lodging_score = if scorer.weights.lodging_weight > 0.0 {
                                        ((normalized_breakdown.lodging_score / scorer.weights.lodging_weight) * 100.0).min(100.0).max(0.0)
                                    } else { 0.0 };
                                    
                                    normalized_breakdown.transportation_score = if scorer.weights.transportation_weight > 0.0 {
                                        ((normalized_breakdown.transportation_score / scorer.weights.transportation_weight) * 100.0).min(100.0).max(0.0)
                                    } else { 0.0 };
                                    
                                    normalized_breakdown.trip_pace_score = if scorer.weights.trip_pace_weight > 0.0 {
                                        ((normalized_breakdown.trip_pace_score / scorer.weights.trip_pace_weight) * 100.0).min(100.0).max(0.0)
                                    } else { 0.0 };

                                    populated.set_score_breakdown(normalized_breakdown);
                                }

                                // Log generated itineraries for frontend visibility
                                if itinerary.tag.as_deref() == Some("generated") {
                                    println!(
                                        "Frontend receiving generated itinerary: {}",
                                        populated.trip_name()
                                    );
                                }

                                Ok(populated)
                            }
                            Err(err) => {
                                eprintln!("Failed to populate itinerary: {:?}", err);
                                Err(err)
                            }
                        }
                    }
                })
                .collect();

            let populate_results = futures::future::join_all(populate_futures).await;

            let mut populated_itineraries = Vec::new();
            for result in populate_results {
                if let Ok(populated) = result {
                    populated_itineraries.push(populated);
                }
            }

            // Copy match scores from populated itineraries to processed itineraries
            let mut processed_itineraries = processed_itineraries;
            for processed in &mut processed_itineraries {
                if let Some(populated) = populated_itineraries.iter().find(|p| p.id() == processed.id) {
                    processed.match_score = populated.match_score;
                    processed.score_breakdown = populated.score_breakdown.clone();
                }
            }

            // Return the base itineraries (not populated) to match expected frontend format
            // The populated version includes full activity objects which are too verbose for the frontend
            HttpResponse::Ok().json(processed_itineraries)
        }
        Err(err) => {
            eprintln!(
                "Failed to search/generate itineraries for frontend: {:?}",
                err
            );
            HttpResponse::InternalServerError().body("Failed to search or generate itineraries")
        }
    }
}

/*
    /api/itineraries/search-or-generate (Explicit search with generation fallback)

    This endpoint provides the same functionality as /search but with explicit naming.
    Both endpoints now use the same intelligent search-or-generate logic.
    This endpoint is kept for API compatibility and explicit use cases.
*/
pub async fn search_or_generate(
    data: web::Data<Arc<Client>>,
    search_params: web::Json<SearchItinerary>,
) -> impl Responder {
    println!("Handling search-or-generate request");
    println!("Search params: {:?}", search_params);

    let client = data.into_inner();
    let search_query = search_params.into_inner();

    // Define minimum results threshold (configurable via env var)
    let min_results_threshold = std::env::var("MIN_SEARCH_RESULTS")
        .ok()
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(3); // Default to 3 minimum results

    // Use search_or_generate_itineraries
    match search_or_generate_itineraries(
        client.as_ref().clone(),
        search_query.clone(),
        min_results_threshold,
    )
    .await
    {
        Ok(itineraries) => {
            if itineraries.is_empty() {
                return HttpResponse::Ok().json(Vec::<PopulatedFeaturedVacation>::new());
            }

            println!("Found/generated {} itineraries", itineraries.len());

            // Process images for all itineraries
            let processed_itineraries = get_images(itineraries).await;

            // Initialize the async search scorer for better activity matching
            let scorer = AsyncSearchScorer::new(client.as_ref().clone());

            // Score all itineraries (including generated ones) with database lookup
            let scored_results = scorer
                .score_and_rank_itineraries(processed_itineraries.clone(), &search_query)
                .await;

            // Calculate max possible score once
            let max_possible_score = scorer.weights.location_weight
                + scorer.weights.activity_weight
                + scorer.weights.group_size_weight
                + scorer.weights.lodging_weight
                + scorer.weights.transportation_weight
                + scorer.weights.trip_pace_weight;

            // Populate all itineraries concurrently with scores
            let populate_futures: Vec<_> = processed_itineraries
                .iter()
                .map(|itinerary| {
                    let client_clone = client.clone();
                    let itinerary_clone = itinerary.clone();
                    let scored_result = scored_results
                        .iter()
                        .find(|s| s.itinerary.id == itinerary.id)
                        .cloned();

                    async move {
                        match itinerary_clone.populate(&client_clone).await {
                            Ok(mut populated) => {
                                // Apply scores if found
                                if let Some(scored) = scored_result {
                                    // Normalize total score to 0-100 scale
                                    let normalized_score = if max_possible_score > 0.0 {
                                        ((scored.total_score / max_possible_score) * 100.0).min(100.0).max(0.0) as u8
                                    } else {
                                        0
                                    };
                                    populated.set_match_score(normalized_score);

                                    // Normalize individual score components to 0-100 range
                                    let mut normalized_breakdown = scored.score_breakdown.clone();
                                    
                                    // Normalize each component based on its maximum possible weight
                                    normalized_breakdown.location_score = if scorer.weights.location_weight > 0.0 {
                                        ((normalized_breakdown.location_score / scorer.weights.location_weight) * 100.0).min(100.0).max(0.0)
                                    } else { 0.0 };
                                    
                                    normalized_breakdown.activity_score = if scorer.weights.activity_weight > 0.0 {
                                        ((normalized_breakdown.activity_score / scorer.weights.activity_weight) * 100.0).min(100.0).max(0.0)
                                    } else { 0.0 };
                                    
                                    normalized_breakdown.group_size_score = if scorer.weights.group_size_weight > 0.0 {
                                        ((normalized_breakdown.group_size_score / scorer.weights.group_size_weight) * 100.0).min(100.0).max(0.0)
                                    } else { 0.0 };
                                    
                                    normalized_breakdown.lodging_score = if scorer.weights.lodging_weight > 0.0 {
                                        ((normalized_breakdown.lodging_score / scorer.weights.lodging_weight) * 100.0).min(100.0).max(0.0)
                                    } else { 0.0 };
                                    
                                    normalized_breakdown.transportation_score = if scorer.weights.transportation_weight > 0.0 {
                                        ((normalized_breakdown.transportation_score / scorer.weights.transportation_weight) * 100.0).min(100.0).max(0.0)
                                    } else { 0.0 };
                                    
                                    normalized_breakdown.trip_pace_score = if scorer.weights.trip_pace_weight > 0.0 {
                                        ((normalized_breakdown.trip_pace_score / scorer.weights.trip_pace_weight) * 100.0).min(100.0).max(0.0)
                                    } else { 0.0 };

                                    populated.set_score_breakdown(normalized_breakdown);
                                }

                                // Mark generated itineraries
                                if itinerary.tag.as_deref() == Some("generated") {
                                    println!(
                                        "Marking itinerary {} as generated",
                                        populated.trip_name()
                                    );
                                }

                                Ok(populated)
                            }
                            Err(err) => {
                                eprintln!("Failed to populate itinerary: {:?}", err);
                                Err(err)
                            }
                        }
                    }
                })
                .collect();

            let populate_results = futures::future::join_all(populate_futures).await;

            let mut populated_itineraries = Vec::new();
            for result in populate_results {
                if let Ok(populated) = result {
                    populated_itineraries.push(populated);
                }
            }

            // Copy match scores from populated itineraries to processed itineraries
            let mut processed_itineraries = processed_itineraries;
            for processed in &mut processed_itineraries {
                if let Some(populated) = populated_itineraries.iter().find(|p| p.id() == processed.id) {
                    processed.match_score = populated.match_score;
                    processed.score_breakdown = populated.score_breakdown.clone();
                }
            }

            // Return the base itineraries (not populated) to match expected frontend format
            // The populated version includes full activity objects which are too verbose for the frontend
            HttpResponse::Ok().json(processed_itineraries)
        }
        Err(err) => {
            eprintln!("Failed to search/generate itineraries: {:?}", err);
            HttpResponse::InternalServerError().body("Failed to search or generate itineraries")
        }
    }
}
