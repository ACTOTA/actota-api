# Admin API Usage Guide

## API Routes vs Scripts

For production, **API routes are recommended** over scripts because they:
- Provide better security with built-in authentication
- Are easier to integrate with frontend applications
- Can be called remotely without server access
- Include proper error handling and responses

## Admin API Endpoints

### 1. List All Users with Roles
```bash
GET /api/admin/users
Authorization: Bearer <admin_token>
```

Returns:
```json
[
  {
    "_id": "6804f27cbc29d111f4e0910d",
    "email": "user@example.com",
    "first_name": "John",
    "last_name": "Doe", 
    "role": "admin",
    "created_at": "2024-01-01T00:00:00Z"
  }
]
```

### 2. Update User Role
```bash
PUT /api/admin/users/6804f27cbc29d111f4e0910d/role
Authorization: Bearer <admin_token>
Content-Type: application/json

{
  "role": "admin"  // or "user"
}
```

Returns:
```json
{
  "success": true,
  "message": "User role updated to admin",
  "user_id": "6804f27cbc29d111f4e0910d",
  "role": "admin"
}
```

## Security Features

1. **Double Authentication Layer**:
   - First layer: `AuthMiddleware` - Validates JWT token
   - Second layer: `RequireRole` - Ensures user has Admin role

2. **Token Validation**:
   - JWT tokens include role information
   - Tokens expire after 24 hours
   - Role is validated on every request

3. **Error Handling**:
   - 401 Unauthorized - No valid token
   - 403 Forbidden - Valid token but insufficient permissions
   - 404 Not Found - User doesn't exist

## Frontend Implementation

```javascript
// Example: List all users (admin only)
const listUsers = async () => {
  try {
    const response = await fetch('/api/admin/users', {
      headers: {
        'Authorization': `Bearer ${localStorage.getItem('authToken')}`
      }
    });
    
    if (response.status === 403) {
      alert('Admin access required');
      return;
    }
    
    const users = await response.json();
    return users;
  } catch (error) {
    console.error('Failed to fetch users:', error);
  }
};

// Example: Update user role
const updateUserRole = async (userId, newRole) => {
  try {
    const response = await fetch(`/api/admin/users/${userId}/role`, {
      method: 'PUT',
      headers: {
        'Authorization': `Bearer ${localStorage.getItem('authToken')}`,
        'Content-Type': 'application/json'
      },
      body: JSON.stringify({
        role: newRole
      })
    });
    
    const result = await response.json();
    if (result.success) {
      alert('Role updated successfully');
    }
  } catch (error) {
    console.error('Failed to update role:', error);
  }
};
```

## Initial Setup

### For First Admin User

Since all new users get "User" role by default, you'll need to:

1. **Option 1: Use the migration script** (one-time setup)
   ```bash
   MONGODB_URI="your_connection_string" cargo run --bin migrate_user_roles
   ```

2. **Option 2: Manually update in MongoDB**
   ```javascript
   db.Users.updateOne(
     { email: "admin@actota.com" },
     { $set: { role: "Admin" } }
   )
   ```

### For Ongoing Management

Once you have at least one admin, use the API routes to manage other users' roles through your admin interface.

## Testing the Implementation

1. **Login as regular user**
   - Check that admin routes return 403 Forbidden

2. **Login as admin**
   - Verify access to `/api/admin/users`
   - Test role update functionality

3. **Role persistence**
   - Verify role is included in JWT token
   - Check that role persists across sessions

## Best Practices

1. **Audit Trail**: Consider logging role changes for security audits
2. **Role Confirmation**: Add confirmation dialogs before role changes
3. **Session Management**: Force re-login after role changes
4. **Error Handling**: Show user-friendly messages for permission errors

## Extending the System

To add new roles:

1. Update `UserRole` enum in `src/models/account.rs`
2. Update role matching in middleware
3. Add new role-specific routes as needed
4. Update frontend role checks

The system is designed to be easily extensible for future permission requirements.