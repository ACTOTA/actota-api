use actix_web::{web, HttpResponse, Responder};
use serde::{Deserialize, Serialize};
use std::{str::FromStr, sync::Arc};

use crate::middleware::auth::Claims;

#[derive(Serialize, Deserialize)]
pub struct PaymentIntentInput {
    user_id: String,
    amount: i64,
    customer_id: String,
    payment_method_id: String,
}

pub async fn create_payment_intent(
    claims: Claims,
    data: web::Data<Arc<stripe::Client>>,
    input: web::Json<PaymentIntentInput>,
) -> impl Responder {
    if claims.user_id != input.user_id {
        return HttpResponse::Forbidden().body("Forbidden");
    }

    let input = input.into_inner();

    let amount = input.amount;
    let customer_id = input.customer_id;
    let payment_method_id = input.payment_method_id;

    let mut create_intent = stripe::CreatePaymentIntent::new(amount, stripe::Currency::USD);

    // Add customer and payment method
    create_intent.customer =
        Some(stripe::CustomerId::from_str(&customer_id).expect("Invalid customer ID"));
    create_intent.payment_method = Some(
        stripe::PaymentMethodId::from_str(&payment_method_id).expect("Invalid payment method ID"),
    );

    // Create the payment intent using the injected client
    match stripe::PaymentIntent::create(data.as_ref(), create_intent).await {
        Ok(intent) => HttpResponse::Ok().json(intent),
        Err(e) => {
            println!("Error creating payment intent: {:?}", e);
            HttpResponse::InternalServerError()
                .body(format!("Failed to create payment intent: {}", e))
        }
    }
}
