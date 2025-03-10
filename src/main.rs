use std::{env, path::PathBuf};

use actix_cors::Cors;
use actix_web::{middleware::Logger, web, App, HttpRequest, HttpResponse, HttpServer, Responder};
use env_logger::Env;

mod db;
mod middleware;
mod models;
mod routes;
mod services;

// General request diagnostic endpoint
async fn request_info(req: HttpRequest) -> impl Responder {
    let protocol = req.connection_info().scheme().to_string();
    let version = format!("{:?}", req.version());

    let headers = req
        .headers()
        .iter()
        .map(|(name, value)| format!("{}: {:?}", name, value))
        .collect::<Vec<String>>()
        .join("\n");

    HttpResponse::Ok().content_type("text/plain").body(format!(
        "Protocol: {}\nHTTP Version: {}\n\nHeaders:\n{}",
        protocol, version, headers
    ))
}

#[cfg(debug_assertions)]
fn setup_credentials() {
    println!("Setting up Google Cloud credentials");

    // Check if credentials are already set in the environment
    if let Ok(existing_creds) = env::var("GOOGLE_APPLICATION_CREDENTIALS") {
        println!(
            "Using Google credentials from environment variable: {}",
            existing_creds
        );
        return;
    }

    // Fall back to file-based credentials
    let credentials_path = PathBuf::from("credentials/service-account.json");
    if credentials_path.exists() {
        println!(
            "Using Google credentials from file: {}",
            credentials_path.display()
        );
        env::set_var(
            "GOOGLE_APPLICATION_CREDENTIALS",
            credentials_path.to_str().unwrap_or_default(),
        );
    } else {
        println!("Warning: No Google credentials found in environment or file system");
        println!("Cloud Storage operations may fail without valid credentials");
    }

    println!("Credentials setup complete");
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    println!("Application starting...");

    // Setup credentials for local development
    #[cfg(debug_assertions)]
    setup_credentials();

    // Initialize logging
    env_logger::init_from_env(
        Env::default().default_filter_or("info,actix_web=debug,actix_http=debug"),
    );
    println!("Logger initialized");

    if cfg!(debug_assertions) {
        dotenv::dotenv().ok();
        println!("Loaded environment from .env file");
    } else {
        println!("Running in release mode, using environment variables from the system");
    }

    // Get port from environment or use default
    let port = std::env::var("PORT")
        .unwrap_or_else(|_| "8080".to_string())
        .parse::<u16>()
        .expect("PORT must be a valid number");

    println!("Attempting to bind to port {}", port);

    // Connect to MongoDB
    let mongo_uri = std::env::var("MONGODB_URI").expect("MONGODB_URI must be set");
    println!("Connecting to MongoDB...");
    let client = db::mongo::create_mongo_client(&mongo_uri).await;
    println!("MongoDB connection established successfully");

    // Create and configure the HTTP server (HTTP/1.1 only)
    HttpServer::new(move || {
        App::new()
            // Add middleware
            .wrap(Logger::default())
            .wrap(actix_web::middleware::DefaultHeaders::new().add(("Server", "actota-api")))
            .wrap(
                Cors::default()
                    .allow_any_origin()
                    .allow_any_method()
                    .allow_any_header()
                    .max_age(240),
            )
            // Add JSON error handling
            .app_data(web::JsonConfig::default().error_handler(|err, _req| {
                let error_message = format!("JSON error: {}", err);
                eprintln!("{}", error_message);
                actix_web::error::InternalError::from_response(
                    err,
                    HttpResponse::BadRequest()
                        .content_type("application/json")
                        .body(format!(r#"{{"error":"{}"}}"#, error_message)),
                )
                .into()
            }))
            // Add diagnostic endpoints
            .route("/health", web::get().to(routes::health::health_check))
            .route("/request-info", web::get().to(request_info))
            .route(
                "/",
                web::get().to(|| async {
                    HttpResponse::Ok()
                        .content_type("text/plain")
                        .body("ACTOTA API is running")
                }),
            )
            // Share MongoDB client with all routes
            .app_data(web::Data::new(client.clone()))
            // Add API routes
            .service(
                web::scope("/api")
                    // Public routes
                    .service(
                        web::scope("/auth")
                            .route("/signup", web::post().to(routes::account::auth::signup))
                            .route("/signin", web::post().to(routes::account::auth::signin))
                            .route(
                                "/google",
                                web::get().to(routes::account::google_auth::google_auth_init),
                            )
                            .route(
                                "/google/callback",
                                web::get().to(routes::account::google_auth::google_auth_callback),
                            )
                            .route(
                                "/facebook",
                                web::get().to(routes::account::facebook_auth::facebook_auth_init),
                            )
                            .route(
                                "/facebook/callback",
                                web::get()
                                    .to(routes::account::facebook_auth::facebook_auth_callback),
                            )
                            .service(web::scope("").wrap(middleware::auth::AuthMiddleware).route(
                                "/session",
                                web::get().to(routes::account::auth::user_session),
                            )),
                    )
                    .service(
                        // Protected routes
                        web::scope("/account")
                            .wrap(middleware::auth::AuthMiddleware)
                            .route(
                                "/{id}",
                                web::get()
                                    .to(routes::account::account_info::get_personal_information),
                            )
                            .route(
                                "/{id}",
                                web::get()
                                    .to(routes::account::account_info::update_personal_information),
                            )
                            .route(
                                "/{id}/favorites",
                                web::get().to(routes::account::favorites::get_favorites),
                            )
                            .route(
                                "/{id}/favorites/{itinerary_id}",
                                web::post().to(routes::account::favorites::add_favorite),
                            )
                            .route(
                                "/{id}/favorites/{itinerary_id}",
                                web::delete().to(routes::account::favorites::remove_favorite),
                            )
                            .route(
                                "/{id}/payment-methods",
                                web::get()
                                    .to(routes::account::payment_methods::get_payment_methods),
                            )
                            .route(
                                "/{id}/payment-methods",
                                web::post()
                                    .to(routes::account::payment_methods::add_payment_method),
                            )
                            .route(
                                "/{id}/customer",
                                web::post()
                                    .to(routes::account::payment_methods::get_or_create_customer),
                            )
                            .route(
                                "/{id}/attach-payment-method",
                                web::post()
                                    .to(routes::account::payment_methods::attach_payment_method),
                            )
                            .route(
                                "/{id}/detach-payment-method",
                                web::post()
                                    .to(routes::account::payment_methods::detach_payment_method),
                            ),
                    )
                    .service(
                        web::scope("")
                            .service(
                                web::scope("/newsletter")
                                    .route(
                                        "/subscribe",
                                        web::post().to(routes::account::auth::newsletter_subscribe),
                                    )
                                    .route(
                                        "/unsubscribe",
                                        web::put()
                                            .to(routes::account::auth::newsletter_unsubscribe),
                                    ),
                            )
                            .route("/locations", web::get().to(routes::location::get_locations))
                            .route("/lodging", web::get().to(routes::lodging::get_lodging))
                            .route(
                                "/activities",
                                web::get().to(routes::activity::get_activities),
                            )
                            .service(
                                web::scope("/itineraries")
                                    // Public routes
                                    .route(
                                        "/featured",
                                        web::get().to(routes::featured_vacation::get_all),
                                    )
                                    .route("", web::get().to(routes::itinerary::get_all))
                                    // Protected routes
                                    .service(
                                        web::scope("")
                                            .wrap(middleware::auth::AuthMiddleware)
                                            .route(
                                                "/featured/add",
                                                web::post().to(routes::featured_vacation::add),
                                            )
                                            .route(
                                                "/find",
                                                web::post().to(routes::dream_vacation::find),
                                            )
                                            .route(
                                                "/{id}",
                                                web::get().to(routes::itinerary::get_by_id),
                                            ),
                                    ),
                            ),
                    ),
            )
    })
    // HTTP/1.1 configuration
    .bind(("0.0.0.0", port))?
    .server_hostname("actota-api") // Set a server hostname
    // Workers default to number of physical CPU cores (better for cloud environments)
    .keep_alive(std::time::Duration::from_secs(75)) // Set an appropriate keep-alive timeout
    .client_request_timeout(std::time::Duration::from_secs(60)) // Set request timeout
    .backlog(1024) // Increase the connection backlog for better performance under load
    .run()
    .await
}
