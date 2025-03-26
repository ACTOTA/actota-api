use crate::{
    middleware::auth::Claims,
    models::{bookings::Booking, itinerary::FeaturedVacation},
    services::itinerary_service::get_images,
};
use actix_web::{web, HttpResponse, Responder};
use bson::{doc, oid::ObjectId};
use futures::TryStreamExt;
use mongodb::Client;
use std::sync::Arc;

pub async fn add_booking(
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

    let collection: mongodb::Collection<Booking> =
        client.database("Account").collection("bookings");

    let filter = doc! {
        "user_id": ObjectId::parse_str(&claims.user_id).unwrap(),
        "itinerary_id": ObjectId::parse_str(&itinerary_id).unwrap(),
    };

    match collection.find_one(filter).await {
        Ok(Some(_)) => {
            // Already a booking
            return HttpResponse::Conflict().body("Booking already exists");
        }
        Ok(None) => {
            // Not a booking yet
            // Add the booking
            let time = chrono::Utc::now();

            let booking = Booking {
                id: None,
                user_id: ObjectId::parse_str(&claims.user_id).unwrap(),
                itinerary_id: ObjectId::parse_str(&itinerary_id).unwrap(),
                status: "ongoing".to_string(),
                created_at: Some(time),
                updated_at: Some(time),
            };

            match collection.insert_one(&booking).await {
                Ok(_) => {
                    return HttpResponse::Ok().body("Booking created for user");
                }
                Err(_) => {
                    return HttpResponse::InternalServerError().body("Failed to add booking");
                }
            }
        }
        Err(_) => {
            return HttpResponse::InternalServerError().body("Failed to check for bookings");
        }
    }
}

pub async fn remove_booking(
    data: web::Data<Arc<Client>>,
    path: web::Path<(String, String)>,
    claims: Claims,
) -> impl Responder {
    let client = data.into_inner();
    let collection: mongodb::Collection<Booking> =
        client.database("Account").collection("bookings");

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
            return HttpResponse::Ok().body("Removed Booking");
        }
        Err(_) => {
            return HttpResponse::InternalServerError().body("Failed to remove booking");
        }
    }
}
