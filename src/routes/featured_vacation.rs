use crate::{
    models::itinerary::base::FeaturedVacation, 
    services::{
        itinerary_service::get_images,
        image_service::{ImageService, ImageData}
    }
};
use actix_multipart::form::json;
use actix_web::{web, HttpResponse, Responder};
use bson::{doc, oid::ObjectId, DateTime};
use futures::TryStreamExt;
use mongodb::Client;
use serde_json::json;
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

            // Populate each vacation to include person_cost
            let mut populated_vacations = Vec::new();

            for vacation in processed_vacations.iter() {
                match vacation.clone().populate(&client).await {
                    Ok(populated) => populated_vacations.push(populated),
                    Err(err) => {
                        eprintln!("Failed to populate vacation: {:?}", err);
                        // Add the original vacation without population
                        // Just to maintain the count
                    }
                }
            }

            if !populated_vacations.is_empty() {
                HttpResponse::Ok().json(populated_vacations)
            } else {
                // Fallback to original vacations if population failed
                HttpResponse::Ok().json(processed_vacations)
            }
        }
        Err(err) => {
            eprintln!("Failed to find documents: {:?}", err);
            return HttpResponse::InternalServerError().body("Failed to find itineraries.");
        }
    }
}

pub async fn add(
    data: web::Data<Arc<Client>>,
    req_body: web::Json<serde_json::Value>,
) -> impl Responder {
    let client = data.into_inner();
    let collection: mongodb::Collection<FeaturedVacation> =
        client.database("Itineraries").collection("Featured");

    let mut body = req_body.into_inner();
    
    let curr_time = DateTime::now();
    
    // Extract images before trying to deserialize FeaturedVacation
    let images_data = body.get("images").cloned();
    // Remove images from the body so FeaturedVacation can deserialize properly
    body.as_object_mut().unwrap().remove("images");
    
    let mut submission: FeaturedVacation = match serde_json::from_value(body.clone()) {
        Ok(sub) => sub,
        Err(err) => {
            eprintln!("Failed to parse request body: {:?}", err);
            return HttpResponse::BadRequest().json(json!({
                "success": false,
                "message": "Invalid request format"
            }));
        }
    };

    submission.updated_at = Some(curr_time);
    submission.created_at = Some(curr_time);

    let temp_insert_result = match collection.insert_one(&submission).await {
        Ok(result) => result,
        Err(err) => {
            eprintln!("Failed to insert document: {:?}", err);
            return HttpResponse::InternalServerError().json(json!({
                "success": false,
                "message": "Failed to submit itinerary."
            }));
        }
    };

    let object_id = temp_insert_result.inserted_id.as_object_id().unwrap();
    let itinerary_id = object_id.to_hex();

    if let Some(images_value) = images_data {
        if let Some(images_array) = images_value.as_array() {
            if !images_array.is_empty() {
                let images: Vec<ImageData> = match serde_json::from_value(images_value.clone()) {
                    Ok(imgs) => imgs,
                    Err(err) => {
                        eprintln!("Failed to parse images: {:?}", err);
                        submission.id = Some(object_id);
                        return HttpResponse::Ok().json(json!({
                            "success": true,
                            "data": submission,
                            "itineraryId": itinerary_id,
                            "warning": "Itinerary saved but images could not be processed"
                        }));
                    }
                };

                match ImageService::new().await {
                    Ok(image_service) => {
                        let upload_results = image_service.upload_images(images, &itinerary_id).await;
                        
                        let mut successful_urls = Vec::new();
                        let mut failed_count = 0;

                        for result in upload_results {
                            match result {
                                Ok(url) => successful_urls.push(url),
                                Err(_) => failed_count += 1,
                            }
                        }

                        if !successful_urls.is_empty() {
                            let update_doc = doc! {
                                "$set": {
                                    "images": &successful_urls,
                                    "updated_at": DateTime::now()
                                }
                            };

                            if let Err(err) = collection.update_one(doc! { "_id": object_id }, update_doc).await {
                                eprintln!("Failed to update itinerary with image URLs: {:?}", err);
                            } else {
                                submission.images = Some(successful_urls);
                            }
                        }

                        if failed_count > 0 {
                            eprintln!("Failed to upload {} images", failed_count);
                        }
                    },
                    Err(err) => {
                        eprintln!("Failed to initialize image service: {:?}", err);
                    }
                }
            }
        }
    }

    submission.id = Some(object_id);

    HttpResponse::Ok().json(json!({
        "success": true,
        "data": submission,
        "itineraryId": itinerary_id
    }))
}

pub async fn update_itinerary_images(
    data: web::Data<Arc<Client>>,
    path: web::Path<String>,
    req_body: web::Json<serde_json::Value>,
) -> impl Responder {
    let itinerary_id = path.into_inner();
    let client = data.into_inner();

    let object_id = match ObjectId::parse_str(&itinerary_id) {
        Ok(id) => id,
        Err(_) => {
            return HttpResponse::BadRequest().json(json!({
                "success": false,
                "message": "Invalid itinerary ID format"
            }));
        }
    };

    let collection: mongodb::Collection<FeaturedVacation> =
        client.database("Itineraries").collection("Featured");

    let images = match req_body.get("images") {
        Some(img_array) => match img_array.as_array() {
            Some(arr) => {
                if arr.is_empty() {
                    return HttpResponse::BadRequest().json(json!({
                        "success": false,
                        "message": "Images array cannot be empty"
                    }));
                }

                for (index, img) in arr.iter().enumerate() {
                    if !img.is_string() {
                        return HttpResponse::BadRequest().json(json!({
                            "success": false,
                            "message": format!("Image at index {} must be a string", index)
                        }));
                    }

                    let img_str = img.as_str().unwrap();
                    if img_str.trim().is_empty() {
                        return HttpResponse::BadRequest().json(json!({
                            "success": false,
                            "message": format!("Image at index {} cannot be empty", index)
                        }));
                    }
                }

                arr.clone()
            }
            None => {
                return HttpResponse::BadRequest().json(json!({
                    "success": false,
                    "message": "Images must be an array"
                }));
            }
        },
        None => {
            return HttpResponse::BadRequest().json(json!({
                "success": false,
                "message": "Images array is required"
            }));
        }
    };

    // Convert images to BSON before using in doc! macro
    let images_bson = bson::to_bson(&images).unwrap_or(bson::Bson::Array(vec![]));
    let update_doc = doc! {
        "$set": {
            "images": images_bson,
            "updated_at": DateTime::now()
        }
    };

    match collection
        .update_one(doc! { "_id": object_id }, update_doc)
        .await
    {
        Ok(update_result) => {
            if update_result.matched_count == 0 {
                HttpResponse::NotFound().json(json!({
                    "success": false,
                    "message": "Itinerary not found"
                }))
            } else {
                HttpResponse::Ok().json(json!({
                    "success": true,
                    "message": "Images updated successfully",
                    "modified_count": update_result.modified_count
                }))
            }
        }
        Err(err) => {
            eprintln!("Failed to update itinerary images: {:?}", err);
            HttpResponse::InternalServerError().json(json!({
                "success": false,
                "message": "Failed to update itinerary images"
            }))
        }
    }
}
