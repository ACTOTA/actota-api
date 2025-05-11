use std::{str::FromStr, sync::Arc};

use actix_web::{web, HttpResponse, Responder};
use bson::{doc, oid::ObjectId};
use futures::TryStreamExt;
use serde::{Deserialize, Serialize};
use stripe::{Charge, CustomerId, ListCharges};

use crate::{
    middleware::auth::Claims,
    models::{account::User, bookings::BookingDetails},
};

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
    println!("\n\nUserId: {:?}", user_id);

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
                    Ok(charges) => {
                        // Get the user's bookings to filter transactions
                        let bookings_collection: mongodb::Collection<BookingDetails> =
                            mongodb_client.database("Account").collection("Bookings");

                        let booking_filter = doc! { "user_id": object_id };

                        // Find all bookings for this user
                        match bookings_collection.find(booking_filter).await {
                            Ok(cursor) => {
                                // Convert cursor to vector of BookingDetails
                                let bookings = cursor.try_collect::<Vec<BookingDetails>>().await;

                                match bookings {
                                    Ok(bookings) => {
                                        // Extract transaction_ids from bookings
                                        let booking_transaction_ids: Vec<String> = bookings
                                            .iter()
                                            .filter_map(|booking| booking.transaction_id.clone())
                                            .collect();

                                        // println!("Charges: {:?}", charges.data.iter());

                                        // Filter charges to only include those with payment_intent IDs matching booking transaction_ids
                                        let filtered_charges = charges
                                            .data
                                            .iter()
                                            .filter(|charge| {
                                                // First try to match with payment_intent ID (most likely stored in transaction_id)
                                                if let Some(payment_intent) = &charge.payment_intent {
                                                    println!("\nPayment Intent: {:?}", &payment_intent);
                                                    match payment_intent {
                                                        stripe::Expandable::Id(id) => booking_transaction_ids.contains(&id.to_string()),
                                                        stripe::Expandable::Object(intent) => booking_transaction_ids.contains(&intent.id.to_string()),
                                                    }
                                                }
                                                // Fall back to charge ID if payment_intent is not available
                                                else {
                                                    println!("\nCharge ID: {:?}", &charge.id);
                                                    booking_transaction_ids.contains(&charge.id.to_string())
                                                }
                                            })
                                            .cloned()
                                            .collect::<Vec<Charge>>();

                                        // Create a new charges object with filtered data
                                        let mut filtered_charges_list = charges.clone();
                                        filtered_charges_list.data = filtered_charges;

                                        HttpResponse::Ok().json(filtered_charges_list)
                                    }
                                    Err(e) => {
                                        eprintln!("Error collecting bookings: {:?}", e);
                                        HttpResponse::InternalServerError()
                                            .body("Error processing bookings")
                                    }
                                }
                            }
                            Err(e) => {
                                eprintln!("Error finding bookings: {:?}", e);
                                HttpResponse::InternalServerError()
                                    .body("Error retrieving bookings")
                            }
                        }
                    }
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
