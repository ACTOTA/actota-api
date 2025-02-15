use stripe::Customer;

use crate::services::payment::interface::{CustomerError, CustomerOperations};

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
    fn create_customer(&self, customer: Customer) -> Result<Customer, CustomerError> {
        todo!()
    }
    fn get_customer(&self, customer_id: String) -> Result<Customer, CustomerError> {
        todo!()
    }
    fn update_customer(
        &self,
        customer_id: String,
        customer: Customer,
    ) -> Result<Customer, CustomerError> {
        todo!()
    }
}
