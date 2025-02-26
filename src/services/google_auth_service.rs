use oauth2::{
    basic::BasicClient, reqwest::async_http_client, AuthUrl, AuthorizationCode,
    AuthorizationRequest, ClientId, ClientSecret, CsrfToken, PkceCodeChallenge, PkceCodeVerifier,
    RedirectUrl, Scope, TokenResponse, TokenUrl,
};
use reqwest::Client as ReqwestClient;
use std::env;
use url::Url;

use crate::models::google_auth::GoogleUserInfo;

// Create a new OAuth client for Google
pub fn create_google_oauth_client() -> BasicClient {
    let google_client_id =
        env::var("GOOGLE_CLIENT_ID").expect("Missing GOOGLE_CLIENT_ID environment variable");
    let google_client_secret = env::var("GOOGLE_CLIENT_SECRET")
        .expect("Missing GOOGLE_CLIENT_SECRET environment variable");
    let google_redirect_url =
        env::var("GOOGLE_REDIRECT_URI").expect("Missing GOOGLE_REDIRECT_URI environment variable");

    BasicClient::new(
        ClientId::new(google_client_id),
        Some(ClientSecret::new(google_client_secret)),
        AuthUrl::new("https://accounts.google.com/o/oauth2/v2/auth".to_string())
            .expect("Invalid authorization endpoint URL"),
        Some(
            TokenUrl::new("https://oauth2.googleapis.com/token".to_string())
                .expect("Invalid token endpoint URL"),
        ),
    )
    .set_redirect_uri(RedirectUrl::new(google_redirect_url).expect("Invalid redirect URL"))
}

// Generate an authorization URL for Google OAuth
pub fn get_google_auth_url(client: &BasicClient) -> (Url, CsrfToken) {
    client
        .authorize_url(CsrfToken::new_random)
        .add_scope(Scope::new("openid".to_string()))
        .add_scope(Scope::new("email".to_string()))
        .add_scope(Scope::new("profile".to_string()))
        .url()
}

// Exchange an authorization code for an access token
pub async fn exchange_code_for_token(
    client: &BasicClient,
    code: AuthorizationCode,
) -> Result<String, String> {
    client
        .exchange_code(code)
        .request_async(async_http_client)
        .await
        .map(|token| token.access_token().secret().clone())
        .map_err(|e| format!("Failed to exchange authorization code: {}", e))
}

// Fetch user information using the access token
pub async fn get_google_user_info(access_token: &str) -> Result<GoogleUserInfo, String> {
    let client = ReqwestClient::new();
    let response = client
        .get("https://www.googleapis.com/oauth2/v2/userinfo")
        .header("Authorization", format!("Bearer {}", access_token))
        .send()
        .await
        .map_err(|e| format!("Failed to request user info: {}", e))?;

    if !response.status().is_success() {
        return Err(format!(
            "Google API returned error status: {}",
            response.status()
        ));
    }

    response
        .json::<GoogleUserInfo>()
        .await
        .map_err(|e| format!("Failed to parse user info: {}", e))
}
