use std::{env, path::PathBuf, sync::Arc};

use actix_cors::Cors;
use actix_web::{middleware::Logger, web, App, HttpRequest, HttpResponse, HttpServer, Responder};
use env_logger::Env;
use routes::payment::{handle_stripe_webhook, StripeConfig};

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

// Setup credentials for local development
#[cfg(debug_assertions)]
fn setup_credentials() {
    println!("Setting up Google Cloud credentials for development");

    // Check if credentials are already set in the environment
    if let Ok(existing_creds) = env::var("GOOGLE_APPLICATION_CREDENTIALS") {
        println!(
            "Using Google credentials from environment variable: {}",
            existing_creds
        );

        // For cloud-storage crate compatibility, also set SERVICE_ACCOUNT_JSON
        // if the credentials file exists and can be read
        if let Ok(creds_content) = std::fs::read_to_string(&existing_creds) {
            env::set_var("SERVICE_ACCOUNT_JSON", creds_content);
            println!("Set SERVICE_ACCOUNT_JSON from credentials file");
        }

        return;
    }

    // Fall back to file-based credentials for local development only
    let credentials_path = PathBuf::from("credentials/service-account.json");
    if credentials_path.exists() {
        println!(
            "Using Google credentials from file: {}",
            credentials_path.display()
        );

        // Set path-based credential variable
        env::set_var(
            "GOOGLE_APPLICATION_CREDENTIALS",
            credentials_path.to_str().unwrap_or_default(),
        );

        // For cloud-storage crate compatibility, also set SERVICE_ACCOUNT_JSON
        if let Ok(creds_content) = std::fs::read_to_string(&credentials_path) {
            env::set_var("SERVICE_ACCOUNT_JSON", creds_content);
            println!("Set SERVICE_ACCOUNT_JSON from credentials file");
        }
    } else {
        println!("No explicit Google credentials found. Using default service account.");
        // Set empty SERVICE_ACCOUNT_JSON to bypass file reading in cloud-storage
        env::set_var("SERVICE_ACCOUNT_JSON", "{}");
    }
}

// For Cloud Run and other production environments
#[cfg(not(debug_assertions))]
fn setup_credentials() {
    println!("Setting up Google Cloud credentials for production");

    // Check if SERVICE_ACCOUNT_JSON is already explicitly set
    if env::var("SERVICE_ACCOUNT_JSON").is_ok() {
        println!("Using SERVICE_ACCOUNT_JSON from environment variable");
        return;
    }

    // For ADC to work in the cloud_storage crate:
    // 1. Set an empty SERVICE_ACCOUNT_JSON to prevent file lookups
    // 2. Set GOOGLE_APPLICATION_CREDENTIALS to a special value
    println!("No explicit SERVICE_ACCOUNT_JSON found. Configuring for ADC...");
    env::set_var("SERVICE_ACCOUNT_JSON", "{}");

    // Only set GOOGLE_APPLICATION_CREDENTIALS if it's not already set
    // This preserves any ADC configuration already in place
    if env::var("GOOGLE_APPLICATION_CREDENTIALS").is_err() {
        println!("Setting GOOGLE_APPLICATION_CREDENTIALS to 'use-adc'");
        env::set_var("GOOGLE_APPLICATION_CREDENTIALS", "use-adc");
    } else {
        println!("Using existing GOOGLE_APPLICATION_CREDENTIALS value");
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    println!("Application starting...");

    // Setup credentials for both development and production
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

    // Initialize the Stripe client
    println!("Initializing Stripe client...");
    let stripe_secret_key =
        std::env::var("STRIPE_SECRET_KEY").expect("STRIPE_SECRET_KEY must be set");
    let stripe_client = Arc::new(stripe::Client::new(stripe_secret_key));
    let stripe_data = web::Data::new(stripe_client);
    println!("Stripe client initialized successfully");

    // Initialize the Stripe configuration for webhook
    let stripe_config = StripeConfig {
        webhook_secret: std::env::var("STRIPE_WEBHOOK_SECRET")
            .expect("STRIPE_WEBHOOK_SECRET must be set"),
    };

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
            .app_data(stripe_data.clone())
            .app_data(web::Data::new(client.clone()))
            .app_data(web::Data::new(stripe_config.clone()))
            .route("/stripe/webhook", web::post().to(handle_stripe_webhook))
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
                        web::scope("payment")
                            .wrap(middleware::auth::AuthMiddleware)
                            .route(
                                "/payment-intent",
                                web::post().to(routes::payment::create_payment_intent),
                            )
                            .route(
                                "/capture-payment",
                                web::post().to(routes::payment::capture_payment),
                            ),
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
                                web::put()
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
                                "/{id}/bookings",
                                web::get().to(routes::account::bookings::get_all_bookings),
                            )
                            .route(
                                "/{id}/bookings/{itinerary_id}",
                                web::get().to(routes::account::bookings::get_booking),
                            )
                            .route(
                                "/{id}/bookings/{itinerary_id}",
                                web::post().to(routes::account::bookings::add_booking),
                            )
                            .route(
                                "/{id}/bookings/{itinerary_id}",
                                web::delete().to(routes::account::bookings::remove_booking),
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
                                "/{id}/payment-methods/{pm_id}",
                                web::delete()
                                    .to(routes::account::payment_methods::remove_payment_method),
                            ), // .route(
                               //     "/{id}/attach-payment-method",
                               //     web::post()
                               //         .to(routes::account::payment_methods::attach_payment_method),
                               // )
                               // .route(
                               //     "/{id}/detach-payment-method",
                               //     web::post()
                               //         .to(routes::account::payment_methods::detach_payment_method),
                               // ),
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
                                    // Get all itineraries or search with filters
                                    .route("", web::get().to(routes::itinerary::get_all))
                                    .route("", web::post().to(routes::itinerary::get_all))
                                    // Public route for getting itinerary by ID
                                    .route("/{id}", web::get().to(routes::itinerary::get_by_id))
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
                                            ),
                                    ),
                            ),
                    ),
            )
    })
    // HTTP/1.1 configuration
    .bind(("0.0.0.0", port))?
    .server_hostname("actota-api") // Set a server hostname
    .keep_alive(std::time::Duration::from_secs(75)) // Set an appropriate keep-alive timeout
    .client_request_timeout(std::time::Duration::from_secs(60)) // Set request timeout
    .backlog(1024) // Increase the connection backlog for better performance under load
    .run()
    .await
}
