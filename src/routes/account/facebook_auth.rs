use actix_web::{http::header, web, HttpResponse, Responder};
use bson::{doc, oid::ObjectId};
use chrono::Utc;
use mongodb::Client;
use oauth2::AuthorizationCode;
use oauth2::CsrfToken;
use std::sync::Arc;

use crate::models::account::User;
use crate::models::facebook_auth::FacebookAuthCallbackParams;
use crate::routes::account::auth::generate_token;
use crate::services::facebook_auth_service::{
    create_facebook_oauth_client, exchange_code_for_token, get_facebook_auth_url,
    get_facebook_user_info,
};

// Initiate Facebook OAuth flow
pub async fn facebook_auth_init() -> impl Responder {
    let client = create_facebook_oauth_client();
    let (auth_url, csrf_token) = get_facebook_auth_url(&client);

    // In a production app, you should store this CSRF token in a secure session
    // or encrypted cookie to validate in the callback
    // For simplicity, we're just passing it through the OAuth flow

    HttpResponse::Found()
        .insert_header((header::LOCATION, auth_url.to_string()))
        .finish()
}

// Handle Facebook OAuth callback
pub async fn facebook_auth_callback(
    data: web::Data<Arc<Client>>,
    query: web::Query<FacebookAuthCallbackParams>,
) -> impl Responder {
    // Validate the callback
    if let Some(error) = &query.error {
        return HttpResponse::BadRequest().body(format!("OAuth error: {}", error));
    }

    let client = create_facebook_oauth_client();
    let code = AuthorizationCode::new(query.code.clone());

    // Exchange the authorization code for an access token
    let access_token = match exchange_code_for_token(&client, code).await {
        Ok(token) => token,
        Err(e) => return HttpResponse::InternalServerError().body(format!("Token error: {}", e)),
    };

    // Get user info using the access token
    let user_info = match get_facebook_user_info(&access_token).await {
        Ok(info) => info,
        Err(e) => {
            return HttpResponse::InternalServerError().body(format!("User info error: {}", e))
        }
    };

    // Use the MongoDB client
    let db_client = data.into_inner();
    let collection: mongodb::Collection<User> = db_client.database("Account").collection("Users");

    // Try to find a user with the same email
    let filter = doc! { "email": &user_info.email };
    let now = Utc::now();

    match collection.find_one(filter.clone()).await {
        Ok(Some(existing_user)) => {
            // User exists, update their sign-in information
            let update = doc! {
                "$set": {
                    "last_signin": now.to_string(),
                    "failed_signins": 0
                }
            };

            if let Err(err) = collection.update_one(filter, update).await {
                eprintln!("Failed to update user sign-in info: {:?}", err);
                return HttpResponse::InternalServerError().body("Failed to update user");
            }

            // Generate JWT token
            match generate_token(&existing_user.email, existing_user.id.unwrap()) {
                Ok(token) => {
                    let frontend_url = std::env::var("FRONTEND_URL")
                        .unwrap_or("http://localhost:3000".to_string());
                    let redirect_url = format!("{}/?token={}", frontend_url, token);

                    HttpResponse::Found()
                        .insert_header((header::LOCATION, redirect_url))
                        .finish()
                }
                Err(_) => HttpResponse::InternalServerError().body("Failed to generate token"),
            }
        }
        Ok(None) => {
            // User doesn't exist, create a new account
            let new_user = User {
                id: None,
                email: user_info.email,
                // We don't set a password for users who sign in with Facebook
                password: bcrypt::hash("", bcrypt::DEFAULT_COST).unwrap_or("".to_string()),
                customer_id: None,
                first_name: user_info.first_name,
                last_name: user_info.last_name,
                phone_number: None,
                birth_date: None,
                last_signin: Some(now),
                last_signin_ip: None,
                failed_signins: Some(0),
                notification: None,
                created_at: Some(now),
                updated_at: Some(now),
            };

            match collection.insert_one(&new_user).await {
                Ok(result) => {
                    let user_id = result.inserted_id.as_object_id().unwrap();

                    // Generate JWT token
                    match generate_token(&new_user.email, user_id) {
                        Ok(token) => {
                            // Redirect to frontend with token
                            let frontend_url = std::env::var("FRONTEND_URL")
                                .unwrap_or("http://localhost:3000".to_string());
                            let redirect_url = format!("{}/?token={}", frontend_url, token);
                            HttpResponse::Found()
                                .insert_header((header::LOCATION, redirect_url))
                                .finish()
                        }
                        Err(_) => {
                            HttpResponse::InternalServerError().body("Failed to generate token")
                        }
                    }
                }
                Err(err) => {
                    eprintln!("Failed to create user: {:?}", err);
                    HttpResponse::InternalServerError().body("Failed to create user")
                }
            }
        }
        Err(err) => {
            eprintln!("Database error: {:?}", err);
            HttpResponse::InternalServerError().body("Database error")
        }
    }
}
