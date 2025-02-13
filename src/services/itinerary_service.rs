use crate::models::itinerary::FeaturedVacation;
use cloud_storage::Client as StorageClient;
use cloud_storage::ListRequest;
use futures::future::join_all;
use futures::StreamExt;
use std::env;
use tokio::pin;

pub async fn get_images(mut vacations: Vec<FeaturedVacation>) -> Vec<FeaturedVacation> {
    // I have absolutely no idea how I got this to work
    // This is fetching the images from the Google Cloud Storage bucket
    let base_url = env::var("CLOUD_STORAGE_URL").unwrap_or("".to_string());
    let bucket_name = env::var("ITINERARY_BUCKET").unwrap_or("".to_string());

    let storage_client = StorageClient::default();

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
    return join_all(futures)
        .await
        .into_iter()
        .filter_map(|r: Result<FeaturedVacation, cloud_storage::Error>| r.ok())
        .collect::<Vec<FeaturedVacation>>();
}
