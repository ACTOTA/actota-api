#!/bin/bash

# Script to populate .env.local with actual values from Google Cloud secrets
# Run this to set up your local development environment

set -e

echo "🔧 Populating .env.local with values from Google Cloud..."

# Make sure we're in the main project
gcloud config set project actota

# Create a new .env.local file
cat > .env.local << 'EOF'
# Local Development Environment Configuration
# Auto-generated from Google Cloud secrets

# Logging
RUST_LOG=actix_web=debug,actota_api=debug

# Database - Use development database (you'll need to set this manually)
MONGODB_URI=mongodb+srv://http-server_dev:<password>@<cluster_name>.1l3na.mongodb.net/?retryWrites=true&w=majority&appName=<cluster-name>

# Cloud Storage Configuration - Use main project for local development
CLOUD_STORAGE_URL=https://storage.googleapis.com
ITINERARY_BUCKET=actota-itineraries
PROFILE_PIC_BUCKET=actota-profile-pictures
ACTIVITY_BUCKET=actota-activity-media

# Vertex AI Search Configuration - Use main project for local development
GOOGLE_CLOUD_PROJECT_ID=actota
VERTEX_SEARCH_LOCATION=global
VERTEX_SEARCH_DATA_STORE_ID=<your_main_project_data_store_id>
VERTEX_SEARCH_SERVING_CONFIG=default_config

# Frontend URL
FRONTEND_URL=http://localhost:3000

# Google OAuth
GOOGLE_REDIRECT_URI=http://localhost:8080/auth/google/callback

# Facebook OAuth  
FACEBOOK_REDIRECT_URI=http://localhost:8080/auth/facebook/callback

# Search Configuration
MIN_SEARCH_RESULTS=3
EOF

# Add secrets from Google Cloud
echo "# JWT Secret" >> .env.local
echo "JWT_SECRET=$(gcloud secrets versions access latest --secret=JWT_SECRET)" >> .env.local

echo "" >> .env.local
echo "# Google OAuth" >> .env.local
echo "GOOGLE_CLIENT_SECRET=$(gcloud secrets versions access latest --secret=GOOGLE_CLIENT_SECRET)" >> .env.local

echo "" >> .env.local
echo "# Facebook OAuth" >> .env.local  
echo "FACEBOOK_CLIENT_SECRET=$(gcloud secrets versions access latest --secret=FACEBOOK_CLIENT_SECRET)" >> .env.local

echo "" >> .env.local
echo "# Google Maps" >> .env.local
echo "GOOGLE_MAPS_API_KEY=$(gcloud secrets versions access latest --secret=GOOGLE_MAPS_KEY)" >> .env.local

echo "" >> .env.local
echo "# Stripe (Test Key)" >> .env.local
echo "STRIPE_SECRET_KEY=$(gcloud secrets versions access latest --secret=STRIPE_SECRET_KEY)" >> .env.local
echo "STRIPE_WEBHOOK_SECRET=$(gcloud secrets versions access latest --secret=STRIPE_WEBHOOK_SECRET)" >> .env.local

echo "✅ .env.local has been populated!"
echo ""
echo "⚠️  You still need to manually set:"
echo "   - MONGODB_URI (development database)"
echo "   - GOOGLE_CLIENT_ID" 
echo "   - FACEBOOK_CLIENT_ID"
echo "   - VERTEX_SEARCH_DATA_STORE_ID (once you create the data store)"
echo "   - SENDGRID_API_KEY (if needed)"
echo ""
echo "🔍 To find your Google Client ID, check the Google Cloud Console > APIs & Services > Credentials"
