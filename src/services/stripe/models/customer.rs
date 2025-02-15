use stripe::{
    Address, CreateCustomer, CreateCustomerCashBalance, CreateCustomerShipping, Currency, Customer,
    CustomerInvoiceSettings, CustomerTax, CustomerTaxExempt, Discount, Metadata, PaymentMethodId,
    TaxIdData, Timestamp,
};

pub struct CustomerData {
    pub id: Option<String>,
    pub address: Option<Address>,
    pub balance: Option<i64>,
    pub cash_balance: Option<CreateCustomerCashBalance>,
    pub created: Option<Timestamp>,
    pub currency: Option<Currency>,
    pub default_source: Option<String>,
    pub delinquent: Option<bool>,
    pub description: Option<String>,
    pub discount: Option<Discount>,
    pub email: Option<String>,
    pub invoice_credit_balance: Option<i64>,
    pub invoice_prefix: Option<String>,
    pub invoice_settings: Option<CustomerInvoiceSettings>,
    pub livemode: Option<bool>,
    pub metadata: Option<Metadata>,
    pub name: Option<String>,
    pub next_invoice_sequence: Option<i64>,
    pub payment_method: Option<PaymentMethodId>,
    pub phone: Option<String>,
    pub preferred_locales: Option<Vec<String>>,
    pub shipping: Option<CreateCustomerShipping>,
    pub tax: Option<CustomerTax>,
    pub tax_exempt: Option<CustomerTaxExempt>,
    pub tax_id_data: Option<Vec<TaxIdData>>,
    pub test_clock: Option<String>,
    pub validate: Option<bool>,
}

impl From<Customer> for CustomerData {
    fn from(customer: Customer) -> Self {
        Self {
            id: Some(customer.id.to_string()),
            address: customer.address,
            balance: customer.balance,
            cash_balance: None, // Convert from CashBalance if needed
            created: customer.created,
            currency: customer.currency,
            default_source: customer.default_source.map(|s| s.id().to_string()),
            delinquent: customer.delinquent,
            description: customer.description,
            discount: customer.discount,
            email: customer.email,
            invoice_credit_balance: customer.invoice_credit_balance,
            invoice_prefix: customer.invoice_prefix,
            invoice_settings: None, // Convert if needed
            livemode: customer.livemode,
            metadata: customer.metadata,
            name: customer.name,
            next_invoice_sequence: customer.next_invoice_sequence,
            payment_method: None,
            phone: customer.phone,
            preferred_locales: customer.preferred_locales,
            shipping: None, // Convert if needed
            tax: customer.tax,
            tax_exempt: customer.tax_exempt,
            tax_id_data: None,
            test_clock: customer.test_clock.map(|t| t.id().to_string()),
            validate: None,
        }
    }
}

impl<'a> From<&'a CustomerData> for CreateCustomer<'a> {
    fn from(data: &'a CustomerData) -> Self {
        let mut customer = CreateCustomer::new();
        customer.address = data.address.clone();
        customer.balance = data.balance;
        customer.cash_balance = data.cash_balance.clone();
        customer.description = data.description.as_deref();
        customer.email = data.email.as_deref();
        customer.expand = &[]; // Set expand fields if needed
        customer.invoice_prefix = data.invoice_prefix.as_deref();
        customer.invoice_settings = data.invoice_settings.clone();
        customer.metadata = data.metadata.clone();
        customer.name = data.name.as_deref();
        customer.next_invoice_sequence = data.next_invoice_sequence;
        customer.payment_method = data.payment_method.clone();
        customer.phone = data.phone.as_deref();
        customer.preferred_locales = data.preferred_locales.clone();
        customer.shipping = data.shipping.clone();
        customer.tax = None;
        customer.tax_exempt = None;
        customer.tax_id_data = data.tax_id_data.clone();
        customer.test_clock = data.test_clock.as_deref();
        customer.validate = data.validate;

        customer
    }
}
