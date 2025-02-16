

pub struct StripeProvider {
    pub client: stripe::Client,
}

impl StripeProvider {
    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            client: stripe::Client::new(api_key.into()),
        }
    }
}
