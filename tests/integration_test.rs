mod common;

use actix_web::{test, web, App, HttpServer, middleware::Logger};
use actix_cors::Cors;
use serial_test::serial;
use std::sync::Arc;
use tokio::time::{sleep, Duration};

use common::{TestApp, cleanup_test_data};

#[actix_rt::test]
#[serial]
async fn test_full_api_integration() {
    let test_app = TestApp::new().await;
    
    // Clean up any existing test data
    cleanup_test_data(&test_app.client).await;
    
    let app = test::init_service(test_app.create_app()).await;

    // Test 1: Health check
    let req = test::TestRequest::get()
        .uri("/health")
        .to_request();
    
    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());
    println!("✓ Health check passed");

    // Test 2: Get all locations
    let req = test::TestRequest::get()
        .uri("/locations")
        .to_request();
    
    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());
    println!("✓ Locations endpoint passed");

    // Test 3: Get all activities
    let req = test::TestRequest::get()
        .uri("/activities")
        .to_request();
    
    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());
    println!("✓ Activities endpoint passed");

    // Test 4: Get all lodging
    let req = test::TestRequest::get()
        .uri("/lodging")
        .to_request();
    
    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());
    println!("✓ Lodging endpoint passed");

    // Test 5: Get all itineraries
    let req = test::TestRequest::get()
        .uri("/itineraries")
        .to_request();
    
    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());
    println!("✓ Itineraries endpoint passed");

    // Test 6: Search itineraries
    let req = test::TestRequest::post()
        .uri("/itineraries/search")
        .set_json(&serde_json::json!({
            "location": "Test City",
            "budget": 1000,
            "duration": 3
        }))
        .to_request();
    
    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());
    println!("✓ Itinerary search endpoint passed");

    // Test 7: Test authentication required endpoints (should fail without auth)
    let req = test::TestRequest::get()
        .uri("/auth/session")
        .to_request();
    
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 401);
    println!("✓ Authentication middleware working correctly");

    // Test 8: Test admin endpoints (should fail without admin auth)
    let req = test::TestRequest::get()
        .uri("/admin/users")
        .to_request();
    
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 401);
    println!("✓ Admin authentication middleware working correctly");

    // Test 9: Test payment endpoints (should fail without auth)
    let req = test::TestRequest::post()
        .uri("/payment/payment-intent")
        .set_json(&serde_json::json!({
            "amount": 1000,
            "currency": "usd"
        }))
        .to_request();
    
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 401);
    println!("✓ Payment authentication middleware working correctly");

    // Test 10: Test method not allowed
    let req = test::TestRequest::post()
        .uri("/health")
        .to_request();
    
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 405);
    println!("✓ HTTP method validation working correctly");

    // Clean up test data
    cleanup_test_data(&test_app.client).await;
    
    println!("\n🎉 All integration tests passed!");
}

#[actix_rt::test]
#[serial]
async fn test_cors_configuration() {
    let test_app = TestApp::new().await;
    let app = test::init_service(test_app.create_app()).await;

    // Test CORS preflight request
    let req = test::TestRequest::with_uri("/health")
        .method(actix_web::http::Method::OPTIONS)
        .insert_header(("Origin", "http://localhost:3000"))
        .insert_header(("Access-Control-Request-Method", "GET"))
        .to_request();
    
    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success() || resp.status() == 200);
    println!("✓ CORS configuration working correctly");
}

#[actix_rt::test]
#[serial]
async fn test_error_handling() {
    let test_app = TestApp::new().await;
    let app = test::init_service(test_app.create_app()).await;

    // Test 404 on non-existent route
    let req = test::TestRequest::get()
        .uri("/non-existent-route")
        .to_request();
    
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 404);
    println!("✓ 404 error handling working correctly");

    // Test malformed JSON
    let req = test::TestRequest::post()
        .uri("/itineraries/search")
        .set_payload("{ invalid json")
        .insert_header((actix_web::http::header::CONTENT_TYPE, "application/json"))
        .to_request();
    
    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_client_error());
    println!("✓ JSON parsing error handling working correctly");
}

#[actix_rt::test]
#[serial]
async fn test_database_connection() {
    let test_app = TestApp::new().await;
    
    // Test database connection by attempting to access a collection
    let db = test_app.client.database("Account");
    let collection = db.collection::<mongodb::bson::Document>("User");
    
    // This should not fail even if the collection is empty
    let result = collection.count_documents(mongodb::bson::doc! {}).await;
    assert!(result.is_ok());
    println!("✓ Database connection working correctly");
}

#[actix_rt::test]
#[serial]
async fn test_concurrent_requests() {
    let test_app = TestApp::new().await;
    let app = test::init_service(test_app.create_app()).await;

    // Test multiple concurrent requests
    let mut handles = Vec::new();
    
    for i in 0..10 {
        let app_clone = &app;
        let handle = tokio::spawn(async move {
            let req = test::TestRequest::get()
                .uri("/health")
                .to_request();
            
            let resp = test::call_service(app_clone, req).await;
            assert!(resp.status().is_success());
            i
        });
        handles.push(handle);
    }
    
    // Wait for all requests to complete
    for handle in handles {
        let _ = handle.await.unwrap();
    }
    
    println!("✓ Concurrent request handling working correctly");
}

#[actix_rt::test]
#[serial]
async fn test_route_parameter_validation() {
    let test_app = TestApp::new().await;
    let app = test::init_service(test_app.create_app()).await;

    // Test with invalid ID format
    let req = test::TestRequest::get()
        .uri("/itineraries/invalid-id-format")
        .to_request();
    
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 404);
    println!("✓ Route parameter validation working correctly");
}

#[actix_rt::test]
#[serial]
async fn test_content_type_handling() {
    let test_app = TestApp::new().await;
    let app = test::init_service(test_app.create_app()).await;

    // Test with wrong content type
    let req = test::TestRequest::post()
        .uri("/itineraries/search")
        .set_payload("location=Paris&budget=1000")
        .insert_header((actix_web::http::header::CONTENT_TYPE, "application/x-www-form-urlencoded"))
        .to_request();
    
    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_client_error());
    println!("✓ Content type validation working correctly");
}

#[actix_rt::test]
#[serial]
async fn test_large_payload_handling() {
    let test_app = TestApp::new().await;
    let app = test::init_service(test_app.create_app()).await;

    // Test with large payload
    let large_interests: Vec<String> = (0..1000).map(|i| format!("interest_{}", i)).collect();
    
    let req = test::TestRequest::post()
        .uri("/itineraries/search")
        .set_json(&serde_json::json!({
            "location": "Test City",
            "budget": 1000,
            "duration": 3,
            "interests": large_interests
        }))
        .to_request();
    
    let resp = test::call_service(&app, req).await;
    // Should either succeed or fail gracefully
    assert!(resp.status().is_success() || resp.status().is_client_error());
    println!("✓ Large payload handling working correctly");
}