use std::{env, path::PathBuf};

use actix_web::{middleware::Logger, web, App, HttpServer};
use env_logger::Env;

mod db;
mod middleware;
mod models;
mod routes;
mod services;

const HOST: &str = "0.0.0.0";
const PORT: u16 = 8080;

#[cfg(debug_assertions)]
fn setup_credentials() {
    println!("Credentials setup complete");

    let credentials_path = PathBuf::from("credentials/service-account.json");
    env::set_var(
        "GOOGLE_APPLICATION_CREDENTIALS",
        credentials_path.to_str().unwrap(),
    );
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    println!("Application starting...");

    #[cfg(debug_assertions)]
    setup_credentials();

    env_logger::init_from_env(Env::default().default_filter_or("info"));
    println!("Logger initialized");

    if cfg!(debug_assertions) {
        dotenv::dotenv().ok();
    } else {
        println!("Release mode");
    }

    let host = std::env::var("HOST").unwrap_or_else(|_| HOST.to_string());
    let port: u16 = std::env::var("PORT")
        .unwrap_or_else(|_| PORT.to_string())
        .parse()
        .unwrap_or(PORT);
    println!("Attempting to bind to {}:{}", host, port);

    let mongo_uri = std::env::var("MONGODB_URI").expect("MONGODB_URI must be set");
    println!("Got MongoDB URI, attempting connection...");
    let client = db::mongo::create_mongo_client(&mongo_uri).await;
    println!("MongoDB connection established");

    println!("Starting HTTP server...");

    HttpServer::new(move || {
        App::new()
            .wrap(Logger::default())
            .route("/health", web::get().to(|| async { "OK" }))
            .app_data(web::Data::new(client.clone()))
            .service(
                web::scope("/api")
                    // Public routes
                    .service(
                        web::scope("/auth")
                            .route("/signup", web::post().to(routes::account::signup))
                            .route("/signin", web::post().to(routes::account::signin))
                            .service(
                                web::scope("").wrap(middleware::auth::AuthMiddleware).route(
                                    "/session",
                                    web::get().to(routes::account::user_session),
                                ),
                            ),
                    )
                    .service(
                        web::scope("")
                            .service(
                                web::scope("/newsletter")
                                    .route(
                                        "/subscribe",
                                        web::post().to(routes::account::newsletter_subscribe),
                                    )
                                    .route(
                                        "/unsubscribe",
                                        web::put().to(routes::account::newsletter_unsubscribe),
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
                                    .route(
                                        "/featured",
                                        web::get().to(routes::featured_vacation::get_all),
                                    )
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
    .bind((host, port))?
    .run()
    .await
}
