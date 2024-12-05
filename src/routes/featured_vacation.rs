use crate::{models::itinerary::FeaturedVacation, services::itinerary_service::get_images};
use actix_web::{web, HttpResponse, Responder};
use bson::doc;
use chrono::Utc;
use futures::TryStreamExt;
use mongodb::Client;
use std::sync::Arc;

/*
    /api/itineraries/featured/
*/
pub async fn get_all(data: web::Data<Arc<Client>>) -> impl Responder {
    let client = data.into_inner();
    let collection: mongodb::Collection<FeaturedVacation> =
        client.database("Itineraries").collection("Featured");

    match collection.find(doc! {}).await {
        Ok(cursor) => match cursor.try_collect::<Vec<FeaturedVacation>>().await {
            Ok(activities) => {
                let processed_activities = get_images(activities).await;
                HttpResponse::Ok().json(processed_activities)
            }
            Err(err) => {
                eprintln!("Failed to collect documents: {:?}", err);
                return HttpResponse::InternalServerError().body("Failed to collect activities.");
            }
        },
        Err(err) => {
            eprintln!("Failed to find documents: {:?}", err);
            return HttpResponse::InternalServerError().body("Failed to find activities.");
        }
    }
}

pub async fn add(
    data: web::Data<Arc<Client>>,
    input: web::Json<FeaturedVacation>,
) -> impl Responder {
    let client = data.into_inner();
    let collection: mongodb::Collection<FeaturedVacation> =
        client.database("Itineraries").collection("Featured");

    println!("Input: {:?}", input);

    let curr_time = Utc::now();
    let mut submission = input.into_inner();
    submission.updated_at = Some(curr_time);
    submission.created_at = Some(curr_time);

    match collection.insert_one(&submission).await {
        Ok(_) => HttpResponse::Ok().json(submission), // Return the created submission
        Err(err) => {
            eprintln!("Failed to insert document: {:?}", err);
            HttpResponse::InternalServerError().body("Failed to submit itinerary.")
        }
    }
}
