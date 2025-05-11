use actix_web::{web, HttpResponse, Responder};
use bson::{doc, oid::ObjectId};
use mongodb::Client;
use serde::{Deserialize, Serialize};
use std::{str::FromStr, sync::Arc};

use crate::{
    middleware::auth::Claims,
    models::account::User,
};

// Request struct for update_customer_id
#[derive(Serialize, Deserialize)]
pub struct UpdateCustomerId {
    customer_id: String,
}

// Update customer_id for a user
pub async fn update_customer_id(
    data: web::Data<Arc<Client>>,
    input: web::Json<UpdateCustomerId>,
    claims: Claims,
    path: web::Path<String>,
) -> impl Responder {
    let user_id = path.into_inner();
    
    // Check authorization - user can only update their own record
    if user_id != claims.user_id {
        return HttpResponse::Forbidden().body("Forbidden");
    }

    let customer_id = input.into_inner().customer_id;
    let client = data.into_inner();
    let collection: mongodb::Collection<User> = client.database("Account").collection("Users");

    let filter = doc! { "_id": ObjectId::from_str(&user_id).unwrap() };
    let update = doc! { "$set": { "customer_id": customer_id.clone() } };

    match collection.update_one(filter, update).await {
        Ok(result) => {
            if result.modified_count == 0 {
                return HttpResponse::NotFound().body("User not found or customer ID not updated");
            }
            HttpResponse::Ok().json(doc! {
                "success": true,
                "message": "Customer ID updated successfully",
                "customer_id": customer_id
            })
        }
        Err(err) => {
            eprintln!("MongoDB Error updating customer_id: {:?}", err);
            HttpResponse::InternalServerError().body("Failed to update customer ID")
        }
    }
}