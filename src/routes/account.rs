use actix_web::{web, HttpResponse, Responder};
use mongodb::Client;
use std::sync::Arc;

use crate::models::user::UserTraveler;

// Sign up for an account
pub async fn create_account(data: web::Data<Arc<Client>>, input: web::Json<UserTraveler>) -> impl Responder {
    println!("Creating account");
    println!("Data: {:?}", data);

    let client = data.into_inner(); // Get the client from App::data()
    

    let collection = client
        .database("Travelers")
        .collection("User");

    let curr_time = chrono::Utc::now();
    
    let mut doc = input.into_inner();
    doc.created_at = Some(curr_time);
    doc.updated_at = Some(curr_time);

    // println!("Creating account: {:?}", doc);
    dbg!(&doc);
    
    match collection.insert_one(doc).await {
        Ok(result) => {
            // Insertion successful
            if let mongodb::results::InsertOneResult { inserted_id: _, .. } = result {
                HttpResponse::Ok().body("Account successfully created.")
            } else {
                // This shouldn't happen, but handle it just in case
                HttpResponse::InternalServerError().body("Failed to create account.") 
            }
        }
        Err(err) => {
            // Error during insertion
            eprintln!("Failed to insert document: {:?}", err); // Log the error
            HttpResponse::InternalServerError().body("Failed to create account.")
        }
    }
}
