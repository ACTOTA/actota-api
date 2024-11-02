use mongodb::{Client, options::ClientOptions};
use std::sync::Arc;

pub async fn create_mongo_client(uri: &String) -> Arc<Client> {
    println!("Connecting to MongoDB: {}", uri);
    let client_options = ClientOptions::parse(uri)
        .await 
        .expect("MongoDB URI may be incorrect! Failed to parse.");

    let client = Client::with_options(client_options).expect("failed to connect");
    Arc::new(client)
}
