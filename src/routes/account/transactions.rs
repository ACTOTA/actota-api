use std::{str::FromStr, sync::Arc};

use actix_web::{web, HttpResponse, Responder};
use bson::{doc, oid::ObjectId};
use futures::TryStreamExt;
use serde::{Deserialize, Serialize};
use stripe::{Charge, CustomerId, ListCharges, ListRefunds, Refund};

use crate::{
    middleware::auth::Claims,
    models::{account::User, bookings::BookingDetails},
};

// Unified transaction type that can be either a charge or refund
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "transaction_type")]
pub enum TransactionWithBooking {
    #[serde(rename = "charge")]
    Charge {
        #[serde(flatten)]
        charge: Charge,
        booking_id: String,
    },
    #[serde(rename = "refund")]
    Refund {
        #[serde(flatten)]
        refund: Refund,
        booking_id: String,
    },
}

// Custom response that mimics Stripe's List response but with our custom transaction type
#[derive(Debug, Serialize, Deserialize, Clone)]
struct TransactionsWithBookingIds {
    object: String, // This will be set to a constant value
    url: String,
    has_more: bool,
    data: Vec<TransactionWithBooking>,
}

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

                // Fetch both charges and refunds
                let charges_result = stripe::Charge::list(&client, &list_charges).await;
                let refunds_result = stripe::Refund::list(&client, &ListRefunds::new()).await;

                match (charges_result, refunds_result) {
                    (Ok(charges), Ok(refunds)) => {
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
                                        let mut transactions_with_bookings = Vec::new();

                                        // Process charges
                                        for charge in charges.data.iter() {
                                            let mut transaction_id = None;

                                            // First try to get payment_intent ID
                                            if let Some(payment_intent) = &charge.payment_intent {
                                                transaction_id = match payment_intent {
                                                    stripe::Expandable::Id(id) => {
                                                        Some(id.to_string())
                                                    }
                                                    stripe::Expandable::Object(intent) => {
                                                        Some(intent.id.to_string())
                                                    }
                                                };
                                            }

                                            // Fall back to charge ID if payment_intent is not available
                                            if transaction_id.is_none() {
                                                transaction_id = Some(charge.id.to_string());
                                            }

                                            if let Some(trans_id) = transaction_id {
                                                // Find matching booking
                                                if let Some(booking) = bookings.iter().find(|b| {
                                                    b.transaction_id
                                                        .as_ref()
                                                        .map_or(false, |id| id == &trans_id)
                                                }) {
                                                    // Create charge transaction with booking ID
                                                    let booking_id = booking.id.map_or_else(
                                                        || "unknown".to_string(),
                                                        |id| id.to_string(),
                                                    );

                                                    transactions_with_bookings.push(
                                                        TransactionWithBooking::Charge {
                                                            charge: charge.clone(),
                                                            booking_id,
                                                        },
                                                    );
                                                }
                                            }
                                        }

                                        // Process refunds
                                        for refund in refunds.data.iter() {
                                            // Match refund to booking using payment_intent or charge ID
                                            let mut refund_transaction_id = None;

                                            // Try to get payment intent ID from refund
                                            if let Some(payment_intent) = &refund.payment_intent {
                                                refund_transaction_id = match payment_intent {
                                                    stripe::Expandable::Id(id) => Some(id.to_string()),
                                                    stripe::Expandable::Object(intent) => Some(intent.id.to_string()),
                                                };
                                            }

                                            // Fall back to charge ID if available
                                            if refund_transaction_id.is_none() {
                                                if let Some(charge) = &refund.charge {
                                                    refund_transaction_id = match charge {
                                                        stripe::Expandable::Id(id) => Some(id.to_string()),
                                                        stripe::Expandable::Object(charge_obj) => Some(charge_obj.id.to_string()),
                                                    };
                                                }
                                            }

                                            if let Some(trans_id) = refund_transaction_id {
                                                // Find matching booking by transaction_id or refund_id
                                                if let Some(booking) = bookings.iter().find(|b| {
                                                    // Check if booking has this transaction_id or refund_id
                                                    b.transaction_id.as_ref().map_or(false, |id| id == &trans_id) ||
                                                    // Check if this booking has been refunded (you might need to add a refund_id field to BookingDetails)
                                                    b.status == "refunded"
                                                }) {
                                                    let booking_id = booking.id.map_or_else(
                                                        || "unknown".to_string(),
                                                        |id| id.to_string(),
                                                    );

                                                    transactions_with_bookings.push(
                                                        TransactionWithBooking::Refund {
                                                            refund: refund.clone(),
                                                            booking_id,
                                                        },
                                                    );
                                                }
                                            }
                                        }

                                        // Sort transactions by created date (newest first)
                                        transactions_with_bookings.sort_by(|a, b| {
                                            let a_created = match a {
                                                TransactionWithBooking::Charge { charge, .. } => charge.created,
                                                TransactionWithBooking::Refund { refund, .. } => refund.created,
                                            };
                                            let b_created = match b {
                                                TransactionWithBooking::Charge { charge, .. } => charge.created,
                                                TransactionWithBooking::Refund { refund, .. } => refund.created,
                                            };
                                            b_created.cmp(&a_created) // Descending order (newest first)
                                        });

                                        // Create custom response with our transactions
                                        let transactions_response = TransactionsWithBookingIds {
                                            object: "list".to_string(),
                                            url: charges.url.clone(),
                                            has_more: charges.has_more || refunds.has_more,
                                            data: transactions_with_bookings,
                                        };

                                        HttpResponse::Ok().json(transactions_response)
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
                    (Err(e), _) => {
                        eprintln!("Error listing charges: {:?}", e);
                        HttpResponse::InternalServerError().body("Failed to list charges")
                    }
                    (_, Err(e)) => {
                        eprintln!("Error listing refunds: {:?}", e);
                        HttpResponse::InternalServerError().body("Failed to list refunds")
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
