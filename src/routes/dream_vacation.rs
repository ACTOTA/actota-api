use crate::{middleware::auth::Claims, models::itinerary::ItinerarySubmission};
use actix_web::{web, HttpResponse, Responder};
use mongodb::{bson::oid::ObjectId, Client};
use std::sync::Arc;

/*
    /api/itineraries/find
*/
pub async fn find(
    claims: web::ReqData<Claims>,
    data: web::Data<Arc<Client>>,
    input: web::Json<ItinerarySubmission>,
) -> impl Responder {
    println!("Input: {:?}", input);
    println!("Claims: {:?}", claims);
    let client = data.into_inner();
    let collection: mongodb::Collection<ItinerarySubmission> =
        client.database("Travelers").collection("Submission");

    let mut submission = input.into_inner();
    submission.user_id =
        Some(ObjectId::parse_str(&claims.user_id).expect("Unable to parse user_id."));

    match collection.insert_one(&submission).await {
        Ok(_) => HttpResponse::Ok().json(submission), // Return the created submission
        Err(err) => {
            eprintln!("Failed to insert document: {:?}", err);
            HttpResponse::InternalServerError().body("Failed to submit itinerary.")
        }
    }
}
