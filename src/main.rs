use std::{env, path::PathBuf};

use actix_web::{middleware::Logger, web, App, HttpServer};
use env_logger::Env;

mod db;
mod middleware;
mod models;
mod routes;

const HOST: &str = "0.0.0.0";
const PORT: u16 = 8080;

fn setup_credentials() {
    let credentials_path = PathBuf::from("credentials/service-account.json");
    env::set_var(
        "GOOGLE_APPLICATION_CREDENTIALS",
        credentials_path.to_str().unwrap(),
    );
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    setup_credentials();

    env_logger::init_from_env(Env::default().default_filter_or("info"));
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

    let mongo_uri = std::env::var("MONGODB_URI").expect("MONGODB_URI must be set");
    println!("MongoDB URI: {}", mongo_uri);
    let client = db::mongo::create_mongo_client(&mongo_uri).await;

    HttpServer::new(move || {
        App::new()
            .wrap(Logger::default())
            .app_data(web::Data::new(client.clone()))
            .service(
                web::scope("/api")
                    // Public routes
                    .service(
                        web::scope("/auth")
                            .route("/signup", web::post().to(routes::account::signup))
                            .route("/signin", web::post().to(routes::account::signin)),
                    )
                    .service(
                        web::scope("/activities")
                            .route("/get", web::get().to(routes::activity::get_activities)),
                    )
                    .service(
                        web::scope("/lodging")
                            .route("/get", web::get().to(routes::lodging::get_lodging)),
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
                                    .route("/find", web::post().to(routes::dream_vacation::find)),
                            ),
                    ),
            )
    })
    .bind((host, port))?
    .run()
    .await
}
