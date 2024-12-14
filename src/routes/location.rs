use actix_web::{web, HttpResponse, Responder};
use futures::TryStreamExt;
use mongodb::{bson::doc, options::FindOptions, Client};
use std::sync::Arc;

use crate::models::location::Location;

#[derive(serde::Deserialize)]
pub struct QueryParams {
    limit: Option<u16>,
}

pub async fn get_locations(
    data: web::Data<Arc<Client>>,
    params: web::Query<QueryParams>,
) -> impl Responder {
    let client = data.into_inner();
    let collection: mongodb::Collection<Location> =
        client.database("Options").collection("Location");

    let mut options = FindOptions::default();
    if let Some(limit) = params.limit {
        options.limit = Some(limit.into());
    }

    match collection.find(doc! {}).with_options(options).await {
        Ok(cursor) => match cursor.try_collect::<Vec<Location>>().await {
            Ok(activities) => return HttpResponse::Ok().json(activities),
            Err(err) => {
                eprintln!("Failed to collect documents: {:?}", err);
                return HttpResponse::InternalServerError().body("Failed to collect lodging.");
            }
        },
        Err(err) => {
            eprintln!("Failed to find documents: {:?}", err);
            return HttpResponse::InternalServerError().body("Failed to find lodging.");
        }
    }
}
