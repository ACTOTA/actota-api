use mongodb::{Client, Collection, Database};


pub fn get_accounts_collection(db: &Database) -> Collection<crate::models::Account> {
    db.collection::<crate::models::Account>("accounts")
}
