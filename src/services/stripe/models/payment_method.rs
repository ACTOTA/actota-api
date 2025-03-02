use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct PaymentMethodRequest {
    pub card: CardDetails,
    pub billing_details: BillingDetails,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct CardDetails {
    pub number: String,
    pub exp_month: u8,
    pub exp_year: u16,
    pub cvc: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct BillingDetails {
    pub name: String,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub address: Option<Address>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Address {
    pub line1: Option<String>,
    pub line2: Option<String>,
    pub city: Option<String>,
    pub state: Option<String>,
    pub postal_code: Option<String>,
    pub country: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct AttachPaymentMethodRequest {
    pub payment_method_id: String,
}