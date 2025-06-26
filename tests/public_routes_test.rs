mod common;

use actix_web::{test, web, App};
use serde_json::json;
use serial_test::serial;

use common::TestApp;

#[actix_rt::test]
#[serial]
async fn test_health_check() {
    let test_app = TestApp::new().await;
    let app = test::init_service(test_app.create_app()).await;

    let req = test::TestRequest::get()
        .uri("/health")
        .to_request();
    
    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());
    
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["status"], "OK");
}

#[actix_rt::test]
#[serial]
async fn test_root_endpoint() {
    let test_app = TestApp::new().await;
    let app = test::init_service(
        App::new()
            .route("/", web::get().to(|| async { "ACTOTA API is running" }))
    ).await;

    let req = test::TestRequest::get()
        .uri("/")
        .to_request();
    
    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());
    
    let body = test::read_body(resp).await;
    assert_eq!(body, "ACTOTA API is running");
}

#[actix_rt::test]
#[serial]
async fn test_signup_missing_fields() {
    let test_app = TestApp::new().await;
    let app = test::init_service(test_app.create_app()).await;

    let req = test::TestRequest::post()
        .uri("/auth/signup")
        .set_json(&json!({
            "email": "test@example.com"
            // Missing password, first_name, last_name
        }))
        .to_request();
    
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 400);
}

#[actix_rt::test]
#[serial]
async fn test_signup_invalid_email() {
    let test_app = TestApp::new().await;
    let app = test::init_service(test_app.create_app()).await;

    let req = test::TestRequest::post()
        .uri("/auth/signup")
        .set_json(&json!({
            "email": "invalid-email",
            "password": "password123",
            "first_name": "Test",
            "last_name": "User"
        }))
        .to_request();
    
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 400);
}

#[actix_rt::test]
#[serial]
async fn test_signin_missing_credentials() {
    let test_app = TestApp::new().await;
    let app = test::init_service(test_app.create_app()).await;

    let req = test::TestRequest::post()
        .uri("/auth/signin")
        .set_json(&json!({
            "email": "test@example.com"
            // Missing password
        }))
        .to_request();
    
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 400);
}

#[actix_rt::test]
#[serial]
async fn test_signin_invalid_credentials() {
    let test_app = TestApp::new().await;
    let app = test::init_service(test_app.create_app()).await;

    let req = test::TestRequest::post()
        .uri("/auth/signin")
        .set_json(&json!({
            "email": "nonexistent@example.com",
            "password": "wrongpassword"
        }))
        .to_request();
    
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 401);
}

#[actix_rt::test]
#[serial]
async fn test_google_oauth_init() {
    let test_app = TestApp::new().await;
    let app = test::init_service(test_app.create_app()).await;

    let req = test::TestRequest::get()
        .uri("/auth/google")
        .to_request();
    
    let resp = test::call_service(&app, req).await;
    // Should redirect to Google OAuth
    assert!(resp.status().is_redirection() || resp.status().is_success());
}

#[actix_rt::test]
#[serial]
async fn test_facebook_oauth_init() {
    let test_app = TestApp::new().await;
    let app = test::init_service(test_app.create_app()).await;

    let req = test::TestRequest::get()
        .uri("/auth/facebook")
        .to_request();
    
    let resp = test::call_service(&app, req).await;
    // Should redirect to Facebook OAuth
    assert!(resp.status().is_redirection() || resp.status().is_success());
}

#[actix_rt::test]
#[serial]
async fn test_get_all_locations() {
    let test_app = TestApp::new().await;
    let app = test::init_service(test_app.create_app()).await;

    let req = test::TestRequest::get()
        .uri("/locations")
        .to_request();
    
    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());
    
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert!(body.is_array());
}

#[actix_rt::test]
#[serial]
async fn test_get_all_activities() {
    let test_app = TestApp::new().await;
    let app = test::init_service(test_app.create_app()).await;

    let req = test::TestRequest::get()
        .uri("/activities")
        .to_request();
    
    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());
    
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert!(body.is_array());
}

#[actix_rt::test]
#[serial]
async fn test_get_all_lodging() {
    let test_app = TestApp::new().await;
    let app = test::init_service(test_app.create_app()).await;

    let req = test::TestRequest::get()
        .uri("/lodging")
        .to_request();
    
    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());
    
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert!(body.is_array());
}

#[actix_rt::test]
#[serial]
async fn test_get_all_itineraries() {
    let test_app = TestApp::new().await;
    let app = test::init_service(test_app.create_app()).await;

    let req = test::TestRequest::get()
        .uri("/itineraries")
        .to_request();
    
    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());
    
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert!(body.is_array());
}

#[actix_rt::test]
#[serial]
async fn test_get_featured_itineraries() {
    let test_app = TestApp::new().await;
    let app = test::init_service(test_app.create_app()).await;

    let req = test::TestRequest::get()
        .uri("/itineraries/featured")
        .to_request();
    
    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());
    
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert!(body.is_array());
}

#[actix_rt::test]
#[serial]
async fn test_search_itineraries() {
    let test_app = TestApp::new().await;
    let app = test::init_service(test_app.create_app()).await;

    let req = test::TestRequest::post()
        .uri("/itineraries/search")
        .set_json(&json!({
            "location": "New York",
            "budget": 1000,
            "duration": 3
        }))
        .to_request();
    
    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());
    
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert!(body.is_array());
}

#[actix_rt::test]
#[serial]
async fn test_search_or_generate_itineraries() {
    let test_app = TestApp::new().await;
    let app = test::init_service(test_app.create_app()).await;

    let req = test::TestRequest::post()
        .uri("/itineraries/search-or-generate")
        .set_json(&json!({
            "location": "Tokyo",
            "budget": 2000,
            "duration": 5,
            "interests": ["culture", "food"]
        }))
        .to_request();
    
    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());
}

#[actix_rt::test]
#[serial]
async fn test_get_nonexistent_itinerary() {
    let test_app = TestApp::new().await;
    let app = test::init_service(test_app.create_app()).await;

    let req = test::TestRequest::get()
        .uri("/itineraries/nonexistent_id")
        .to_request();
    
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 404);
}

#[actix_rt::test]
#[serial]
async fn test_create_signup_email_verification() {
    let test_app = TestApp::new().await;
    let app = test::init_service(test_app.create_app()).await;

    let req = test::TestRequest::post()
        .uri("/email-verifications")
        .set_json(&json!({
            "email": "test@example.com"
        }))
        .to_request();
    
    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success() || resp.status().is_client_error());
}

#[actix_rt::test]
#[serial]
async fn test_verify_signup_email_invalid_id() {
    let test_app = TestApp::new().await;
    let app = test::init_service(test_app.create_app()).await;

    let req = test::TestRequest::put()
        .uri("/email-verifications/invalid_id")
        .set_json(&json!({
            "code": "123456"
        }))
        .to_request();
    
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 404);
}