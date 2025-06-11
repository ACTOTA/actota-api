use serde::{Deserialize, Serialize};

// Query parameters from Google OAuth callback
#[derive(Debug, Deserialize)]
pub struct GoogleAuthCallbackParams {
    pub code: String,
    pub state: Option<String>,
    pub scope: Option<String>,
    pub error: Option<String>,
}

// User info from Google
#[derive(Debug, Serialize, Deserialize)]
pub struct GoogleUserInfo {
    pub id: String,
    pub email: String,
    pub verified_email: bool,
    pub name: Option<String>,
    pub given_name: Option<String>,
    pub family_name: Option<String>,
    pub picture: Option<String>,
    pub locale: Option<String>,
}
