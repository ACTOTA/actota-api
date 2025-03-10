use crate::models::itinerary::FeaturedVacation;
use cloud_storage::Client as StorageClient;
use cloud_storage::ListRequest;
use futures::future::join_all;
use futures::StreamExt;
use std::env;
use std::path::Path;
use tokio::pin;

pub async fn get_images(mut vacations: Vec<FeaturedVacation>) -> Vec<FeaturedVacation> {
    let base_url = env::var("CLOUD_STORAGE_URL").unwrap_or_else(|_| {
        println!("Warning: CLOUD_STORAGE_URL not set, defaulting to storage.googleapis.com");
        "https://storage.googleapis.com".to_string()
    });

    let bucket_name = env::var("ITINERARY_BUCKET").unwrap_or_else(|_| {
        println!("Warning: ITINERARY_BUCKET not set, defaulting to actota-itineraries");
        "actota-itineraries".to_string()
    });

    println!("Retrieving images from: {}/{}", base_url, bucket_name);

    let storage_client = StorageClient::default();

    // Create futures for each vacation
    let futures: Vec<_> = vacations
        .iter_mut()
        .map(|vacation| async {
            let vacation_id = vacation
                .id
                .unwrap_or(bson::oid::ObjectId::new())
                .to_string();

            println!("Looking for images for vacation ID: {}", vacation_id);

            let list_request = ListRequest {
                prefix: Some(vacation_id.clone()),
                ..Default::default()
            };

            let mut files = Vec::new();

            match storage_client
                .object()
                .list(&bucket_name, list_request)
                .await
            {
                Ok(stream) => {
                    pin!(stream);

                    while let Some(object_result) = stream.next().await {
                        match object_result {
                            Ok(object) => {
                                for item in object.items {
                                    if item.name.ends_with(".jpg") || item.name.ends_with(".png") {
                                        let url =
                                            format!("{}/{}/{}", base_url, bucket_name, item.name);
                                        println!("Found image: {}", url);
                                        files.push(url);
                                    }
                                }
                            }
                            Err(e) => {
                                println!("Error processing object in stream: {:?}", e);
                            }
                        }
                    }

                    vacation.images = Some(files);
                    Ok(vacation.clone())
                }
                Err(e) => {
                    println!(
                        "Error listing objects for vacation {}: {:?}",
                        vacation_id, e
                    );
                    // Return the vacation without images rather than failing completely
                    vacation.images = Some(vec![]);
                    Ok(vacation.clone())
                }
            }
        })
        .collect();

    // Execute all futures concurrently
    let results = join_all(futures).await;

    // Log any errors but still return vacations
    let processed_vacations = results
        .into_iter()
        .filter_map(|r: Result<FeaturedVacation, cloud_storage::Error>| {
            if let Err(e) = &r {
                println!("Error processing vacation images: {:?}", e);
            }
            r.ok()
        })
        .collect::<Vec<FeaturedVacation>>();

    println!(
        "Processed {} vacations with images",
        processed_vacations.len()
    );
    processed_vacations
}
