use crate::{
    middleware::auth::Claims,
    models::{
        bookings::{BookingDetails, BookingInput, BookingWithPaymentInput},
        itinerary::base::FeaturedVacation,
        account::User,
    },
    services::account_service::EmailService,
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

    println!("\n\n");
    println!("input: {:?}", input);

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

    // Create the booking directly without checking for duplicates
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
        Ok(insert_result) => {
            let booking_id = insert_result
                .inserted_id
                .as_object_id()
                .unwrap()
                .to_string();
            
            return HttpResponse::Ok().json(serde_json::json!({
                "success": true,
                "booking_id": booking_id,
                "status": "ongoing",
                "message": "Booking created successfully"
            }));
        }
        Err(e) => {
            return HttpResponse::InternalServerError().json(serde_json::json!({
                "success": false,
                "error": "Failed to add booking",
                "message": e.to_string()
            }));
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

    // Create the booking directly without checking for duplicates
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

                            let booking_object_id = insert_result.inserted_id.as_object_id().unwrap();
                            let update_filter = doc! {
                                "_id": booking_object_id
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
                                    // If payment succeeded, send confirmation email
                                    if update_status == "confirmed" {
                                        // Get user details for email
                                        let users_collection: mongodb::Collection<User> = 
                                            client.database("Account").collection("Users");
                                        
                                        if let Ok(Some(user)) = users_collection.find_one(doc! {
                                            "_id": ObjectId::parse_str(&claims.user_id).unwrap()
                                        }).await {
                                            // Get itinerary details
                                            if let Ok(Some(itinerary)) = itinerary.find_one(doc! {
                                                "_id": ObjectId::parse_str(&itinerary_id).unwrap()
                                            }).await {
                                                // Initialize email service and send confirmation
                                                if let Ok(email_service) = EmailService::new() {
                                                    let amount = captured_intent.amount as f64 / 100.0; // Convert cents to dollars
                                                    let currency = captured_intent.currency.to_string();
                                                    
                                                    // Create updated booking with ID for email
                                                    let mut booking_for_email = booking.clone();
                                                    booking_for_email.id = Some(booking_object_id);
                                                    
                                                    let user_name = user.first_name
                                                        .map(|first| {
                                                            user.last_name
                                                                .map(|last| format!("{} {}", first, last))
                                                                .unwrap_or(first)
                                                        })
                                                        .unwrap_or_else(|| "Valued Customer".to_string());
                                                    
                                                    if let Err(e) = email_service.send_booking_confirmation_email(
                                                        &user.email,
                                                        &user_name,
                                                        &booking_for_email,
                                                        &itinerary.trip_name,
                                                        amount,
                                                        &currency,
                                                        &payment_intent_id
                                                    ).await {
                                                        eprintln!("Failed to send booking confirmation email: {:?}", e);
                                                        // Don't fail the booking if email fails
                                                    }
                                                }
                                            }
                                        }
                                    }
                                    
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

pub async fn cancel_booking_with_refund(
    mongodb_data: web::Data<Arc<Client>>,
    stripe_data: web::Data<Arc<stripe::Client>>,
    path: web::Path<(String, String)>,
    claims: Claims,
) -> impl Responder {
    let (user_id, booking_id) = path.into_inner();
    if user_id != claims.user_id {
        return HttpResponse::Forbidden().body("Forbidden");
    }

    let client = mongodb_data.into_inner();
    let collection: mongodb::Collection<BookingDetails> =
        client.database("Account").collection("Bookings");

    // Find the booking by ID
    let booking_object_id = match ObjectId::parse_str(&booking_id) {
        Ok(id) => id,
        Err(_) => return HttpResponse::BadRequest().body("Invalid booking ID"),
    };

    let filter = doc! {
        "_id": booking_object_id,
        "user_id": ObjectId::parse_str(&claims.user_id).unwrap(),
    };

    // Get the booking details first
    let booking = match collection.find_one(filter.clone()).await {
        Ok(Some(booking)) => booking,
        Ok(None) => return HttpResponse::NotFound().body("Booking not found"),
        Err(e) => {
            eprintln!("Error finding booking: {:?}", e);
            return HttpResponse::InternalServerError().body("Failed to find booking");
        }
    };

    // Check if booking is already cancelled
    if booking.status == "cancelled" || booking.status == "refunded" {
        return HttpResponse::BadRequest().json(serde_json::json!({
            "success": false,
            "error": "Booking is already cancelled or refunded"
        }));
    }

    // Check if there's a transaction ID for refund
    let transaction_id = match booking.transaction_id {
        Some(id) => id,
        None => {
            // If no transaction, just cancel the booking
            let update = doc! {
                "$set": {
                    "status": "cancelled",
                    "updated_at": DateTime::now()
                }
            };

            match collection.update_one(filter, update).await {
                Ok(_) => {
                    return HttpResponse::Ok().json(serde_json::json!({
                        "success": true,
                        "message": "Booking cancelled successfully (no payment to refund)",
                        "booking_id": booking_id
                    }));
                }
                Err(e) => {
                    return HttpResponse::InternalServerError().json(serde_json::json!({
                        "success": false,
                        "error": format!("Failed to cancel booking: {}", e)
                    }));
                }
            }
        }
    };

    // Process refund through Stripe (95% of the original amount)
    println!("Processing refund for payment intent: {}", transaction_id);
    
    // First, retrieve the payment intent to get the amount
    let payment_intent = match stripe::PaymentIntent::retrieve(
        stripe_data.as_ref(),
        &stripe::PaymentIntentId::from_str(&transaction_id).expect("Invalid payment intent ID"),
        &[],
    )
    .await
    {
        Ok(intent) => intent,
        Err(e) => {
            eprintln!("Error retrieving payment intent: {:?}", e);
            return HttpResponse::InternalServerError().json(serde_json::json!({
                "success": false,
                "error": format!("Failed to retrieve payment details: {}", e)
            }));
        }
    };

    // Calculate 95% refund (5% cancellation fee)
    let refund_amount = (payment_intent.amount as f64 * 0.95) as i64;

    // Create the refund
    let refund_params = stripe::CreateRefund {
        payment_intent: Some(payment_intent.id.clone()),
        amount: Some(refund_amount),
        ..Default::default()
    };

    match stripe::Refund::create(stripe_data.as_ref(), refund_params).await {
        Ok(refund) => {
            // Update booking status to refunded
            let update = doc! {
                "$set": {
                    "status": "refunded",
                    "refund_id": refund.id.to_string(),
                    "refund_amount": refund_amount,
                    "updated_at": DateTime::now()
                }
            };

            match collection.update_one(filter, update).await {
                Ok(_) => {
                    // Send cancellation email notification
                    let users_collection: mongodb::Collection<User> = 
                        client.database("Account").collection("Users");
                    
                    if let Ok(Some(user)) = users_collection.find_one(doc! {
                        "_id": ObjectId::parse_str(&claims.user_id).unwrap()
                    }).await {
                        if let Ok(email_service) = EmailService::new() {
                            // You might want to implement send_cancellation_email method
                            // For now, we'll just log it
                            println!("Booking cancelled and refunded for user: {}", user.email);
                        }
                    }

                    return HttpResponse::Ok().json(serde_json::json!({
                        "success": true,
                        "message": "Booking cancelled and refunded successfully",
                        "booking_id": booking_id,
                        "refund": {
                            "id": refund.id.to_string(),
                            "amount": refund_amount,
                            "percentage": 95,
                            "status": refund.status.as_ref().map(|s| s.as_str()).unwrap_or("unknown"),
                            "currency": refund.currency.to_string()
                        }
                    }));
                }
                Err(e) => {
                    eprintln!("Error updating booking after refund: {:?}", e);
                    // Refund was successful but failed to update booking status
                    return HttpResponse::Ok().json(serde_json::json!({
                        "success": true,
                        "warning": "Refund processed but failed to update booking status",
                        "booking_id": booking_id,
                        "refund": {
                            "id": refund.id.to_string(),
                            "amount": refund_amount,
                            "percentage": 95,
                            "status": refund.status.as_ref().map(|s| s.as_str()).unwrap_or("unknown")
                        }
                    }));
                }
            }
        }
        Err(e) => {
            eprintln!("Error creating refund: {:?}", e);
            return HttpResponse::InternalServerError().json(serde_json::json!({
                "success": false,
                "error": format!("Failed to process refund: {}", e)
            }));
        }
    }
}
