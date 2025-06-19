use base64::{engine::general_purpose, Engine as _};
use google_cloud_storage::client::{Client, ClientConfig};
use google_cloud_storage::http::objects::upload::{Media, UploadObjectRequest, UploadType};
use serde::{Deserialize, Serialize};
use std::env;
use uuid::Uuid;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ImageData {
    pub data: String,
    #[serde(rename = "fileName")]
    pub file_name: String,
    #[serde(rename = "fileType")]
    pub file_type: String,
    #[serde(rename = "fileSize")]
    pub file_size: u64,
}

#[derive(Debug)]
pub enum ImageUploadError {
    Base64DecodeError(String),
    GcsError(String),
    InvalidImageFormat(String),
    EnvironmentError(String),
}

impl std::fmt::Display for ImageUploadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ImageUploadError::Base64DecodeError(err) => write!(f, "Base64 decode error: {}", err),
            ImageUploadError::GcsError(err) => write!(f, "GCS upload error: {}", err),
            ImageUploadError::InvalidImageFormat(err) => write!(f, "Invalid image format: {}", err),
            ImageUploadError::EnvironmentError(err) => write!(f, "Environment error: {}", err),
        }
    }
}

impl std::error::Error for ImageUploadError {}

pub struct ImageService {
    client: Client,
    bucket_name: String,
}

impl ImageService {
    pub async fn new() -> Result<Self, ImageUploadError> {
        let bucket_name = env::var("ITINERARY_BUCKET")
            .map_err(|_| ImageUploadError::EnvironmentError("ITINERARY_BUCKET not set".to_string()))?;

        let config = ClientConfig::default().with_auth().await
            .map_err(|e| ImageUploadError::GcsError(format!("Failed to create GCS client: {}", e)))?;
        
        let client = Client::new(config);

        Ok(Self {
            client,
            bucket_name,
        })
    }

    pub async fn upload_images(&self, images: Vec<ImageData>, itinerary_id: &str) -> Vec<Result<String, ImageUploadError>> {
        let mut results = Vec::new();
        
        for image in images {
            match self.upload_single_image(image, itinerary_id).await {
                Ok(url) => results.push(Ok(url)),
                Err(e) => {
                    eprintln!("Failed to upload image: {}", e);
                    results.push(Err(e));
                }
            }
        }
        
        results
    }

    async fn upload_single_image(&self, image: ImageData, itinerary_id: &str) -> Result<String, ImageUploadError> {
        let base64_data = if image.data.starts_with("data:") {
            image.data.split(',').nth(1)
                .ok_or_else(|| ImageUploadError::InvalidImageFormat("Invalid base64 data format".to_string()))?
        } else {
            &image.data
        };

        let image_bytes = general_purpose::STANDARD.decode(base64_data)
            .map_err(|e| ImageUploadError::Base64DecodeError(e.to_string()))?;

        let file_extension = self.get_file_extension(&image.file_type)?;
        let timestamp = chrono::Utc::now().timestamp();
        let random_id = Uuid::new_v4();
        let object_name = format!("{}/{}-{}.{}", itinerary_id, timestamp, random_id, file_extension);

        let upload_type = UploadType::Simple(Media::new(object_name.clone()));
        let upload_request = UploadObjectRequest {
            bucket: self.bucket_name.clone(),
            ..Default::default()
        };

        self.client.upload_object(&upload_request, image_bytes, &upload_type).await
            .map_err(|e| ImageUploadError::GcsError(format!("Failed to upload to GCS: {}", e)))?;

        let public_url = format!(
            "https://storage.googleapis.com/{}/{}",
            self.bucket_name,
            object_name
        );

        Ok(public_url)
    }

    fn get_file_extension(&self, file_type: &str) -> Result<String, ImageUploadError> {
        match file_type {
            "image/jpeg" => Ok("jpg".to_string()),
            "image/jpg" => Ok("jpg".to_string()),
            "image/png" => Ok("png".to_string()),
            "image/gif" => Ok("gif".to_string()),
            "image/webp" => Ok("webp".to_string()),
            _ => Err(ImageUploadError::InvalidImageFormat(format!("Unsupported file type: {}", file_type))),
        }
    }
}