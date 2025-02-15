use stripe::Customer;

pub enum CustomerError {
    NotFound,
    InternalServerError,
}

pub trait CustomerOperations {
    fn get_customer(&self, customer_id: String) -> Result<Customer, CustomerError>;
    fn create_customer(&self, customer: Customer) -> Result<Customer, CustomerError>;
    fn update_customer(
        &self,
        customer_id: String,
        customer: Customer,
    ) -> Result<Customer, CustomerError>;
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
