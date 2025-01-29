# ACTOTA Server
*This API is used by the ACTOTA application. It includes the core functionality and calls to access data in the database.*

## Prerequisites
1. [Rust](https://www.rust-lang.org/)
2. [Cargo](https://crates.io/)
> ***Installing Rust with `rustup` will install all necessary tooling.***
> - Install [`rustup`](https://www.rust-lang.org/tools/install)

## Project setup
- Requires a `.env` file in the root directory. Reference .env.example for the required environment variables.
- Requires `credentials/service-account.json` for Google Cloud Storage access.

### Run locally
To run, simply run the following command:
```
cargo run
```
> Additional Cargo commands can be found here: https://doc.rust-lang.org/cargo/commands/index.html

### Deploy to Cloud via Docker
**Note:** Ensure that you have the Google Cloud SDK installed and configured.

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

