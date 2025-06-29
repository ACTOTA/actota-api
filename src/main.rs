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
    } else {
        println!("No explicit Google credentials found. Using Application Default Credentials.");
    }
}

#[cfg(not(debug_assertions))]
fn setup_credentials() {
    println!("Setting up Google Cloud credentials for production");

    // Check if we're running in Cloud Run
    let is_cloud_run = env::var("K_SERVICE").is_ok();

    if is_cloud_run {
        println!("Detected Cloud Run environment - using Application Default Credentials");
        // When running in Cloud Run, the google-cloud-storage crate
        // will automatically use the service account attached to the service
    } else {
        println!(
            "Not running in Cloud Run - will try to use local Application Default Credentials"
        );
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
            // API Routes - organized by domain
            
            // Authentication routes
            .service(
                web::scope("/auth")
                            // Public auth routes (no authentication required)
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
                            // Protected auth routes (require authentication)
                            .route(
                                "/session",
                                web::get()
                                    .to(routes::account::auth::user_session)
                                    .wrap(middleware::auth::AuthMiddleware),
                            ),
            )
            
            // Payment routes (protected)
            .service(
                web::scope("/payment")
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
            
            // Account routes (protected)
            .service(
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
                                "/{id}/bookings/{booking_id}",
                                web::get().to(routes::account::bookings::get_booking_by_id),
                            )
                            .route(
                                "/{id}/bookings/itinerary/{itinerary_id}",
                                web::get().to(routes::account::bookings::get_booking),
                            )
                            .route(
                                "/{id}/bookings/itinerary/{itinerary_id}",
                                web::post().to(routes::account::bookings::add_booking),
                            )
                            .route(
                                "/{id}/bookings/itinerary/{itinerary_id}",
                                web::delete().to(routes::account::bookings::remove_booking),
                            )
                            .route(
                                "/{id}/bookings/itinerary/{itinerary_id}/payment",
                                web::put().to(routes::account::bookings::update_booking_payment),
                            )
                            .route(
                                "/{id}/bookings/itinerary/{itinerary_id}/with-payment",
                                web::post().to(routes::account::bookings::add_booking_with_payment),
                            )
                            .route(
                                "/{id}/bookings/{booking_id}/cancel",
                                web::post().to(routes::account::bookings::cancel_booking_with_refund),
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
                                "/{id}/transactions",
                                web::get().to(routes::account::transactions::get_transactions),
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
                            )
                            .route(
                                "/{id}/payment-methods/attach",
                                web::post()
                                    .to(routes::account::payment_methods::attach_payment_method),
                            )
                            .route(
                                "/{id}/payment-methods/detach",
                                web::post() // Using post to send data in body
                                    .to(routes::account::payment_methods::detach_payment_method),
                            )
                            .route(
                                "/{id}/update-customer-id",
                                web::post()
                                    .to(routes::account::payment_methods_update::update_customer_id),
                            )
                            .route(
                                "/{id}/profile-picture",
                                web::post()
                                    .to(routes::account::account_info::upload_profile_pic),
                            )
                            .service(
                                web::scope("/{id}/email-verifications")
                                    .route("", web::post().to(routes::account::email_verification::create_user_email_verification))
                                    .route("", web::get().to(routes::account::email_verification::get_user_email_verifications))
                                    .route("/{verification_id}", web::put().to(routes::account::email_verification::verify_user_email_code))
                            ),
            )
            
            // Admin routes (protected with role check)
            .service(
                web::scope("/admin")
                            .wrap(middleware::role_auth::RequireRole::new(models::account::UserRole::Admin))
                            .wrap(middleware::auth::AuthMiddleware)
                            .service(
                                web::scope("/users")
                                    .route("", web::get().to(routes::account::role_management::list_users_with_roles))
                                    .route("/{id}/role", web::put().to(routes::account::role_management::update_user_role))
                            )
                            .service(
                                web::scope("/itineraries")
                                    .route(
                                        "/featured/add",
                                        web::post().to(routes::featured_vacation::add),
                                    )
                                    .service(
                                        web::scope("/{id}")
                                            .route("/images",
                                                web::put().to(routes::featured_vacation::update_itinerary_images)
                                            )
                                    )
                            )
            )
            
            // Newsletter routes
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
            
            // Email verification routes (public for signup)
            .service(
                web::scope("/email-verifications")
                                    .route("", web::post().to(routes::account::email_verification::create_signup_email_verification))
                                    .route("/{id}", web::put().to(routes::account::email_verification::verify_signup_email_code))
            )
            
            // Public content routes
            .route("/locations", web::get().to(routes::location::get_locations))
            .route("/lodging", web::get().to(routes::lodging::get_lodging))
            .route("/activities", web::get().to(routes::activity::get_activities))
            
            // Itinerary routes
            .service(
                web::scope("/itineraries")
                                    // Public routes
                                    // This route is to be removed
                                    // .route(
                                    //     "/featured",
                                    //     web::get().to(routes::featured_vacation::get_all),
                                    // )
                                    // Get all itineraries
                                    .route("", web::get().to(routes::itinerary::get_all))
                                    // Search itineraries with filters
                                    .route("/search", web::post().to(routes::itinerary::search_itineraries_endpoint))
                                    // Search with generation fallback
                                    .route("/search-or-generate", web::post().to(routes::itinerary::search_or_generate))
                                    // Public route for getting itinerary by ID
                                    .route("/{id}", web::get().to(routes::itinerary::get_by_id))
                                    // Protected routes
                                    .service(
                                        web::scope("")
                                            .wrap(middleware::auth::AuthMiddleware)
                                    
                                            .route(
                                                "/find",
                                                web::post().to(routes::dream_vacation::find),
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
