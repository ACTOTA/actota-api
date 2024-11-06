use actix_web::{middleware::Logger, web, App, HttpServer};
use env_logger::Env;

mod db;
mod models;
mod routes;

const HOST: &str = "0.0.0.0";
const PORT: u16 = 8080;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::init_from_env(Env::default().default_filter_or("info"));
    // Get env vars from .env if not --release
    if cfg!(debug_assertions) {
        dotenv::dotenv().ok();
    } else {
        println!("Release mode");
    }
    // Get host and port from env vars
    let host = std::env::var("HOST").unwrap_or_else(|_| HOST.to_string());
    let port: u16 = std::env::var("PORT")
        .unwrap_or_else(|_| PORT.to_string())
        .parse()
        .unwrap_or(PORT);

    // Check mongodb connection
    let mongo_uri = std::env::var("MONGODB_URI").expect("MONGODB_URI must be set");
    println!("MongoDB URI: {}", mongo_uri);
    let client = db::mongo::create_mongo_client(&mongo_uri).await;

    HttpServer::new(move || {
        // move is necessary to move ownership of client to the closure
        App::new()
            .wrap(Logger::default())
            .app_data(web::Data::new(client.clone())) // Clone the client to pass to each route
            .service(
                web::scope("/api")
                    .route("/signup", web::post().to(routes::account::signup))
                    .route("/signin", web::post().to(routes::account::signin))
                    .service(web::scope("/accounts")),
            )
    })
    .bind((host, port))?
    .run()
    .await
}
