use crate::models::itinerary::FeaturedVacation;
use actix_web::{web, HttpResponse, Responder};
use bson::doc;
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
            Ok(activities) => return HttpResponse::Ok().json(activities),
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
