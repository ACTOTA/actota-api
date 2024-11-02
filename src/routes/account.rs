use actix_web::{web, HttpResponse, Responder};
use bcrypt::verify;
use mongodb::Client;
use mongodb::bson::doc;
use std::sync::Arc;
use chrono::Utc;

use crate::models::user::UserTraveler;

// Sign up for an account
pub async fn signup(data: web::Data<Arc<Client>>, input: web::Json<UserTraveler>) -> impl Responder {
    let client = data.into_inner(); // Get the client from App::data()

    let collection = client
        .database("Travelers")
        .collection("User");

    let curr_time = chrono::Utc::now();

    let mut doc = input.into_inner();
    doc.created_at = Some(curr_time);
    doc.updated_at = Some(curr_time);

    match collection.insert_one(doc).await {
        Ok(_) => {
            HttpResponse::Ok().body("Account successfully created.")
        }
        Err(err) => {
            // Error during insertion
            eprintln!("Failed to insert document: {:?}", err); // Log the error
            HttpResponse::InternalServerError().body("Failed to create account.")
        }
    }
}


// Sign in to an account
pub async fn signin(data: web::Data<Arc<Client>>, input: web::Json<UserTraveler>) -> impl Responder {
    let client = data.into_inner(); // Get the client from App::data()

    let collection: mongodb::Collection<UserTraveler> = client
        .database("Travelers")
        .collection("User");

    let doc = input.into_inner();
    let email = doc.email;
    // This should be hashed
    // We will implement hashing after initial testing
    let password: String = doc.password;

    let filter = doc! { "email": email };
    let user: Option<_> = match collection.find_one(filter).await {
        Ok(user) => user,
        Err(err) => {
            eprintln!("Failed to find document: {:?}", err); // Log the error
            return HttpResponse::InternalServerError().body("Failed to sign in.");
        }
    };
 
    let doc;
    match verify(password, &user.as_ref().unwrap().password) {
        Ok(true) => {
            // Password is correct
            // Update last_signin and last_signin_ip
            let curr_time: String = Utc::now().to_string();
            doc = doc! {
                "$set": {
                    "last_signin": curr_time,
                    "failed_signins": 0
                }
            };
        },
        Ok(false) => {
            // Password is incorrect
            // Increment failed_signins
            let failed_signins = user.unwrap().failed_signins.unwrap_or(0) + 1;
            doc = doc! {
                "$set": {
                    "failed_signins": failed_signins
                }
            };
        },
        Err(err) => {
            eprintln!("Failed to verify password: {:?}", err); // Log the error
            return HttpResponse::InternalServerError().body("Failed to sign in.");
        }
    }


    match collection.find_one(doc).await {
        Ok(_) => {
            HttpResponse::Ok().body("Account successfully signed in.")
        }
        Err(err) => {
            // Error during insertion
            eprintln!("Failed to find document: {:?}", err); // Log the error
            HttpResponse::InternalServerError().body("Failed to sign in.")
        }
    }
}
