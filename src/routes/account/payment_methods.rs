use actix_web::{web, HttpResponse, Responder};
use bson::{doc, oid::ObjectId};
use mongodb::Client;
use std::{str::FromStr, sync::Arc};

use crate::{
    middleware::auth::Claims,
    models::account::User,
    services::{
        payment::interface::CustomerOperations,
        stripe::{models::customer::CustomerData, provider::StripeProvider},
    },
};

pub async fn add_payment_method(
    data: web::Data<Arc<Client>>,
    input: web::Json<CustomerData>,
    claims: Claims,
    path: web::Path<(String,)>,
) -> impl Responder {
    let customer = input.into_inner();

    let user_id = path.into_inner().0;
    if user_id != claims.user_id {
        return HttpResponse::Forbidden().body("Forbidden");
    }

    let client = data.into_inner();
    let collection: mongodb::Collection<User> = client.database("Account").collection("Users");

    let stripe_op = StripeProvider::new(std::env::var("STRIPE_SECRET_KEY").unwrap());

    let filter = doc! { "_id": ObjectId::from_str(&user_id).unwrap() };

    match collection.find_one(filter.clone()).await {
        Ok(Some(user)) => {
            if user.customer_id.is_none() {
                let cust_id = match stripe_op.create_customer(customer).await {
                    Ok(customer) => customer.id,
                    Err(_) => {
                        return HttpResponse::InternalServerError()
                            .body("Failed to create customer");
                    }
                };

                let update = doc! { "$set": { "customer_id": cust_id.clone() } };
                match collection.update_one(filter, update).await {
                    // Handle this error!
                    Ok(_) => return HttpResponse::Ok().body("Payment method added."), // Success!
                    Err(err) => {
                        eprintln!("MongoDB Update Error: {:?}", err); // Log the error
                        return HttpResponse::InternalServerError()  // Return an error response
                      .body("Failed to update user with customer ID");
                    }
                };
            } else {
                return HttpResponse::Ok().body("Payment method already exists.");
            }
        }
        Ok(None) => {
            // If the user is not found
            println!("User not found");
            return HttpResponse::NotFound().body("User not found");
        }
        Err(err) => {
            eprintln!("MongoDB Error: {:?}", err);
            return HttpResponse::InternalServerError().body("Failed to add payment method");
        }
    }
}
