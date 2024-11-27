use actix_web::{middleware::Logger, web, App, HttpServer};
use env_logger::Env;

mod db;
mod middleware;
mod models;
mod routes;

const HOST: &str = "0.0.0.0";
const PORT: u16 = 8080;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
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
            // Public routes
            .service(
                web::scope("/api/auth")
                    .route("/signup", web::post().to(routes::account::signup))
                    .route("/signin", web::post().to(routes::account::signin)),
            )
            // Public activities route (moved outside /api scope)
            .service(
                web::scope("/api/activities")
                    .route("/get", web::get().to(routes::activity::get_activities)),
            )
            .service(
                web::scope("/api/lodging")
                    .route("/get", web::get().to(routes::lodging::get_lodging)),
            )
            // Protected routes
            .service(
                web::scope("/api")
                    .wrap(middleware::auth::AuthMiddleware)
                    .service(
                        web::scope("/itineraries")
                            .route("/find", web::post().to(routes::dream_vacation::find)),
                    )
                    .service(web::scope("/accounts")),
            )
    })
    .bind((host, port))?
    .run()
    .await
}
