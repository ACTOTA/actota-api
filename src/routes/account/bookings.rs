use crate::{
    middleware::auth::Claims,
    models::{
        bookings::{BookingDetails, BookingInput, BookingWithPaymentInput},
        itinerary::base::FeaturedVacation,
    },
};
use actix_web::{web, HttpResponse, Responder};
use bson::{doc, oid::ObjectId, DateTime};
use futures::TryStreamExt;
use mongodb::Client;
use std::{str::FromStr, sync::Arc};
use stripe::CapturePaymentIntent;

pub async fn add_booking(
    data: web::Data<Arc<Client>>,
    input: web::Json<BookingInput>,
    path: web::Path<(String, String)>,
    claims: Claims,
) -> impl Responder {
    // Get the itinerary_id from the path
    let (user_id, itinerary_id) = path.into_inner();
    if user_id != claims.user_id {
        return HttpResponse::Forbidden().body("Forbidden");
    }

    let client = data.into_inner();
    let input = input.into_inner();

    // println!("")

    // Verify itinerary exists in the database
    let itinerary: mongodb::Collection<FeaturedVacation> =
        client.database("Itineraries").collection("Featured");

    if itinerary
        .find_one(doc! { "_id": ObjectId::parse_str(&itinerary_id).unwrap() })
        .await
        .is_err()
    {
        return HttpResponse::NotFound().body("Itinerary not found");
    }

    let arrival_datetime = input.arrival_datetime;
    let departure_datetime = input.departure_datetime;

    let collection: mongodb::Collection<BookingDetails> =
        client.database("Account").collection("Bookings");

    let filter = doc! {
        "user_id": ObjectId::parse_str(&claims.user_id).unwrap(),
        "itinerary_id": ObjectId::parse_str(&itinerary_id).unwrap(),
    };

    match collection.find_one(filter).await {
        Ok(Some(_)) => {
            // Already a booking
            return HttpResponse::Conflict().body("Booking already exists");
        }
        Ok(None) => {
            // Not a booking yet
            // Add the booking
            let time = DateTime::now();

            let booking = BookingDetails {
                id: None,
                user_id: ObjectId::parse_str(&claims.user_id).unwrap(),
                itinerary_id: ObjectId::parse_str(&itinerary_id).unwrap(),
                customer_id: input.customer_id,
                transaction_id: input.transaction_id,
                status: "ongoing".to_string(),
                arrival_datetime,
                departure_datetime,
                bookings: None,
                created_at: Some(time),
                updated_at: Some(time),
            };

            match collection.insert_one(&booking).await {
                Ok(_) => {
                    return HttpResponse::Ok().body("Booking created for user");
                }
                Err(_) => {
                    return HttpResponse::InternalServerError().body("Failed to add booking");
                }
            }
        }
        Err(_) => {
            return HttpResponse::InternalServerError().body("Failed to check for bookings");
        }
    }
}

pub async fn remove_booking(
    data: web::Data<Arc<Client>>,
    path: web::Path<(String, String)>,
    claims: Claims,
) -> impl Responder {
    let client = data.into_inner();
    let collection: mongodb::Collection<BookingDetails> =
        client.database("Account").collection("Bookings");

    let (user_id, itinerary_id) = path.into_inner();
    if user_id != claims.user_id {
        return HttpResponse::Forbidden().body("Forbidden");
    }

    let filter = doc! {
        "user_id": ObjectId::parse_str(&claims.user_id).unwrap(),
        "itinerary_id": ObjectId::parse_str(itinerary_id).unwrap(),
    };

    match collection.delete_one(filter).await {
        Ok(_) => {
            return HttpResponse::Ok().body("Removed Booking");
        }
        Err(_) => {
            return HttpResponse::InternalServerError().body("Failed to remove booking");
        }
    }
}

pub async fn get_booking(
    data: web::Data<Arc<Client>>,
    path: web::Path<(String, String)>,
    claims: Claims,
) -> impl Responder {
    let client = data.into_inner();
    let collection: mongodb::Collection<BookingDetails> =
        client.database("Account").collection("Bookings");

    let (user_id, itinerary_id) = path.into_inner();
    if user_id != claims.user_id {
        return HttpResponse::Forbidden().body("Forbidden");
    }

    let filter = doc! {
        "user_id": ObjectId::parse_str(&claims.user_id).unwrap(),
        "itinerary_id": ObjectId::parse_str(itinerary_id).unwrap(),
    };

    match collection.find_one(filter).await {
        Ok(Some(booking)) => {
            return HttpResponse::Ok().json(booking);
        }
        Ok(None) => {
            return HttpResponse::NotFound().body("Booking not found");
        }
        Err(_) => {
            return HttpResponse::InternalServerError().body("Failed to fetch booking");
        }
    }
}

pub async fn update_booking_payment(
    data: web::Data<Arc<Client>>,
    input: web::Json<serde_json::Value>,
    path: web::Path<(String, String)>,
    claims: Claims,
) -> impl Responder {
    let client = data.into_inner();
    let collection: mongodb::Collection<BookingDetails> =
        client.database("Account").collection("Bookings");

    let (user_id, itinerary_id) = path.into_inner();
    if user_id != claims.user_id {
        return HttpResponse::Forbidden().body("Forbidden");
    }

    // Parse the input to get customer_id and transaction_id
    let customer_id = input
        .get("customer_id")
        .and_then(|v| v.as_str())
        .map(String::from);
    let transaction_id = input
        .get("transaction_id")
        .and_then(|v| v.as_str())
        .map(String::from);

    // Make sure at least one field is provided
    if customer_id.is_none() && transaction_id.is_none() {
        return HttpResponse::BadRequest()
            .body("At least one of customer_id or transaction_id must be provided");
    }

    // Create the update document
    let mut update_doc = doc! {};

    if let Some(customer_id) = customer_id {
        update_doc.insert("customer_id", customer_id);
    }

    if let Some(transaction_id) = transaction_id {
        update_doc.insert("transaction_id", transaction_id);
    }

    let update = doc! { "$set": update_doc };

    let filter = doc! {
        "user_id": ObjectId::parse_str(&claims.user_id).unwrap(),
        "itinerary_id": ObjectId::parse_str(itinerary_id).unwrap(),
    };

    match collection.update_one(filter, update).await {
        Ok(result) => {
            if result.matched_count == 0 {
                return HttpResponse::NotFound().body("Booking not found");
            }
            return HttpResponse::Ok().body("Booking payment information updated");
        }
        Err(_) => {
            return HttpResponse::InternalServerError().body("Failed to update booking");
        }
    }
}

pub async fn get_all_bookings(
    data: web::Data<Arc<Client>>,
    path: web::Path<(String,)>,
    claims: Claims,
) -> impl Responder {
    let client = data.into_inner();
    let collection: mongodb::Collection<BookingDetails> =
        client.database("Account").collection("Bookings");

    if path.into_inner().0 != claims.user_id {
        return HttpResponse::Forbidden().body("Forbidden");
    }

    let filter = doc! {
        "user_id": ObjectId::parse_str(&claims.user_id).unwrap()
    };

    println!("Getting bookings!");

    match collection.find(filter).await {
        Ok(cursor) => {
            let results = cursor.try_collect::<Vec<BookingDetails>>().await;
            match results {
                Ok(bookings) => return HttpResponse::Ok().json(bookings),
                Err(err) => {
                    eprintln!("Error retrieving booking: {:?}", err);
                    HttpResponse::InternalServerError().body("Failed to retrieve booking")
                }
            }
        }
        Err(err) => {
            eprintln!("Error fetching bookings: {:?}", err);
            HttpResponse::InternalServerError().body("Failed to fetch bookings")
        }
    }
}

pub async fn get_booking_by_id(
    data: web::Data<Arc<Client>>,
    path: web::Path<(String, String)>,
    claims: Claims,
) -> impl Responder {
    let client = data.into_inner();
    let collection: mongodb::Collection<BookingDetails> =
        client.database("Account").collection("Bookings");

    let (user_id, booking_id) = path.into_inner();
    if user_id != claims.user_id {
        return HttpResponse::Forbidden().body("Forbidden");
    }

    // Parse booking ObjectId
    let booking_object_id = match ObjectId::parse_str(&booking_id) {
        Ok(id) => id,
        Err(e) => {
            eprintln!("Invalid booking ID format: {:?}", e);
            return HttpResponse::BadRequest().body("Invalid booking ID format");
        }
    };

    // Create filter to check both user_id and booking ID
    let filter = doc! {
        "_id": booking_object_id,
        "user_id": ObjectId::parse_str(&claims.user_id).unwrap(),
    };

    println!("Getting booking by ID: {}", booking_id);

    match collection.find_one(filter).await {
        Ok(Some(booking)) => {
            return HttpResponse::Ok().json(booking);
        }
        Ok(None) => {
            return HttpResponse::NotFound().body("Booking not found");
        }
        Err(e) => {
            eprintln!("Error fetching booking: {:?}", e);
            HttpResponse::InternalServerError().body("Failed to fetch booking")
        }
    }
}

pub async fn add_booking_with_payment(
    mongodb_data: web::Data<Arc<Client>>,
    stripe_data: web::Data<Arc<stripe::Client>>,
    input: web::Json<BookingWithPaymentInput>,
    path: web::Path<(String, String)>,
    claims: Claims,
) -> impl Responder {
    // Get the user_id and itinerary_id from the path
    let (user_id, itinerary_id) = path.into_inner();
    if user_id != claims.user_id {
        return HttpResponse::Forbidden().body("Forbidden");
    }

    let client = mongodb_data.into_inner();

    let input = input.into_inner();
    println!(
        "Parsed dates - arrival: {:?}, departure: {:?}",
        input.arrival_datetime, input.departure_datetime
    );

    let payment_intent_id = input.payment_intent_id.clone();

    // 1. First verify the payment intent exists and is in a capturable state
    println!("Verifying payment intent: {}", payment_intent_id);
    let payment_intent_result = stripe::PaymentIntent::retrieve(
        stripe_data.as_ref(),
        &stripe::PaymentIntentId::from_str(&payment_intent_id).expect("Invalid payment intent ID"),
        &[],
    )
    .await;

    match payment_intent_result {
        Ok(intent) => {
            // Check if the payment intent is in a capturable state
            if intent.status != stripe::PaymentIntentStatus::RequiresCapture {
                return HttpResponse::BadRequest().body(format!(
                    "Payment intent is not in a capturable state. Current status: {:?}",
                    intent.status
                ));
            }
        }
        Err(e) => {
            println!("Error retrieving payment intent: {:?}", e);
            return HttpResponse::InternalServerError()
                .body(format!("Failed to retrieve payment intent: {}", e));
        }
    }

    // 2. Verify itinerary exists in the database
    let itinerary: mongodb::Collection<FeaturedVacation> =
        client.database("Itineraries").collection("Featured");

    if itinerary
        .find_one(doc! { "_id": ObjectId::parse_str(&itinerary_id).unwrap() })
        .await
        .is_err()
    {
        return HttpResponse::NotFound().body("Itinerary not found");
    }

    // 3. Create the booking
    let collection: mongodb::Collection<BookingDetails> =
        client.database("Account").collection("Bookings");

    let filter = doc! {
        "user_id": ObjectId::parse_str(&claims.user_id).unwrap(),
        "itinerary_id": ObjectId::parse_str(&itinerary_id).unwrap(),
    };

    // Check if booking already exists
    match collection.find_one(filter).await {
        Ok(Some(_)) => {
            return HttpResponse::Conflict().body("Booking already exists");
        }
        Ok(None) => {
            // Not a booking yet - create it
            let time = DateTime::now();

            let booking = BookingDetails {
                id: None,
                user_id: ObjectId::parse_str(&claims.user_id).unwrap(),
                itinerary_id: ObjectId::parse_str(&itinerary_id).unwrap(),
                customer_id: Some(input.customer_id),
                transaction_id: Some(payment_intent_id.clone()),
                status: "pending".to_string(), // Start with pending status
                arrival_datetime: input.arrival_datetime,
                departure_datetime: input.departure_datetime,
                bookings: None,
                created_at: Some(time),
                updated_at: Some(time),
            };

            match collection.insert_one(&booking).await {
                Ok(insert_result) => {
                    let booking_id = insert_result
                        .inserted_id
                        .as_object_id()
                        .unwrap()
                        .to_string();

                    // 4. Capture the payment
                    println!("Capturing payment intent: {}", payment_intent_id);
                    match stripe::PaymentIntent::capture(
                        stripe_data.as_ref(),
                        &payment_intent_id,
                        CapturePaymentIntent::default(),
                    )
                    .await
                    {
                        Ok(captured_intent) => {
                            // 5. Update booking status based on payment result
                            let payment_status = captured_intent.status.to_string();
                            let update_status = if payment_status == "succeeded" {
                                "confirmed"
                            } else {
                                "pending_payment"
                            };

                            let update_filter = doc! {
                                "_id": insert_result.inserted_id
                            };

                            let update = doc! {
                                "$set": {
                                    "status": update_status,
                                    "updated_at": DateTime::now()
                                }
                            };

                            // Update booking with payment status
                            match collection.update_one(update_filter, update).await {
                                Ok(_) => {
                                    // Return success with all the details
                                    return HttpResponse::Ok().json(serde_json::json!({
                                        "success": true,
                                        "booking_id": booking_id,
                                        "payment_intent": captured_intent,
                                        "status": update_status
                                    }));
                                }
                                Err(update_err) => {
                                    println!("Error updating booking status: {:?}", update_err);
                                    // Payment was captured but status update failed
                                    return HttpResponse::Ok().json(serde_json::json!({
                                        "success": true,
                                        "warning": "Booking created and payment captured, but failed to update booking status",
                                        "booking_id": booking_id,
                                        "payment_intent": captured_intent
                                    }));
                                }
                            }
                        }
                        Err(capture_err) => {
                            println!("Error capturing payment: {:?}", capture_err);
                            // Mark booking as failed since payment failed
                            let update_filter = doc! {
                                "_id": insert_result.inserted_id
                            };

                            let update = doc! {
                                "$set": {
                                    "status": "payment_failed",
                                    "updated_at": DateTime::now()
                                }
                            };

                            // Try to update the booking status to failed
                            let _ = collection.update_one(update_filter, update).await;

                            return HttpResponse::InternalServerError()
                                .json(serde_json::json!({
                                    "success": false,
                                    "booking_id": booking_id,
                                    "error": format!("Booking created but payment capture failed: {}", capture_err)
                                }));
                        }
                    }
                }
                Err(err) => {
                    println!("Error creating booking: {:?}", err);
                    return HttpResponse::InternalServerError()
                        .body(format!("Failed to create booking: {}", err));
                }
            }
        }
        Err(err) => {
            println!("Error checking for existing booking: {:?}", err);
            return HttpResponse::InternalServerError()
                .body(format!("Failed to check for existing booking: {}", err));
        }
    }
}
