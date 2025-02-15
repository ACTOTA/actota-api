use actix_web::{web, HttpResponse, Responder};
use bson::doc;
use mongodb::Client;
use std::sync::Arc;
use stripe::{CreateCustomer, Customer};

use crate::{
    middleware::auth::Claims, models::account::User,
    services::payment::interface::CustomerOperations,
};

pub async fn add_payment_method(
    data: web::Data<Arc<Client>>,
    input: web::Json<CreateCustomer>,
    claims: Claims,
    path: web::Path<(String,)>,
) -> impl Responder {
    let customer = input.into_inner();

    let user_id = path.into_inner().0;
    if user_id != claims.user_id {
        return HttpResponse::Forbidden().body("Forbidden");
    }

    let client = data.into_inner();
    let collection: mongodb::Collection<User> = client.database("Account").collection("User");

    let filter = doc! { "_id": claims.user_id.clone() };
    let customer_id = match collection.find_one(filter).await {
        Ok(Some(user)) => user.customer_id,
        Ok(None) => {
            // Customer not found, so create a new customer
            // let result = CustomerOperations::create_customer(&self, input).await;
            CustomerOperations::create_customer(&self, input).await
        }
        Err(_) => {
            return HttpResponse::InternalServerError().body("Failed to add payment method");
        }
    };

    return HttpResponse::Ok().body("Payment method added");
}


