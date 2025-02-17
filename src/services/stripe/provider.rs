use std::str::FromStr;

use stripe::{CustomerId, PaymentMethod};

use crate::services::payment::interface::{CustomerError, PaymentError, PaymentOperations};

use super::models::customer::CustomerData;

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


impl PaymentOperations for StripeProvider {
    async fn create_customer(&self, customer: CustomerData) -> Result<CustomerData, CustomerError> {
        let create_customer: stripe::CreateCustomer<'_> = (&customer).into();

        match stripe::Customer::create(&self.client, create_customer).await {
            Ok(stripe_customer) => Ok(CustomerData::from(stripe_customer)),
            Err(_) => Err(CustomerError::InternalServerError),
        }
    }

    async fn get_customer(&self, customer_id: &str) -> Result<CustomerData, CustomerError> {
        let cust_id = CustomerId::from_str(customer_id).map_err(|_| CustomerError::NotFound)?;
        let expand = &[];
        match stripe::Customer::retrieve(&self.client, &cust_id, expand).await {
            Ok(customer) => Ok(customer.into()),
            Err(_) => Err(CustomerError::NotFound),
        }
    }

    async fn update_customer(
        &self,
        customer_id: String,
        customer: CustomerData,
    ) -> Result<CustomerData, CustomerError> {
        todo!()
    }
    async fn get_payment_method(&self, payment_id: String) -> Result<PaymentMethod, PaymentError> {
        todo!()
    }

    async fn get_cust_payment_methods(
        &self,
        customer_id: String,
    ) -> Result<Vec<PaymentMethod>, PaymentError> {
        let api_key = std::env::var("STRIPE_SECRET_KEY").unwrap();

        println!("api_key: {}", api_key);

        let client = reqwest::Client::new();
        let url = format!(
            "https://api.stripe.com/v1/customers/{}/payment_methods",
            customer_id
        );

        println!("url: {}", url);

        let res = match client
            .get(&url)
            .header("Authorization", format!("Bearer {}", api_key))
            .send()
            .await
        {
            Ok(res) => res,
            Err(_) => return Err(PaymentError::InternalServerError),
        };

        println!("res: {:?}", res);

        if res.status().is_success() {
            let body = res.text().await.unwrap();
            let payment_methods: Vec<PaymentMethod> = match serde_json::from_str(&body) {
                Ok(payment_methods) => payment_methods,
                Err(_) => return Ok(Vec::new()),
            };

            return Ok(payment_methods);
        }

        Err(PaymentError::NotFound)
    }

    async fn create_payment_method(
        &self,
        payment: PaymentMethod,
    ) -> Result<PaymentMethod, PaymentError> {
        todo!()
    }

    async fn update_payment_method(
        &self,
        payment_id: String,
        payment: PaymentMethod,
    ) -> Result<PaymentMethod, PaymentError> {
        todo!()
    }
}

