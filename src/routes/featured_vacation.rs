use crate::models::itinerary::FeaturedVacation;
use actix_web::{web, HttpResponse, Responder};
use bson::doc;
use chrono::Utc;
use cloud_storage::Client as StorageClient;
use cloud_storage::ListRequest;
use futures::future::join_all;
use futures::StreamExt;
use futures::TryStreamExt;
use mongodb::Client;
use std::{env, sync::Arc};
use tokio::pin;

/*
    /api/itineraries/featured/
*/
pub async fn get_all(data: web::Data<Arc<Client>>) -> impl Responder {
    let client = data.into_inner();
    let collection: mongodb::Collection<FeaturedVacation> =
        client.database("Itineraries").collection("Featured");

    let storage_client = StorageClient::default();

    match collection.find(doc! {}).await {
        Ok(cursor) => match cursor.try_collect::<Vec<FeaturedVacation>>().await {
            Ok(mut activities) => {
                // I have absolutely no idea how I got this to work
                // This is fetching the images from the Google Cloud Storage bucket
                let base_url = env::var("CLOUD_STORAGE_URL").unwrap_or("".to_string());
                let bucket_name = env::var("ITINERARY_BUCKET").unwrap_or("".to_string());

                // Create futures for each activity
                let futures: Vec<_> = activities
                    .iter_mut()
                    .map(|activity| async {
                        let list_request = ListRequest {
                            prefix: Some(
                                activity
                                    .id
                                    .unwrap_or(bson::oid::ObjectId::new())
                                    .to_string(),
                            ),
                            ..Default::default()
                        };

                        let mut files = Vec::new();

                        let stream = storage_client
                            .object()
                            .list(&bucket_name, list_request)
                            .await?;
                        pin!(stream);

                        while let Some(object_result) = stream.next().await {
                            if let Ok(object) = object_result {
                                for item in object.items {
                                    if item.name.ends_with(".jpg") || item.name.ends_with(".png") {
                                        let url =
                                            format!("{}/{}/{}", base_url, bucket_name, item.name);
                                        files.push(url);
                                    }
                                }
                            }
                        }

                        activity.images = Some(files);
                        Ok(activity.clone())
                    })
                    .collect();

                // Execute all futures concurrently
                let processed_activities = join_all(futures)
                    .await
                    .into_iter()
                    .filter_map(|r: Result<FeaturedVacation, cloud_storage::Error>| r.ok())
                    .collect::<Vec<FeaturedVacation>>();

                println!("\n\nProcessed activities: {:?}\n\n", processed_activities);
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
