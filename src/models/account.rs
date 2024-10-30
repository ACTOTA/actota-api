use serde::{Deserialize, Serialize};
use mongodb::bson::oid::ObjectId;

#[derive(Debug, Deserialize, Serialize)]
pub struct Account {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,
    pub access_token: String,
    pub expires_at: i64,
    pub id_token: String,
    pub provider: String,
    pub providerAccountId: String,
    pub scope: String,
    pub token_type: String,
    pub account_type: String,
    pub user_id: ObjectId,
}
