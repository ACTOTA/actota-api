use actix_web::{web, HttpResponse, Responder};
use google_cloud_storage::client::{Client as GcsClient, ClientConfig};
use google_cloud_storage::http::objects::list::ListObjectsRequest;
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
        environment: env::var("RUST_ENV").unwrap_or("development".to_string()),
        version: env!("CARGO_PKG_VERSION").to_string(),
    };

    // Check MongoDB connection
    let mongo_result = check_mongodb(&client).await;
    health
        .services
        .insert("mongodb".to_string(), mongo_result.clone());

    // Check Stripe API (just validate key existence for now)
    let stripe_result = check_stripe_api().await;
    health
        .services
        .insert("stripe".to_string(), stripe_result.clone());

    // Check Google Auth API connection
    let google_auth_result = check_google_auth().await;
    health
        .services
        .insert("google_auth".to_string(), google_auth_result.clone());

    // Check Facebook Auth API connection
    let facebook_auth_result = check_facebook_auth().await;
    health
        .services
        .insert("facebook_auth".to_string(), facebook_auth_result.clone());

    // Check Cloud Storage connection
    let cloud_storage_result = check_cloud_storage().await;
    health
        .services
        .insert("cloud_storage".to_string(), cloud_storage_result.clone());

    // Determine overall status (if any service is not ok, the overall status is degraded)
    if mongo_result.status != "ok"
        || stripe_result.status != "ok"
        || google_auth_result.status != "ok"
        || facebook_auth_result.status != "ok"
        || cloud_storage_result.status != "ok"
    {
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
            details: Some("Connected successfully to MongoDB".to_string()),
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
        Ok(key) => {
            let masked_key = if key.len() > 8 {
                format!("{}***{}", &key[0..4], &key[key.len() - 4..])
            } else {
                "***".to_string()
            };

            ServiceStatus {
                status: "ok".to_string(),
                details: Some(format!("Stripe API key configured ({})", masked_key)),
            }
        }
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
        let id = client_id.unwrap();
        let masked_id = if id.len() > 8 {
            format!("{}...{}", &id[0..6], &id[id.len() - 4..])
        } else {
            "***".to_string()
        };

        ServiceStatus {
            status: "ok".to_string(),
            details: Some(format!(
                "Google Auth configured, Client ID: {}, Redirect: {}",
                masked_id,
                redirect_uri.unwrap()
            )),
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
        let id = client_id.unwrap();
        let masked_id = if id.len() > 8 {
            format!("{}...{}", &id[0..6], &id[id.len() - 4..])
        } else {
            "***".to_string()
        };

        ServiceStatus {
            status: "ok".to_string(),
            details: Some(format!(
                "Facebook Auth configured, App ID: {}, Redirect: {}",
                masked_id,
                redirect_uri.unwrap()
            )),
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

async fn check_cloud_storage() -> ServiceStatus {
    // Check if required environment variables are set
    let itinerary_bucket = env::var("ITINERARY_BUCKET").ok();
    let profile_pic_bucket = env::var("PROFILE_PIC_BUCKET").ok();

    if itinerary_bucket.is_none() || profile_pic_bucket.is_none() {
        let mut missing = Vec::new();

        if itinerary_bucket.is_none() {
            missing.push("ITINERARY_BUCKET");
        }
        if profile_pic_bucket.is_none() {
            missing.push("PROFILE_PIC_BUCKET");
        }

        return ServiceStatus {
            status: "error".to_string(),
            details: Some(format!(
                "Missing bucket configuration: {}",
                missing.join(", ")
            )),
        };
    }

    // Capture the bucket name before using it in the async operation
    let bucket_name = itinerary_bucket.as_ref().unwrap().clone();

    // Create Google Cloud Storage client
    let client_config = match ClientConfig::default().with_auth().await {
        Ok(config) => config,
        Err(e) => {
            return ServiceStatus {
                status: "error".to_string(),
                details: Some(format!("Failed to initialize GCS client config: {}", e)),
            };
        }
    };

    let gcs_client = GcsClient::new(client_config);

    // Create a list request for the bucket with a limit of 1 object
    let list_request = ListObjectsRequest {
        bucket: bucket_name.clone(),
        max_results: Some(1),
        ..Default::default()
    };

    // Test the connection by trying to list objects
    match gcs_client.list_objects(&list_request).await {
        Ok(_) => ServiceStatus {
            status: "ok".to_string(),
            details: Some(format!(
                "Connected to Cloud Storage: itinerary bucket '{}' accessible",
                bucket_name
            )),
        },
        Err(e) => ServiceStatus {
            status: "error".to_string(),
            details: Some(format!("Failed to access Cloud Storage bucket: {}", e)),
        },
    }
}
