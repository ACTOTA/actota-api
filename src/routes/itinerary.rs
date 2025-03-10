use crate::{models::itinerary::FeaturedVacation, services::itinerary_service::get_images};
use actix_web::{web, HttpResponse, Responder};
use bson::doc;
use futures::TryStreamExt;
use mongodb::{bson::oid::ObjectId, Client};
use std::sync::Arc;

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

    match collection.find_one(filter).await {
        Ok(Some(doc)) => {
            let processed_doc = get_images(vec![doc.clone()]).await;
            HttpResponse::Ok().json(processed_doc[0].clone())
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
pub async fn get_all(data: web::Data<Arc<Client>>) -> impl Responder {
    let client = data.into_inner();
    let collection = client.database("Itineraries").collection::<FeaturedVacation>("Featured");
    
    // Get all itineraries
    let sort_options = doc! { "createdAt": -1 };
    let cursor = collection.find(doc! {})
        .sort(sort_options)
        .limit(100)
        .await;
    
    match cursor {
        Ok(cursor) => match cursor.try_collect::<Vec<FeaturedVacation>>().await {
            Ok(itineraries) => {
                if itineraries.is_empty() {
                    return HttpResponse::Ok().json(Vec::<FeaturedVacation>::new());
                }
                
                // Process images for all itineraries
                let processed_itineraries = get_images(itineraries).await;
                HttpResponse::Ok().json(processed_itineraries)
            },
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
