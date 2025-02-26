# ACTOTA Server
*This API is used by the ACTOTA application. It includes the core functionality and calls to access data in the database.*

## Prerequisites
1. [Rust](https://www.rust-lang.org/)
2. [Cargo](https://crates.io/)
> ***Installing Rust with `rustup` will install all necessary tooling.***
> - Install [`rustup`](https://www.rust-lang.org/tools/install)

## Project setup
- Install Rust: https://www.rust-lang.org/tools/install
- Requires a `.env` file in the root directory. Reference .env.example for the required environment variables.
- Requires `credentials/service-account.json` for Google Cloud Storage access.

### Run locally
To run, simply run the following command:
```
cargo run
```
> Additional Cargo commands can be found here: https://doc.rust-lang.org/cargo/commands/index.html

### Deploy to Cloud via Docker
```
# Build with platform specification
docker buildx build --platform linux/amd64 -t gcr.io/actota/actota-api .

docker buildx create --name mybuilder --use

# Build and push directly
docker buildx build --platform linux/amd64 \
  -t gcr.io/actota/actota-api \
  --push .
```
**Note:** Ensure that you have the Google Cloud SDK installed and configured.

1. Build the docker image
```
docker build -t actota-api .
```
2. Authenticate with Google Cloud
```
gcloud auth configure-docker
```

3. Upload the docker image to the Google Cloud Container Registry
```
docker tag actota-api gcr.io/actota/actota-api
docker build --platform linux/amd64 -t gcr.io/actota/actota-api .
docker push gcr.io/actota/actota-api
```

4. Deploy the image to the Google Cloud
```
gcloud run deploy actota-api \
  --image gcr.io/actota/actota-api \
  --platform managed \
  --region us-central1 \
  --allow-unauthenticated
```

