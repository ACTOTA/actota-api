use actix_web::{web, HttpResponse, Responder};
use bson::doc;
use chrono::Utc;
use mongodb::Client;
use oauth2::AuthorizationCode;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::middleware::auth::Claims;
use crate::models::account::{LinkAccountRequest, LinkedAccount, User};
use crate::models::facebook_auth::FacebookUserInfo;
use crate::models::google_auth::GoogleUserInfo;
use crate::services::facebook_auth_service::{
    create_facebook_oauth_client, exchange_code_for_token as exchange_facebook_code,
    get_facebook_user_info,
};
use crate::services::google_auth_service::{
    create_google_oauth_client, exchange_code_for_token as exchange_google_code,
    get_google_user_info,
};

#[derive(Debug, Serialize, Deserialize)]
pub struct UnlinkAccountRequest {
    pub provider: String, // "google" or "facebook"
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LinkedAccountsResponse {
    pub linked_accounts: Vec<LinkedAccount>,
}

// Link a social account (Google or Facebook) to an existing user account
pub async fn link_account(
    claims: web::ReqData<Claims>,
    data: web::Data<Arc<Client>>,
    path: web::Path<String>,
    input: web::Json<LinkAccountRequest>,
) -> impl Responder {
    let client = data.into_inner();
    let collection: mongodb::Collection<User> = client.database("Account").collection("Users");

    // Get the user_id from path
    let path_user_id = match bson::oid::ObjectId::parse_str(&path.into_inner()) {
        Ok(id) => id,
        Err(_) => return HttpResponse::BadRequest().body("Invalid user ID in path"),
    };

    // Get the user_id from claims
    let claims_user_id = match bson::oid::ObjectId::parse_str(&claims.user_id) {
        Ok(id) => id,
        Err(_) => return HttpResponse::BadRequest().body("Invalid user ID in token"),
    };

    // Verify that the user is only accessing their own account
    if path_user_id != claims_user_id {
        return HttpResponse::Forbidden().body("You can only manage your own account");
    }

    let user_id = claims_user_id;

    // Find the current user
    let user = match collection.find_one(doc! { "_id": user_id }).await {
        Ok(Some(user)) => user,
        Ok(None) => return HttpResponse::NotFound().body("User not found"),
        Err(err) => {
            eprintln!("Database error: {:?}", err);
            return HttpResponse::InternalServerError().body("Database error");
        }
    };

    // Process based on provider
    let provider = input.provider.to_lowercase();
    let (provider_id, provider_email) = match provider.as_str() {
        "google" => {
            // Process Google auth
            let client = create_google_oauth_client();
            let code = AuthorizationCode::new(input.code.clone());

            // Exchange the authorization code for an access token
            let access_token = match exchange_google_code(&client, code).await {
                Ok(token) => token,
                Err(e) => {
                    return HttpResponse::InternalServerError()
                        .body(format!("Failed to exchange Google auth code: {}", e))
                }
            };

            // Get user info using the access token
            let user_info: GoogleUserInfo = match get_google_user_info(&access_token).await {
                Ok(info) => info,
                Err(e) => {
                    return HttpResponse::InternalServerError()
                        .body(format!("Failed to get Google user info: {}", e))
                }
            };

            (user_info.id, user_info.email)
        }
        "facebook" => {
            // Process Facebook auth
            let client = create_facebook_oauth_client();
            let code = AuthorizationCode::new(input.code.clone());

            // Exchange the authorization code for an access token
            let access_token = match exchange_facebook_code(&client, code).await {
                Ok(token) => token,
                Err(e) => {
                    return HttpResponse::InternalServerError()
                        .body(format!("Failed to exchange Facebook auth code: {}", e))
                }
            };

            // Get user info using the access token
            let user_info: FacebookUserInfo = match get_facebook_user_info(&access_token).await {
                Ok(info) => info,
                Err(e) => {
                    return HttpResponse::InternalServerError()
                        .body(format!("Failed to get Facebook user info: {}", e))
                }
            };

            (user_info.id, user_info.email)
        }
        _ => return HttpResponse::BadRequest().body("Unsupported provider"),
    };

    // Check if the email from the social account matches the user's email
    if provider_email != user.email {
        return HttpResponse::BadRequest().body(
            "The email address associated with this social account does not match your account email"
        );
    }

    // Check if this social account is already linked to this user
    let mut existing_linked_accounts = user.linked_accounts.unwrap_or_else(Vec::new);
    if existing_linked_accounts
        .iter()
        .any(|account| account.provider == provider && account.provider_id == provider_id)
    {
        return HttpResponse::BadRequest().body("This account is already linked");
    }

    // Add the new linked account
    existing_linked_accounts.push(LinkedAccount {
        provider: provider.clone(),
        provider_id,
        linked_at: Utc::now(),
    });

    // Update the user with the new linked account
    let update = doc! {
        "$set": {
            "linked_accounts": bson::to_bson(&existing_linked_accounts).unwrap(),
            "updated_at": Utc::now()
        }
    };

    match collection
        .update_one(doc! { "_id": user_id }, update, None)
        .await
    {
        Ok(_) => HttpResponse::Ok().json(serde_json::json!({
            "message": format!("{} account linked successfully", provider)
        })),
        Err(err) => {
            eprintln!("Failed to update user: {:?}", err);
            HttpResponse::InternalServerError().body("Failed to link account")
        }
    }
}

// Unlink a social account from a user account
pub async fn unlink_account(
    claims: web::ReqData<Claims>,
    data: web::Data<Arc<Client>>,
    path: web::Path<String>,
    input: web::Json<UnlinkAccountRequest>,
) -> impl Responder {
    let client = data.into_inner();
    let collection: mongodb::Collection<User> = client.database("Account").collection("Users");

    // Get the user_id from path
    let path_user_id = match bson::oid::ObjectId::parse_str(&path.into_inner()) {
        Ok(id) => id,
        Err(_) => return HttpResponse::BadRequest().body("Invalid user ID in path"),
    };

    // Get the user_id from claims
    let claims_user_id = match bson::oid::ObjectId::parse_str(&claims.user_id) {
        Ok(id) => id,
        Err(_) => return HttpResponse::BadRequest().body("Invalid user ID in token"),
    };

    // Verify that the user is only accessing their own account
    if path_user_id != claims_user_id {
        return HttpResponse::Forbidden().body("You can only manage your own account");
    }

    let user_id = claims_user_id;

    // Find the current user
    let user = match collection.find_one(doc! { "_id": user_id }, None).await {
        Ok(Some(user)) => user,
        Ok(None) => return HttpResponse::NotFound().body("User not found"),
        Err(err) => {
            eprintln!("Database error: {:?}", err);
            return HttpResponse::InternalServerError().body("Database error");
        }
    };

    let provider = input.provider.to_lowercase();
    if provider != "google" && provider != "facebook" {
        return HttpResponse::BadRequest().body("Unsupported provider");
    }

    // Get existing linked accounts
    let mut linked_accounts = match user.linked_accounts {
        Some(accounts) => accounts,
        None => return HttpResponse::BadRequest().body("No linked accounts found"),
    };

    // Check if there's an account with the specified provider to unlink
    let initial_count = linked_accounts.len();
    linked_accounts.retain(|account| account.provider != provider);

    if linked_accounts.len() == initial_count {
        return HttpResponse::BadRequest()
            .body(format!("No {} account linked to your account", provider));
    }

    let now = Utc::now();
    // Update the user with the modified linked accounts
    let update = doc! {
        "$set": {
            "linked_accounts": bson::to_bson(&linked_accounts).unwrap(),
            "updated_at": bson::to_bson(&now).unwrap()
        }
    };

    match collection.update_one(doc! { "_id": user_id }, update).await {
        Ok(_) => HttpResponse::Ok().json(serde_json::json!({
            "message": format!("{} account unlinked successfully", provider)
        })),
        Err(err) => {
            eprintln!("Failed to update user: {:?}", err);
            HttpResponse::InternalServerError().body("Failed to unlink account")
        }
    }
}

// Get all linked accounts for a user
pub async fn get_linked_accounts(
    claims: web::ReqData<Claims>,
    data: web::Data<Arc<Client>>,
    path: web::Path<String>,
) -> impl Responder {
    let client = data.into_inner();
    let collection: mongodb::Collection<User> = client.database("Account").collection("Users");

    // Get the user_id from path
    let path_user_id = match bson::oid::ObjectId::parse_str(&path.into_inner()) {
        Ok(id) => id,
        Err(_) => return HttpResponse::BadRequest().body("Invalid user ID in path"),
    };

    // Get the user_id from claims
    let claims_user_id = match bson::oid::ObjectId::parse_str(&claims.user_id) {
        Ok(id) => id,
        Err(_) => return HttpResponse::BadRequest().body("Invalid user ID in token"),
    };

    // Verify that the user is only accessing their own account
    if path_user_id != claims_user_id {
        return HttpResponse::Forbidden().body("You can only view your own linked accounts");
    }

    let user_id = claims_user_id;

    // Find the current user
    match collection.find_one(doc! { "_id": user_id }).await {
        Ok(Some(user)) => {
            let linked_accounts = user.linked_accounts.unwrap_or_else(Vec::new);
            HttpResponse::Ok().json(LinkedAccountsResponse { linked_accounts })
        }
        Ok(None) => HttpResponse::NotFound().body("User not found"),
        Err(err) => {
            eprintln!("Database error: {:?}", err);
            HttpResponse::InternalServerError().body("Database error")
        }
    }
}

