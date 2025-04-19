use crate::{
    middleware::auth::Claims,
    models::{bookings::Booking, itinerary::FeaturedVacation},
    services::itinerary_service::get_images,
};
use actix_web::{web, HttpResponse, Responder};
use bson::{doc, oid::ObjectId};
use futures::TryStreamExt;
use mongodb::Client;
use std::sync::Arc;

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

pub async fn update_booking(
    data: web::Data<Arc<Client>>,
    claims: Claims,
    path: web::Path<(String, String)>,
    input: web::Json<Booking>,
) -> impl Responder {

    // Get the booking_id from the path
    let (user_id, booking_id) = path.into_inner();
    if user_id != claims.user_id {
        return HttpResponse::Forbidden().body("Forbidden");
    }

    let client = data.into_inner();

    let new_booking = input.into_inner();

    // Verify booking exists in the database
    let collection: mongodb::Collection<Booking> =
        client.database("Account").collection("Bookings");

    let filter = doc! { "_id": ObjectId::parse_str(&booking_id).unwrap() };

    let mut curr_booking = match collection.find_one(filter.clone()).await {
        Ok(curr_booking) => match curr_booking {
            Some(curr_booking) => curr_booking,
            None => return HttpResponse::NotFound().body("booking not found"),
        },
        Err(_) => return HttpResponse::InternalServerError().body("Failed to find booking"),
    };


    curr_booking = new_booking;
    curr_booking.id = Some(ObjectId::parse_str(&booking_id).unwrap());
    curr_booking.updated_at = Some(chrono::Utc::now());

    let updates = bson::to_document(&curr_booking).unwrap();
    let update_doc = doc! { "$set": updates }; // $set is a MongoDB operator to update fields

    match collection.update_one(filter, update_doc).await {
        Ok(result) if result.modified_count > 0 => {
            return HttpResponse::Ok().body("booking information updated");
        }
        Ok(_) => {
            HttpResponse::NotModified().body("No changes applied")
        }
        Err(_) => {
            return HttpResponse::InternalServerError().body("Failed to update booking information")
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
                Ok(bookings) => {
                    return HttpResponse::Ok().json(bookings)
                }
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

// pub async fn get_bookings(
//     data: web::Data<Arc<Client>>,
//     claims: Claims,
//     path: web::Path<(String,)>,
// ) -> impl Responder {
//     if path.into_inner().0 != claims.user_id {
//         return HttpResponse::Forbidden().body("Forbidden");
//     }

//     let client = data.into_inner();
//     let collection: mongodb::Collection<Booking> =
//         client.database("Account").collection("Bookings");

//     let filter = doc! {
//         "user_id": ObjectId::parse_str(&claims.user_id).unwrap(),
//     };

//     match collection.find(filter).await {
//         Ok(cursor) => {
//             let results = cursor.try_collect::<Vec<Booking>>().await;
//             match results {
//                 Ok(bookings) => {
//                     // Extract itinerary IDs from bookings
//                     let itinerary_ids: Vec<ObjectId> = bookings
//                         .iter()
//                         .map(|booking| booking.itinerary_id.clone())
//                         .collect();

//                     // Fetch itineraries from Itineraries.Featured collection
//                     let itineraries_collection: mongodb::Collection<FeaturedVacation> =
//                         client.database("Itineraries").collection("Featured");

//                     let itinerary_filter = doc! {
//                         "_id": { "$in": itinerary_ids }
//                     };

//                     match itineraries_collection.find(itinerary_filter).await {
//                         Ok(itinerary_cursor) => {
//                             match itinerary_cursor
//                                 .try_collect::<Vec<FeaturedVacation>>()
//                                 .await
//                             {
//                                 Ok(mut featured_itineraries) => {
//                                     // Fetch images for each itinerary
//                                     featured_itineraries = get_images(featured_itineraries).await;

//                                     HttpResponse::Ok().json(featured_itineraries)
//                                 }
//                                 Err(err) => {
//                                     eprintln!("Error fetching itineraries: {:?}", err);
//                                     HttpResponse::InternalServerError()
//                                         .body("Failed to retrieve itineraries")
//                                 }
//                             }
//                         }
//                         Err(err) => {
//                             eprintln!("Error fetching itineraries: {:?}", err);
//                             HttpResponse::InternalServerError()
//                                 .body("Failed to retrieve itineraries")
//                         }
//                     }
//                 }
//                 Err(err) => {
//                     eprintln!("Error retrieving bookings: {:?}", err);
//                     HttpResponse::InternalServerError().body("Failed to retrieve bookings")
//                 }
//             }
//         }
//         Err(err) => {
//             eprintln!("Error fetching bookings: {:?}", err);
//             HttpResponse::InternalServerError().body("Failed to fetch bookings")
//         }
//     }
// }

