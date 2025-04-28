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
        Ok(mut cursor) => {
            let mut valid_vacations = Vec::new();

            while let Ok(Some(vacation)) = cursor.try_next().await {
                valid_vacations.push(vacation);
            }

            let processed_vacations = get_images(valid_vacations).await;
            HttpResponse::Ok().json(processed_vacations)
        }
        Err(err) => {
            eprintln!("Failed to find documents: {:?}", err);
            return HttpResponse::InternalServerError().body("Failed to find itineraries.");
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
