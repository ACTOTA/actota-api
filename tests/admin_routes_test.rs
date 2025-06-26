mod common;

use actix_web::{test, http::header};
use serde_json::json;
use serial_test::serial;

use common::{TestApp, get_test_user_id};

async fn create_admin_jwt_token() -> String {
    // In a real implementation, this would create a valid JWT token with admin role
    // For testing purposes, we'll use a mock admin token
    "Bearer admin_jwt_token".to_string()
}

async fn create_user_jwt_token() -> String {
    // In a real implementation, this would create a valid JWT token with user role
    // For testing purposes, we'll use a mock user token
    "Bearer user_jwt_token".to_string()
}

#[actix_rt::test]
#[serial]
async fn test_list_users_without_auth() {
    let test_app = TestApp::new().await;
    let app = test::init_service(test_app.create_app()).await;

    let req = test::TestRequest::get()
        .uri("/admin/users")
        .to_request();
    
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 401);
}

#[actix_rt::test]
#[serial]
async fn test_list_users_without_admin_role() {
    let test_app = TestApp::new().await;
    let app = test::init_service(test_app.create_app()).await;

    let user_token = create_user_jwt_token().await;

    let req = test::TestRequest::get()
        .uri("/admin/users")
        .insert_header((header::AUTHORIZATION, user_token))
        .to_request();
    
    let resp = test::call_service(&app, req).await;
    // Should fail because user doesn't have admin role
    assert!(resp.status() == 403 || resp.status() == 401);
}

#[actix_rt::test]
#[serial]
async fn test_update_user_role_without_auth() {
    let test_app = TestApp::new().await;
    let app = test::init_service(test_app.create_app()).await;

    let user_id = get_test_user_id();

    let req = test::TestRequest::put()
        .uri(&format!("/admin/users/{}/role", user_id))
        .set_json(&json!({
            "role": "Admin"
        }))
        .to_request();
    
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 401);
}

#[actix_rt::test]
#[serial]
async fn test_update_user_role_without_admin_role() {
    let test_app = TestApp::new().await;
    let app = test::init_service(test_app.create_app()).await;

    let user_token = create_user_jwt_token().await;
    let user_id = get_test_user_id();

    let req = test::TestRequest::put()
        .uri(&format!("/admin/users/{}/role", user_id))
        .insert_header((header::AUTHORIZATION, user_token))
        .set_json(&json!({
            "role": "Admin"
        }))
        .to_request();
    
    let resp = test::call_service(&app, req).await;
    // Should fail because user doesn't have admin role
    assert!(resp.status() == 403 || resp.status() == 401);
}

#[actix_rt::test]
#[serial]
async fn test_update_user_role_invalid_role() {
    let test_app = TestApp::new().await;
    let app = test::init_service(test_app.create_app()).await;

    let admin_token = create_admin_jwt_token().await;
    let user_id = get_test_user_id();

    let req = test::TestRequest::put()
        .uri(&format!("/admin/users/{}/role", user_id))
        .insert_header((header::AUTHORIZATION, admin_token))
        .set_json(&json!({
            "role": "InvalidRole"
        }))
        .to_request();
    
    let resp = test::call_service(&app, req).await;
    // Should fail with invalid role
    assert!(resp.status().is_client_error() || resp.status().is_server_error());
}

#[actix_rt::test]
#[serial]
async fn test_update_user_role_missing_role() {
    let test_app = TestApp::new().await;
    let app = test::init_service(test_app.create_app()).await;

    let admin_token = create_admin_jwt_token().await;
    let user_id = get_test_user_id();

    let req = test::TestRequest::put()
        .uri(&format!("/admin/users/{}/role", user_id))
        .insert_header((header::AUTHORIZATION, admin_token))
        .set_json(&json!({}))  // Missing role field
        .to_request();
    
    let resp = test::call_service(&app, req).await;
    // Should fail with missing role
    assert!(resp.status().is_client_error() || resp.status().is_server_error());
}

#[actix_rt::test]
#[serial]
async fn test_update_user_role_nonexistent_user() {
    let test_app = TestApp::new().await;
    let app = test::init_service(test_app.create_app()).await;

    let admin_token = create_admin_jwt_token().await;

    let req = test::TestRequest::put()
        .uri("/admin/users/nonexistent_user_id/role")
        .insert_header((header::AUTHORIZATION, admin_token))
        .set_json(&json!({
            "role": "User"
        }))
        .to_request();
    
    let resp = test::call_service(&app, req).await;
    // Should fail with user not found
    assert!(resp.status() == 404 || resp.status().is_server_error());
}

#[actix_rt::test]
#[serial]
async fn test_add_featured_itinerary_without_auth() {
    let test_app = TestApp::new().await;
    let app = test::init_service(test_app.create_app()).await;

    let req = test::TestRequest::post()
        .uri("/admin/itineraries/featured/add")
        .set_json(&json!({
            "itinerary_id": "test_itinerary_123"
        }))
        .to_request();
    
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 401);
}

#[actix_rt::test]
#[serial]
async fn test_add_featured_itinerary_without_admin_role() {
    let test_app = TestApp::new().await;
    let app = test::init_service(test_app.create_app()).await;

    let user_token = create_user_jwt_token().await;

    let req = test::TestRequest::post()
        .uri("/admin/itineraries/featured/add")
        .insert_header((header::AUTHORIZATION, user_token))
        .set_json(&json!({
            "itinerary_id": "test_itinerary_123"
        }))
        .to_request();
    
    let resp = test::call_service(&app, req).await;
    // Should fail because user doesn't have admin role
    assert!(resp.status() == 403 || resp.status() == 401);
}

#[actix_rt::test]
#[serial]
async fn test_add_featured_itinerary_missing_itinerary_id() {
    let test_app = TestApp::new().await;
    let app = test::init_service(test_app.create_app()).await;

    let admin_token = create_admin_jwt_token().await;

    let req = test::TestRequest::post()
        .uri("/admin/itineraries/featured/add")
        .insert_header((header::AUTHORIZATION, admin_token))
        .set_json(&json!({}))  // Missing itinerary_id
        .to_request();
    
    let resp = test::call_service(&app, req).await;
    // Should fail with missing itinerary_id
    assert!(resp.status().is_client_error() || resp.status().is_server_error());
}

#[actix_rt::test]
#[serial]
async fn test_add_featured_itinerary_nonexistent_itinerary() {
    let test_app = TestApp::new().await;
    let app = test::init_service(test_app.create_app()).await;

    let admin_token = create_admin_jwt_token().await;

    let req = test::TestRequest::post()
        .uri("/admin/itineraries/featured/add")
        .insert_header((header::AUTHORIZATION, admin_token))
        .set_json(&json!({
            "itinerary_id": "nonexistent_itinerary_id"
        }))
        .to_request();
    
    let resp = test::call_service(&app, req).await;
    // Should fail with itinerary not found
    assert!(resp.status() == 404 || resp.status().is_server_error());
}

#[actix_rt::test]
#[serial]
async fn test_update_itinerary_images_without_auth() {
    let test_app = TestApp::new().await;
    let app = test::init_service(test_app.create_app()).await;

    let itinerary_id = "test_itinerary_123";

    let req = test::TestRequest::put()
        .uri(&format!("/admin/itineraries/{}/images", itinerary_id))
        .set_json(&json!({
            "images": ["image1.jpg", "image2.jpg"]
        }))
        .to_request();
    
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 401);
}

#[actix_rt::test]
#[serial]
async fn test_update_itinerary_images_without_admin_role() {
    let test_app = TestApp::new().await;
    let app = test::init_service(test_app.create_app()).await;

    let user_token = create_user_jwt_token().await;
    let itinerary_id = "test_itinerary_123";

    let req = test::TestRequest::put()
        .uri(&format!("/admin/itineraries/{}/images", itinerary_id))
        .insert_header((header::AUTHORIZATION, user_token))
        .set_json(&json!({
            "images": ["image1.jpg", "image2.jpg"]
        }))
        .to_request();
    
    let resp = test::call_service(&app, req).await;
    // Should fail because user doesn't have admin role
    assert!(resp.status() == 403 || resp.status() == 401);
}

#[actix_rt::test]
#[serial]
async fn test_update_itinerary_images_missing_images() {
    let test_app = TestApp::new().await;
    let app = test::init_service(test_app.create_app()).await;

    let admin_token = create_admin_jwt_token().await;
    let itinerary_id = "test_itinerary_123";

    let req = test::TestRequest::put()
        .uri(&format!("/admin/itineraries/{}/images", itinerary_id))
        .insert_header((header::AUTHORIZATION, admin_token))
        .set_json(&json!({}))  // Missing images field
        .to_request();
    
    let resp = test::call_service(&app, req).await;
    // Should fail with missing images
    assert!(resp.status().is_client_error() || resp.status().is_server_error());
}

#[actix_rt::test]
#[serial]
async fn test_update_itinerary_images_nonexistent_itinerary() {
    let test_app = TestApp::new().await;
    let app = test::init_service(test_app.create_app()).await;

    let admin_token = create_admin_jwt_token().await;

    let req = test::TestRequest::put()
        .uri("/admin/itineraries/nonexistent_itinerary_id/images")
        .insert_header((header::AUTHORIZATION, admin_token))
        .set_json(&json!({
            "images": ["image1.jpg", "image2.jpg"]
        }))
        .to_request();
    
    let resp = test::call_service(&app, req).await;
    // Should fail with itinerary not found
    assert!(resp.status() == 404 || resp.status().is_server_error());
}

#[actix_rt::test]
#[serial]
async fn test_admin_routes_with_wrong_http_methods() {
    let test_app = TestApp::new().await;
    let app = test::init_service(test_app.create_app()).await;

    // Test POST on GET-only endpoint
    let req = test::TestRequest::post()
        .uri("/admin/users")
        .to_request();
    
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 405); // Method Not Allowed

    // Test GET on PUT-only endpoint
    let req = test::TestRequest::get()
        .uri("/admin/users/test_user/role")
        .to_request();
    
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 405); // Method Not Allowed

    // Test DELETE on POST-only endpoint
    let req = test::TestRequest::delete()
        .uri("/admin/itineraries/featured/add")
        .to_request();
    
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 405); // Method Not Allowed
}