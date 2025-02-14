use std::{env, path::PathBuf};

use actix_cors::Cors;
use actix_web::{middleware::Logger, web, App, HttpResponse, HttpServer};
use env_logger::Env;

mod db;
mod middleware;
mod models;
mod routes;
mod services;

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

    let port = std::env::var("PORT")
        .unwrap_or_else(|_| "8080".to_string())
        .parse::<u16>()
        .expect("PORT must be a valid number");

    println!("Attempting to bind to {}", port);

    let mongo_uri = std::env::var("MONGODB_URI").expect("MONGODB_URI must be set");
    println!("Got MongoDB URI, attempting connection...");
    let client = db::mongo::create_mongo_client(&mongo_uri).await;
    println!("MongoDB connection established");

    HttpServer::new(move || {
        App::new()
            .wrap(Logger::default())
            .wrap(actix_web::middleware::DefaultHeaders::new().add(("Server", "actix-web")))
            .wrap(
                Cors::default()
                    .allow_any_origin()
                    .allow_any_method()
                    .allow_any_header()
                    .max_age(240),
            )
            .route("/health", web::get().to(|| async { "OK" }))
            .route(
                "/",
                web::get()
                    .to(|| async { HttpResponse::Ok().content_type("text/plain").body("OK") }),
            )
            .app_data(web::Data::new(client.clone()))
            .service(
                web::scope("/api")
                    // Public routes
                    .service(
                        web::scope("/auth")
                            .route("/signup", web::post().to(routes::account::auth::signup))
                            .route("/signin", web::post().to(routes::account::auth::signin))
                            .service(web::scope("").wrap(middleware::auth::AuthMiddleware).route(
                                "/session",
                                web::get().to(routes::account::auth::user_session),
                            )),
                    )
                    .service(
                        web::scope("/account")
                            .wrap(middleware::auth::AuthMiddleware)
                            // .route(
                            //     "/{id}/profile-pic",
                            //     web::post().to(routes::account::account_info::upload_profile_pic),
                            // )
                            .route(
                                "/{id}",
                                web::get().to(routes::account::account_info::personal_information),
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
    .bind(("0.0.0.0", port))?
    .run()
    .await
}
