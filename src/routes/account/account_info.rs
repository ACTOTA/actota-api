use actix_multipart::Multipart;
use actix_web::{web, HttpResponse, Responder};
use bson::{doc, oid::ObjectId};
use futures::{StreamExt, TryStreamExt};
use google_cloud_storage::client::{Client as GcsClient, ClientConfig};
use google_cloud_storage::http::objects::upload::{Media, UploadObjectRequest, UploadType};
use mongodb::Client;
use std::{env, str::FromStr, sync::Arc};

use crate::{
    middleware::auth::Claims,
    models::account::{PersonalInformation, User},
};

pub async fn update_personal_information(
    data: web::Data<Arc<Client>>,
    claims: Claims,
    path: web::Path<(String,)>,
    input: web::Json<PersonalInformation>,
) -> impl Responder {
    let user_id = path.into_inner().0;
    if user_id != claims.user_id {
        return HttpResponse::Forbidden().body("Forbidden");
    }

    let client = data.into_inner();

    let personal_info = input.into_inner();

    let collection: mongodb::Collection<User> = client.database("Account").collection("Users");

    let filter = doc! { "_id": ObjectId::from_str(&user_id).unwrap() };

    println!("Filter: {:?}", filter);

    let mut user = match collection.find_one(filter.clone()).await {
        Ok(user) => match user {
            Some(user) => user,
            None => return HttpResponse::NotFound().body("User not found"),
        },
        Err(_) => return HttpResponse::InternalServerError().body("Failed to find user"),
    };

    // Directly update top-level fields if provided in input
    if let Some(email) = personal_info.email {
        user.email = email;
    }
    if let Some(password) = personal_info.password {
        user.password =
            bcrypt::hash(&password, bcrypt::DEFAULT_COST).unwrap_or(user.password.clone());
    }
    if let Some(first_name) = personal_info.first_name {
        user.first_name = Some(first_name);
    }
    if let Some(last_name) = personal_info.last_name {
        user.last_name = Some(last_name);
    }
    if let Some(phone_number) = personal_info.phone_number {
        user.phone_number = Some(phone_number);
    }
    if let Some(birth_date) = personal_info.birth_date {
        user.birth_date = Some(birth_date);
    }

    user.updated_at = Some(chrono::Utc::now());
    // let mut info = input.into_inner();
    // info.updated_at = Some(chrono::Utc::now());

    let updates = bson::to_document(&user).unwrap();
    let update_doc = doc! { "$set": updates }; // $set is a MongoDB operator to update fields

    match collection.update_one(filter, update_doc).await {
        Ok(result) if result.modified_count > 0 => {
            return HttpResponse::Ok().body("User information updated");
        }
        Ok(_) => HttpResponse::NotModified().body("No changes applied"),
        Err(_) => {
            return HttpResponse::InternalServerError().body("Failed to update user information")
        }
    }
}

pub async fn get_personal_information(
    data: web::Data<Arc<Client>>,
    claims: Claims,
    path: web::Path<(String,)>,
) -> impl Responder {
    let user_id = path.into_inner().0;
    if user_id != claims.user_id {
        return HttpResponse::Forbidden().body("Forbidden");
    }
    let client = data.into_inner();
    let collection: mongodb::Collection<User> = client.database("Account").collection("Users");
    let filter = doc! { "_id": ObjectId::from_str(&user_id).unwrap() };
    match collection.find_one(filter).await {
        Ok(user) => match user {
            Some(user) => HttpResponse::Ok().json(user),
            None => HttpResponse::NotFound().body("User not found"),
        },
        Err(_) => HttpResponse::InternalServerError().body("Failed to find user"),
    }
}

pub async fn upload_profile_pic(
    data: web::Data<Arc<Client>>,
    claims: Claims,
    path: web::Path<(String,)>,
    mut payload: Multipart,
) -> impl Responder {
    let user_id = path.into_inner().0;

    // Check authorization - user can only update their own profile
    if user_id != claims.user_id {
        return HttpResponse::Forbidden().body("Forbidden");
    }

    let client = data.into_inner();
    let bucket_name = env::var("PROFILE_PIC_BUCKET").expect("PROFILE_PIC_BUCKET must be set");
    let cloud_storage_url = env::var("CLOUD_STORAGE_URL")
        .unwrap_or_else(|_| "https://storage.googleapis.com".to_string());

    // Initialize Google Cloud Storage client
    let client_config = match ClientConfig::default().with_auth().await {
        Ok(config) => config,
        Err(e) => {
            eprintln!("Failed to initialize GCS client config: {}", e);
            return HttpResponse::InternalServerError()
                .body("Failed to initialize cloud storage client");
        }
    };

    let gcs_client = GcsClient::new(client_config);

    // Process the multipart form data
    let mut file_bytes: Option<Vec<u8>> = None;
    let mut file_name: Option<String> = None;
    let mut content_type: Option<String> = None;

    while let Ok(Some(mut field)) = payload.try_next().await {
        let content_disposition = field.content_disposition();

        // Skip fields without content disposition
        if content_disposition.is_none() {
            continue;
        }

        let content_disp = content_disposition.unwrap();

        // Skip fields without a name
        let field_name = match content_disp.get_name() {
            Some(name) => name,
            None => continue,
        };

        if field_name == "file" {
            // Get the filename from the Content-Disposition header
            if let Some(filename) = content_disp.get_filename() {
                // Determine file extension from the filename
                let extension = std::path::Path::new(filename)
                    .extension()
                    .and_then(|ext| ext.to_str())
                    .unwrap_or("jpg")
                    .to_lowercase();

                // Set the filename based on the extension
                file_name = Some(format!("profile-pic.{}", extension));

                // Set content type based on extension
                content_type = Some(
                    match extension.as_str() {
                        "jpg" | "jpeg" => "image/jpeg",
                        "png" => "image/png",
                        "gif" => "image/gif",
                        _ => "application/octet-stream",
                    }
                    .to_string(),
                );
            }

            // Read the file data
            let mut data = Vec::new();
            while let Some(chunk) = field.next().await {
                match chunk {
                    Ok(bytes) => {
                        data.extend_from_slice(&bytes);
                    }
                    Err(e) => {
                        eprintln!("Error reading file chunk: {}", e);
                        return HttpResponse::InternalServerError()
                            .body("Failed to read uploaded file");
                    }
                }
            }

            if !data.is_empty() {
                file_bytes = Some(data);
            }
        }
    }

    // Check if we have a file to upload
    if file_bytes.is_none() || file_name.is_none() {
        return HttpResponse::BadRequest().body("No file uploaded or invalid file");
    }

    let file_data = file_bytes.unwrap();
    let file_name = file_name.unwrap();
    let content_type = content_type.unwrap_or_else(|| "application/octet-stream".to_string());

    // Create the folder path in the bucket (using user_id as folder)
    let storage_path = format!("{}/{}", user_id, file_name);

    // Construct the URL to the file
    let file_url = format!("{}/{}/{}", cloud_storage_url, bucket_name, storage_path);

    // Upload the file to Google Cloud Storage
    let upload_request = UploadObjectRequest {
        bucket: bucket_name.clone(),
        // name: storage_path.clone(),
        // content_type: Some(content_type.clone()),
        ..Default::default()
    };

    let media = Media::new(file_name);

    let upload_type = UploadType::Simple(media);

    match gcs_client
        .upload_object(&upload_request, file_data, &upload_type)
        .await
    {
        Ok(_) => {
            // Update the user record with the profile picture URL
            let collection: mongodb::Collection<User> =
                client.database("Account").collection("Users");
            let filter = doc! { "_id": ObjectId::from_str(&user_id).unwrap() };
            let update = doc! { "$set": { "profile_picture": file_url.clone() } };

            match collection.update_one(filter, update).await {
                Ok(_) => {
                    // Return success with the file URL
                    return HttpResponse::Ok().json(serde_json::json!({
                        "success": true,
                        "message": "Profile picture updated successfully",
                        "profile_picture_url": file_url
                    }));
                }
                Err(e) => {
                    eprintln!("Failed to update user with profile picture URL: {}", e);
                    return HttpResponse::InternalServerError()
                        .body("Failed to update user record");
                }
            }
        }
        Err(e) => {
            eprintln!("Failed to upload file to cloud storage: {}", e);
            return HttpResponse::InternalServerError()
                .body("Failed to upload file to cloud storage");
        }
    }
}
