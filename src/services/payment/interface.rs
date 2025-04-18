use crate::services::stripe::models::customer::CustomerData;
use actix_web::HttpResponse;
use stripe::PaymentMethod;

pub enum CustomerError {
    NotFound,
    InternalServerError,
}
#[derive(Debug)]
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

    async fn attach_payment_method(
        &self,
        customer_id: String,
        payment_id: String,
    ) -> Result<HttpResponse, PaymentError>;

    async fn detach_payment_method(
        &self,
        customer_id: String,
        payment_id: String,
    ) -> Result<HttpResponse, PaymentError>;
}
