mod common;

use actix_web::{test, http::header};
use serde_json::json;
use serial_test::serial;

use common::{TestApp, get_test_user_id, get_test_email, cleanup_test_data};

async fn create_test_jwt_token() -> String {
    // In a real implementation, this would create a valid JWT token
    // For testing purposes, we'll use a mock token
    "Bearer test_jwt_token".to_string()
}

#[actix_rt::test]
#[serial]
async fn test_get_session_without_auth() {
    let test_app = TestApp::new().await;
    let app = test::init_service(test_app.create_app()).await;

    let req = test::TestRequest::get()
        .uri("/auth/session")
        .to_request();
    
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 401);
}

#[actix_rt::test]
#[serial]
async fn test_get_account_info_without_auth() {
    let test_app = TestApp::new().await;
    let app = test::init_service(test_app.create_app()).await;
    
    let user_id = get_test_user_id();

    let req = test::TestRequest::get()
        .uri(&format!("/account/{}", user_id))
        .to_request();
    
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 401);
}

#[actix_rt::test]
#[serial]
async fn test_update_account_info_without_auth() {
    let test_app = TestApp::new().await;
    let app = test::init_service(test_app.create_app()).await;
    
    let user_id = get_test_user_id();

    let req = test::TestRequest::put()
        .uri(&format!("/account/{}", user_id))
        .set_json(&json!({
            "first_name": "Updated",
            "last_name": "Name"
        }))
        .to_request();
    
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 401);
}

#[actix_rt::test]
#[serial]
async fn test_upload_profile_picture_without_auth() {
    let test_app = TestApp::new().await;
    let app = test::init_service(test_app.create_app()).await;
    
    let user_id = get_test_user_id();

    let req = test::TestRequest::post()
        .uri(&format!("/account/{}/profile-picture", user_id))
        .to_request();
    
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 401);
}

#[actix_rt::test]
#[serial]
async fn test_get_favorites_without_auth() {
    let test_app = TestApp::new().await;
    let app = test::init_service(test_app.create_app()).await;
    
    let user_id = get_test_user_id();

    let req = test::TestRequest::get()
        .uri(&format!("/account/{}/favorites", user_id))
        .to_request();
    
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 401);
}

#[actix_rt::test]
#[serial]
async fn test_add_favorite_without_auth() {
    let test_app = TestApp::new().await;
    let app = test::init_service(test_app.create_app()).await;
    
    let user_id = get_test_user_id();
    let itinerary_id = "test_itinerary_123";

    let req = test::TestRequest::post()
        .uri(&format!("/account/{}/favorites/{}", user_id, itinerary_id))
        .to_request();
    
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 401);
}

#[actix_rt::test]
#[serial]
async fn test_remove_favorite_without_auth() {
    let test_app = TestApp::new().await;
    let app = test::init_service(test_app.create_app()).await;
    
    let user_id = get_test_user_id();
    let itinerary_id = "test_itinerary_123";

    let req = test::TestRequest::delete()
        .uri(&format!("/account/{}/favorites/{}", user_id, itinerary_id))
        .to_request();
    
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 401);
}

#[actix_rt::test]
#[serial]
async fn test_get_all_bookings_without_auth() {
    let test_app = TestApp::new().await;
    let app = test::init_service(test_app.create_app()).await;
    
    let user_id = get_test_user_id();

    let req = test::TestRequest::get()
        .uri(&format!("/account/{}/bookings", user_id))
        .to_request();
    
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 401);
}

#[actix_rt::test]
#[serial]
async fn test_get_booking_without_auth() {
    let test_app = TestApp::new().await;
    let app = test::init_service(test_app.create_app()).await;
    
    let user_id = get_test_user_id();
    let booking_id = "test_booking_123";

    let req = test::TestRequest::get()
        .uri(&format!("/account/{}/bookings/{}", user_id, booking_id))
        .to_request();
    
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 401);
}

#[actix_rt::test]
#[serial]
async fn test_add_booking_without_auth() {
    let test_app = TestApp::new().await;
    let app = test::init_service(test_app.create_app()).await;
    
    let user_id = get_test_user_id();
    let itinerary_id = "test_itinerary_123";

    let req = test::TestRequest::post()
        .uri(&format!("/account/{}/bookings/itinerary/{}", user_id, itinerary_id))
        .set_json(&json!({
            "booking_date": "2024-12-25",
            "travelers": 2
        }))
        .to_request();
    
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 401);
}

#[actix_rt::test]
#[serial]
async fn test_remove_booking_without_auth() {
    let test_app = TestApp::new().await;
    let app = test::init_service(test_app.create_app()).await;
    
    let user_id = get_test_user_id();
    let itinerary_id = "test_itinerary_123";

    let req = test::TestRequest::delete()
        .uri(&format!("/account/{}/bookings/itinerary/{}", user_id, itinerary_id))
        .to_request();
    
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 401);
}

#[actix_rt::test]
#[serial]
async fn test_cancel_booking_without_auth() {
    let test_app = TestApp::new().await;
    let app = test::init_service(test_app.create_app()).await;
    
    let user_id = get_test_user_id();
    let booking_id = "test_booking_123";

    let req = test::TestRequest::post()
        .uri(&format!("/account/{}/bookings/{}/cancel", user_id, booking_id))
        .to_request();
    
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 401);
}

#[actix_rt::test]
#[serial]
async fn test_get_payment_methods_without_auth() {
    let test_app = TestApp::new().await;
    let app = test::init_service(test_app.create_app()).await;
    
    let user_id = get_test_user_id();

    let req = test::TestRequest::get()
        .uri(&format!("/account/{}/payment-methods", user_id))
        .to_request();
    
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 401);
}

#[actix_rt::test]
#[serial]
async fn test_create_payment_method_without_auth() {
    let test_app = TestApp::new().await;
    let app = test::init_service(test_app.create_app()).await;
    
    let user_id = get_test_user_id();

    let req = test::TestRequest::post()
        .uri(&format!("/account/{}/payment-methods", user_id))
        .set_json(&json!({
            "payment_method_id": "pm_test_123"
        }))
        .to_request();
    
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 401);
}

#[actix_rt::test]
#[serial]
async fn test_delete_payment_method_without_auth() {
    let test_app = TestApp::new().await;
    let app = test::init_service(test_app.create_app()).await;
    
    let user_id = get_test_user_id();
    let pm_id = "pm_test_123";

    let req = test::TestRequest::delete()
        .uri(&format!("/account/{}/payment-methods/{}", user_id, pm_id))
        .to_request();
    
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 401);
}

#[actix_rt::test]
#[serial]
async fn test_get_transactions_without_auth() {
    let test_app = TestApp::new().await;
    let app = test::init_service(test_app.create_app()).await;
    
    let user_id = get_test_user_id();

    let req = test::TestRequest::get()
        .uri(&format!("/account/{}/transactions", user_id))
        .to_request();
    
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 401);
}

#[actix_rt::test]
#[serial]
async fn test_create_user_email_verification_without_auth() {
    let test_app = TestApp::new().await;
    let app = test::init_service(test_app.create_app()).await;
    
    let user_id = get_test_user_id();

    let req = test::TestRequest::post()
        .uri(&format!("/account/{}/email-verifications", user_id))
        .set_json(&json!({
            "email": "test@example.com"
        }))
        .to_request();
    
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 401);
}

#[actix_rt::test]
#[serial]
async fn test_get_user_email_verifications_without_auth() {
    let test_app = TestApp::new().await;
    let app = test::init_service(test_app.create_app()).await;
    
    let user_id = get_test_user_id();

    let req = test::TestRequest::get()
        .uri(&format!("/account/{}/email-verifications", user_id))
        .to_request();
    
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 401);
}

#[actix_rt::test]
#[serial]
async fn test_verify_user_email_without_auth() {
    let test_app = TestApp::new().await;
    let app = test::init_service(test_app.create_app()).await;
    
    let user_id = get_test_user_id();
    let verification_id = "verification_123";

    let req = test::TestRequest::put()
        .uri(&format!("/account/{}/email-verifications/{}", user_id, verification_id))
        .set_json(&json!({
            "code": "123456"
        }))
        .to_request();
    
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 401);
}

#[actix_rt::test]
#[serial]
async fn test_find_dream_vacation_without_auth() {
    let test_app = TestApp::new().await;
    let app = test::init_service(test_app.create_app()).await;

    let req = test::TestRequest::post()
        .uri("/itineraries/find")
        .set_json(&json!({
            "location": "Paris",
            "budget": 3000,
            "duration": 7,
            "interests": ["art", "culture"]
        }))
        .to_request();
    
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 401);
}

#[actix_rt::test]
#[serial]
async fn test_get_or_create_customer_without_auth() {
    let test_app = TestApp::new().await;
    let app = test::init_service(test_app.create_app()).await;
    
    let user_id = get_test_user_id();

    let req = test::TestRequest::post()
        .uri(&format!("/account/{}/customer", user_id))
        .to_request();
    
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 401);
}

#[actix_rt::test]
#[serial]
async fn test_update_customer_id_without_auth() {
    let test_app = TestApp::new().await;
    let app = test::init_service(test_app.create_app()).await;
    
    let user_id = get_test_user_id();

    let req = test::TestRequest::post()
        .uri(&format!("/account/{}/update-customer-id", user_id))
        .set_json(&json!({
            "customer_id": "cus_test_123"
        }))
        .to_request();
    
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 401);
}

// Test cleanup after each test
#[actix_rt::test]
#[serial]
async fn test_cleanup() {
    let test_app = TestApp::new().await;
    cleanup_test_data(&test_app.client).await;
}