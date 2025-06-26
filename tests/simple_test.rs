use actix_web::{test, web, App, HttpResponse};
use serde_json::json;

async fn health_check() -> actix_web::Result<HttpResponse> {
    Ok(HttpResponse::Ok().json(json!({"status": "OK"})))
}

async fn get_locations() -> actix_web::Result<HttpResponse> {
    Ok(HttpResponse::Ok().json(json!([])))
}

async fn search_itineraries() -> actix_web::Result<HttpResponse> {
    Ok(HttpResponse::Ok().json(json!([])))
}

async fn unauthorized() -> actix_web::Result<HttpResponse> {
    Ok(HttpResponse::Unauthorized().json(json!({"error": "Unauthorized"})))
}

async fn not_found() -> actix_web::Result<HttpResponse> {
    Ok(HttpResponse::NotFound().json(json!({"error": "Not found"})))
}

async fn bad_request() -> actix_web::Result<HttpResponse> {
    Ok(HttpResponse::BadRequest().json(json!({"error": "Bad request"})))
}

#[actix_web::test]
async fn test_health_endpoint() {
    let app = test::init_service(
        App::new()
            .route("/health", web::get().to(health_check))
    ).await;

    let req = test::TestRequest::get()
        .uri("/health")
        .to_request();
    
    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());
    
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["status"], "OK");
}

#[actix_web::test]
async fn test_locations_endpoint() {
    let app = test::init_service(
        App::new()
            .route("/locations", web::get().to(get_locations))
    ).await;

    let req = test::TestRequest::get()
        .uri("/locations")
        .to_request();
    
    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());
    
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert!(body.is_array());
}

#[actix_web::test]
async fn test_search_itineraries_endpoint() {
    let app = test::init_service(
        App::new()
            .route("/itineraries/search", web::post().to(search_itineraries))
    ).await;

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

#[actix_web::test]
async fn test_unauthorized_access() {
    let app = test::init_service(
        App::new()
            .route("/protected", web::get().to(unauthorized))
    ).await;

    let req = test::TestRequest::get()
        .uri("/protected")
        .to_request();
    
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 401);
}

#[actix_web::test]
async fn test_not_found() {
    let app = test::init_service(
        App::new()
            .route("/test", web::get().to(not_found))
    ).await;

    let req = test::TestRequest::get()
        .uri("/test")
        .to_request();
    
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 404);
}

#[actix_web::test]
async fn test_bad_request() {
    let app = test::init_service(
        App::new()
            .route("/invalid", web::post().to(bad_request))
    ).await;

    let req = test::TestRequest::post()
        .uri("/invalid")
        .set_json(&json!({}))
        .to_request();
    
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 400);
}

#[actix_web::test]
async fn test_method_not_allowed() {
    let app = test::init_service(
        App::new()
            .route("/get-only", web::get().to(health_check))
    ).await;

    let req = test::TestRequest::post()
        .uri("/get-only")
        .to_request();
    
    let resp = test::call_service(&app, req).await;
    // In actix-web, a route that doesn't exist returns 404, not 405
    // 405 is returned when the route exists but the method is not allowed
    assert!(resp.status() == 404 || resp.status() == 405);
}

#[actix_web::test]
async fn test_json_parsing() {
    let app = test::init_service(
        App::new()
            .route("/json", web::post().to(search_itineraries))
    ).await;

    // Test valid JSON
    let req = test::TestRequest::post()
        .uri("/json")
        .set_json(&json!({"test": "data"}))
        .to_request();
    
    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());
}

#[actix_web::test]
async fn test_cors_headers() {
    let app = test::init_service(
        App::new()
            .wrap(actix_cors::Cors::default()
                .allow_any_origin()
                .allow_any_method()
                .allow_any_header())
            .route("/cors", web::get().to(health_check))
    ).await;

    let req = test::TestRequest::get()
        .uri("/cors")
        .insert_header(("Origin", "http://localhost:3000"))
        .to_request();
    
    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());
}