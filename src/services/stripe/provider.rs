use actix_web::HttpResponse;
use serde::{Deserialize, Serialize};
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

#[derive(Deserialize, Serialize, Debug)]
struct PaymentMethodList {
    data: Vec<PaymentMethod>, // This is what we want!
    has_more: bool,
    url: String,
    object: String,
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

        if res.status().is_success() {
            let body = res.text().await.unwrap();
            let payment_list = serde_json::from_str::<PaymentMethodList>(&body).unwrap();
            println!("vec: {:?}", payment_list);
            let payment_methods: Vec<PaymentMethod> = payment_list.data;

            return Ok(payment_methods);
        }

        Err(PaymentError::NotFound)
    }

    async fn attach_payment_method(
        &self,
        customer_id: String,
        payment_id: String,
    ) -> Result<HttpResponse, PaymentError> {
        let api_key = std::env::var("STRIPE_SECRET_KEY").unwrap();

        let client = reqwest::Client::new();
        let url = format!(
            "https://api.stripe.com/v1/payment_methods/{}/attach",
            payment_id
        );

        // Create form parameters to include the customer ID
        let params = [("customer", customer_id)];

        let res = match client
            .post(&url) // Changed to POST from GET
            .header("Authorization", format!("Bearer {}", api_key))
            .form(&params) // Added form parameters
            .send()
            .await
        {
            Ok(res) => res,
            Err(_) => return Err(PaymentError::InternalServerError),
        };

        if res.status().is_success() {
            return Ok(HttpResponse::Ok().body("Payment method added"));
        } else {
            return Ok(HttpResponse::InternalServerError().body("Failed to add payment method"));
        }
    }

    async fn detach_payment_method(
        &self,
        customer_id: String,
        payment_id: String,
    ) -> Result<HttpResponse, PaymentError> {
        let api_key = std::env::var("STRIPE_SECRET_KEY").unwrap();

        let client = reqwest::Client::new();
        let url = format!(
            "https://api.stripe.com/v1/payment_methods/{}/detach",
            payment_id
        );

        println!("Detaching payment method: {} from customer: {}", payment_id, customer_id);
        println!("URL: {}", url);

        // For detach operation in Stripe, we don't need to send the customer_id
        // The payment method already knows which customer it's attached to
        let res = match client
            .post(&url)  // Stripe uses POST not DELETE for the detach operation
            .header("Authorization", format!("Bearer {}", api_key))
            .send()
            .await
        {
            Ok(res) => res,
            Err(err) => {
                eprintln!("Request error: {:?}", err);
                return Err(PaymentError::InternalServerError);
            }
        };

        // Get the response body for better error logging
        let status = res.status();
        let body = match res.text().await {
            Ok(body) => body,
            Err(err) => {
                eprintln!("Failed to read response body: {:?}", err);
                return Err(PaymentError::InternalServerError);
            }
        };
        
        println!("Response status: {}, body: {}", status, body);

        if status.is_success() {
            return Ok(HttpResponse::Ok().body("Payment method deleted"));
        } else {
            eprintln!("Stripe error: {}", body);
            return Ok(HttpResponse::InternalServerError().body(format!("Failed to delete payment method: {}", body)));
        }
    }
}
