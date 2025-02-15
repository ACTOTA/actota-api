use crate::services::stripe::models::customer::CustomerData;
use stripe::Customer;

pub enum CustomerError {
    NotFound,
    InternalServerError,
}

pub trait CustomerOperations {
    async fn get_customer(&self, customer_id: &str) -> Result<CustomerData, CustomerError>;
    async fn create_customer(&self, customer: CustomerData) -> Result<CustomerData, CustomerError>;
    async fn update_customer(
        &self,
        customer_id: String,
        customer: CustomerData,
    ) -> Result<CustomerData, CustomerError>;
}

// pub trait PaymentOperations {
// fn get_payment_method(&self, payment_id: String) -> Result<PaymentMethod, Error>;
// fn get_all_payment_methods(&self, customer_id: String) -> Result<Vec<PaymentMethod>, Error>;
// fn create_payment_method(&self, payment: PaymentMethod) -> Result<PaymentMethod, Error>;
// fn update_payment_method(
//     &self,
//         payment_id: String,
//         payment: PaymentMethod,
//     ) -> Result<PaymentMethod, Error>;
// }
