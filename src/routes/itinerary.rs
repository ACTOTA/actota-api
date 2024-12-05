use crate::{models::itinerary::FeaturedVacation, services::itinerary_service::get_images};
use actix_web::{web, HttpResponse, Responder};
use bson::doc;
use mongodb::{bson::oid::ObjectId, Client};
use std::sync::Arc;

/*
    /api/itineraries/{id}
*/
pub async fn get_by_id(id: web::Path<String>, data: web::Data<Arc<Client>>) -> impl Responder {
    let client = data.into_inner();
    let collection: mongodb::Collection<FeaturedVacation> =
        client.database("Itineraries").collection("Featured");

    let object_id = match ObjectId::parse_str(&id.as_str()) {
        Ok(oid) => oid,
        Err(_) => return HttpResponse::BadRequest().body("Invalid ObjectId format"),
    };

    match collection.find_one(doc! { "_id": object_id }).await {
        Ok(Some(doc)) => {
            let processed_doc = get_images(vec![doc.clone()]).await;
            HttpResponse::Ok().json(processed_doc)
        }
        Ok(None) => HttpResponse::NotFound().body("Itinerary not found"),
        Err(err) => {
            eprintln!("Failed to retrieve itinerary: {:?}", err);
            HttpResponse::InternalServerError().body("Failed to retrieve itinerary")
        }
    }
}
