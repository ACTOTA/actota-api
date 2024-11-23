use std::future::{ready, Ready};

use actix_web::{
    dev::Payload, error::ErrorUnauthorized, Error, FromRequest, HttpMessage, HttpRequest,
};

use crate::middleware::auth::Claims;

#[derive(Clone)]
pub struct AuthenticatedUser {
    pub user_id: String,
    pub email: String,
}

impl FromRequest for AuthenticatedUser {
    type Error = Error;
    type Future = Ready<Result<Self, Self::Error>>;

    fn from_request(req: &HttpRequest, _: &mut Payload) -> Self::Future {
        if let Some(claims) = req.extensions().get::<Claims>() {
            ready(Ok(AuthenticatedUser {
                user_id: claims.user_id.clone(),
                email: claims.sub.clone(),
            }))
        } else {
            ready(Err(ErrorUnauthorized("User not authenticated")))
        }
    }
}
