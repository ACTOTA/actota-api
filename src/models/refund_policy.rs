use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc, Duration};

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct RefundPolicy {
    pub name: String,
    pub description: String,
    pub full_refund_window_hours: i64,
    pub partial_refund_window_hours: i64,
    pub partial_refund_percentage: f64,
}

impl RefundPolicy {
    pub fn calculate_refund_amount(&self, payment_amount: i64, booking_time: DateTime<Utc>) -> i64 {
        let now = Utc::now();
        let booking_age = now.signed_duration_since(booking_time);
        
        // Convert hours to chrono::Duration for comparison
        let full_refund_duration = Duration::hours(self.full_refund_window_hours);
        let partial_refund_duration = Duration::hours(self.partial_refund_window_hours);
        
        if booking_age <= full_refund_duration {
            // Full refund
            payment_amount
        } else if booking_age <= partial_refund_duration {
            // Partial refund
            ((payment_amount as f64) * self.partial_refund_percentage / 100.0) as i64
        } else {
            // No refund
            0
        }
    }
}

impl Default for RefundPolicy {
    fn default() -> Self {
        Self {
            name: "Standard".to_string(),
            description: "Full refund within 24 hours, 50% refund within 72 hours, no refund after".to_string(),
            full_refund_window_hours: 24,
            partial_refund_window_hours: 72,
            partial_refund_percentage: 50.0,
        }
    }
}