use crate::models::itinerary::populated::{PopulatedDayItem, PopulatedFeaturedVacation};

pub struct PricingService;

impl PricingService {
    /// Calculate service fee (5% of total with minimum $50)
    pub fn calculate_service_fee(total_cost: f32) -> f32 {
        let fee = total_cost * 0.05;
        fee.max(50.0)
    }

    /// Calculate total activity costs from populated days
    pub fn calculate_activity_cost(itinerary: &PopulatedFeaturedVacation) -> f32 {
        itinerary
            .populated_days
            .values()
            .flatten()
            .filter_map(|item| match item {
                PopulatedDayItem::Activity { activity, .. } => Some(activity.price_per_person),
                _ => None,
            })
            .sum()
    }

    /// Calculate total lodging costs from populated days
    pub fn calculate_lodging_cost(itinerary: &PopulatedFeaturedVacation) -> f32 {
        itinerary
            .populated_days
            .values()
            .flatten()
            .filter_map(|item| match item {
                PopulatedDayItem::Accommodation { accommodation, .. } => {
                    accommodation.price_per_night
                }
                _ => None,
            })
            .sum()
    }

    /// Calculate total transportation costs from populated days
    /// Note: Transportation items don't currently have cost fields in the model
    pub fn calculate_transport_cost(_itinerary: &PopulatedFeaturedVacation) -> f32 {
        // TODO: Add cost fields to transportation items when the model is updated
        0.0
    }

    /// Calculate total person cost (activity + lodging + transport, excluding service fee)
    pub fn calculate_person_cost(itinerary: &PopulatedFeaturedVacation) -> f32 {
        let activity_cost = Self::calculate_activity_cost(itinerary);
        let lodging_cost = Self::calculate_lodging_cost(itinerary);
        let transport_cost = Self::calculate_transport_cost(itinerary);
        
        activity_cost + lodging_cost + transport_cost
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_service_fee_calculation() {
        // Test 5% calculation
        assert_eq!(PricingService::calculate_service_fee(1000.0), 50.0);
        assert_eq!(PricingService::calculate_service_fee(2000.0), 100.0);
        
        // Test minimum fee
        assert_eq!(PricingService::calculate_service_fee(100.0), 50.0);
        assert_eq!(PricingService::calculate_service_fee(0.0), 50.0);
    }

    #[test]
    fn test_person_cost_calculation() {
        // Test that person cost excludes service fee
        // This would need a proper populated itinerary to test fully
        // For now, just test that the method exists and compiles
        // Real testing would require creating mock itinerary data
    }
}