use actix_web::{web, HttpResponse, Responder};
use bson::{doc, oid::ObjectId};
use mongodb::Client;
use std::{str::FromStr, sync::Arc};

use crate::{
    middleware::auth::Claims,
    models::account::User,
    services::{
        payment::interface::{PaymentError, PaymentOperations},
        stripe::{models::customer::CustomerData, provider::StripeProvider},
    },
};
// Check for customer_id
// If customer_id exists, return it
async fn get_customer_id(client: &Arc<Client>, user_id: String) -> Option<String> {
    let collection: mongodb::Collection<User> = client.database("Account").collection("Users");

    let filter = doc! { "_id": ObjectId::from_str(&user_id).unwrap() };

    match collection.find_one(filter).await {
        Ok(Some(user)) => {
            if let Some(customer_id) = user.customer_id {
                Some(customer_id)
            } else {
                None
            }
        }
        Ok(None) => {
            println!("User not found");
            None
        }
        Err(err) => {
            eprintln!("MongoDB Error: {:?}", err);
            None
        }
    }
}

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
                    Ok(_) => {
                        // let customer_id = cust_id.expect("Unable to read customer_id");
                        // let payment_methods =
                        //     match get_payment_methods(stripe_op, customer_id).await {
                        //         Ok(methods) => methods,
                        //         Err(_) => {
                        //             return HttpResponse::InternalServerError()
                        //                 .body("Failed to retrieve payment methods");
                        //         }
                        //     };

                        // return HttpResponse::Ok().json(payment_methods);
                        return HttpResponse::Ok().body("Payment method added");
                    }
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

pub async fn get_payment_methods(data: web::Data<Arc<Client>>, claims: Claims) -> impl Responder {
    let stripe_op = StripeProvider::new(std::env::var("STRIPE_SECRET_KEY").unwrap());
    let client = data.into_inner();

    let customer_id = match get_customer_id(&client, claims.user_id.clone()).await {
        Some(id) => id,
        None => {
            return HttpResponse::NotFound().body("Customer not found");
        }
    };

    let methods = match stripe_op.get_cust_payment_methods(customer_id).await {
        Ok(methods) => methods,
        Err(_) => {
            return HttpResponse::InternalServerError().body("Failed to retrieve payment methods");
        }
    };

    HttpResponse::Ok().json(methods)
}

async fn payment_method(stripe_op: StripeProvider) {}
