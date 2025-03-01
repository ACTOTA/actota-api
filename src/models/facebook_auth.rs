use serde::{Deserialize, Serialize};

// Query parameters from Facebook OAuth callback
#[derive(Deserialize)]
pub struct FacebookAuthCallbackParams {
    pub code: String,
    pub state: String,
    pub error: Option<String>,
    pub error_reason: Option<String>,
    pub error_description: Option<String>,
}

// User info from Facebook
#[derive(Debug, Serialize, Deserialize)]
pub struct FacebookUserInfo {
    pub id: String,
    pub email: String,
    pub name: Option<String>,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub picture: Option<FacebookPicture>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FacebookPicture {
    pub data: FacebookPictureData,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FacebookPictureData {
    pub url: String,
    pub width: Option<i32>,
    pub height: Option<i32>,
}

