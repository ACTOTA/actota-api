use actix_http::Payload;
use actix_web::{
    dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform},
    error::{ErrorForbidden, ErrorUnauthorized},
    Error, FromRequest, HttpMessage, HttpRequest,
};
use futures::future::{ready, LocalBoxFuture, Ready};
use crate::middleware::auth::Claims;
use crate::models::account::UserRole;

pub struct RequireRole {
    required_role: UserRole,
}

impl RequireRole {
    pub fn new(role: UserRole) -> Self {
        RequireRole { required_role: role }
    }
}

impl<S, B> Transform<S, ServiceRequest> for RequireRole
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Transform = RequireRoleService<S>;
    type InitError = ();
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(RequireRoleService {
            service,
            required_role: self.required_role.clone(),
        }))
    }
}

pub struct RequireRoleService<S> {
    service: S,
    required_role: UserRole,
}

impl<S, B> Service<ServiceRequest> for RequireRoleService<S>
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
        let required_role = self.required_role.clone();

        // Clone the claims before checking
        let claims = req.extensions().get::<Claims>().cloned();
        
        println!("Role auth check - required role: {:?}", required_role);
        println!("Role auth check - claims: {:?}", claims);
        
        if let Some(claims) = claims {
            if let Some(role_str) = &claims.role {
                println!("Role string from token: {}", role_str);
                let user_role = match role_str.as_str() {
                    "admin" => UserRole::Admin,
                    "user" => UserRole::User,
                    _ => {
                        println!("Unknown role: {}", role_str);
                        UserRole::User
                    }
                };
                
                println!("Parsed user role: {:?}, Required role: {:?}", user_role, required_role);

                if user_role == required_role || user_role == UserRole::Admin {
                    println!("Access granted");
                    return Box::pin(self.service.call(req));
                }
                println!("Access denied - insufficient permissions");
            } else {
                println!("No role in claims");
            }
            Box::pin(ready(Err(ErrorForbidden("Insufficient permissions"))))
        } else {
            println!("No claims found in request extensions");
            Box::pin(ready(Err(ErrorUnauthorized("No authorization"))))
        }
    }
}