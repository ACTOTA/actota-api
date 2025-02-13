use actix_web::{
    dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform},
    error::ErrorUnauthorized,
    Error, HttpMessage,
};
use futures::future::{ready, LocalBoxFuture, Ready};
use jsonwebtoken::{decode, Algorithm, DecodingKey, Validation};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Claims {
    pub sub: String,     // subject (email)
    pub exp: usize,      // expiration time
    pub iat: usize,      // issued at
    pub user_id: String, // optional: add more claims as needed
}

pub struct AuthMiddleware;

impl<S, B> Transform<S, ServiceRequest> for AuthMiddleware
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Transform = AuthMiddlewareService<S>;
    type InitError = ();
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(AuthMiddlewareService { service }))
    }
}

pub struct AuthMiddlewareService<S> {
    service: S,
}

impl<S, B> Service<ServiceRequest> for AuthMiddlewareService<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let auth_header = req.headers().get("Authorization");

        if let Some(auth_header) = auth_header {
            if let Ok(auth_str) = auth_header.to_str() {
                if auth_str.starts_with("Bearer ") {
                    let token = &auth_str[7..];
                    let key = std::env::var("JWT_SECRET")
                        .unwrap_or_else(|_| "default_secret".to_string());

                    println!("Key: {}", key);
                    let mut validation = Validation::new(Algorithm::HS256);
                    validation.validate_exp = true;
                    validation.set_required_spec_claims(&["exp", "iat", "sub", "user_id"]);

                    match decode::<Claims>(
                        token,
                        &DecodingKey::from_secret(key.as_bytes()),
                        &validation,
                    ) {
                        Ok(token_data) => {
                            req.extensions_mut().insert(token_data.claims);
                            return Box::pin(self.service.call(req));
                        }
                        Err(err) => {
                            println!("Error decoding token: {:?}", err);
                            return Box::pin(ready(Err(ErrorUnauthorized("Invalid token"))));
                        }
                    }
                }
            }
        }
        Box::pin(ready(Err(ErrorUnauthorized("No authorization header"))))
    }
}
