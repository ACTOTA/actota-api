use actix_web::{web, HttpResponse, Responder};
// use dotenv::dotenv;
// use bcrypt::verify;
use mongodb::Client;
use mongodb::bson::doc;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use chrono::Utc;
use jsonwebtoken::{encode, EncodingKey, Header};


use crate::models::{self, user::UserTraveler};

#[derive(Serialize, Deserialize)]
struct TokenResponse {
    auth_token: String,
}

#[derive(Serialize, Deserialize)]
struct Claims {
    sub: String,
    exp: u32
}

// Sign up for an account
pub async fn signup(data: web::Data<Arc<Client>>, input: web::Json<UserTraveler>) -> impl Responder {
    let client = data.into_inner(); // Get the client from App::data()

    let collection: mongodb::Collection<models::user::UserTraveler> = client
        .database("Travelers")
        .collection("User");

    let curr_time = chrono::Utc::now();

    let mut doc = input.into_inner();
    doc.created_at = Some(curr_time);
    doc.updated_at = Some(curr_time);

    match collection.insert_one(&doc).await {
        Ok(_) => {
            let token = get_jwt_token(doc.email);
            let response = TokenResponse {
                auth_token: token,
            };
            HttpResponse::Ok().json(response)
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
    let client = data.into_inner();

    let collection: mongodb::Collection<UserTraveler> = client
        .database("Travelers")
        .collection("User");

    let doc = input.into_inner();
    let email = doc.email;
    let password = doc.password; // No need for type annotation here

    let filter = doc! { "email": &email};
    println!("Filter: {:?}", filter);

    match collection.find_one(filter).await {
        Ok(Some(user)) => { // Check if a user was found
            if password == user.password {
                // Password is correct
                let curr_time: String = Utc::now().to_string();
                let update = doc! {
                    "$set": {
                        "last_signin": curr_time,
                        "failed_signins": 0
                    }
                };

                // Update the user document (replace with your actual update logic)
                match collection.update_one(doc! { "email": &email }, update).await {
                    Ok(_) => {

                        let token = get_jwt_token(email);
                        let response = TokenResponse {
                            auth_token: token,
                        };

                        HttpResponse::Ok().json(response)
                        // HttpResponse::Ok().json({ "authToken": token })
                    },
                    Err(err) => {
                        eprintln!("Failed to update document: {:?}", err);
                        HttpResponse::InternalServerError().body("Failed to sign in.")
                    }
                }
            } else {
                // Password is incorrect
                let failed_signins = user.failed_signins.unwrap_or(0) + 1;
                let update = doc! {
                    "$set": {
                        "failed_signins": failed_signins
                    }
                };

                // Update the user document (replace with your actual update logic)
                match collection.update_one(doc! { "email": email }, update).await {
                    Ok(_) => HttpResponse::Ok().body("Incorrect password."),
                    Err(err) => {
                        eprintln!("Failed to update document: {:?}", err);
                        HttpResponse::InternalServerError().body("Failed to sign in.")
                    }
                }
            }
        }
        Ok(None) => {
            // No user found with that email
            HttpResponse::NotFound().body("User not found.")
        }
        Err(err) => {
            eprintln!("Failed to find document: {:?}", err);
            HttpResponse::InternalServerError().body("Failed to sign in.")
        }
    }
}

// Provide jwt token
fn get_jwt_token(email: String) -> String {
    if cfg!(debug_assertions) {
        dotenv::dotenv().ok();
    }

    let secret = std::env::var("JWT_SECRET").expect("JWT_SECRET must be set");
    let now = Utc::now().timestamp();
    let claims = Claims {
        sub: email,
        exp: now as u32 + (60 * 60) // 1 hour
    };

    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_ref()),
    );

    token.unwrap()
}
