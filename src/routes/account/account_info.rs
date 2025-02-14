use actix_multipart::Multipart;
use actix_web::{web, HttpResponse, Responder};
use bson::doc;
use cloud_storage::{Client as StorageClient, Object};
use futures::{StreamExt, TryStreamExt};
use mongodb::Client;
use std::{env, sync::Arc};

use crate::{middleware::auth::Claims, models::account::User};

pub async fn personal_information(
    data: web::Data<Arc<Client>>,
    claims: Claims,
    path: web::Path<(String,)>,
    input: web::Json<User>,
) -> impl Responder {
    if path.into_inner().0 != claims.user_id {
        return HttpResponse::Forbidden().body("Forbidden");
    }

    let client = data.into_inner();

    let collection: mongodb::Collection<User> =
        client.database("Account").collection("UserTraveler");

    let mut info = input.into_inner();
    info.updated_at = Some(chrono::Utc::now());

    let updates = bson::to_document(&info).unwrap();

    let filter = doc! { "_id": claims.user_id.clone() };
    match collection.update_one(filter, updates).await {
        Ok(_) => {
            return HttpResponse::Ok().body("User information updated");
        }
        Err(_) => {
            return HttpResponse::InternalServerError().body("Failed to update user information")
        }
    }
}

// pub async fn upload_profile_pic(
//     data: web::Data<Arc<Client>>,
//     claims: Claims,
//     mut payload: web::Payload,
// ) -> impl Responder {
//     let client = data.into_inner();
//
//     let bucket_name = env::var("PROFILE_PIC_BUCKET").unwrap_or("".to_string());
//     let storage_client = StorageClient::default();
//     let base_url = env::var("CLOUD_STORAGE_URL").unwrap_or("".to_string());
//
//     println!("Bucket name: {}", bucket_name);
//
//     // Upload the profile picture to the cloud storage bucket
//     let file_name = "profile_picture.jpg";
//     let folder = claims.user_id.clone();
//
//     // Read file from payload
//     let mut bytes = web::BytesMut::new();
//     while let Some(item) = payload.next().await {
//         let chunk = match item {
//             Ok(chunk) => chunk,
//             Err(_) => {
//                 return HttpResponse::InternalServerError().body("Failed to read upload data")
//             }
//         };
//         bytes.extend_from_slice(&chunk);
//     }
//     println!("Bytes: {:?}", bytes);
//
//     // Upload the file to the cloud storage bucket
//     let bucket
//
//     println!("Bucket: {:?}", bucket);
//
//     let collection: mongodb::Collection<User> = client.database("Account").collection("User");
//
//     let filter = doc! { "_id": claims.user_id.clone() };
//     let updates = doc! { "$set": { "profile_picture": "profile_picture.jpg" } };
//
//     match collection.update_one(filter, updates).await {
//         Ok(_) => {
//             return HttpResponse::Ok().body("Profile picture updated");
//         }
//         Err(_) => {
//             return HttpResponse::InternalServerError().body("Failed to update profile picture")
//         }
//     }
// }
