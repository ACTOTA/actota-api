#!/bin/bash
# ACTOTA API Deployment Script for Google Cloud Run
set -e

# Configuration (modify these values)
PROJECT_ID="actota"  # Your Google Cloud project ID
REGION="us-central1" # Deployment region
SERVICE_NAME="actota-api" # Cloud Run service name
IMAGE_NAME="gcr.io/${PROJECT_ID}/${SERVICE_NAME}"

# Check prerequisites
echo "=== Checking prerequisites ==="
command -v gcloud >/dev/null 2>&1 || { echo "Error: gcloud CLI is required but not installed. Please install it."; exit 1; }
command -v docker >/dev/null 2>&1 || { echo "Error: docker is required but not installed. Please install it."; exit 1; }

echo "=== Starting deployment of ${SERVICE_NAME} to Google Cloud Run ==="

# Authenticate with Google Cloud
echo "=== Authenticating with Google Cloud ==="
gcloud auth configure-docker

# Build and push the Docker image for AMD64 architecture
echo "=== Building and pushing Docker image for AMD64 architecture ==="
docker buildx build --platform linux/amd64 \
  --tag ${IMAGE_NAME}:amd64 \
  --push \
  .

# Deploy to Cloud Run
echo "=== Deploying to Cloud Run ==="
gcloud run deploy ${SERVICE_NAME} \
  --image ${IMAGE_NAME}:amd64 \
  --platform managed \
  --region ${REGION} \
  --set-env-vars="RUST_LOG=actix_web=debug" \
  --set-env-vars="CLOUD_STORAGE_URL=https://storage.googleapis.com" \
  --set-env-vars="ITINERARY_BUCKET=actota-itineraries" \
  --set-env-vars="PROFILE_PIC_BUCKET=actota-profile-pictures" \
  --set-env-vars="GOOGLE_CLIENT_ID=324035621794-2ahq7okguqovgd84l9pehjnjc43oh3j4.apps.googleusercontent.com" \
  --set-env-vars="GOOGLE_REDIRECT_URI=https://actota-api-324035621794.us-central1.run.app/api/auth/google/callback" \
  --set-env-vars="FACEBOOK_CLIENT_ID=536393542826375" \
  --set-env-vars="FACEBOOK_REDIRECT_URI=https://actota-api-324035621794.us-central1.run.app/api/auth/facebook/callback" \
  --set-env-vars="FRONTEND_URL=https://actota-frontend-324035621794.us-central1.run.app" \
  # Removed GOOGLE_APPLICATION_CREDENTIALS as we're using ADC \
  --service-account actota-api@${PROJECT_ID}.iam.gserviceaccount.com \
  --allow-unauthenticated \
  --cpu=1 \
  --memory=512Mi \
  --min-instances=0 \
  --max-instances=10 \
  --concurrency=80 \
  --timeout=300s

# Get the service URL
SERVICE_URL=$(gcloud run services describe ${SERVICE_NAME} --platform managed --region ${REGION} --format="value(status.url)")
echo "=== Deployment complete! ==="
echo "Your service is available at: ${SERVICE_URL}"
echo ""
echo "To view logs:"
echo "gcloud logging read \"resource.type=cloud_run_revision AND resource.labels.service_name=${SERVICE_NAME}\" --limit 20"
