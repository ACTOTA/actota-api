use actix_web::{web, HttpResponse, Responder};
use mongodb::{bson::doc, Client};
use serde::Serialize;
use std::collections::HashMap;
use std::env;
use std::sync::Arc;

#[derive(Serialize)]
struct HealthStatus {
    status: String,
    services: HashMap<String, ServiceStatus>,
    environment: String,
    version: String,
}

#[derive(Serialize, Clone)]
struct ServiceStatus {
    status: String,
    details: Option<String>,
}

pub async fn health_check(client: web::Data<Arc<Client>>) -> impl Responder {
    let mut health = HealthStatus {
        status: "ok".to_string(),
        services: HashMap::new(),
        environment: if cfg!(debug_assertions) {
            "development"
        } else {
            "production"
        }
        .to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
    };

    // Check MongoDB connection
    let mongo_result = check_mongodb(&client).await;
    health.services.insert("mongodb".to_string(), mongo_result.clone());
    
    // Check Stripe API (just validate key existence for now)
    let stripe_result = check_stripe_api().await;
    health.services.insert("stripe".to_string(), stripe_result.clone());

    // Check Google Auth API connection
    let google_auth_result = check_google_auth().await;
    health.services.insert("google_auth".to_string(), google_auth_result.clone());

    // Check Facebook Auth API connection
    let facebook_auth_result = check_facebook_auth().await;
    health.services.insert("facebook_auth".to_string(), facebook_auth_result.clone());

    // Determine overall status (if any service is not ok, the overall status is degraded)
    if mongo_result.status != "ok" || 
       stripe_result.status != "ok" ||
       google_auth_result.status != "ok" ||
       facebook_auth_result.status != "ok" {
        health.status = "degraded".to_string();
    }

    HttpResponse::Ok().json(health)
}

async fn check_mongodb(client: &web::Data<Arc<Client>>) -> ServiceStatus {
    match client
        .database("Account")
        .run_command(doc! {"ping": 1})
        .await
    {
        Ok(_) => ServiceStatus {
            status: "ok".to_string(),
            details: None,
        },
        Err(e) => {
            // Log error for internal visibility
            eprintln!("MongoDB health check failed: {}", e);
            
            ServiceStatus {
                status: "error".to_string(),
                details: Some(format!("Failed to connect: {}", e)),
            }
        }
    }
}

async fn check_stripe_api() -> ServiceStatus {
    // Just validate key existence for basic check
    // In a more comprehensive check, you could make a test API call
    match env::var("STRIPE_SECRET_KEY") {
        Ok(_) => ServiceStatus {
            status: "ok".to_string(),
            details: None,
        },
        Err(_) => ServiceStatus {
            status: "error".to_string(),
            details: Some("STRIPE_SECRET_KEY not configured".to_string()),
        },
    }
}

async fn check_google_auth() -> ServiceStatus {
    // Check if required environment variables are set
    let client_id = env::var("GOOGLE_CLIENT_ID").ok();
    let client_secret = env::var("GOOGLE_CLIENT_SECRET").ok();
    let redirect_uri = env::var("GOOGLE_REDIRECT_URI").ok();
    
    if client_id.is_some() && client_secret.is_some() && redirect_uri.is_some() {
        ServiceStatus {
            status: "ok".to_string(),
            details: None,
        }
    } else {
        let mut missing = Vec::new();
        
        if client_id.is_none() {
            missing.push("GOOGLE_CLIENT_ID");
        }
        if client_secret.is_none() {
            missing.push("GOOGLE_CLIENT_SECRET");
        }
        if redirect_uri.is_none() {
            missing.push("GOOGLE_REDIRECT_URI");
        }
        
        ServiceStatus {
            status: "error".to_string(),
            details: Some(format!("Missing configuration: {}", missing.join(", "))),
        }
    }
}

async fn check_facebook_auth() -> ServiceStatus {
    // Check if required environment variables are set
    let client_id = env::var("FACEBOOK_CLIENT_ID").ok();
    let client_secret = env::var("FACEBOOK_CLIENT_SECRET").ok();
    let redirect_uri = env::var("FACEBOOK_REDIRECT_URI").ok();
    
    if client_id.is_some() && client_secret.is_some() && redirect_uri.is_some() {
        ServiceStatus {
            status: "ok".to_string(),
            details: None,
        }
    } else {
        let mut missing = Vec::new();
        
        if client_id.is_none() {
            missing.push("FACEBOOK_CLIENT_ID");
        }
        if client_secret.is_none() {
            missing.push("FACEBOOK_CLIENT_SECRET");
        }
        if redirect_uri.is_none() {
            missing.push("FACEBOOK_REDIRECT_URI");
        }
        
        ServiceStatus {
            status: "error".to_string(),
            details: Some(format!("Missing configuration: {}", missing.join(", "))),
        }
    }
}