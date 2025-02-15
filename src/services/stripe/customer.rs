use std::str::FromStr;

use stripe::{CreateCustomer, Customer, CustomerId};

use crate::services::payment::interface::{CustomerError, CustomerOperations};

use super::models::customer::CustomerData;

pub struct StripeCustomer {
    client: stripe::Client,
}

impl StripeCustomer {
    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            client: stripe::Client::new(api_key.into()),
        }
    }
}

impl CustomerOperations for StripeCustomer {
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
}
