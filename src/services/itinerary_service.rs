use crate::models::itinerary::FeaturedVacation;
use cloud_storage::Client as StorageClient;
use cloud_storage::ListRequest;
use futures::future::join_all;
use futures::StreamExt;
use std::env;
use std::path::Path;
use tokio::pin;

async fn create_configured_storage_client() -> StorageClient {
    // Diagnostic logging
    println!("Cloud Storage authentication setup:");
    println!(
        "  GOOGLE_APPLICATION_CREDENTIALS present: {}",
        std::env::var("GOOGLE_APPLICATION_CREDENTIALS").is_ok()
    );
    println!(
        "  SERVICE_ACCOUNT_JSON present: {}",
        std::env::var("SERVICE_ACCOUNT_JSON").is_ok()
    );

    // Check if we're running in Cloud Run (where we should use ADC)
    let is_cloud_run = std::env::var("K_SERVICE").is_ok();
    
    if is_cloud_run {
        println!("Detected Cloud Run environment, using Application Default Credentials");
        // For cloud_storage to use ADC, we need to set an empty SERVICE_ACCOUNT_JSON
        std::env::set_var("SERVICE_ACCOUNT_JSON", "{}");
    } else {
        // Local development
        if std::env::var("SERVICE_ACCOUNT_JSON").is_err() {
            // Try to use GOOGLE_APPLICATION_CREDENTIALS path to set SERVICE_ACCOUNT_JSON
            if let Ok(creds_path) = std::env::var("GOOGLE_APPLICATION_CREDENTIALS") {
                match std::fs::read_to_string(&creds_path) {
                    Ok(content) => {
                        println!("Setting SERVICE_ACCOUNT_JSON from GOOGLE_APPLICATION_CREDENTIALS file");
                        std::env::set_var("SERVICE_ACCOUNT_JSON", content);
                    }
                    Err(e) => {
                        println!("Warning: Could not read credentials file: {}", e);
                        std::env::set_var("SERVICE_ACCOUNT_JSON", "{}");
                    }
                }
            } else {
                println!("Setting SERVICE_ACCOUNT_JSON to empty object to enable ADC");
                std::env::set_var("SERVICE_ACCOUNT_JSON", "{}");
            }
        }
    }

    // Create the client using configured settings
    StorageClient::default()
}

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

    let storage_client = create_configured_storage_client().await;

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
