# server

## Project setup

1. Build the docker image
```
docker build -t actota-api .
```

2. Upload the docker image to the Google Cloud Container Registry
```
docker tag actota-api actota/actota-api
docker push actota/actota-api
```

3. Deploy the image to the Google Cloud
```
gcloud run deploy actota-api \
  --image actota/actota-api \
  --platform managed \
  --region us-central1 \
  --allow-unauthenticated
```

