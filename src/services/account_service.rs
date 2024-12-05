// use mongodb::{bson::doc, Collection};
//
// use crate::models::Account;
//
// pub async fn create_account(
//     accounts_collection: &Collection<Account>,
//     account: &Account,
// ) -> Result<Account, mongodb::error::Error> {
//     // Check if an account with the same provider and providerAccountId already exists
//     let existing_account = accounts_collection
//         .find_one(
//             doc! {
//                 "provider": &account.provider,
//                 "providerAccountId": &account.providerAccountId
//             },
//             None,
//         )
//         .await?;
//
//     if existing_account.is_some() {
//         return Err(mongodb::error::Error::from(
//             mongodb::error::ErrorKind::Write(mongodb::error::WriteFailure::from_message(
//                 "Account with this provider and providerAccountId already exists".to_string(),
//             )),
//         ));
//     }
//
//     let insert_result = accounts_collection.insert_one(account, None).await?;
//     let new_account = accounts_collection
//         .find_one(doc! { "_id": insert_result.inserted_id }, None)
//         .await?
//         .unwrap();
//     Ok(new_account)
// }
