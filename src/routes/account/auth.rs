use actix_web::{web, HttpResponse, Responder};
use chrono::{Duration, Utc};
use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
use mongodb::bson::doc;
use mongodb::bson::oid::ObjectId;
use mongodb::error::WriteError;
use mongodb::Client;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::middleware::auth::Claims;
use crate::models::account::User;
use crate::models::user::{Newsletter, UserSession};

#[derive(Debug, Serialize, Deserialize)]
pub struct TokenResponse {
    auth_token: String,
}

pub async fn signup(data: web::Data<Arc<Client>>, input: web::Json<User>) -> impl Responder {
    let client = data.into_inner();
    let collection: mongodb::Collection<User> = client.database("Account").collection("Users");

    if !is_valid_email(&input.email) {
        return HttpResponse::BadRequest().body("Invalid email address");
    }

    let curr_time = Utc::now();
    let mut doc = input.into_inner();

    doc.password = bcrypt::hash(doc.password, bcrypt::DEFAULT_COST).unwrap_or("".to_string());
    doc.created_at = Some(curr_time);
    doc.updated_at = Some(curr_time);

    match collection.insert_one(&doc).await {
        Ok(result) => {
            match generate_token(&doc.email, result.inserted_id.as_object_id().unwrap()) {
                Ok(token) => HttpResponse::Ok().json(TokenResponse { auth_token: token }),
                Err(_) => HttpResponse::InternalServerError().body("Token generation failed"),
            }
        }
        Err(err) => match *err.kind {
            mongodb::error::ErrorKind::Write(error_info) => match error_info {
                mongodb::error::WriteFailure::WriteError(WriteError { code, .. }) => {
                    if code == 11000 {
                        HttpResponse::Conflict().body("User already exists")
                    } else {
                        println!("Error code: {}", code);
                        HttpResponse::InternalServerError().body("Failed to create user")
                    }
                }
                _ => HttpResponse::InternalServerError().body("Failed to create user"),
            },
            _ => HttpResponse::InternalServerError().body("Failed to create user"),
        },
    }
}

pub async fn signin(data: web::Data<Arc<Client>>, input: web::Json<User>) -> impl Responder {
    let client = data.into_inner();
    let collection: mongodb::Collection<User> = client.database("Account").collection("Users");

    let doc = input.into_inner();
    let email = doc.email;

    let filter = doc! { "email": &email };

    match collection.find_one(filter).await {
        Ok(Some(user)) => {
            println!("User: {:?}", user);
            if bcrypt::verify(&doc.password, &user.password).unwrap_or(false) {
                let update = doc! {
                    "$set": {
                        "last_signin": Utc::now().to_string(),
                        "failed_signins": 0
                    }
                };

                match collection
                    .update_one(doc! { "email": &email }, update)
                    .await
                {
                    Ok(_) => {
                        let token =
                            generate_token(&email, user.id.expect("Unable to read user_id."))
                                .map_err(|_| {
                                    HttpResponse::InternalServerError()
                                        .body("Token generation failed")
                                });

                        HttpResponse::Ok().json(TokenResponse {
                            auth_token: token.unwrap(),
                        })
                    }
                    Err(err) => {
                        eprintln!("Failed to update document: {:?}", err);
                        HttpResponse::InternalServerError().body("Failed to sign in.")
                    }
                }
            } else {
                let failed_signins = user.failed_signins.unwrap_or(0) + 1;
                let update = doc! {
                    "$set": { "failed_signins": failed_signins }
                };

                match collection
                    .update_one(doc! { "email": &email }, update)
                    .await
                {
                    Ok(_) => HttpResponse::Unauthorized().body("Invalid credentials"),
                    Err(err) => {
                        eprintln!("Failed to update failed signins: {:?}", err);
                        HttpResponse::InternalServerError().body("Failed to process signin")
                    }
                }
            }
        }
        Ok(None) => HttpResponse::NotFound().body("User not found"),
        Err(err) => {
            eprintln!("Database error: {:?}", err);
            HttpResponse::InternalServerError().body("Failed to process signin")
        }
    }
}

pub async fn user_session(
    claims: web::ReqData<Claims>,
    data: web::Data<Arc<Client>>,
) -> impl Responder {
    let client = data.into_inner();
    let collection: mongodb::Collection<User> = client.database("Account").collection("Users");

    let user_id = ObjectId::parse_str(&claims.user_id)
        .map_err(|_| HttpResponse::BadRequest().body("Invalid user ID"));
    match user_id {
        Ok(user_id) => match collection.find_one(doc! { "_id": user_id }).await {
            Ok(Some(user)) => {
                let user_session = UserSession {
                    id: user.id.unwrap_or_default(),
                    email: user.email,
                    first_name: user.first_name.unwrap_or_default(),
                    last_name: user.last_name.unwrap_or_default(),
                    created_at: user.created_at.unwrap_or_default(),
                };
                HttpResponse::Ok().json(user_session)
            }
            Ok(None) => HttpResponse::NotFound().body("User not found"),
            Err(err) => {
                eprintln!("Failed to fetch user: {:?}", err);
                HttpResponse::InternalServerError().body("Failed to fetch user")
            }
        },
        Err(resp) => resp,
    }
}

fn is_valid_email(email: &str) -> bool {
    let re = regex::Regex::new(
        r"^[a-zA-Z0-9.!#$%&'*+/=?^_`{|}~-]+@[a-zA-Z0-9](?:[a-zA-Z0-9-]*[a-zA-Z0-9])?(?:\.[a-zA-Z0-9](?:[a-zA-Z0-9-]*[a-zA-Z0-9])?)*$",
    );
    return re.unwrap().is_match(email);
}

fn generate_token(email: &str, user_id: ObjectId) -> Result<String, jsonwebtoken::errors::Error> {
    let secret = std::env::var("JWT_SECRET").expect("JWT_SECRET must be set");
    let now = Utc::now();

    let claims = Claims {
        sub: email.to_string(),
        iat: now.timestamp() as usize,
        exp: (now + Duration::hours(24)).timestamp() as usize,
        user_id: user_id.to_string(),
    };

    let header = Header::new(Algorithm::HS256);
    encode(&header, &claims, &EncodingKey::from_secret(secret.as_ref()))
}

pub async fn newsletter_subscribe(
    data: web::Data<Arc<Client>>,
    input: web::Json<Newsletter>,
) -> impl Responder {
    let client = data.into_inner();
    let collection: mongodb::Collection<Newsletter> =
        client.database("Travelers").collection("Newsletter");

    let mut doc = input.into_inner();
    doc.created_at = Some(Utc::now());
    doc.updated_at = Some(Utc::now());
    doc.subscribed = Some(true);

    match collection.insert_one(&doc).await {
        Ok(_) => HttpResponse::Ok().body("Subscribed to newsletter"),
        Err(err) => {
            eprintln!("Failed to insert document: {:?}", err);
            HttpResponse::InternalServerError().body("Failed to subscribe to newsletter")
        }
    }
}

pub async fn newsletter_unsubscribe(
    data: web::Data<Arc<Client>>,
    input: web::Json<Newsletter>,
) -> impl Responder {
    let client = data.into_inner();
    let collection: mongodb::Collection<Newsletter> =
        client.database("Travelers").collection("Newsletter");

    let doc = input.into_inner();
    let filter = doc! { "email": doc.email };
    let update = doc! { "$set": { "subscribed": false } };

    match collection.update_one(filter, update).await {
        Ok(_) => HttpResponse::Ok().body("Unsubscribed from newsletter"),
        Err(err) => {
            eprintln!("Failed to update document: {:?}", err);
            HttpResponse::InternalServerError().body("Failed to unsubscribe from newsletter")
        }
    }
}
