use actix_web::{web, App, HttpResponse, middleware::Logger, HttpRequest, Responder};
use actix_cors::Cors;
use std::sync::Arc;
use tokio::time::{sleep, Duration};

use actota_api::db::mongo::create_mongo_client;

pub struct TestApp {
    pub client: Arc<mongodb::Client>,
}

impl TestApp {
    pub async fn new() -> Self {
        let mongo_uri = std::env::var("MONGODB_URI")
            .unwrap_or_else(|_| "mongodb://localhost:27017".to_string());
        let client = create_mongo_client(&mongo_uri).await;

        Self { client }
    }

    pub fn create_app(&self) -> App<
        impl actix_web::dev::ServiceFactory<
            actix_web::dev::ServiceRequest,
            Config = (),
            Response = actix_web::dev::ServiceResponse,
            Error = actix_web::Error,
            InitError = (),
        >,
    > {
        App::new()
            .app_data(web::Data::new(self.client.clone()))
            .wrap(
                Cors::default()
                    .allow_any_origin()
                    .allow_any_method()
                    .allow_any_header()
                    .max_age(3600),
            )
            .wrap(Logger::default())
            .route("/", web::get().to(|| async { "ACTOTA API is running" }))
            .route("/health", web::get().to(health_check))
            .route("/locations", web::get().to(get_locations))
            .route("/activities", web::get().to(get_activities))
            .route("/lodging", web::get().to(get_lodging))
            .route("/itineraries", web::get().to(get_itineraries))
            .route("/itineraries/featured", web::get().to(get_featured_itineraries))
            .route("/itineraries/search", web::post().to(search_itineraries))
            .route("/itineraries/search-or-generate", web::post().to(search_or_generate))
            .route("/itineraries/{id}", web::get().to(get_itinerary_by_id))
            .service(
                web::scope("/auth")
                    .route("/signin", web::post().to(signin))
                    .route("/signup", web::post().to(signup))
                    .route("/google", web::get().to(google_oauth))
                    .route("/facebook", web::get().to(facebook_oauth))
                    .route("/session", web::get().to(unauthorized_handler))
            )
            .service(
                web::scope("/account/{id}")
                    .route("", web::get().to(unauthorized_handler))
                    .route("", web::put().to(unauthorized_handler))
                    .route("/favorites", web::get().to(unauthorized_handler))
                    .route("/favorites/{itinerary_id}", web::post().to(unauthorized_handler))
                    .route("/favorites/{itinerary_id}", web::delete().to(unauthorized_handler))
                    .route("/bookings", web::get().to(unauthorized_handler))
                    .route("/bookings/{booking_id}", web::get().to(unauthorized_handler))
                    .route("/bookings/itinerary/{itinerary_id}", web::post().to(unauthorized_handler))
                    .route("/bookings/itinerary/{itinerary_id}", web::delete().to(unauthorized_handler))
                    .route("/payment-methods", web::get().to(unauthorized_handler))
                    .route("/transactions", web::get().to(unauthorized_handler))
                    .route("/email-verifications", web::post().to(unauthorized_handler))
                    .route("/email-verifications", web::get().to(unauthorized_handler))
                    .route("/email-verifications/{verification_id}", web::put().to(unauthorized_handler))
            )
            .service(
                web::scope("/payment")
                    .route("/payment-intent", web::post().to(unauthorized_handler))
                    .route("/capture-payment", web::post().to(unauthorized_handler))
            )
            .service(
                web::scope("/admin")
                    .route("/users", web::get().to(unauthorized_handler))
                    .route("/users/{id}/role", web::put().to(unauthorized_handler))
                    .route("/itineraries/featured/add", web::post().to(unauthorized_handler))
                    .route("/itineraries/{id}/images", web::put().to(unauthorized_handler))
            )
            .route("/itineraries/find", web::post().to(unauthorized_handler))
            .route("/email-verifications", web::post().to(create_email_verification))
            .route("/email-verifications/{id}", web::put().to(verify_email))
            .route("/stripe/webhook", web::post().to(stripe_webhook))
    }
}

// Mock handler functions for testing
async fn health_check() -> impl Responder {
    HttpResponse::Ok().json(serde_json::json!({"status": "OK"}))
}

async fn get_locations() -> impl Responder {
    HttpResponse::Ok().json(serde_json::json!([]))
}

async fn get_activities() -> impl Responder {
    HttpResponse::Ok().json(serde_json::json!([]))
}

async fn get_lodging() -> impl Responder {
    HttpResponse::Ok().json(serde_json::json!([]))
}

async fn get_itineraries() -> impl Responder {
    HttpResponse::Ok().json(serde_json::json!([]))
}

async fn get_featured_itineraries() -> impl Responder {
    HttpResponse::Ok().json(serde_json::json!([]))
}

async fn search_itineraries() -> impl Responder {
    HttpResponse::Ok().json(serde_json::json!([]))
}

async fn search_or_generate() -> impl Responder {
    HttpResponse::Ok().json(serde_json::json!({"status": "success"}))
}

async fn get_itinerary_by_id() -> impl Responder {
    HttpResponse::NotFound().json(serde_json::json!({"error": "Itinerary not found"}))
}

async fn signin() -> impl Responder {
    HttpResponse::Unauthorized().json(serde_json::json!({"error": "Invalid credentials"}))
}

async fn signup() -> impl Responder {
    HttpResponse::BadRequest().json(serde_json::json!({"error": "Invalid input"}))
}

async fn google_oauth() -> impl Responder {
    HttpResponse::Found().header("Location", "https://accounts.google.com/oauth").finish()
}

async fn facebook_oauth() -> impl Responder {
    HttpResponse::Found().header("Location", "https://www.facebook.com/oauth").finish()
}

async fn create_email_verification() -> impl Responder {
    HttpResponse::Ok().json(serde_json::json!({"status": "created"}))
}

async fn verify_email() -> impl Responder {
    HttpResponse::NotFound().json(serde_json::json!({"error": "Verification not found"}))
}

async fn stripe_webhook() -> impl Responder {
    HttpResponse::BadRequest().json(serde_json::json!({"error": "Invalid webhook"}))
}

async fn unauthorized_handler() -> impl Responder {
    HttpResponse::Unauthorized().json(serde_json::json!({"error": "Unauthorized"}))
}

pub fn get_test_user_id() -> String {
    "test_user_123".to_string()
}

pub fn get_test_email() -> String {
    "test@example.com".to_string()
}

pub fn get_test_password() -> String {
    "testpassword123".to_string()
}

pub async fn cleanup_test_data(client: &mongodb::Client) {
    let db = client.database("Account");
    
    // Clean up test collections
    let collections = ["User", "Bookings", "EmailVerifications", "Favorites"];
    for collection_name in collections {
        let collection = db.collection::<mongodb::bson::Document>(collection_name);
        let _ = collection.delete_many(
            mongodb::bson::doc! {
                "$or": [
                    {"email": {"$regex": "test.*@example.com"}},
                    {"user_id": {"$regex": "test_user_.*"}},
                ]
            }
        ).await;
    }
}

pub async fn wait_for_server_ready(port: u16) {
    for _ in 0..30 {
        if let Ok(_) = reqwest::get(&format!("http://localhost:{}/health", port)).await {
            return;
        }
        sleep(Duration::from_millis(100)).await;
    }
    panic!("Server failed to start within timeout");
}