use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize)]
pub struct StripeCustomer {
    pub id: String,
    pub object: String,
    pub address: Option<Address>,
    pub balance: i64,
    pub created: i64,
    pub currency: Option<String>,
    pub default_source: Option<String>,
    pub delinquent: bool,
    pub description: Option<String>,
    pub discount: Option<Discount>,
    pub email: Option<String>,
    pub invoice_prefix: String,
    pub invoice_settings: InvoiceSettings,
    pub livemode: bool,
    pub metadata: HashMap<String, String>,
    pub name: Option<String>,
    pub next_invoice_sequence: i32,
    pub phone: Option<String>,
    pub preferred_locales: Vec<String>,
    pub shipping: Option<Shipping>,
    pub tax_exempt: String,
    pub test_clock: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct InvoiceSettings {
    pub custom_fields: Option<serde_json::Value>,
    pub default_payment_method: Option<String>,
    pub footer: Option<String>,
    pub rendering_options: Option<serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Address {
    pub city: Option<String>,
    pub country: Option<String>,
    pub line1: Option<String>,
    pub line2: Option<String>,
    pub postal_code: Option<String>,
    pub state: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Shipping {
    pub address: Option<Address>,
    pub name: Option<String>,
    pub phone: Option<String>,
}

// Placeholder for Discount - expand as needed
#[derive(Debug, Serialize, Deserialize)]
pub struct Discount {}
