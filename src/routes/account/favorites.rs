use crate::{
    middleware::auth::Claims,
    models::{account::Favorite, itinerary::FeaturedVacation},
};
use actix_web::{web, HttpMessage, HttpRequest, HttpResponse, Responder};
use bson::{doc, oid::ObjectId};
use mongodb::Client;
use std::sync::Arc;

pub async fn add_favorite(
    req: HttpRequest,
    data: web::Data<Arc<Client>>,
    path: web::Path<(String, String)>,
) -> impl Responder {
    // Get the user_id from bearer token
    let default_claims = Claims {
        exp: 0,
        sub: "0".to_string(),
        iat: 0,
        user_id: "0".to_string(),
    };
    let claims = req
        .extensions_mut()
        .get::<Claims>()
        .unwrap_or(&default_claims)
        .clone(); // Use default_claims

    // Get the itinerary_id from the path
    let (_, itinerary_id) = path.into_inner();

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
        "user_id": claims.user_id.clone(),
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
    req: HttpRequest,
    data: web::Data<Arc<Client>>,
    path: web::Path<(String, String)>,
) -> impl Responder {
    let default_claims = Claims {
        exp: 0,
        sub: "0".to_string(),
        iat: 0,
        user_id: "0".to_string(),
    };
    let claims = req
        .extensions_mut()
        .get::<Claims>()
        .unwrap_or(&default_claims)
        .clone(); // Use default_claims

    let client = data.into_inner();
    let collection: mongodb::Collection<Favorite> =
        client.database("Account").collection("Favorites");

    let (_, itinerary_id) = path.into_inner();

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

pub async fn get_favorites() -> impl Responder {
    // connect to the database
    // get the favorites collection
    // find all the favorites
    // return the favorites

    HttpResponse::Ok().body("Favorites")
}
