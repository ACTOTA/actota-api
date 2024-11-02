use actix_web::{web, App, HttpServer, Responder, middleware::Logger};
use env_logger::Env;

mod routes;
mod db;
mod models;

async fn index() -> impl Responder {
    "Hello world!"
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {

    env_logger::init_from_env(Env::default().default_filter_or("info"));
    // Get env vars from .env if not --release
    if cfg!(debug_assertions) {
        dotenv::dotenv().ok();
    } else {
        println!("Release mode");
    }
    
    // Check mongodb connection
    let mongo_uri = std::env::var("MONGODB_URI").expect("MONGODB_URI must be set"); 
    println!("MongoDB URI: {}", mongo_uri);
    let client = db::mongo::create_mongo_client(&mongo_uri).await;
    
    HttpServer::new(move || { // move is necessary to move ownership of client to the closure
        App::new().wrap(Logger::default())
            .app_data(web::Data::new(client.clone())) // Clone the client to pass to each route
            .service(
            web::scope("/api")
                .route("/index.html", web::get().to(index))
                .service(
                    web::scope("/accounts")
                    .route("/create", web::post().to(routes::account::create_account))
                ),
        )
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await

}
