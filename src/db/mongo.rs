use mongodb::{
    options::{ClientOptions, ServerApi, ServerApiVersion},
    Client,
};
use std::sync::Arc;
use std::time::Duration;

pub async fn create_mongo_client(uri: &String) -> Arc<Client> {
    println!("Connecting to MongoDB: {}", uri);

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
