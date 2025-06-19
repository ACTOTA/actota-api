use mongodb::{
    options::{ClientOptions, ServerApi, ServerApiVersion},
    Client,
};
use std::sync::Arc;
use std::time::Duration;

/// Masks the username and password in a MongoDB URI for safe logging
fn mask_mongodb_uri(uri: &str) -> String {
    // Check if it's a mongodb+srv:// or mongodb:// URI
    if uri.starts_with("mongodb+srv://") || uri.starts_with("mongodb://") {
        // Find the @ symbol that separates credentials from host
        if let Some(at_pos) = uri.find('@') {
            // Find the :// part
            if let Some(protocol_end) = uri.find("://") {
                let protocol = &uri[..protocol_end + 3];
                let host_and_params = &uri[at_pos + 1..];
                
                // Check if there are credentials (contains : between protocol and @)
                let credentials_part = &uri[protocol_end + 3..at_pos];
                if credentials_part.contains(':') {
                    return format!("{}***:***@{}", protocol, host_and_params);
                }
            }
        }
    }
    
    // If it doesn't match the expected pattern, return as-is (shouldn't happen)
    uri.to_string()
}

pub async fn create_mongo_client(uri: &String) -> Arc<Client> {
    // Mask credentials in the URI for logging
    let masked_uri = mask_mongodb_uri(uri);
    println!("Connecting to MongoDB: {}", masked_uri);

    // Configure MongoDB client options with more robust settings
    let mut client_options = ClientOptions::parse(uri)
        .await
        .expect("MongoDB URI may be incorrect! Failed to parse.");

    // Set a reasonable timeout for operations
    client_options.connect_timeout = Some(Duration::from_secs(10));
    client_options.server_selection_timeout = Some(Duration::from_secs(10));
    client_options.max_pool_size = Some(10);
    client_options.min_pool_size = Some(1);

    // Set the server API if using MongoDB 5.0+
    let server_api = ServerApi::builder().version(ServerApiVersion::V1).build();
    client_options.server_api = Some(server_api);

    // Create the client and check if it can connect
    let client =
        Client::with_options(client_options).expect("Failed to create MongoDB client with options");

    // Test the connection to make sure it works
    match client
        .database("Account")
        .run_command(mongodb::bson::doc! {"ping": 1})
        .await
    {
        Ok(_) => println!("Successfully connected to MongoDB and verified with ping command"),
        Err(e) => {
            eprintln!("WARNING: Connected to MongoDB but ping test failed: {}", e);
            eprintln!("The API may still work, but some functionality might be impaired");
        }
    }

    Arc::new(client)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mask_mongodb_uri() {
        // Test with credentials
        let uri_with_creds = "mongodb+srv://username:password123@cluster.mongodb.net/database?retryWrites=true";
        let masked = mask_mongodb_uri(uri_with_creds);
        assert_eq!(masked, "mongodb+srv://***:***@cluster.mongodb.net/database?retryWrites=true");

        // Test with complex password
        let uri_complex = "mongodb+srv://prod_admin:ActotaProd2025@actota-platform-prod.1jwskux.mongodb.net/?retryWrites=true&w=majority&appName=actota-platform-prod";
        let masked_complex = mask_mongodb_uri(uri_complex);
        assert_eq!(masked_complex, "mongodb+srv://***:***@actota-platform-prod.1jwskux.mongodb.net/?retryWrites=true&w=majority&appName=actota-platform-prod");

        // Test regular mongodb:// protocol
        let uri_regular = "mongodb://user:pass@localhost:27017/mydb";
        let masked_regular = mask_mongodb_uri(uri_regular);
        assert_eq!(masked_regular, "mongodb://***:***@localhost:27017/mydb");

        // Test URI without credentials (should remain unchanged)
        let uri_no_creds = "mongodb+srv://cluster.mongodb.net/database";
        let masked_no_creds = mask_mongodb_uri(uri_no_creds);
        assert_eq!(masked_no_creds, "mongodb+srv://cluster.mongodb.net/database");
    }
}
