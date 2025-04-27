use crate::{
    middleware::auth::Claims,
    models::{
        bookings::{Booking, BookingStatus},
        itinerary::FeaturedVacation,
        refund_policy::RefundPolicy,
    },
    services::{itinerary_service::get_images, payment::interface::PaymentOperations},
};
use actix_web::{web, HttpResponse, Responder};
use bson::{doc, oid::ObjectId};
use chrono::Utc;
use futures::TryStreamExt;
use mongodb::Client;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Debug, Serialize, Deserialize)]
pub struct CancelResponse {
    pub success: bool,
    pub message: String,
    pub refund_amount: Option<i64>,
    pub refund_id: Option<String>,
}

// Create a booking
pub async fn add_booking(
    data: web::Data<Arc<Client>>,
    path: web::Path<(String, String)>,
    claims: Claims,
) -> impl Responder {
    // Get the itinerary_id from the path
    let (user_id, itinerary_id) = path.into_inner();
    if user_id != claims.user_id {
        return HttpResponse::Forbidden().body("Forbidden");
    }

    let client = data.into_inner();

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

    let collection: mongodb::Collection<Booking> =
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
            let time = chrono::Utc::now();

            let booking = Booking {
                id: None,
                user_id: ObjectId::parse_str(&claims.user_id).unwrap(),
                itinerary_id: ObjectId::parse_str(&itinerary_id).unwrap(),
                status: "ongoing".to_string(),
                start_datetime: None,
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
    let collection: mongodb::Collection<Booking> =
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
    let collection: mongodb::Collection<Booking> =
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

pub async fn get_all_bookings(
    data: web::Data<Arc<Client>>,
    path: web::Path<(String,)>,
    claims: Claims,
) -> impl Responder {
    let client = data.into_inner();
    let collection: mongodb::Collection<Booking> =
        client.database("Account").collection("Bookings");

    if path.into_inner().0 != claims.user_id {
        return HttpResponse::Forbidden().body("Forbidden");
    }

    let filter = doc! {
        "user_id": ObjectId::parse_str(&claims.user_id).unwrap()
    };

    match collection.find(filter).await {
        Ok(cursor) => {
            let results = cursor.try_collect::<Vec<Booking>>().await;
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

pub async fn cancel_booking(
    mongo_client: web::Data<Arc<Client>>,
    payment_provider: web::Data<Arc<dyn PaymentOperations + 'static>>,
    path: web::Path<(String, String)>,
    claims: Claims,
) -> impl Responder {
    let (user_id, booking_id) = path.into_inner();

    // Authorization check
    if user_id != claims.user_id {
        return HttpResponse::Forbidden().json(CancelResponse {
            success: false,
            message: "Unauthorized access".to_string(),
            refund_amount: None,
            refund_id: None,
        });
    }

    let client = mongo_client.into_inner();
    let bookings_collection: mongodb::Collection<Booking> =
        client.database("Account").collection("Bookings");

    let booking_object_id = match ObjectId::parse_str(&booking_id) {
        Ok(id) => id,
        Err(_) => {
            return HttpResponse::BadRequest().json(CancelResponse {
                success: false,
                message: "Invalid booking ID format".to_string(),
                refund_amount: None,
                refund_id: None,
            });
        }
    };

    // Get booking details
    let filter = doc! { "_id": booking_object_id };
    let booking = match bookings_collection.find_one(filter.clone()).await {
        Ok(Some(booking)) => booking,
        Ok(None) => {
            return HttpResponse::NotFound().json(CancelResponse {
                success: false,
                message: "Booking not found".to_string(),
                refund_amount: None,
                refund_id: None,
            });
        }
        Err(_) => {
            return HttpResponse::InternalServerError().json(CancelResponse {
                success: false,
                message: "Error retrieving booking".to_string(),
                refund_amount: None,
                refund_id: None,
            });
        }
    };

    // Check if booking is already cancelled or refunded
    if booking.status == BookingStatus::Cancelled.to_string()
        || booking.status == BookingStatus::Refunded.to_string()
    {
        return HttpResponse::BadRequest().json(CancelResponse {
            success: false,
            message: "Booking is already cancelled or refunded".to_string(),
            refund_amount: None,
            refund_id: None,
        });
    }

    // Apply refund policy if payment exists
    let mut refund_id = None;
    let mut refund_amount = None;

    if let (Some(payment_intent_id), Some(payment_amount)) =
        (&booking.payment_intent_id, booking.payment_amount)
    {
        // Apply refund policy
        let refund_policy = RefundPolicy::default();
        let created_at = booking.created_at.unwrap_or_else(Utc::now);
        let calculated_refund_amount =
            refund_policy.calculate_refund_amount(payment_amount, created_at);

        if calculated_refund_amount > 0 {
            // Process refund through payment provider
            match payment_provider
                .create_refund(payment_intent_id, Some(calculated_refund_amount))
                .await
            {
                Ok(refund) => {
                    refund_id = Some(refund.id.to_string());
                    refund_amount = Some(calculated_refund_amount);
                }
                Err(e) => {
                    eprintln!("Refund error: {:?}", e);
                    return HttpResponse::InternalServerError().json(CancelResponse {
                        success: false,
                        message: "Failed to process refund".to_string(),
                        refund_amount: None,
                        refund_id: None,
                    });
                }
            }
        }
    }

    // Update booking status
    let new_status = if refund_id.is_some() {
        BookingStatus::Refunded.to_string()
    } else {
        BookingStatus::Cancelled.to_string()
    };

    let update = doc! {
        "$set": {
            "status": new_status,
            "updated_at": Utc::now(),
            "refund_id": refund_id.clone()
        }
    };

    match bookings_collection.update_one(filter, update).await {
        Ok(_) => {
            let message = if refund_amount.is_some() {
                format!(
                    "Booking cancelled with refund of ${:.2}",
                    refund_amount.unwrap() as f64 / 100.0
                )
            } else {
                "Booking cancelled without refund".to_string()
            };

            HttpResponse::Ok().json(CancelResponse {
                success: true,
                message,
                refund_amount,
                refund_id,
            })
        }
        Err(_) => HttpResponse::InternalServerError().json(CancelResponse {
            success: false,
            message: "Failed to update booking status".to_string(),
            refund_amount: None,
            refund_id: None,
        }),
    }
}
