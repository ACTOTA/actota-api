use actix_web::{web, HttpRequest, HttpResponse, Responder};
use serde::{Deserialize, Serialize};
use std::{str::FromStr, sync::Arc};
use stripe::{CapturePaymentIntent, EventObject, EventType, Webhook};

use crate::middleware::auth::Claims;

#[derive(Serialize, Deserialize)]
pub struct PaymentIntentInput {
    user_id: String,
    amount: i64,
    customer_id: String,
    payment_method_id: String,
}

#[derive(Serialize, Deserialize)]
pub struct CapturePayment {
    user_id: String,
    payment_intent_id: String,
}

#[derive(Clone)]
pub struct StripeConfig {
    pub webhook_secret: String,
}

pub async fn create_payment_intent(
    claims: Claims,
    data: web::Data<Arc<stripe::Client>>,
    input: web::Json<PaymentIntentInput>,
) -> impl Responder {
    println!("Creating payment intent...");

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
    // Manual, as we capture on the frontend
    create_intent.capture_method = Some(stripe::PaymentIntentCaptureMethod::Manual);

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

pub async fn capture_payment(
    claims: Claims,
    data: web::Data<Arc<stripe::Client>>,
    input: web::Json<CapturePayment>,
) -> impl Responder {
    println!("Capturing payment...");
    if claims.user_id != input.user_id {
        return HttpResponse::Forbidden().body("Forbidden");
    }

    let input = input.into_inner();
    let payment_intent_id = input.payment_intent_id;

    // First retrieve the payment intent to check its status
    match stripe::PaymentIntent::retrieve(
        data.as_ref(),
        &stripe::PaymentIntentId::from_str(&payment_intent_id).expect("Invalid payment intent ID"),
        &[],
    )
    .await
    {
        Ok(intent) => {
            // Check if the payment intent is in a capturable state
            if intent.status != stripe::PaymentIntentStatus::RequiresCapture {
                return HttpResponse::BadRequest().body(format!(
                    "Payment intent is not in a capturable state. Current status: {:?}",
                    intent.status
                ));
            }

            // Proceed with capture since status is correct
            match stripe::PaymentIntent::capture(
                data.as_ref(),
                &payment_intent_id,
                CapturePaymentIntent::default(),
            )
            .await
            {
                Ok(captured_intent) => HttpResponse::Ok().json(captured_intent),
                Err(e) => {
                    println!("Error capturing payment: {:?}", e);
                    HttpResponse::InternalServerError()
                        .body(format!("Failed to capture payment: {}", e))
                }
            }
        }
        Err(e) => {
            println!("Error retrieving payment intent: {:?}", e);
            HttpResponse::InternalServerError()
                .body(format!("Failed to retrieve payment intent: {}", e))
        }
    }
}

pub async fn handle_stripe_webhook(
    req: HttpRequest,
    payload: web::Bytes,
    stripe_config: web::Data<StripeConfig>,
) -> impl Responder {
    // Get the Stripe-Signature header
    let signature = match req.headers().get("stripe-signature") {
        Some(sig) => sig.to_str().unwrap_or(""),
        None => {
            return HttpResponse::BadRequest().body("Missing stripe-signature header");
        }
    };

    // Verify the webhook signature and parse the event
    let payload_str = match String::from_utf8(payload.to_vec()) {
        Ok(s) => s,
        Err(_) => {
            return HttpResponse::BadRequest().body("Invalid payload encoding");
        }
    };

    let event =
        match Webhook::construct_event(&payload_str, signature, &stripe_config.webhook_secret) {
            Ok(event) => event,
            Err(e) => {
                println!("Webhook error: {:?}", e);
                return HttpResponse::BadRequest().body(format!("Webhook error: {}", e));
            }
        };

    // Check the event type and handle accordingly
    match event.type_ {
        EventType::PaymentIntentSucceeded => {
            if let EventObject::PaymentIntent(_payment_intent) = event.data.object {
                HttpResponse::Ok().json(serde_json::json!({ "received": true }))
            } else {
                HttpResponse::BadRequest().body("Invalid payment intent object")
            }
        }

        EventType::PaymentIntentPaymentFailed => {
            if let EventObject::PaymentIntent(payment_intent) = event.data.object {
                println!("Payment failed: {}", payment_intent.id);

                // Update your database to mark booking as failed

                HttpResponse::Ok().json(serde_json::json!({ "received": true }))
            } else {
                HttpResponse::BadRequest().body("Invalid payment intent object")
            }
        }

        EventType::ChargeSucceeded => {
            if let EventObject::Charge(charge) = event.data.object {
                println!("Charge succeeded: {}", charge.id);
                // Handle successful charge if needed
                HttpResponse::Ok().json(serde_json::json!({ "received": true }))
            } else {
                HttpResponse::BadRequest().body("Invalid charge object")
            }
        }

        // Handle other event types as needed
        _ => {
            println!("Unhandled event type: {:?}", event.type_);
            HttpResponse::Ok().json(serde_json::json!({ "received": true }))
        }
    }
}
