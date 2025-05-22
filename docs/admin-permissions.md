# Admin/Permission System Implementation

This document describes the admin/permission system implementation for the ACTOTA API.

## Overview

The system implements role-based access control (RBAC) with two initial roles:
- `User`: Standard user with regular permissions
- `Admin`: Administrative user with elevated permissions

## Backend Implementation

### 1. User Model Updates

The `User` model now includes a `role` field:

```rust
#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
pub enum UserRole {
    User,
    Admin,
}

pub struct User {
    // ... other fields
    pub role: Option<UserRole>,
    // ... other fields
}
```

### 2. JWT Token Updates

JWT tokens now include the user's role:

```rust
pub struct Claims {
    pub sub: String,           // email
    pub exp: usize,           // expiration time
    pub iat: usize,           // issued at
    pub user_id: String,
    pub role: Option<String>, // User role (admin, user, etc.)
}
```

### 3. Authentication Flow

- During **signup**, new users are assigned the `User` role by default
- During **signin**, the user's role is fetched from the database and included in the JWT token
- The **user_session** endpoint returns the user's role in the `UserSession` response

### 4. Role-Based Access Control Middleware

A new middleware `RequireRole` can be used to protect routes that require specific roles:

```rust
use crate::middleware::role_auth::RequireRole;

// Protect a route that requires admin access
.route("/admin/users", web::get().to(list_users_with_roles)
    .wrap(AuthMiddleware)
    .wrap(RequireRole::new(UserRole::Admin)))
```

### 5. Role Management Functions

Admin users can update other users' roles using these endpoints:

- `POST /account/admin/update-role`: Update a user's role
- `GET /account/admin/users`: List all users with their roles

## Frontend Implementation

The frontend should:

1. Store the user's role from the `user_session` response
2. Use the role to conditionally render admin-only features
3. Handle 403 Forbidden responses for unauthorized access

### Example Frontend Storage

```javascript
// After successful login
const userSession = await fetchUserSession();
localStorage.setItem('userRole', userSession.role);

// Check if user is admin
const isAdmin = () => {
  return localStorage.getItem('userRole') === 'Admin';
};

// Conditionally render admin features
{isAdmin() && <AdminDashboard />}
```

## Security Considerations

1. Always validate permissions on the backend - frontend checks are for UX only
2. The JWT secret must be kept secure
3. Role updates should be logged for audit purposes
4. Consider implementing role hierarchies for more complex permission systems

## Extending the System

To add new roles:

1. Add the new role to the `UserRole` enum
2. Update the role matching logic in the middleware
3. Update the frontend to handle the new role
4. Add any new role-specific endpoints

## API Usage Examples

### Check Current User's Role
```bash
curl -H "Authorization: Bearer <token>" \
     http://localhost:8080/account/session
```

### Update User Role (Admin Only)
```bash
curl -X POST -H "Authorization: Bearer <admin_token>" \
     -H "Content-Type: application/json" \
     -d '{"user_id": "user_id_here", "role": "Admin"}' \
     http://localhost:8080/account/admin/update-role
```

### List All Users (Admin Only)
```bash
curl -H "Authorization: Bearer <admin_token>" \
     http://localhost:8080/account/admin/users
```