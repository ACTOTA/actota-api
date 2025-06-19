use actix_web::{web, HttpResponse, Responder};
use bson::oid::ObjectId;
use mongodb::{Client, bson::doc};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::Arc;
use futures::TryStreamExt;

use crate::services::account_service::{EmailService, EmailError, EmailVerification};

#[derive(Debug, Deserialize)]
pub struct CreateVerificationRequest {
    pub email: String,
}

#[derive(Debug, Deserialize)]
pub struct VerifyCodeRequest {
    pub code: String,
}

#[derive(Debug, Serialize)]
pub struct VerificationResponse {
    pub id: String,
    pub email: String,
    pub expires_at: String,
    pub created_at: String,
}

#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub error: String,
    pub message: String,
}

// POST /api/users/{user_id}/email-verifications
pub async fn create_user_email_verification(
    data: web::Data<Arc<Client>>,
    path: web::Path<String>,
    req_body: web::Json<CreateVerificationRequest>,
) -> impl Responder {
    let user_id_str = path.into_inner();
    let user_id = match ObjectId::parse_str(&user_id_str) {
        Ok(id) => id,
        Err(_) => {
            return HttpResponse::BadRequest().json(ErrorResponse {
                error: "invalid_user_id".to_string(),
                message: "Invalid user ID format".to_string(),
            });
        }
    };

    let client = data.into_inner();
    
    let email_service = match EmailService::new() {
        Ok(service) => service,
        Err(err) => {
            eprintln!("Failed to initialize email service: {:?}", err);
            return HttpResponse::InternalServerError().json(ErrorResponse {
                error: "service_error".to_string(),
                message: "Failed to initialize email service".to_string(),
            });
        }
    };

    match email_service
        .send_verification_html_email(&req_body.email, Some(user_id), &client)
        .await
    {
        Ok(verification_code) => {
            // Get the created verification record to return its details
            let collection = client.database("actota").collection::<EmailVerification>("email_verifications");
            match collection.find_one(mongodb::bson::doc! {
                "email": &req_body.email,
                "user_id": user_id,
                "verified": false
            }).await {
                Ok(Some(verification)) => {
                    HttpResponse::Created().json(VerificationResponse {
                        id: verification.id.unwrap().to_hex(),
                        email: verification.email,
                        expires_at: verification.expires_at.try_to_rfc3339_string().unwrap_or_default(),
                        created_at: verification.created_at.try_to_rfc3339_string().unwrap_or_default(),
                    })
                }
                _ => HttpResponse::Created().json(json!({
                    "message": "Verification email sent successfully"
                }))
            }
        }
        Err(err) => {
            eprintln!("Error sending verification: {:?}", err);
            HttpResponse::InternalServerError().json(ErrorResponse {
                error: "send_failed".to_string(),
                message: "Failed to send verification email".to_string(),
            })
        }
    }
}

// POST /api/email-verifications (for signup flow)
pub async fn create_signup_email_verification(
    data: web::Data<Arc<Client>>,
    req_body: web::Json<CreateVerificationRequest>,
) -> impl Responder {
    let client = data.into_inner();
    
    let email_service = match EmailService::new() {
        Ok(service) => service,
        Err(err) => {
            eprintln!("Failed to initialize email service: {:?}", err);
            return HttpResponse::InternalServerError().json(ErrorResponse {
                error: "service_error".to_string(),
                message: "Failed to initialize email service".to_string(),
            });
        }
    };

    match email_service
        .send_verification_html_email(&req_body.email, None, &client)
        .await
    {
        Ok(_) => {
            // Get the created verification record
            let collection = client.database("actota").collection::<EmailVerification>("email_verifications");
            match collection.find_one(mongodb::bson::doc! {
                "email": &req_body.email,
                "user_id": mongodb::bson::Bson::Null,
                "verified": false
            }).await {
                Ok(Some(verification)) => {
                    HttpResponse::Created().json(VerificationResponse {
                        id: verification.id.unwrap().to_hex(),
                        email: verification.email,
                        expires_at: verification.expires_at.try_to_rfc3339_string().unwrap_or_default(),
                        created_at: verification.created_at.try_to_rfc3339_string().unwrap_or_default(),
                    })
                }
                _ => HttpResponse::Created().json(json!({
                    "message": "Verification email sent successfully"
                }))
            }
        }
        Err(err) => {
            eprintln!("Error sending verification: {:?}", err);
            HttpResponse::InternalServerError().json(ErrorResponse {
                error: "send_failed".to_string(),
                message: "Failed to send verification email".to_string(),
            })
        }
    }
}

// PUT /api/users/{user_id}/email-verifications/{verification_id}
pub async fn verify_user_email_code(
    data: web::Data<Arc<Client>>,
    path: web::Path<(String, String)>,
    req_body: web::Json<VerifyCodeRequest>,
) -> impl Responder {
    let (user_id_str, verification_id_str) = path.into_inner();
    
    let user_id = match ObjectId::parse_str(&user_id_str) {
        Ok(id) => id,
        Err(_) => {
            return HttpResponse::BadRequest().json(ErrorResponse {
                error: "invalid_user_id".to_string(),
                message: "Invalid user ID format".to_string(),
            });
        }
    };

    let verification_id = match ObjectId::parse_str(&verification_id_str) {
        Ok(id) => id,
        Err(_) => {
            return HttpResponse::BadRequest().json(ErrorResponse {
                error: "invalid_verification_id".to_string(),
                message: "Invalid verification ID format".to_string(),
            });
        }
    };

    let client = data.into_inner();

    // Get the verification record first to check ownership
    let collection = client.database("actota").collection::<EmailVerification>("email_verifications");
    let verification = match collection.find_one(mongodb::bson::doc! {
        "_id": verification_id,
        "user_id": user_id,
        "verified": false
    }).await {
        Ok(Some(v)) => v,
        Ok(None) => {
            return HttpResponse::NotFound().json(ErrorResponse {
                error: "verification_not_found".to_string(),
                message: "Verification not found or already completed".to_string(),
            });
        }
        Err(err) => {
            eprintln!("Database error: {}", err);
            return HttpResponse::InternalServerError().json(ErrorResponse {
                error: "database_error".to_string(),
                message: "Database error occurred".to_string(),
            });
        }
    };

    match EmailService::verify_email_code(&verification.email, &req_body.code, &client).await {
        Ok(true) => HttpResponse::Ok().json(json!({
            "verified": true,
            "email": verification.email,
            "verified_at": mongodb::bson::DateTime::now().try_to_rfc3339_string().unwrap_or_default()
        })),
        Ok(false) => HttpResponse::BadRequest().json(ErrorResponse {
            error: "verification_failed".to_string(),
            message: "Verification failed".to_string(),
        }),
        Err(EmailError::InvalidCode) => HttpResponse::BadRequest().json(ErrorResponse {
            error: "invalid_code".to_string(),
            message: "Invalid verification code".to_string(),
        }),
        Err(EmailError::CodeExpired) => HttpResponse::BadRequest().json(ErrorResponse {
            error: "code_expired".to_string(),
            message: "Verification code has expired".to_string(),
        }),
        Err(err) => {
            eprintln!("Verification error: {}", err);
            HttpResponse::InternalServerError().json(ErrorResponse {
                error: "verification_failed".to_string(),
                message: "Verification failed".to_string(),
            })
        }
    }
}

// PUT /api/email-verifications/{verification_id}
pub async fn verify_signup_email_code(
    data: web::Data<Arc<Client>>,
    path: web::Path<String>,
    req_body: web::Json<VerifyCodeRequest>,
) -> impl Responder {
    let verification_id_str = path.into_inner();
    
    let verification_id = match ObjectId::parse_str(&verification_id_str) {
        Ok(id) => id,
        Err(_) => {
            return HttpResponse::BadRequest().json(ErrorResponse {
                error: "invalid_verification_id".to_string(),
                message: "Invalid verification ID format".to_string(),
            });
        }
    };

    let client = data.into_inner();

    // Get the verification record
    let collection = client.database("actota").collection::<EmailVerification>("email_verifications");
    let verification = match collection.find_one(mongodb::bson::doc! {
        "_id": verification_id,
        "user_id": mongodb::bson::Bson::Null,
        "verified": false
    }).await {
        Ok(Some(v)) => v,
        Ok(None) => {
            return HttpResponse::NotFound().json(ErrorResponse {
                error: "verification_not_found".to_string(),
                message: "Verification not found or already completed".to_string(),
            });
        }
        Err(err) => {
            eprintln!("Database error: {}", err);
            return HttpResponse::InternalServerError().json(ErrorResponse {
                error: "database_error".to_string(),
                message: "Database error occurred".to_string(),
            });
        }
    };

    match EmailService::verify_email_code(&verification.email, &req_body.code, &client).await {
        Ok(true) => HttpResponse::Ok().json(json!({
            "verified": true,
            "email": verification.email,
            "verified_at": mongodb::bson::DateTime::now().try_to_rfc3339_string().unwrap_or_default()
        })),
        Ok(false) => HttpResponse::BadRequest().json(ErrorResponse {
            error: "verification_failed".to_string(),
            message: "Verification failed".to_string(),
        }),
        Err(EmailError::InvalidCode) => HttpResponse::BadRequest().json(ErrorResponse {
            error: "invalid_code".to_string(),
            message: "Invalid verification code".to_string(),
        }),
        Err(EmailError::CodeExpired) => HttpResponse::BadRequest().json(ErrorResponse {
            error: "code_expired".to_string(),
            message: "Verification code has expired".to_string(),
        }),
        Err(err) => {
            eprintln!("Verification error: {}", err);
            HttpResponse::InternalServerError().json(ErrorResponse {
                error: "verification_failed".to_string(),
                message: "Verification failed".to_string(),
            })
        }
    }
}

// GET /api/users/{user_id}/email-verifications
pub async fn get_user_email_verifications(
    data: web::Data<Arc<Client>>,
    path: web::Path<String>,
) -> impl Responder {
    let user_id_str = path.into_inner();
    let user_id = match ObjectId::parse_str(&user_id_str) {
        Ok(id) => id,
        Err(_) => {
            return HttpResponse::BadRequest().json(ErrorResponse {
                error: "invalid_user_id".to_string(),
                message: "Invalid user ID format".to_string(),
            });
        }
    };

    let client = data.into_inner();
    let collection = client.database("actota").collection::<EmailVerification>("email_verifications");
    
    match collection.find(mongodb::bson::doc! {
        "user_id": user_id,
        "verified": false,
        "expires_at": { "$gt": mongodb::bson::DateTime::now() }
    }).await {
        Ok(mut cursor) => {
            let mut verifications = Vec::new();
            while let Ok(Some(verification)) = cursor.try_next().await {
                verifications.push(VerificationResponse {
                    id: verification.id.unwrap().to_hex(),
                    email: verification.email,
                    expires_at: verification.expires_at.try_to_rfc3339_string().unwrap_or_default(),
                    created_at: verification.created_at.try_to_rfc3339_string().unwrap_or_default(),
                });
            }
            HttpResponse::Ok().json(verifications)
        }
        Err(err) => {
            eprintln!("Database error: {}", err);
            HttpResponse::InternalServerError().json(ErrorResponse {
                error: "database_error".to_string(),
                message: "Failed to retrieve verifications".to_string(),
            })
        }
    }
}