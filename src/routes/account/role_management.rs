use actix_web::{web, HttpResponse, Responder};
use mongodb::bson::doc;
use mongodb::bson::oid::ObjectId;
use mongodb::Client;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use futures::StreamExt;

use crate::models::account::UserRole;

#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateRoleRequest {
    pub role: UserRole,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateRoleResponse {
    pub success: bool,
    pub message: String,
    pub user_id: String,
    pub role: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UserListItem {
    #[serde(rename = "_id")]
    pub id: String,
    pub email: String, 
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub role: Option<String>,
    pub created_at: Option<String>,
}

// Admin-only endpoint to update user roles
pub async fn update_user_role(
    data: web::Data<Arc<Client>>,
    path: web::Path<String>,
    input: web::Json<UpdateRoleRequest>,
) -> impl Responder {
    let client = data.into_inner();
    let collection = client.database("Account").collection::<mongodb::bson::Document>("Users");

    let user_id_str = path.into_inner();
    let user_id = match ObjectId::parse_str(&user_id_str) {
        Ok(id) => id,
        Err(_) => {
            return HttpResponse::BadRequest().json(UpdateRoleResponse {
                success: false,
                message: "Invalid user ID format".to_string(),
                user_id: user_id_str,
                role: format!("{:?}", input.role),
            });
        }
    };

    let role_string = match input.role {
        UserRole::Admin => "admin",
        UserRole::User => "user",
    };
    
    // First check if the user exists and what format they have
    match collection.find_one(doc! { "_id": user_id }).await {
        Ok(Some(doc)) => {
            println!("Found user: {:#?}", doc);
            
            // Figure out what format the role is stored in
            let update = if doc.get_document("role").is_ok() {
                // Role is stored as a document
                println!("Role is stored as a BSON document");
                doc! {
                    "$set": {
                        "role": match input.role {
                            UserRole::Admin => doc! { "$serde_name": "admin" },
                            UserRole::User => doc! { "$serde_name": "user" },
                        }
                    }
                }
            } else {
                // Role is stored as a string or doesn't exist
                println!("Role is stored as a string or doesn't exist");
                doc! {
                    "$set": {
                        "role": role_string
                    }
                }
            };
            
            println!("Using update: {}", update);
            
            // Now update the user
            match collection.update_one(doc! { "_id": user_id }, update).await {
                Ok(result) => {
                    println!("Update result: {:?}", result);
                    HttpResponse::Ok().json(UpdateRoleResponse {
                        success: true,
                        message: format!("User role updated to {}", role_string),
                        user_id: user_id.to_hex(),
                        role: role_string.to_string(),
                    })
                },
                Err(err) => {
                    println!("Update error: {:?}", err);
                    HttpResponse::InternalServerError().json(UpdateRoleResponse {
                        success: false,
                        message: format!("Failed to update user role: {}", err),
                        user_id: user_id.to_hex(),
                        role: role_string.to_string(),
                    })
                }
            }
        },
        Ok(None) => {
            HttpResponse::NotFound().json(UpdateRoleResponse {
                success: false,
                message: "User not found".to_string(),
                user_id: user_id.to_hex(),
                role: role_string.to_string(),
            })
        },
        Err(err) => {
            HttpResponse::InternalServerError().json(UpdateRoleResponse {
                success: false,
                message: format!("Database error: {}", err),
                user_id: user_id.to_hex(),
                role: role_string.to_string(),
            })
        }
    }
}

// Helper function to dump a single user for debugging
async fn dump_user_schema(client: &Client, email: &str) -> Result<(), mongodb::error::Error> {
    let collection = client.database("Account").collection::<mongodb::bson::Document>("Users");
    let user = collection.find_one(doc! { "email": email }).await?;
    
    if let Some(doc) = user {
        println!("USER SCHEMA DUMP for {}: {:#?}", email, doc);
        
        // Check if role exists and how it's stored
        if let Ok(role_doc) = doc.get_document("role") {
            println!("ROLE TYPE: Document = {:#?}", role_doc);
        } else if let Ok(role_str) = doc.get_str("role") {
            println!("ROLE TYPE: String = {}", role_str);
        } else {
            println!("ROLE TYPE: Not found or unknown type");
        }
    } else {
        println!("USER not found: {}", email);
    }
    
    Ok(())
}

// Admin-only endpoint to get all users with their roles
pub async fn list_users_with_roles(data: web::Data<Arc<Client>>) -> impl Responder {
    let client = data.into_inner();
    
    // Debug: dump schema for an admin user
    if let Err(e) = dump_user_schema(&client, "admin@actota.com").await {
        println!("Error dumping user schema: {:?}", e);
    }
    
    let collection = client.database("Account").collection::<mongodb::bson::Document>("Users");

    let projection = doc! {
        "_id": 1,
        "email": 1,
        "first_name": 1,
        "last_name": 1,
        "role": 1,
        "created_at": 1
    };

    match collection.find(doc! {}).projection(projection).await {
        Ok(mut cursor) => {
            let mut users = Vec::new();
            while let Some(result) = cursor.next().await {
                if let Ok(doc) = result {
                    // Convert ObjectId to string for proper JSON serialization
                    let id = match doc.get_object_id("_id") {
                        Ok(oid) => oid.to_hex(),
                        Err(_) => continue, // Skip documents without valid _id
                    };
                    
                    // Extract email (required field)
                    let email = match doc.get_str("email") {
                        Ok(e) => e.to_string(),
                        Err(_) => continue, // Skip documents without email
                    };
                    
                    // Extract optional fields
                    let first_name = doc.get_str("first_name").ok().map(|s| s.to_string());
                    let last_name = doc.get_str("last_name").ok().map(|s| s.to_string());
                    
                    // Extract role - could be a string or document depending on how it's stored
                    let role = if let Ok(role_doc) = doc.get_document("role") {
                        // If role is stored as a document with a type field
                        role_doc.get_str("$serde_name").ok().map(|s| s.to_string())
                    } else {
                        // If role is stored as a string
                        doc.get_str("role").ok().map(|s| s.to_string())
                    };
                    
                    let created_at = doc.get_datetime("created_at")
                        .ok()
                        .map(|dt| dt.to_string());
                    
                    users.push(UserListItem {
                        id,
                        email,
                        first_name,
                        last_name,
                        role,
                        created_at,
                    });
                }
            }
            HttpResponse::Ok().json(users)
        }
        Err(err) => {
            eprintln!("Failed to fetch users: {:?}", err);
            HttpResponse::InternalServerError().body("Failed to fetch users")
        }
    }
}