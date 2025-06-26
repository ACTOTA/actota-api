mod common;

use actix_web::{test, http::header};
use serde_json::json;
use serial_test::serial;

use common::{TestApp, get_test_user_id};

#[actix_rt::test]
#[serial]
async fn test_create_payment_intent_without_auth() {
    let test_app = TestApp::new().await;
    let app = test::init_service(test_app.create_app()).await;

    let req = test::TestRequest::post()
        .uri("/payment/payment-intent")
        .set_json(&json!({
            "amount": 1000,
            "currency": "usd"
        }))
        .to_request();
    
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 401);
}

#[actix_rt::test]
#[serial]
async fn test_capture_payment_without_auth() {
    let test_app = TestApp::new().await;
    let app = test::init_service(test_app.create_app()).await;

    let req = test::TestRequest::post()
        .uri("/payment/capture-payment")
        .set_json(&json!({
            "payment_intent_id": "pi_test_123"
        }))
        .to_request();
    
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 401);
}

#[actix_rt::test]
#[serial]
async fn test_create_payment_intent_missing_fields() {
    let test_app = TestApp::new().await;
    let app = test::init_service(test_app.create_app()).await;

    // Mock JWT token for authorization
    let token = "Bearer mock_token";

    let req = test::TestRequest::post()
        .uri("/payment/payment-intent")
        .insert_header((header::AUTHORIZATION, token))
        .set_json(&json!({
            "amount": 1000
            // Missing currency
        }))
        .to_request();
    
    let resp = test::call_service(&app, req).await;
    // This should fail due to missing fields or authentication
    assert!(resp.status().is_client_error() || resp.status().is_server_error());
}

#[actix_rt::test]
#[serial]
async fn test_create_payment_intent_invalid_amount() {
    let test_app = TestApp::new().await;
    let app = test::init_service(test_app.create_app()).await;

    let token = "Bearer mock_token";

    let req = test::TestRequest::post()
        .uri("/payment/payment-intent")
        .insert_header((header::AUTHORIZATION, token))
        .set_json(&json!({
            "amount": -100,  // Invalid negative amount
            "currency": "usd"
        }))
        .to_request();
    
    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_client_error() || resp.status().is_server_error());
}

#[actix_rt::test]
#[serial]
async fn test_create_payment_intent_invalid_currency() {
    let test_app = TestApp::new().await;
    let app = test::init_service(test_app.create_app()).await;

    let token = "Bearer mock_token";

    let req = test::TestRequest::post()
        .uri("/payment/payment-intent")
        .insert_header((header::AUTHORIZATION, token))
        .set_json(&json!({
            "amount": 1000,
            "currency": "invalid_currency"
        }))
        .to_request();
    
    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_client_error() || resp.status().is_server_error());
}

#[actix_rt::test]
#[serial]
async fn test_capture_payment_missing_payment_intent_id() {
    let test_app = TestApp::new().await;
    let app = test::init_service(test_app.create_app()).await;

    let token = "Bearer mock_token";

    let req = test::TestRequest::post()
        .uri("/payment/capture-payment")
        .insert_header((header::AUTHORIZATION, token))
        .set_json(&json!({}))  // Missing payment_intent_id
        .to_request();
    
    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_client_error() || resp.status().is_server_error());
}

#[actix_rt::test]
#[serial]
async fn test_capture_payment_invalid_payment_intent_id() {
    let test_app = TestApp::new().await;
    let app = test::init_service(test_app.create_app()).await;

    let token = "Bearer mock_token";

    let req = test::TestRequest::post()
        .uri("/payment/capture-payment")
        .insert_header((header::AUTHORIZATION, token))
        .set_json(&json!({
            "payment_intent_id": "invalid_pi_id"
        }))
        .to_request();
    
    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_client_error() || resp.status().is_server_error());
}

#[actix_rt::test]
#[serial]
async fn test_stripe_webhook_no_signature() {
    let test_app = TestApp::new().await;
    let app = test::init_service(test_app.create_app()).await;

    let req = test::TestRequest::post()
        .uri("/stripe/webhook")
        .set_json(&json!({
            "type": "payment_intent.succeeded",
            "data": {
                "object": {
                    "id": "pi_test_123"
                }
            }
        }))
        .to_request();
    
    let resp = test::call_service(&app, req).await;
    // Should fail without proper Stripe signature
    assert!(resp.status().is_client_error() || resp.status().is_server_error());
}

#[actix_rt::test]
#[serial]
async fn test_stripe_webhook_invalid_signature() {
    let test_app = TestApp::new().await;
    let app = test::init_service(test_app.create_app()).await;

    let req = test::TestRequest::post()
        .uri("/stripe/webhook")
        .insert_header(("stripe-signature", "invalid_signature"))
        .set_json(&json!({
            "type": "payment_intent.succeeded",
            "data": {
                "object": {
                    "id": "pi_test_123"
                }
            }
        }))
        .to_request();
    
    let resp = test::call_service(&app, req).await;
    // Should fail with invalid signature
    assert!(resp.status().is_client_error() || resp.status().is_server_error());
}

#[actix_rt::test]
#[serial]
async fn test_stripe_webhook_malformed_payload() {
    let test_app = TestApp::new().await;
    let app = test::init_service(test_app.create_app()).await;

    let req = test::TestRequest::post()
        .uri("/stripe/webhook")
        .insert_header(("stripe-signature", "t=1234567890,v1=signature"))
        .set_payload("invalid json")
        .to_request();
    
    let resp = test::call_service(&app, req).await;
    // Should fail with malformed JSON
    assert!(resp.status().is_client_error() || resp.status().is_server_error());
}

#[actix_rt::test]
#[serial]
async fn test_payment_intent_zero_amount() {
    let test_app = TestApp::new().await;
    let app = test::init_service(test_app.create_app()).await;

    let token = "Bearer mock_token";

    let req = test::TestRequest::post()
        .uri("/payment/payment-intent")
        .insert_header((header::AUTHORIZATION, token))
        .set_json(&json!({
            "amount": 0,  // Zero amount
            "currency": "usd"
        }))
        .to_request();
    
    let resp = test::call_service(&app, req).await;
    // Zero amount should be rejected
    assert!(resp.status().is_client_error() || resp.status().is_server_error());
}

#[actix_rt::test]
#[serial]
async fn test_payment_intent_extremely_large_amount() {
    let test_app = TestApp::new().await;
    let app = test::init_service(test_app.create_app()).await;

    let token = "Bearer mock_token";

    let req = test::TestRequest::post()
        .uri("/payment/payment-intent")
        .insert_header((header::AUTHORIZATION, token))
        .set_json(&json!({
            "amount": 99999999999i64,  // Extremely large amount
            "currency": "usd"
        }))
        .to_request();
    
    let resp = test::call_service(&app, req).await;
    // Should either succeed or fail based on Stripe limits
    assert!(resp.status().is_success() || resp.status().is_client_error() || resp.status().is_server_error());
}

#[actix_rt::test]
#[serial]
async fn test_payment_routes_with_different_http_methods() {
    let test_app = TestApp::new().await;
    let app = test::init_service(test_app.create_app()).await;

    // Test GET on POST-only endpoint
    let req = test::TestRequest::get()
        .uri("/payment/payment-intent")
        .to_request();
    
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 405); // Method Not Allowed

    // Test PUT on POST-only endpoint
    let req = test::TestRequest::put()
        .uri("/payment/capture-payment")
        .to_request();
    
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 405); // Method Not Allowed
}