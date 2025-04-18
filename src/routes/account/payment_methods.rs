use actix_web::{web, HttpResponse, Responder};
use bson::{doc, oid::ObjectId};
use mongodb::Client;
use serde::{Deserialize, Serialize};
use std::{str::FromStr, sync::Arc};

use crate::{
    middleware::auth::Claims,
    models::account::User,
    services::{
        payment::interface::{CustomerError, PaymentError, PaymentOperations},
        stripe::{models::customer::CustomerData, provider::StripeProvider},
    },
};

// Response struct for get_or_create_customer
#[derive(Debug, Serialize, Deserialize)]
struct CustomerResponse {
    customer_id: String,
    created: bool,
}

// Request struct for attach_payment_method
#[derive(Serialize, Deserialize)]
pub struct AttachPaymentMethod {
    customer_id: String,
    payment_id: String,
    default: bool,
}

#[derive(Serialize, Deserialize)]
pub struct DetachPaymentMethod {
    customer_id: String,
    payment_id: String,
}

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

// Update or set customer_id for a user
async fn update_user_customer_id(
    client: &Arc<Client>,
    user_id: String,
    customer_id: String,
) -> Result<(), String> {
    let collection: mongodb::Collection<User> = client.database("Account").collection("Users");

    let filter = doc! { "_id": ObjectId::from_str(&user_id).map_err(|e| e.to_string())? };
    let update = doc! { "$set": { "customer_id": customer_id } };

    match collection.update_one(filter, update).await {
        Ok(_) => Ok(()),
        Err(err) => {
            eprintln!("MongoDB Error updating customer_id: {:?}", err);
            Err(err.to_string())
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

pub async fn get_or_create_customer(
    data: web::Data<Arc<Client>>,
    path: web::Path<String>,
    claims: Claims,
) -> impl Responder {
    let user_id = path.into_inner();

    // Verify user has permission
    if user_id != claims.user_id {
        return HttpResponse::Forbidden().body("Forbidden");
    }

    let client = data.into_inner();
    let stripe_op = StripeProvider::new(std::env::var("STRIPE_SECRET_KEY").unwrap());

    // First check if customer already exists in our database
    let existing_customer_id = get_customer_id(&client, user_id.clone()).await;

    if let Some(customer_id) = existing_customer_id {
        // Customer ID already exists, verify it's valid in Stripe
        match stripe_op.get_customer(&customer_id).await {
            Ok(_) => {
                return HttpResponse::Ok().json(CustomerResponse {
                    customer_id,
                    created: false,
                });
            }
            Err(_) => {
                // Customer exists in our DB but not in Stripe, we'll create a new one
                println!("Customer ID exists in DB but not in Stripe, creating new customer");
            }
        }
    }

    // Get user from MongoDB to create a new Stripe customer
    let collection: mongodb::Collection<User> = client.database("Account").collection("Users");
    let filter = doc! { "_id": ObjectId::from_str(&user_id).unwrap() };

    let user = match collection.find_one(filter).await {
        Ok(Some(user)) => user,
        Ok(None) => {
            return HttpResponse::NotFound().body("User not found");
        }
        Err(err) => {
            eprintln!("MongoDB Error: {:?}", err);
            return HttpResponse::InternalServerError().body("Failed to retrieve user data");
        }
    };

    // Create a new CustomerData for Stripe
    let mut customer_data = CustomerData::default();
    customer_data.email = user.email.clone().into();
    customer_data.name = user.first_name.clone().map(|f| {
        if let Some(last_name) = user.last_name.clone() {
            format!("{} {}", f, last_name)
        } else {
            f
        }
    });
    customer_data.phone = user.phone_number.clone();

    // Create customer in Stripe
    let new_customer = match stripe_op.create_customer(customer_data).await {
        Ok(customer) => customer,
        Err(CustomerError::InternalServerError) => {
            return HttpResponse::InternalServerError().body("Failed to create customer in Stripe");
        }
        Err(CustomerError::NotFound) => {
            return HttpResponse::InternalServerError().body("Unexpected error creating customer");
        }
    };

    // Extract the new customer ID
    let customer_id = match new_customer.id {
        Some(id) => id,
        None => {
            return HttpResponse::InternalServerError()
                .body("Failed to get customer ID from Stripe");
        }
    };

    // Update user in MongoDB with the new customer ID
    if let Err(err) = update_user_customer_id(&client, user_id, customer_id.clone()).await {
        eprintln!("Failed to update user with customer ID: {}", err);
        return HttpResponse::InternalServerError().body("Failed to update user record");
    }

    // Return the new customer ID
    HttpResponse::Ok().json(CustomerResponse {
        customer_id,
        created: true,
    })
}

pub async fn remove_payment_method(
    data: web::Data<Arc<Client>>,
    path: web::Path<(String, String)>,
    claims: Claims,
) -> impl Responder {
    let (user_id, payment_id) = path.into_inner();

    // Verify user has permission
    if user_id != claims.user_id {
        return HttpResponse::Forbidden().body("Forbidden");
    }

    // Get customer_id from the database
    let client = data.into_inner();
    let customer_id = match get_customer_id(&client, user_id).await {
        Some(id) => id,
        None => {
            return HttpResponse::NotFound().body("Customer not found");
        }
    };

    let stripe_op = StripeProvider::new(std::env::var("STRIPE_SECRET_KEY").unwrap());

    match stripe_op
        .detach_payment_method(customer_id, payment_id)
        .await
    {
        Ok(res) => return res,
        Err(err) => {
            eprintln!("Failed to remove payment method: {:?}", err);
            return HttpResponse::InternalServerError().body("Failed to remove payment method");
        }
    }
}

// pub async fn attach_payment_method(input: web::Json<AttachPaymentMethod>) -> impl Responder {
//     let stripe_op = StripeProvider::new(std::env::var("STRIPE_SECRET_KEY").unwrap());
//     let customer_id = &input.customer_id;
//     let payment_id = &input.payment_id;
//     let _default = input.default;
//
//     match stripe_op
//         .attach_payment_method(customer_id.to_string(), payment_id.to_string())
//         .await
//     {
//         Ok(res) => return res,
//         Err(_) => {
//             return HttpResponse::InternalServerError().body("Failed to attach payment method")
//         }
//     }
// }

// pub async fn detach_payment_method(input: web::Json<DetachPaymentMethod>) -> impl Responder {
//     let stripe_op = StripeProvider::new(std::env::var("STRIPE_SECRET_KEY").unwrap());
//     let customer_id = &input.customer_id;
//     let payment_id = &input.payment_id;
//
//     match stripe_op
//         .detach_payment_method(customer_id.to_string(), payment_id.to_string())
//         .await
//     {
//         Ok(res) => return res,
//         Err(_) => {
//             return HttpResponse::InternalServerError().body("Failed to detach payment method")
//         }
//     }
// }
