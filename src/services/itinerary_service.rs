use crate::models::itinerary::FeaturedVacation;
use cloud_storage::Client as StorageClient;
use cloud_storage::ListRequest;
use futures::future::join_all;
use futures::StreamExt;
use std::env;
use std::path::Path;
use tokio::pin;

pub async fn get_images(mut vacations: Vec<FeaturedVacation>) -> Vec<FeaturedVacation> {
    // I have absolutely no idea how I got this to work
    // This is fetching the images from the Google Cloud Storage bucket
    let base_url = env::var("CLOUD_STORAGE_URL").unwrap_or("".to_string());
    let bucket_name = env::var("ITINERARY_BUCKET").unwrap_or("".to_string());
    
    // Check environment to determine authentication approach
    let env_type = env::var("RUST_ENV").unwrap_or_else(|_| "development".to_string());
    let storage_client = if env_type == "production" {
        // In production (Cloud Run), use the default service account
        StorageClient::default()
    } else {
        // In development, check for local credentials file
        let credentials_path = Path::new("credentials/service-account.json");
        if credentials_path.exists() {
            // Set environment variable if not already set
            if env::var("GOOGLE_APPLICATION_CREDENTIALS").is_err() {
                env::set_var("GOOGLE_APPLICATION_CREDENTIALS", credentials_path.to_str().unwrap());
            }
            StorageClient::default()
        } else {
            eprintln!("Warning: Local credentials file not found at credentials/service-account.json");
            StorageClient::default()
        }
    };

    // Create futures for each vacation
    let futures: Vec<_> = vacations
        .iter_mut()
        .map(|vacation| async {
            let list_request = ListRequest {
                prefix: Some(
                    vacation
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
                            let url = format!("{}/{}/{}", base_url, bucket_name, item.name);
                            files.push(url);
                        }
                    }
                }
            }

            vacation.images = Some(files);
            Ok(vacation.clone())
        })
        .collect();

    // Execute all futures concurrently
    let results = join_all(futures).await;
    
    // Improved error handling - log any errors before filtering them out
    let processed_results: Vec<FeaturedVacation> = results
        .into_iter()
        .filter_map(|r: Result<FeaturedVacation, cloud_storage::Error>| {
            match r {
                Ok(vacation) => Some(vacation),
                Err(err) => {
                    eprintln!("Error fetching images from Cloud Storage: {}", err);
                    None
                }
            }
        })
        .collect();
    
    if processed_results.is_empty() && !vacations.is_empty() {
        eprintln!("Warning: All itineraries failed to load images from Cloud Storage");
    }
    
    return processed_results;
}
