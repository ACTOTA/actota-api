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
            let mut count = 0;

            loop {
                let result = cursor.try_next().await;
                match result {
                    Ok(Some(vacation)) => {
                        count += 1;

                        valid_vacations.push(vacation);
                    }
                    Ok(None) => {
                        println!("Reached end of cursor after {} vacations", count);
                        break;
                    }
                    Err(err) => {
                        println!("Error reading vacation #{}: {:?}", count + 1, err);
                        // Continue to next record rather than breaking
                    }
                }
            }

            // Store the original count to compare later
            let original_count = valid_vacations.len();

            let processed_vacations = get_images(valid_vacations.clone()).await;

            // Check if we lost any vacations during processing
            if processed_vacations.len() < original_count {
                println!(
                    "Warning: Lost {} vacations during image processing",
                    original_count - processed_vacations.len()
                );

                // If we have missing vacations, use the original ones but with empty images
                if processed_vacations.len() < original_count {
                    let mut result = valid_vacations;
                    // Set empty images array for all vacations
                    for vacation in &mut result {
                        if vacation.images.is_none() {
                            vacation.images = Some(vec![]);
                        }
                    }
                    return HttpResponse::Ok().json(result);
                }
            }

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
