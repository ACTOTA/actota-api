# ACTOTA API Authentication Documentation

## Overview

This document outlines the authentication mechanisms implemented for the ACTOTA API running on Google Cloud Run in production. The API is configured with restricted access to ensure only authorized applications (specifically the Vercel-hosted frontend) can make requests.

## Why Restricted Authentication?

### Security Benefits
- **Prevents unauthorized access** to sensitive API endpoints
- **Protects against abuse** and excessive usage
- **Maintains data integrity** by controlling who can access the system
- **Compliance** with security best practices for production APIs
- **Cost control** by preventing unauthorized usage that could result in unexpected charges

### Implementation Approach
We use Google Cloud IAM (Identity and Access Management) with service account authentication, which provides:
- **Server-side authentication** - credentials never exposed to client browsers
- **Automatic token management** - tokens refresh automatically
- **Fine-grained permissions** - can be restricted to specific services/resources
- **Audit trail** - all requests are logged and traceable
- **Google-native security** - leverages Google's robust authentication infrastructure

## Current Configuration

### Cloud Run Service Details
- **Service Name**: `actota-api`
- **Project**: `actota-prod`
- **Region**: `us-central1`
- **URL**: `https://actota-api-7g4csnbc2a-uc.a.run.app`

### Authentication Method
- **Type**: Google Cloud IAM with Service Account
- **Service Account**: `actota-frontend@actota-prod.iam.gserviceaccount.com`
- **Role**: `roles/run.invoker`
- **Access**: Restricted to authorized service account only

## Authentication Methods

### Method 1: Service Account Authentication (Recommended)

This is the currently implemented and recommended approach.

#### How It Works
1. A dedicated service account (`actota-frontend`) is created with permission to invoke the Cloud Run service
2. The service account's private key is used to generate access tokens
3. Each API request includes a valid Bearer token in the Authorization header
4. Cloud Run validates the token and allows/denies access based on IAM policies

#### Advantages
- ✅ Most secure method
- ✅ Tokens automatically expire and refresh
- ✅ No credentials exposed in client-side code
- ✅ Full audit trail of API access
- ✅ Can be restricted to specific operations

#### Implementation

**Prerequisites**: Service account key stored securely (currently in Google Secret Manager as `ACTOTA_FRONTEND_SERVICE_ACCOUNT`)

**In Vercel Frontend:**
```javascript
// utils/auth.js
import { GoogleAuth } from 'google-auth-library';

const auth = new GoogleAuth({
  credentials: JSON.parse(process.env.GOOGLE_APPLICATION_CREDENTIALS_JSON),
  scopes: 'https://www.googleapis.com/auth/cloud-platform'
});

export async function getAuthenticatedClient() {
  return await auth.getClient();
}

export async function makeAuthenticatedRequest(url, options = {}) {
  const client = await getAuthenticatedClient();
  const headers = await client.getRequestHeaders();
  
  return fetch(url, {
    ...options,
    headers: {
      ...headers,
      ...options.headers,
    },
  });
}

// Usage example
const response = await makeAuthenticatedRequest(
  'https://actota-api-7g4csnbc2a-uc.a.run.app/api/users',
  {
    method: 'GET',
    headers: {
      'Content-Type': 'application/json'
    }
  }
);
```

### Method 2: Direct API Call with gcloud CLI

For testing or administrative purposes, you can make direct calls using the gcloud CLI.

#### Using gcloud auth
```bash
# Authenticate with your Google account
gcloud auth login

# Make authenticated request
curl -H "Authorization: Bearer $(gcloud auth print-access-token)" \
  https://actota-api-7g4csnbc2a-uc.a.run.app/api/endpoint
```

#### Using service account directly
```bash
# Activate service account
gcloud auth activate-service-account actota-frontend@actota-prod.iam.gserviceaccount.com \
  --key-file=path/to/service-account-key.json

# Make authenticated request
curl -H "Authorization: Bearer $(gcloud auth print-access-token)" \
  https://actota-api-7g4csnbc2a-uc.a.run.app/api/endpoint
```

### Method 3: Using Service Account Key Directly

For programmatic access outside of the frontend application.

```bash
# Using curl with service account
ACCESS_TOKEN=$(gcloud auth application-default print-access-token)
curl -H "Authorization: Bearer $ACCESS_TOKEN" \
  https://actota-api-7g4csnbc2a-uc.a.run.app/api/endpoint
```

## Credential Generation Steps

### Step 1: Create Service Account

```bash
# Create the service account
gcloud iam service-accounts create actota-frontend \
  --description="Service account for ACTOTA frontend on Vercel" \
  --display-name="ACTOTA Frontend" \
  --project=actota-prod
```

### Step 2: Grant Cloud Run Invoker Permission

```bash
# Grant permission to invoke the Cloud Run service
gcloud run services add-iam-policy-binding actota-api \
  --member="serviceAccount:actota-frontend@actota-prod.iam.gserviceaccount.com" \
  --role="roles/run.invoker" \
  --project=actota-prod \
  --region=us-central1
```

### Step 3: Generate Service Account Key

```bash
# Create a service account key
gcloud iam service-accounts keys create actota-frontend-key.json \
  --iam-account=actota-frontend@actota-prod.iam.gserviceaccount.com \
  --project=actota-prod
```

### Step 4: Store Key Securely

```bash
# Store the key in Google Secret Manager
gcloud secrets create ACTOTA_FRONTEND_SERVICE_ACCOUNT \
  --data-file=actota-frontend-key.json \
  --project=actota-prod

# Clean up local key file
rm actota-frontend-key.json
```

### Step 5: Retrieve Key for Use

```bash
# Retrieve the service account key from Secret Manager
gcloud secrets versions access latest \
  --secret=ACTOTA_FRONTEND_SERVICE_ACCOUNT \
  --project=actota-prod
```

## Environment Configuration

### Vercel Environment Variables

Set these in your Vercel project settings:

```env
GOOGLE_APPLICATION_CREDENTIALS_JSON={"type":"service_account","project_id":"actota-prod",...}
ACTOTA_API_BASE_URL=https://actota-api-7g4csnbc2a-uc.a.run.app
```

### Local Development

For local development, you can:

1. **Download service account key:**
```bash
gcloud secrets versions access latest \
  --secret=ACTOTA_FRONTEND_SERVICE_ACCOUNT \
  --project=actota-prod > service-account-key.json
```

2. **Set environment variable:**
```bash
export GOOGLE_APPLICATION_CREDENTIALS=./service-account-key.json
```

3. **Or use application default credentials:**
```bash
gcloud auth application-default login
```

## Testing Authentication

### Test with curl
```bash
# Get access token
TOKEN=$(gcloud auth print-access-token)

# Test API endpoint
curl -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  https://actota-api-7g4csnbc2a-uc.a.run.app/api/health

# Expected responses:
# 200 OK - Authentication successful
# 403 Forbidden - Authentication failed
# 401 Unauthorized - No/invalid token
```

### Test with Node.js
```javascript
const { GoogleAuth } = require('google-auth-library');

async function testAuth() {
  const auth = new GoogleAuth({
    keyFilename: './service-account-key.json',
    scopes: 'https://www.googleapis.com/auth/cloud-platform'
  });
  
  const client = await auth.getClient();
  const headers = await client.getRequestHeaders();
  
  const response = await fetch('https://actota-api-7g4csnbc2a-uc.a.run.app/api/health', {
    headers
  });
  
  console.log('Status:', response.status);
  console.log('Response:', await response.text());
}

testAuth();
```

## Troubleshooting

### Common Issues

1. **403 Forbidden Error**
   - Check if service account has `roles/run.invoker` permission
   - Verify the service account is correct
   - Ensure the token is valid and not expired

2. **401 Unauthorized Error**
   - Check if Authorization header is properly formatted
   - Verify access token is valid
   - Ensure credentials are properly configured

3. **Token Expiration**
   - Access tokens typically expire after 1 hour
   - The `google-auth-library` handles automatic refresh
   - For manual calls, generate a new token

### Debugging Commands

```bash
# Check current authentication
gcloud auth list

# Check service account permissions
gcloud projects get-iam-policy actota-prod \
  --flatten="bindings[].members" \
  --filter="bindings.members:actota-frontend@actota-prod.iam.gserviceaccount.com"

# Check Cloud Run service IAM policy
gcloud run services get-iam-policy actota-api \
  --project=actota-prod \
  --region=us-central1

# Test token validity
gcloud auth print-access-token | cut -d. -f2 | base64 -d
```

## Security Best Practices

1. **Never commit service account keys** to version control
2. **Use environment variables** for storing credentials
3. **Rotate service account keys** regularly
4. **Monitor access logs** for unusual activity
5. **Use least privilege principle** - only grant necessary permissions
6. **Store keys in secure secret management systems** (Google Secret Manager, Vercel environment variables)

## Alternative Authentication Methods

While service account authentication is recommended, here are alternative approaches:

### API Key Authentication
- Simpler to implement but less secure
- Requires API key to be included in requests
- Keys don't automatically expire

### CORS-based Restriction
- Relies on browser CORS policies
- Can be bypassed by non-browser clients
- Less secure than token-based authentication

### Custom Authentication
- Implement custom token validation in your API
- Requires additional infrastructure
- More complex to maintain

## Monitoring and Logging

Authentication events are logged in Google Cloud Logging:

```bash
# View authentication logs
gcloud logging read 'resource.type="cloud_run_revision" AND 
  resource.labels.service_name="actota-api" AND 
  (protoPayload.authenticationInfo.principalEmail!="" OR 
   httpRequest.status>=400)' \
  --project=actota-prod \
  --limit=50
```

## Contact

For questions about authentication or access issues, contact the development team or refer to the Google Cloud Run authentication documentation.

---

*Last updated: June 2025*
*Current API URL: https://actota-api-7g4csnbc2a-uc.a.run.app*

