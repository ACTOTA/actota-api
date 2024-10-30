use actix_web::{web, App, HttpServer, Responder};
mod routes;

async fn index() -> impl Responder {
    "Hello world!"
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    HttpServer::new(|| {
        App::new().service(
            // prefixes all resources and routes attached to it...
            web::scope("/")
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
