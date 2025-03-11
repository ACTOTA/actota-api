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

// Setup credentials for production environment
#[cfg(not(debug_assertions))]
fn setup_credentials() {
    println!("Setting up Google Cloud credentials for production");

    // For cloud-storage crate compatibility in production (Cloud Run),
    // we need to handle the case where credentials are provided via ADC
    // (Application Default Credentials) rather than as a file.

    // Check if SERVICE_ACCOUNT_JSON is already set
    if let Ok(json_content) = env::var("SERVICE_ACCOUNT_JSON") {
        println!("Using SERVICE_ACCOUNT_JSON from environment variable");
        return;
    }

    // Check if GOOGLE_APPLICATION_CREDENTIALS_JSON is set (contains the actual JSON)
    if let Ok(json_content) = env::var("GOOGLE_APPLICATION_CREDENTIALS_JSON") {
        println!("Using GOOGLE_APPLICATION_CREDENTIALS_JSON content");
        env::set_var("SERVICE_ACCOUNT_JSON", json_content);
        return;
    }

    // Check if GOOGLE_APPLICATION_CREDENTIALS points to a file
    if let Ok(creds_path) = env::var("GOOGLE_APPLICATION_CREDENTIALS") {
        println!(
            "Using credentials from GOOGLE_APPLICATION_CREDENTIALS: {}",
            creds_path
        );

        // For cloud-storage crate compatibility:
        // If the file exists, read its content and set SERVICE_ACCOUNT_JSON
        match std::fs::read_to_string(&creds_path) {
            Ok(content) => {
                println!("Read credentials file successfully");
                env::set_var("SERVICE_ACCOUNT_JSON", content);
            }
            Err(e) => {
                println!("Warning: Could not read credentials file: {}", e);
                // We're in Cloud Run, so likely using ADC - set a null string to prevent file access
                env::set_var("SERVICE_ACCOUNT_JSON", "{}");
                // Also set GOOGLE_APPLICATION_CREDENTIALS to a special value
                env::set_var("GOOGLE_APPLICATION_CREDENTIALS", "use-adc");
            }
        }
        return;
    }

    // If none of the above, we're using ADC in Cloud Run
    println!("No explicit credentials found. Using Application Default Credentials (ADC)");
    env::set_var("SERVICE_ACCOUNT_JSON", "{}");
    env::set_var("GOOGLE_APPLICATION_CREDENTIALS", "use-adc");
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
    .keep_alive(std::time::Duration::from_secs(75)) // Set an appropriate keep-alive timeout
    .client_request_timeout(std::time::Duration::from_secs(60)) // Set request timeout
    .backlog(1024) // Increase the connection backlog for better performance under load
    .run()
    .await
}
