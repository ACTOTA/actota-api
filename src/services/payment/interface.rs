use crate::services::stripe::models::customer::CustomerData;
use stripe::PaymentMethod;

pub enum CustomerError {
    NotFound,
    InternalServerError,
}
pub enum PaymentError {
    NotFound,
    InternalServerError,
}

pub trait PaymentOperations {
    async fn get_customer(&self, customer_id: &str) -> Result<CustomerData, CustomerError>;
    async fn create_customer(&self, customer: CustomerData) -> Result<CustomerData, CustomerError>;
    async fn update_customer(
        &self,
        customer_id: String,
        customer: CustomerData,
    ) -> Result<CustomerData, CustomerError>;

    async fn get_payment_method(&self, payment_id: String) -> Result<PaymentMethod, PaymentError>;
    async fn get_cust_payment_methods(
        &self,
        customer_id: String,
    ) -> Result<Vec<PaymentMethod>, PaymentError>;

    async fn create_payment_method(
        &self,
        payment: PaymentMethod,
    ) -> Result<PaymentMethod, PaymentError>;
    async fn update_payment_method(
        &self,
        payment_id: String,
        payment: PaymentMethod,
    ) -> Result<PaymentMethod, PaymentError>;
}
