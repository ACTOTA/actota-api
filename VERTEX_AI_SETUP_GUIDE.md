# Vertex AI Search Setup Guide

## ✅ Completed Automatically

The automated setup script has completed the following for both **actota-uat** and **actota-prod** projects:

### Infrastructure Setup
- ✅ Enabled required APIs:
  - Discovery Engine API
  - Cloud Storage API  
  - Cloud Run API
  - Secret Manager API

### Cloud Storage Buckets
- ✅ Created storage buckets:
  - `{project-id}-activities` - For Vertex AI Search data
  - `{project-id}-activity-media` - For activity images/media
  - `{project-id}-itineraries` - For itinerary data
  - `{project-id}-profile-pictures` - For user profile pictures
  - `{project-id}-transportation-media` - For transportation images

### IAM Permissions
- ✅ Created `actota-api` service account (if it didn't exist)
- ✅ Granted necessary permissions:
  - `roles/storage.admin` - Full storage access
  - `roles/secretmanager.secretAccessor` - Access to secrets
  - `roles/discoveryengine.admin` - Full Vertex AI Search access
  - `roles/discoveryengine.viewer` - Read access to search
  - `roles/run.admin` - Cloud Run management

## 🔧 Manual Steps Required

### 1. Create Vertex AI Search Data Stores

For **both** actota-uat and actota-prod projects:

1. Go to [Google Cloud Console - Vertex AI Search](https://console.cloud.google.com/ai/search)

2. **Create Search App for UAT:**
   - Switch to `actota-uat` project
   - Click "Create App"
   - Choose "Search" as app type
   - Select "Generic" for content type
   - Choose "Unstructured data" 
   - App name: `actota-activities-search-uat`
   - Location: `global`
   - Click "Continue"

3. **Create Data Store for UAT:**
   - Data store name: `activities-data-store-uat`
   - Data type: "Unstructured documents"
   - Location: `global`
   - Source: "Cloud Storage"
   - Bucket: `gs://actota-uat-activities`
   - Click "Create"

4. **Repeat for Production:**
   - Switch to `actota-prod` project
   - App name: `actota-activities-search-prod`
   - Data store name: `activities-data-store-prod`
   - Bucket: `gs://actota-prod-activities`

5. **Note the Data Store IDs:**
   After creation, you'll see data store IDs like:
   - UAT: `projects/403386996094/locations/global/dataStores/activities-data-store-uat_xxxxxxxx`
   - Prod: `projects/88694943961/locations/global/dataStores/activities-data-store-prod_xxxxxxxx`

### 2. Copy Activity Data from Main Project

Get the current data from the main actota project and copy it to UAT/Prod:

```bash
# Switch to main project and export data
gcloud config set project actota

# Copy activities data to UAT
gsutil -m cp -r gs://actota-activities/* gs://actota-uat-activities/

# Copy activities data to Production  
gsutil -m cp -r gs://actota-activities/* gs://actota-prod-activities/

# Copy media files to UAT
gsutil -m cp -r gs://actota-activity-media/* gs://actota-uat-activity-media/

# Copy media files to Production
gsutil -m cp -r gs://actota-activity-media/* gs://actota-prod-activity-media/
```

### 3. Update GitHub Environment Variables

Add these variables to your GitHub repository environments:

#### For UAT Environment:
```yaml
GOOGLE_CLOUD_PROJECT_ID: "actota-uat"
VERTEX_SEARCH_LOCATION: "global"
VERTEX_SEARCH_DATA_STORE_ID: "activities-data-store-uat_xxxxxxxx"  # Replace with actual ID
VERTEX_SEARCH_SERVING_CONFIG: "default_config"

# Update existing bucket variables for UAT
ITINERARY_BUCKET: "actota-uat-itineraries"
PROFILE_PIC_BUCKET: "actota-uat-profile-pictures"
ACTIVITY_BUCKET: "actota-uat-activity-media"
CLOUD_STORAGE_URL: "https://storage.googleapis.com"
```

#### For Production Environment:
```yaml
GOOGLE_CLOUD_PROJECT_ID: "actota-prod"
VERTEX_SEARCH_LOCATION: "global"
VERTEX_SEARCH_DATA_STORE_ID: "activities-data-store-prod_xxxxxxxx"  # Replace with actual ID
VERTEX_SEARCH_SERVING_CONFIG: "default_config"

# Update existing bucket variables for Production
ITINERARY_BUCKET: "actota-prod-itineraries"
PROFILE_PIC_BUCKET: "actota-prod-profile-pictures"
ACTIVITY_BUCKET: "actota-prod-activity-media"
CLOUD_STORAGE_URL: "https://storage.googleapis.com"
```

### 4. Update GitHub Secrets (if needed)

Ensure these secrets exist for both environments:
- `MONGODB_URI` - Database connection for UAT
- `MONGODB_URI_UAT` - UAT-specific database
- All other existing secrets should work as-is

### 5. Test the Configuration

After deployment, test Vertex AI Search integration:

```bash
# Test UAT endpoint
curl -X POST https://actota-api-uat-{project-number}.us-central1.run.app/itineraries/search \
  -H "Content-Type: application/json" \
  -d '{
    "locations": ["Denver"],
    "activities": ["Adventure"],
    "adults": 2
  }'

# Test Production endpoint  
curl -X POST https://actota-api-prod-{project-number}.us-central1.run.app/itineraries/search \
  -H "Content-Type: application/json" \
  -d '{
    "locations": ["Denver"],
    "activities": ["Adventure"], 
    "adults": 2
  }'
```

### 6. Monitor Vertex AI Search Usage

- Check the [Vertex AI Search console](https://console.cloud.google.com/ai/search) for search analytics
- Monitor Cloud Storage usage for data store buckets
- Review Cloud Run logs for any Vertex AI Search errors

## 🔍 Troubleshooting

### Common Issues:

1. **"Data store not found" errors:**
   - Verify `VERTEX_SEARCH_DATA_STORE_ID` environment variable is correct
   - Check that the data store was created in the same location (`global`)

2. **Authentication errors:**
   - Ensure the Cloud Run service is using the `actota-api` service account
   - Verify IAM permissions are correctly assigned

3. **No search results:**
   - Check that activity data was uploaded to the storage buckets
   - Verify the data store has finished indexing (can take 10-30 minutes)
   - Test with simple search queries first

4. **Storage access errors:**
   - Verify bucket names in environment variables match actual bucket names
   - Check that buckets have correct public access settings for media files

## 📊 Resources Created

### actota-uat project (403386996094):
- Buckets: `actota-uat-*`
- Service Account: `actota-api@actota-uat.iam.gserviceaccount.com`
- APIs: Discovery Engine, Storage, Run, Secret Manager

### actota-prod project (88694943961):
- Buckets: `actota-prod-*`  
- Service Account: `actota-api@actota-prod.iam.gserviceaccount.com`
- APIs: Discovery Engine, Storage, Run, Secret Manager

Both environments are now ready for Vertex AI Search integration once the manual steps are completed!
