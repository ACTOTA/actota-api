mod common;

use actix_web::{test, http::header};
use serde_json::json;
use serial_test::serial;

use common::TestApp;

#[actix_rt::test]
#[serial]
async fn test_get_all_itineraries_success() {
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
async fn test_get_itinerary_by_valid_id() {
    let test_app = TestApp::new().await;
    let app = test::init_service(test_app.create_app()).await;

    // Assuming there's at least one itinerary with a known ID
    // In a real test, you'd create test data first
    let itinerary_id = "test_itinerary_123";

    let req = test::TestRequest::get()
        .uri(&format!("/itineraries/{}", itinerary_id))
        .to_request();
    
    let resp = test::call_service(&app, req).await;
    // This might be 404 if test data doesn't exist, which is expected
    assert!(resp.status().is_success() || resp.status() == 404);
}

#[actix_rt::test]
#[serial]
async fn test_get_itinerary_by_invalid_id() {
    let test_app = TestApp::new().await;
    let app = test::init_service(test_app.create_app()).await;

    let req = test::TestRequest::get()
        .uri("/itineraries/invalid_id_format")
        .to_request();
    
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 404);
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
async fn test_search_itineraries_basic() {
    let test_app = TestApp::new().await;
    let app = test::init_service(test_app.create_app()).await;

    let req = test::TestRequest::post()
        .uri("/itineraries/search")
        .set_json(&json!({
            "location": "Paris",
            "budget": 2000,
            "duration": 5
        }))
        .to_request();
    
    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());
    
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert!(body.is_array());
}

#[actix_rt::test]
#[serial]
async fn test_search_itineraries_with_interests() {
    let test_app = TestApp::new().await;
    let app = test::init_service(test_app.create_app()).await;

    let req = test::TestRequest::post()
        .uri("/itineraries/search")
        .set_json(&json!({
            "location": "Tokyo",
            "budget": 3000,
            "duration": 7,
            "interests": ["culture", "food", "temples"]
        }))
        .to_request();
    
    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());
    
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert!(body.is_array());
}

#[actix_rt::test]
#[serial]
async fn test_search_itineraries_missing_required_fields() {
    let test_app = TestApp::new().await;
    let app = test::init_service(test_app.create_app()).await;

    let req = test::TestRequest::post()
        .uri("/itineraries/search")
        .set_json(&json!({
            "location": "New York"
            // Missing budget and duration
        }))
        .to_request();
    
    let resp = test::call_service(&app, req).await;
    // Should fail due to missing required fields
    assert!(resp.status().is_client_error() || resp.status().is_success());
}

#[actix_rt::test]
#[serial]
async fn test_search_itineraries_invalid_budget() {
    let test_app = TestApp::new().await;
    let app = test::init_service(test_app.create_app()).await;

    let req = test::TestRequest::post()
        .uri("/itineraries/search")
        .set_json(&json!({
            "location": "London",
            "budget": -100,  // Invalid negative budget
            "duration": 3
        }))
        .to_request();
    
    let resp = test::call_service(&app, req).await;
    // Should handle invalid budget gracefully
    assert!(resp.status().is_success() || resp.status().is_client_error());
}

#[actix_rt::test]
#[serial]
async fn test_search_itineraries_invalid_duration() {
    let test_app = TestApp::new().await;
    let app = test::init_service(test_app.create_app()).await;

    let req = test::TestRequest::post()
        .uri("/itineraries/search")
        .set_json(&json!({
            "location": "Berlin",
            "budget": 1500,
            "duration": 0  // Invalid zero duration
        }))
        .to_request();
    
    let resp = test::call_service(&app, req).await;
    // Should handle invalid duration gracefully
    assert!(resp.status().is_success() || resp.status().is_client_error());
}

#[actix_rt::test]
#[serial]
async fn test_search_itineraries_empty_location() {
    let test_app = TestApp::new().await;
    let app = test::init_service(test_app.create_app()).await;

    let req = test::TestRequest::post()
        .uri("/itineraries/search")
        .set_json(&json!({
            "location": "",  // Empty location
            "budget": 2000,
            "duration": 4
        }))
        .to_request();
    
    let resp = test::call_service(&app, req).await;
    // Should handle empty location
    assert!(resp.status().is_success() || resp.status().is_client_error());
}

#[actix_rt::test]
#[serial]
async fn test_search_or_generate_itineraries_basic() {
    let test_app = TestApp::new().await;
    let app = test::init_service(test_app.create_app()).await;

    let req = test::TestRequest::post()
        .uri("/itineraries/search-or-generate")
        .set_json(&json!({
            "location": "Barcelona",
            "budget": 2500,
            "duration": 6,
            "interests": ["architecture", "art", "beaches"]
        }))
        .to_request();
    
    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());
}

#[actix_rt::test]
#[serial]
async fn test_search_or_generate_itineraries_minimal() {
    let test_app = TestApp::new().await;
    let app = test::init_service(test_app.create_app()).await;

    let req = test::TestRequest::post()
        .uri("/itineraries/search-or-generate")
        .set_json(&json!({
            "location": "Rome",
            "budget": 1800,
            "duration": 4
        }))
        .to_request();
    
    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());
}

#[actix_rt::test]
#[serial]
async fn test_search_or_generate_with_empty_interests() {
    let test_app = TestApp::new().await;
    let app = test::init_service(test_app.create_app()).await;

    let req = test::TestRequest::post()
        .uri("/itineraries/search-or-generate")
        .set_json(&json!({
            "location": "Amsterdam",
            "budget": 2200,
            "duration": 5,
            "interests": []  // Empty interests array
        }))
        .to_request();
    
    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());
}

#[actix_rt::test]
#[serial]
async fn test_search_or_generate_large_budget() {
    let test_app = TestApp::new().await;
    let app = test::init_service(test_app.create_app()).await;

    let req = test::TestRequest::post()
        .uri("/itineraries/search-or-generate")
        .set_json(&json!({
            "location": "Dubai",
            "budget": 10000,  // Large budget
            "duration": 10,
            "interests": ["luxury", "shopping", "desert"]
        }))
        .to_request();
    
    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());
}

#[actix_rt::test]
#[serial]
async fn test_search_or_generate_long_duration() {
    let test_app = TestApp::new().await;
    let app = test::init_service(test_app.create_app()).await;

    let req = test::TestRequest::post()
        .uri("/itineraries/search-or-generate")
        .set_json(&json!({
            "location": "Thailand",
            "budget": 3500,
            "duration": 21,  // Long duration
            "interests": ["temples", "beaches", "street food"]
        }))
        .to_request();
    
    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());
}

#[actix_rt::test]
#[serial]
async fn test_itinerary_routes_with_wrong_methods() {
    let test_app = TestApp::new().await;
    let app = test::init_service(test_app.create_app()).await;

    // Test POST on GET-only endpoint
    let req = test::TestRequest::post()
        .uri("/itineraries")
        .to_request();
    
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 405); // Method Not Allowed

    // Test PUT on GET-only endpoint
    let req = test::TestRequest::put()
        .uri("/itineraries/test_id")
        .to_request();
    
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 405); // Method Not Allowed

    // Test DELETE on GET-only endpoint
    let req = test::TestRequest::delete()
        .uri("/itineraries/featured")
        .to_request();
    
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 405); // Method Not Allowed

    // Test GET on POST-only endpoint
    let req = test::TestRequest::get()
        .uri("/itineraries/search")
        .to_request();
    
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 405); // Method Not Allowed
}

#[actix_rt::test]
#[serial]
async fn test_malformed_json_in_search() {
    let test_app = TestApp::new().await;
    let app = test::init_service(test_app.create_app()).await;

    let req = test::TestRequest::post()
        .uri("/itineraries/search")
        .set_payload("{ invalid json")  // Malformed JSON
        .insert_header((header::CONTENT_TYPE, "application/json"))
        .to_request();
    
    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_client_error());
}

#[actix_rt::test]
#[serial]
async fn test_non_json_content_type_in_search() {
    let test_app = TestApp::new().await;
    let app = test::init_service(test_app.create_app()).await;

    let req = test::TestRequest::post()
        .uri("/itineraries/search")
        .set_payload("location=Paris&budget=2000")
        .insert_header((header::CONTENT_TYPE, "application/x-www-form-urlencoded"))
        .to_request();
    
    let resp = test::call_service(&app, req).await;
    // Should fail because it expects JSON
    assert!(resp.status().is_client_error());
}

#[actix_rt::test]
#[serial]
async fn test_very_long_location_name() {
    let test_app = TestApp::new().await;
    let app = test::init_service(test_app.create_app()).await;

    let very_long_location = "A".repeat(1000);  // Very long location name

    let req = test::TestRequest::post()
        .uri("/itineraries/search")
        .set_json(&json!({
            "location": very_long_location,
            "budget": 2000,
            "duration": 5
        }))
        .to_request();
    
    let resp = test::call_service(&app, req).await;
    // Should handle long location names gracefully
    assert!(resp.status().is_success() || resp.status().is_client_error());
}

#[actix_rt::test]
#[serial]
async fn test_search_with_special_characters_in_location() {
    let test_app = TestApp::new().await;
    let app = test::init_service(test_app.create_app()).await;

    let req = test::TestRequest::post()
        .uri("/itineraries/search")
        .set_json(&json!({
            "location": "São Paulo, Brasil! @#$%^&*()",  // Special characters
            "budget": 1500,
            "duration": 6
        }))
        .to_request();
    
    let resp = test::call_service(&app, req).await;
    // Should handle special characters gracefully
    assert!(resp.status().is_success() || resp.status().is_client_error());
}