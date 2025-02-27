# ACTOTA Server
*This API is used by the ACTOTA application. It includes the core functionality and calls to access data in the database.*

## Prerequisites
1. [Rust](https://www.rust-lang.org/)
2. [Cargo](https://crates.io/)
   > ***Installing Rust with `rustup` will install all necessary tooling.***
   > - Install [`rustup`](https://www.rust-lang.org/tools/install)
3. [Google Cloud SDK](https://cloud.google.com/sdk/docs/install)
4. [Docker](https://docs.docker.com/get-docker/)

## Project setup
- Install Rust: https://www.rust-lang.org/tools/install
- Requires a `.env` file in the root directory. Reference .env.example for the required environment variables.
- For local development, requires `credentials/service-account.json` for Google Cloud Storage access.

### Run locally
To run, simply run the following command:
```bash
cargo run
```
> Additional Cargo commands can be found here: https://doc.rust-lang.org/cargo/commands/index.html

### Run locally with Docker

1. Build the image:
```bash
docker build -t actota-api-local .
```

2. Run the Docker container:
```bash
docker run -p 8080:8080 \
  --env-file .env \
  actota-api-local
```

## Deploying to Google Cloud

### Prerequisites
1. Google Cloud project with billing enabled
2. Permissions to create and deploy Cloud Run services
3. Permissions to use Google Container Registry (GCR) or Artifact Registry
4. MongoDB Atlas instance or similar for database (the URI will be needed)

### Step 1: Configure `.dockerignore`
Create or update your `.dockerignore` file to ensure sensitive files aren't included in your Docker image:

```
.env
.env.*
credentials/
target/
.git/
.gitignore
```

### Step 2: Configure your Dockerfile
Create a Dockerfile in your project root with the following contents:

```dockerfile
# Use the official Rust image as a builder
FROM rust:slim as builder

# Install build dependencies including pkg-config and OpenSSL dev libraries
# These are needed to compile the openssl-sys crate and other dependencies
RUN apt-get update && \
    apt-get install -y pkg-config libssl-dev && \
    rm -rf /var/lib/apt/lists/*

# Create a new empty shell project
WORKDIR /usr/src/app

# Copy over your manifests and lock file
COPY Cargo.toml ./

# Copy your source code
COPY src/ ./src/

# Build your application for release
RUN cargo build --release

# Now, use a Debian slim image for the final base
FROM debian:bullseye-slim

# Install only runtime dependencies (OpenSSL libraries needed at runtime)
# libssl1.1 provides the OpenSSL runtime libraries
# ca-certificates are needed for HTTPS connections
RUN apt-get update && \
    apt-get install -y ca-certificates libssl1.1 && \
    rm -rf /var/lib/apt/lists/*

# Copy the build artifact from the builder stage
COPY --from=builder /usr/src/app/target/release/actota-api /usr/local/bin/actota-api

# Set the working directory
WORKDIR /usr/local/bin

# Expose the port your app runs on
EXPOSE 8080

# Run the binary
CMD ["actota-api"]
```

### Step 3: Set up Google Cloud Storage buckets

```bash
# Create storage buckets
gcloud storage buckets create gs://actota-itineraries --location=us-central1
gcloud storage buckets create gs://actota-profile-pictures --location=us-central1

# Make buckets publicly readable (if needed for your use case)
gcloud storage buckets add-iam-policy-binding gs://actota-itineraries \
  --member=allUsers \
  --role=roles/storage.objectViewer
  
gcloud storage buckets add-iam-policy-binding gs://actota-profile-pictures \
  --member=allUsers \
  --role=roles/storage.objectViewer
```

### Step 4: Create a Service Account for Cloud Storage

```bash
# Create a service account for your Cloud Run service
gcloud iam service-accounts create actota-service-account

# Grant permissions to access storage buckets
gcloud storage buckets add-iam-policy-binding gs://actota-itineraries \
  --member=serviceAccount:actota-service-account@your-project-id.iam.gserviceaccount.com \
  --role=roles/storage.objectAdmin

gcloud storage buckets add-iam-policy-binding gs://actota-profile-pictures \
  --member=serviceAccount:actota-service-account@your-project-id.iam.gserviceaccount.com \
  --role=roles/storage.objectAdmin
```

### Step 5: Build and Push Docker Image

```bash
# Authenticate with Google Cloud
gcloud auth configure-docker

# Build the Docker image
docker build -t gcr.io/actota/actota-api .

# Push to Google Container Registry
docker push gcr.io/actota/actota-api
```

### Step 6: Deploy to Cloud Run with Environment Variables

#### Option 1: Using the automated deployment script

We've provided a deployment script (`deploy.sh`) that automates the process:

1. Copy the script to your project root:

```bash
# Make the script executable
chmod +x deploy.sh

# Edit the script to set your PROJECT_ID and other configuration
nano deploy.sh

# Run the deployment script
./deploy.sh
```

The script handles building the Docker image, pushing it to Google Container Registry, and deploying to Cloud Run with environment variables from your `.env` file.

#### Option 2: Manual deployment

If you prefer to run the commands manually:

```bash
# This command reads your .env file and formats the variables for gcloud
ENV_VARS=$(grep -v '^#' .env | sed 's/^/--set-env-vars=/' | tr '\n' ' ')

# Deploy with environment variables
gcloud run deploy actota-api \
  --image gcr.io/actota/actota-api \
  --platform managed \
  --region us-central1 \
  $ENV_VARS \
  --service-account actota-api@actota.iam.gserviceaccount.com \
  --allow-unauthenticated
```

Both approaches:
- Read your existing `.env` file and transform each line into a Cloud Run environment variable setting
- Keep your Docker image clean without any embedded sensitive information
- Allow you to update environment variables without rebuilding the image

### Step 7: Verify Deployment

After deployment, you'll receive a service URL (e.g., `https://actota-api-xxxxx-uc.a.run.app`). Verify your deployment with:

```bash
# Test the health endpoint
curl https://actota-api-xxxxx-uc.a.run.app/health
```

## Managing Environment Variables

### Updating Environment Variables

If you need to update environment variables later:

1. **Using Google Cloud Console:**
   - Navigate to Cloud Run > actota-api > Edit and Deploy New Revision
   - Click "Container, Variables & Secrets" tab
   - Update environment variables as needed
   - Click "Deploy"

2. **Using Command Line with your local .env file:**
   - Update your local `.env` file
   - Run the deployment command from Step 6 again

### Viewing Environment Variables

To view currently configured environment variables:

```bash
gcloud run services describe actota-api --format="yaml(spec.template.spec.containers[0].env)"
```

## Monitoring Your Deployment

### View logs
```bash
# View recent logs
gcloud logging read "resource.type=cloud_run_revision AND resource.labels.service_name=actota-api" --limit 20

# Stream logs in real-time
gcloud logging read "resource.type=cloud_run_revision AND resource.labels.service_name=actota-api" --limit 20 --stream
```

### Monitor performance
Navigate to the Google Cloud Console > Cloud Run > actota-api to view:
- Request volume
- CPU and memory usage
- Error rates
- Response latency

## Troubleshooting

### CORS Issues
If you encounter CORS issues, ensure the frontend URL is correctly configured in the Cloud Run service's environment variables and that your CORS middleware in the application is properly set up.

### Authentication Issues
For OAuth flows (Google Sign-in), make sure:
1. The redirect URIs are correctly configured in both your Google OAuth console and environment variables
2. The `GOOGLE_REDIRECT_URI` matches the exact URL of your deployed Cloud Run service

### Storage Access Issues
If your application cannot access Cloud Storage:
1. Verify the service account attached to your Cloud Run service has the necessary permissions
2. Confirm the bucket names in your environment variables are correct
3. Check Cloud Run logs for any permission-related errors

## Deployment Script

The project includes a deployment script (`deploy.sh`) that automates the entire deployment process. Here's what the script does:

1. Checks for prerequisites (gcloud, docker, .env file)
2. Authenticates with Google Cloud
3. Builds the Docker image
4. Pushes the image to Google Container Registry
5. Formats environment variables from your .env file
6. Deploys to Cloud Run with proper configuration
7. Outputs the service URL and log viewing commands

### Script Usage

```bash
# First time setup
chmod +x deploy.sh  # Make the script executable
nano deploy.sh      # Edit the script to set your PROJECT_ID

# Deploy
./deploy.sh
```

You'll need to modify the following variables at the top of the script:
- `PROJECT_ID`: Your Google Cloud project ID
- `REGION`: Deployment region (default: us-central1)
- `SERVICE_NAME`: Cloud Run service name (default: actota-api)

This script is ideal for regular deployments after your initial setup of buckets and service accounts.

## Continuous Deployment (optional)

For fully automated deployments, you can set up Cloud Build with this configuration:

```yaml
# cloudbuild.yaml
steps:
  # Build the container image
  - name: 'gcr.io/cloud-builders/docker'
    args: ['build', '-t', 'gcr.io/$PROJECT_ID/actota-api', '.']
  
  # Push the container image to Container Registry
  - name: 'gcr.io/cloud-builders/docker'
    args: ['push', 'gcr.io/$PROJECT_ID/actota-api']
  
  # Deploy container image to Cloud Run
  - name: 'gcr.io/google.com/cloudsdktool/cloud-sdk'
    entrypoint: gcloud
    args:
      - 'run'
      - 'deploy'
      - 'actota-api'
      - '--image'
      - 'gcr.io/$PROJECT_ID/actota-api'
      - '--region'
      - 'us-central1'
      - '--platform'
      - 'managed'
      - '--allow-unauthenticated'
      - '--service-account'
      - 'actota-service-account@$PROJECT_ID.iam.gserviceaccount.com'
      # You'll need to specify environment variables separately for CI/CD
      # Consider using Secret Manager for sensitive values

images:
  - 'gcr.io/$PROJECT_ID/actota-api'
```

To set up the Cloud Build trigger:
```bash
gcloud builds triggers create github \
  --repo-name=your-repo-name \
  --branch-pattern=main \
  --build-config=cloudbuild.yaml
```
