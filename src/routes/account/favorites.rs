use crate::{
    middleware::auth::Claims,
    models::{account::Favorite, itinerary::base::FeaturedVacation},
    services::itinerary_service::get_images,
};
use actix_web::{web, HttpResponse, Responder};
use bson::{doc, oid::ObjectId};
use futures::TryStreamExt;
use mongodb::Client;
use serde_json::json;
use std::sync::Arc;

pub async fn add_favorite(
    data: web::Data<Arc<Client>>,
    path: web::Path<(String, String)>,
    claims: Claims,
) -> impl Responder {
    // Get the itinerary_id from the path
    let (user_id, itinerary_id) = path.into_inner();
    if user_id != claims.user_id {
        return HttpResponse::Forbidden().json(json!({"error": "Forbidden"}));
    }

    let client = data.into_inner();

    // Verify itinerary exists in the database
    let itinerary: mongodb::Collection<FeaturedVacation> =
        client.database("Itineraries").collection("Featured");
    if itinerary
        .find_one(doc! { "_id": ObjectId::parse_str(&itinerary_id).unwrap() })
        .await
        .is_err()
    {
        return HttpResponse::NotFound().json(json!({"error": "Itinerary not found"}));
    }

    let collection: mongodb::Collection<Favorite> =
        client.database("Account").collection("Favorites");

    let filter = doc! {
        "user_id": ObjectId::parse_str(&claims.user_id).unwrap(),
        "itinerary_id": ObjectId::parse_str(&itinerary_id).unwrap(),
    };

    match collection.find_one(filter).await {
        Ok(Some(_)) => {
            // Already a favorite
            return HttpResponse::Conflict().json(json!({"error": "Favorite already exists"}));
        }
        Ok(None) => {
            // Not a favorite yet
            // Add the favorite
            let time = chrono::Utc::now();

            let favorite = Favorite {
                id: None,
                user_id: ObjectId::parse_str(&claims.user_id).unwrap(),
                itinerary_id: ObjectId::parse_str(&itinerary_id).unwrap(),
                created_at: Some(time),
                updated_at: Some(time),
            };

            match collection.insert_one(&favorite).await {
                Ok(_) => {
                    return HttpResponse::Ok().json(json!({"status": "success", "message": "Itinerary added to favorites"}));
                }
                Err(_) => {
                    return HttpResponse::InternalServerError().json(json!({"error": "Failed to add favorite"}));
                }
            }
        }
        Err(_) => {
            return HttpResponse::InternalServerError().json(json!({"error": "Failed to check for favorite"}));
        }
    }
}

pub async fn remove_favorite(
    data: web::Data<Arc<Client>>,
    path: web::Path<(String, String)>,
    claims: Claims,
) -> impl Responder {
    let client = data.into_inner();
    let collection: mongodb::Collection<Favorite> =
        client.database("Account").collection("Favorites");

    let (user_id, itinerary_id) = path.into_inner();
    if user_id != claims.user_id {
        return HttpResponse::Forbidden().json(json!({"error": "Forbidden"}));
    }

    let filter = doc! {
        "user_id": ObjectId::parse_str(&claims.user_id).unwrap(),
        "itinerary_id": ObjectId::parse_str(itinerary_id).unwrap(),
    };

    match collection.delete_one(filter).await {
        Ok(_) => {
            return HttpResponse::Ok().json(json!({"status": "success", "message": "Removed Favorite"}));
        }
        Err(_) => {
            return HttpResponse::InternalServerError().json(json!({"error": "Failed to remove favorite"}));
        }
    }
}

pub async fn get_favorites(
    data: web::Data<Arc<Client>>,
    claims: Claims,
    path: web::Path<(String,)>,
) -> impl Responder {
    if path.into_inner().0 != claims.user_id {
        return HttpResponse::Forbidden().json(json!({"error": "Forbidden"}));
    }

    let client = data.into_inner();
    let collection: mongodb::Collection<Favorite> =
        client.database("Account").collection("Favorites");

    let filter = doc! {
        "user_id": ObjectId::parse_str(&claims.user_id).unwrap(),
    };

    match collection.find(filter).await {
        Ok(cursor) => {
            let results = cursor.try_collect::<Vec<Favorite>>().await;
            match results {
                Ok(favorites) => {
                    // Extract itinerary IDs from favorites
                    let itinerary_ids: Vec<ObjectId> = favorites
                        .iter()
                        .map(|favorite| favorite.itinerary_id.clone())
                        .collect();

                    println!("\n\n Itinerary IDs: {:?}", itinerary_ids);

                    // Fetch itineraries from Itineraries.Featured collection
                    let itineraries_collection: mongodb::Collection<FeaturedVacation> =
                        client.database("Itineraries").collection("Featured");

                    let itinerary_filter = doc! {
                        "_id": { "$in": itinerary_ids }
                    };

                    match itineraries_collection.find(itinerary_filter).await {
                        Ok(itinerary_cursor) => {
                            match itinerary_cursor
                                .try_collect::<Vec<FeaturedVacation>>()
                                .await
                            {
                                Ok(mut featured_itineraries) => {
                                    // Fetch images for each itinerary
                                    featured_itineraries = get_images(featured_itineraries).await;
                                    
                                    // Populate each itinerary to include person_cost
                                    let mut populated_itineraries = Vec::new();
                                    
                                    for itinerary in featured_itineraries.clone() {
                                        match itinerary.populate(&client).await {
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
                                        HttpResponse::Ok().json(featured_itineraries)
                                    }
                                }
                                Err(err) => {
                                    eprintln!("Error fetching itineraries: {:?}", err);
                                    HttpResponse::InternalServerError()
                                        .json(json!({"error": "Failed to retrieve itineraries"}))
                                }
                            }
                        }
                        Err(err) => {
                            eprintln!("Error fetching itineraries: {:?}", err);
                            HttpResponse::InternalServerError()
                                .json(json!({"error": "Failed to retrieve itineraries"}))
                        }
                    }
                }
                Err(err) => {
                    eprintln!("Error retrieving favorites: {:?}", err);
                    HttpResponse::InternalServerError().json(json!({"error": "Failed to retrieve favorites"}))
                }
            }
        }
        Err(err) => {
            eprintln!("Error fetching favorites: {:?}", err);
            HttpResponse::InternalServerError().json(json!({"error": "Failed to fetch favorites"}))
        }
    }
}
