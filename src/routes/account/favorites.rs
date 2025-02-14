use crate::{
    middleware::auth::Claims,
    models::{account::Favorite, itinerary::FeaturedVacation},
};
use actix_web::{web, HttpMessage, HttpRequest, HttpResponse, Responder};
use bson::{doc, oid::ObjectId};
use futures::TryStreamExt;
use mongodb::Client;
use std::sync::Arc;

pub async fn add_favorite(
    data: web::Data<Arc<Client>>,
    path: web::Path<(String, String)>,
    claims: Claims,
) -> impl Responder {
    // Get the itinerary_id from the path
    let (user_id, itinerary_id) = path.into_inner();
    if user_id != claims.user_id {
        return HttpResponse::Forbidden().body("Forbidden");
    }

    let client = data.into_inner();

    // Verify itinerary exists in the database
    let itinerary: mongodb::Collection<FeaturedVacation> =
        client.database("Itinerary").collection("Featured");
    if itinerary
        .find_one(doc! { "_id": ObjectId::parse_str(&itinerary_id).unwrap() })
        .await
        .is_err()
    {
        return HttpResponse::NotFound().body("Itinerary not found");
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
            return HttpResponse::Conflict().body("Favorite already exists");
        }
        Ok(None) => {
            // Not a favorite yet
            // Add the favorite
            let time = chrono::Utc::now();

            let favorite = Favorite {
                _id: None,
                user_id: ObjectId::parse_str(&claims.user_id).unwrap(),
                itinerary_id: ObjectId::parse_str(&itinerary_id).unwrap(),
                created_at: Some(time),
                updated_at: Some(time),
            };

            match collection.insert_one(&favorite).await {
                Ok(_) => {
                    return HttpResponse::Ok().body("Itinerary added to favorites");
                }
                Err(_) => {
                    return HttpResponse::InternalServerError().body("Failed to add favorite");
                }
            }
        }
        Err(_) => {
            return HttpResponse::InternalServerError().body("Failed to check for favorite");
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
        return HttpResponse::Forbidden().body("Forbidden");
    }

    let filter = doc! {
        "user_id": ObjectId::parse_str(&claims.user_id).unwrap(),
        "itinerary_id": ObjectId::parse_str(itinerary_id).unwrap(),
    };

    match collection.delete_one(filter).await {
        Ok(_) => {
            return HttpResponse::Ok().body("Removed Favorite");
        }
        Err(_) => {
            return HttpResponse::InternalServerError().body("Failed to remove favorite");
        }
    }
}

pub async fn get_favorites(
    data: web::Data<Arc<Client>>,
    claims: Claims,
    path: web::Path<(String,)>,
) -> impl Responder {
    if path.into_inner().0 != claims.user_id {
        return HttpResponse::Forbidden().body("Forbidden");
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
                                Ok(featured_itineraries) => {
                                    HttpResponse::Ok().json(featured_itineraries)
                                }
                                Err(err) => {
                                    eprintln!("Error fetching itineraries: {:?}", err);
                                    HttpResponse::InternalServerError()
                                        .body("Failed to retrieve itineraries")
                                }
                            }
                        }
                        Err(err) => {
                            eprintln!("Error fetching itineraries: {:?}", err);
                            HttpResponse::InternalServerError()
                                .body("Failed to retrieve itineraries")
                        }
                    }
                }
                Err(err) => {
                    eprintln!("Error retrieving favorites: {:?}", err);
                    HttpResponse::InternalServerError().body("Failed to retrieve favorites")
                }
            }
        }
        Err(err) => {
            eprintln!("Error fetching favorites: {:?}", err);
            HttpResponse::InternalServerError().body("Failed to fetch favorites")
        }
    }
}
