use actix_web::{web, HttpResponse, Responder};
use futures::TryStreamExt;
use mongodb::{bson::doc, Client};
use std::sync::Arc;

use crate::models::activity::Activity;

pub async fn get_activities(data: web::Data<Arc<Client>>) -> impl Responder {
    let client = data.into_inner();
    let collection: mongodb::Collection<Activity> =
        client.database("Options").collection("Activity");

    match collection.find(doc! {}).await {
        Ok(cursor) => match cursor.try_collect::<Vec<Activity>>().await {
            Ok(activities) => return HttpResponse::Ok().json(activities),
            Err(err) => {
                eprintln!("Failed to collect documents: {:?}", err);
                return HttpResponse::InternalServerError().body("Failed to collect activities.");
            }
        },
        Err(err) => {
            eprintln!("Failed to find documents: {:?}", err);
            return HttpResponse::InternalServerError().body("Failed to find activities.");
        }
    }
}
