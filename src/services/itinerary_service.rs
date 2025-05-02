use bson::datetime::Error;
use futures::future::join_all;
use google_cloud_storage::client::{Client, ClientConfig};
use google_cloud_storage::http::objects::list::ListObjectsRequest;
use std::env;

use crate::models::itinerary::base::FeaturedVacation;

// Create a storage client with automatic authentication
async fn create_storage_client() -> Client {
    // Diagnostic logging
    println!("Initializing Google Cloud Storage client");
    let is_cloud_run = env::var("K_SERVICE").is_ok();

    if is_cloud_run {
        println!("Detected Cloud Run environment, using Application Default Credentials");
    } else {
        println!("Using local credentials (GOOGLE_APPLICATION_CREDENTIALS or ADC)");
    }

    // The ClientConfig::default() automatically uses:
    // 1. GOOGLE_APPLICATION_CREDENTIALS environment variable if set
    // 2. Application Default Credentials (ADC) otherwise

    let config = ClientConfig::default()
        .with_auth()
        .await
        .expect("Unable to setup Cloud Storage config");
    Client::new(config)
}

pub async fn get_images(mut vacations: Vec<FeaturedVacation>) -> Vec<FeaturedVacation> {
    let base_url = "https://storage.googleapis.com";
    let bucket_name = env::var("ITINERARY_BUCKET").unwrap_or_else(|_| {
        println!("Warning: ITINERARY_BUCKET not set, defaulting to actota-itineraries");
        "actota-itineraries".to_string()
    });

    println!("Retrieving images from: {}/{}", base_url, bucket_name);

    // Create GCS client
    let storage_client = create_storage_client().await;

    // Process each vacation to find its images
    let futures: Vec<_> = vacations
        .iter_mut()
        .map(|vacation| async {
            let vacation_id = vacation
                .id
                .unwrap_or(bson::oid::ObjectId::new())
                .to_string();

            println!("Looking for images for vacation ID: {}", vacation_id);

            // Create list request with bucket name and prefix
            let list_request = ListObjectsRequest {
                bucket: bucket_name.clone(),
                prefix: Some(vacation_id.clone()),
                ..Default::default()
            };

            let mut files = Vec::new();

            // List objects in the bucket with the prefix
            match storage_client.list_objects(&list_request).await {
                Ok(response) => {
                    let items = response.items.unwrap_or_default();
                    println!(
                        "Found {} potential image items for vacation ID: {}",
                        items.len(),
                        vacation_id
                    );

                    for item in items {
                        let name = &item.name;

                        if name.ends_with(".jpg")
                            || name.ends_with(".jpeg")
                            || name.ends_with(".png")
                        {
                            let url = format!("{}/{}/{}", base_url, bucket_name, name);
                            println!("Found image: {}", url);
                            files.push(url);
                        } else {
                            println!("Skipping non-image item: {}", name);
                        }
                    }

                    vacation.images = Some(files);
                    Result::<FeaturedVacation, Error>::Ok(vacation.clone())
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
    let results: Vec<Result<FeaturedVacation, _>> = join_all(futures).await;

    // Process results and handle any errors
    let mut processed_vacations = Vec::new();

    for (i, result) in results.into_iter().enumerate() {
        match result {
            Ok(vacation) => {
                processed_vacations.push(vacation);
            }
            Err(e) => {
                println!("Error processing vacation #{} images: {:?}", i + 1, e);
                // Don't filter out vacations with errors, but we can't recover them here
                // They will be handled in the calling function
            }
        }
    }

    println!(
        "Processed {} vacations with images (there may be errors)",
        processed_vacations.len()
    );
    processed_vacations
}
