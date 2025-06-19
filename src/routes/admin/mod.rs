use actix_web::web;
use crate::routes::account::role_management::{update_user_role, list_users_with_roles};
use crate::middleware::auth::AuthMiddleware;
use crate::middleware::role_auth::RequireRole;
use crate::models::account::UserRole;

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/admin")
            .wrap(AuthMiddleware)
            .wrap(RequireRole::new(UserRole::Admin))
            .route("/users", web::get().to(list_users_with_roles))
            .route("/update-role", web::post().to(update_user_role))
    );
}