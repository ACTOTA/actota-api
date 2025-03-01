use oauth2::{
    basic::BasicClient, reqwest::async_http_client, AuthUrl, AuthorizationCode, ClientId,
    ClientSecret, CsrfToken, RedirectUrl, Scope, TokenResponse, TokenUrl,
};
use reqwest::Client as ReqwestClient;
use std::env;
use url::Url;

use crate::models::facebook_auth::FacebookUserInfo;

// Create a new OAuth client for Facebook
pub fn create_facebook_oauth_client() -> BasicClient {
    let facebook_client_id =
        env::var("FACEBOOK_CLIENT_ID").expect("Missing FACEBOOK_CLIENT_ID environment variable");
    let facebook_client_secret = env::var("FACEBOOK_CLIENT_SECRET")
        .expect("Missing FACEBOOK_CLIENT_SECRET environment variable");
    let facebook_redirect_url = env::var("FACEBOOK_REDIRECT_URI")
        .expect("Missing FACEBOOK_REDIRECT_URI environment variable");

    BasicClient::new(
        ClientId::new(facebook_client_id),
        Some(ClientSecret::new(facebook_client_secret)),
        AuthUrl::new("https://www.facebook.com/v18.0/dialog/oauth".to_string())
            .expect("Invalid authorization endpoint URL"),
        Some(
            TokenUrl::new("https://graph.facebook.com/v18.0/oauth/access_token".to_string())
                .expect("Invalid token endpoint URL"),
        ),
    )
    .set_redirect_uri(RedirectUrl::new(facebook_redirect_url).expect("Invalid redirect URL"))
}

// Generate an authorization URL for Facebook OAuth
pub fn get_facebook_auth_url(client: &BasicClient) -> (Url, CsrfToken) {
    client
        .authorize_url(CsrfToken::new_random)
        .add_scope(Scope::new("email".to_string()))
        .add_scope(Scope::new("public_profile".to_string()))
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
pub async fn get_facebook_user_info(access_token: &str) -> Result<FacebookUserInfo, String> {
    let client = ReqwestClient::new();
    let response = client
        .get("https://graph.facebook.com/v18.0/me")
        .query(&[
            ("fields", "id,email,name,first_name,last_name,picture"),
            ("access_token", access_token),
        ])
        .send()
        .await
        .map_err(|e| format!("Failed to request user info: {}", e))?;

    if !response.status().is_success() {
        return Err(format!(
            "Facebook API returned error status: {}",
            response.status()
        ));
    }

    response
        .json::<FacebookUserInfo>()
        .await
        .map_err(|e| format!("Failed to parse user info: {}", e))
}
