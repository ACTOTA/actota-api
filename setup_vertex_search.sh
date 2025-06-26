#!/bin/bash

# Script to set up Vertex AI Search configuration for UAT and Production environments
# This replicates the configuration from the main 'actota' project

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo -e "${GREEN}Setting up Vertex AI Search for UAT and Production environments${NC}"

# Function to setup Vertex AI Search for a project
setup_vertex_search() {
    local project_id=$1
    local env_name=$2
    
    echo -e "\n${YELLOW}Setting up Vertex AI Search for project: ${project_id} (${env_name})${NC}"
    
    # Set the project
    gcloud config set project $project_id
    
    # Enable required APIs
    echo "Enabling required APIs..."
    gcloud services enable discoveryengine.googleapis.com --quiet
    gcloud services enable storage.googleapis.com --quiet
    gcloud services enable run.googleapis.com --quiet
    gcloud services enable secretmanager.googleapis.com --quiet
    
    # Create Cloud Storage buckets (if they don't exist)
    echo "Creating Cloud Storage buckets..."
    
    # Activities bucket for Vertex AI Search data
    gsutil mb -p $project_id gs://${project_id}-activities 2>/dev/null || echo "Bucket gs://${project_id}-activities already exists"
    gsutil mb -p $project_id gs://${project_id}-activity-media 2>/dev/null || echo "Bucket gs://${project_id}-activity-media already exists"
    gsutil mb -p $project_id gs://${project_id}-itineraries 2>/dev/null || echo "Bucket gs://${project_id}-itineraries already exists"
    gsutil mb -p $project_id gs://${project_id}-profile-pictures 2>/dev/null || echo "Bucket gs://${project_id}-profile-pictures already exists"
    gsutil mb -p $project_id gs://${project_id}-transportation-media 2>/dev/null || echo "Bucket gs://${project_id}-transportation-media already exists"
    
    # Set bucket permissions to be publicly readable for media files
    gsutil iam ch allUsers:objectViewer gs://${project_id}-activity-media
    gsutil iam ch allUsers:objectViewer gs://${project_id}-transportation-media
    gsutil iam ch allUsers:objectViewer gs://${project_id}-profile-pictures
    
    # Create service account for the API if it doesn't exist
    echo "Creating service account..."
    gcloud iam service-accounts create actota-api \
        --description="Service account for ACTOTA API" \
        --display-name="actota-api" 2>/dev/null || echo "Service account already exists"
    
    # Grant necessary permissions to the service account
    echo "Granting permissions to service account..."
    gcloud projects add-iam-policy-binding $project_id \
        --member="serviceAccount:actota-api@${project_id}.iam.gserviceaccount.com" \
        --role="roles/storage.admin"
    
    gcloud projects add-iam-policy-binding $project_id \
        --member="serviceAccount:actota-api@${project_id}.iam.gserviceaccount.com" \
        --role="roles/secretmanager.secretAccessor"
    
    gcloud projects add-iam-policy-binding $project_id \
        --member="serviceAccount:actota-api@${project_id}.iam.gserviceaccount.com" \
        --role="roles/discoveryengine.admin"
    
    gcloud projects add-iam-policy-binding $project_id \
        --member="serviceAccount:actota-api@${project_id}.iam.gserviceaccount.com" \
        --role="roles/run.admin"
    
    # Grant Discovery Engine permissions
    gcloud projects add-iam-policy-binding $project_id \
        --member="serviceAccount:actota-api@${project_id}.iam.gserviceaccount.com" \
        --role="roles/discoveryengine.viewer"
    
    echo -e "${GREEN}✓ Vertex AI Search setup completed for ${project_id} (${env_name})${NC}"
}

# Setup for UAT
setup_vertex_search "actota-uat" "UAT"

# Setup for Production  
setup_vertex_search "actota-prod" "Production"

echo -e "\n${GREEN}✅ Vertex AI Search setup completed for both environments!${NC}"

echo -e "\n${YELLOW}Next steps:${NC}"
echo "1. Create Vertex AI Search data stores in the Google Cloud Console for each project:"
echo "   - Go to https://console.cloud.google.com/ai/search"
echo "   - Create a new search app with unstructured data"
echo "   - Note the data store IDs for environment variables"
echo ""
echo "2. Upload your activity data to the data stores"
echo ""
echo "3. Update GitHub environment variables with the data store IDs:"
echo "   - VERTEX_SEARCH_DATA_STORE_ID"
echo "   - VERTEX_SEARCH_LOCATION (usually 'global')"
echo "   - VERTEX_SEARCH_SERVING_CONFIG (usually 'default_config')"
