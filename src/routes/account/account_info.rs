use actix_multipart::Multipart;
use actix_web::{web, HttpResponse, Responder};
use bson::{doc, oid::ObjectId};
use futures::{StreamExt, TryStreamExt};
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
