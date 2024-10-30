use actix_web::{post, web, HttpResponse, Responder};
use mongodb::Database;

use crate::{
    models::Account,
    services::account_service,
};

#[post("/accounts")]
pub async fn create_account(
    db: web::Data<Database>,
    account: web::Json<Account>,
) -> impl Responder {
    let accounts_collection = db::mongo::get_accounts_collection(&db);
    match account_service::create_account(&accounts_collection, &account).await {
        Ok(new_account) => HttpResponse::Created().json(new_account),
        Err(err) => {
            eprintln!("Error creating account: {:?}", err);
            // More specific error handling based on err type (e.g., duplicate key)
            HttpResponse::InternalServerError().body(format!("Error creating account: {:?}", err))
        }
    }
}
