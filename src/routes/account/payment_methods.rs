use actix_web::{web, HttpResponse, Responder};
use mongodb::Client;
use std::sync::Arc;

use crate::middleware::auth::Claims;

pub async fn add_payment_method(data: web::Data<Arc<Client>>, claims: Claims) -> impl Responder {
    HttpResponse::Ok().body("add_payment_method")
}

