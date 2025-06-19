use actix_web::{web, HttpResponse, Responder};
use futures::TryStreamExt;
use mongodb::{
    bson::{doc, oid::ObjectId, Document},
    Client,
};
use std::sync::Arc;

use crate::models::activity::Activity;

pub async fn get_activities(data: web::Data<Arc<Client>>) -> impl Responder {
    println!("GETTING ACTIVITIES");

    let client = data.into_inner();
    let db = client.database("Options");

    // First get raw documents to prevent deserialization errors from blocking everything
    let raw_collection = db.collection::<Document>("Activity");

    match raw_collection.find(doc! {}).await {
        Ok(mut cursor) => {
            let mut successful_activities = Vec::new();
            let mut error_count = 0;
            let mut failed_ids = Vec::new();

            // Process each document individually
            while let Ok(Some(doc)) = cursor.try_next().await {
                // Extract the ID before attempting deserialization
                let id_string = match doc.get("_id") {
                    Some(id_bson) => match id_bson {
                        mongodb::bson::Bson::ObjectId(oid) => oid.to_string(),
                        _ => format!("Non-ObjectId: {:?}", id_bson),
                    },
                    None => "missing_id".to_string(),
                };

                // Try to convert the document to an Activity
                match mongodb::bson::from_document::<Activity>(doc.clone()) {
                    Ok(activity) => {
                        successful_activities.push(activity);
                    }
                    Err(err) => {
                        error_count += 1;
                        // Add to failed IDs list
                        failed_ids.push(id_string.clone());

                        // Print the error with the document ID
                        eprintln!(
                            "Failed to deserialize document with _id {}: {:?}",
                            id_string, err
                        );

                        // Print a few key fields if available to help debug
                        if let Some(title) = doc.get("title").and_then(|t| t.as_str()) {
                            eprintln!("  Title: {}", title);
                        }

                        // Optionally print more fields that might be useful for debugging
                        // eprintln!("Document fields: {}", doc.keys().collect::<Vec<_>>().join(", "));
                    }
                }
            }

            // Print summary of failed IDs
            if !failed_ids.is_empty() {
                eprintln!("\nFailed Activity IDs:");
                for (i, id) in failed_ids.iter().enumerate() {
                    eprintln!("{}. {}", i + 1, id);
                }
                eprintln!("\nTotal failures: {}\n", failed_ids.len());
            }

            // Construct response with metadata about errors
            let response = serde_json::json!({
                "activities": successful_activities,
                "metadata": {
                    "total_successful": successful_activities.len(),
                    "total_failed": error_count,
                    "failed_ids": failed_ids,
                    "success_rate": if successful_activities.len() + error_count > 0 {
                        (successful_activities.len() as f64 / (successful_activities.len() + error_count) as f64) * 100.0
                    } else {
                        0.0
                    }
                }
            });

            // Log summary
            println!(
                "Processed {} activities successfully, {} failed",
                successful_activities.len(),
                error_count
            );

            return HttpResponse::Ok().json(response);
        }
        Err(err) => {
            eprintln!("Failed to find documents: {:?}", err);
            return HttpResponse::InternalServerError().body("Failed to find activities.");
        }
    }
}
