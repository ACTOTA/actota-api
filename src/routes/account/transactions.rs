use std::{str::FromStr, sync::Arc};

use actix_web::{web, HttpResponse, Responder};
use bson::{doc, oid::ObjectId};
use serde::{Deserialize, Serialize};
use stripe::{CustomerId, ListCharges};

use crate::{middleware::auth::Claims, models::account::User};

#[derive(Serialize, Deserialize)]
pub struct TransactionsInput {
    user_id: String,
    customer_id: String,
}

pub async fn get_transactions(
    claims: Claims,
    stripe_data: web::Data<Arc<stripe::Client>>,
    mongodb_data: web::Data<Arc<mongodb::Client>>,
    path: web::Path<String>,
) -> impl Responder {
    let user_id = path.into_inner();
    println!("UserId: {:?}", user_id);

    if claims.user_id != user_id {
        return HttpResponse::Forbidden().body("Forbidden");
    }
    // Get customer_id
    let mongodb_client = mongodb_data.into_inner();

    let collection: mongodb::Collection<User> =
        mongodb_client.database("Account").collection("Users");

    let object_id = ObjectId::parse_str(user_id).expect("Unable to get user ID");

    let filter = doc! { "_id": object_id };

    // Query the database for the user
    match collection.find_one(filter).await {
        Ok(Some(user)) => {
            // Assuming customer_id is a field in your User document
            if let Some(customer_id) = user.customer_id {
                // Now you have the customer_id
                let customer_id_str = customer_id.to_string();

                // Use the customer_id with Stripe
                let customer_id: CustomerId =
                    CustomerId::from_str(&customer_id_str).expect("Unable to get Customer ID");

                // Continue with your Stripe API call...
                let mut list_charges = ListCharges::new();
                list_charges.customer = Some(customer_id);

                let client = stripe_data.into_inner();

                match stripe::Charge::list(&client, &list_charges).await {
                    Ok(charges) => HttpResponse::Ok().json(charges),
                    Err(e) => {
                        eprintln!("Error listing charges: {:?}", e);
                        HttpResponse::InternalServerError().body("Failed to list charges")
                    }
                }
            } else {
                HttpResponse::NotFound().body("Customer ID not found for this user")
            }
        }
        Ok(None) => HttpResponse::NotFound().body("User not found"),
        Err(e) => {
            eprintln!("Database error: {:?}", e);
            HttpResponse::InternalServerError().body("Database error")
        }
    }
}
