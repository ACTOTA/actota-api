use crate::models::{activity::Activity, itinerary::base::FeaturedVacation, search::SearchItinerary};
use futures::TryStreamExt;
use mongodb::{bson::oid::ObjectId, Client};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchWeights {
    /// Weight for location matching (start/end cities)
    pub location_weight: f32,
    /// Weight for activity matching
    pub activity_weight: f32,
    /// Weight for group size compatibility
    pub group_size_weight: f32,
    /// Weight for lodging/accommodation matching
    pub lodging_weight: f32,
    /// Weight for transportation matching
    pub transportation_weight: f32,
    /// Weight for trip pace matching
    pub trip_pace_weight: f32,
    /// Minimum score required to include in results
    pub minimum_score: f32,
}

impl Default for SearchWeights {
    fn default() -> Self {
        Self {
            location_weight: 35.0,
            activity_weight: 30.0,
            group_size_weight: 15.0,
            lodging_weight: 5.0,
            transportation_weight: 3.0,
            trip_pace_weight: 12.0,
            minimum_score: 15.0,
        }
    }
}

impl SearchWeights {
    /// Create weights from environment variables or use defaults
    pub fn from_env() -> Self {
        let defaults = Self::default();

        Self {
            location_weight: std::env::var("SEARCH_LOCATION_WEIGHT")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(defaults.location_weight),
            activity_weight: std::env::var("SEARCH_ACTIVITY_WEIGHT")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(defaults.activity_weight),
            group_size_weight: std::env::var("SEARCH_GROUP_SIZE_WEIGHT")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(defaults.group_size_weight),
            lodging_weight: std::env::var("SEARCH_LODGING_WEIGHT")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(defaults.lodging_weight),
            transportation_weight: std::env::var("SEARCH_TRANSPORT_WEIGHT")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(defaults.transportation_weight),
            trip_pace_weight: std::env::var("SEARCH_TRIP_PACE_WEIGHT")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(defaults.trip_pace_weight),
            minimum_score: std::env::var("SEARCH_MIN_SCORE")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(defaults.minimum_score),
        }
    }
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct ScoredItinerary {
    pub itinerary: FeaturedVacation,
    pub total_score: f32,
    pub score_breakdown: ScoreBreakdown,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ScoreBreakdown {
    pub location_score: f32,
    pub activity_score: f32,
    pub group_size_score: f32,
    pub lodging_score: f32,
    pub transportation_score: f32,
    pub trip_pace_score: f32,
}

#[derive(Default)]
pub struct SearchScorer {
    pub weights: SearchWeights,
}

pub struct AsyncSearchScorer {
    pub weights: SearchWeights,
    pub client: Arc<Client>,
}

impl SearchScorer {
    pub fn new() -> Self {
        let weights = SearchWeights::from_env();
        println!("SearchScorer initialized with weights: {:?}", weights);
        Self { weights }
    }

    pub fn with_weights(weights: SearchWeights) -> Self {
        Self { weights }
    }

    /// Score an itinerary against search criteria
    pub fn score_itinerary(
        &self,
        itinerary: &FeaturedVacation,
        search: &SearchItinerary,
    ) -> ScoredItinerary {
        let location_score = self.score_location(itinerary, search);
        let activity_score = self.score_activities(itinerary, search);
        let group_size_score = self.score_group_size(itinerary, search);
        let lodging_score = self.score_lodging(itinerary, search);
        let transportation_score = self.score_transportation(itinerary, search);
        let trip_pace_score = self.score_trip_pace(itinerary, search);

        let total_score = location_score
            + activity_score
            + group_size_score
            + lodging_score
            + transportation_score
            + trip_pace_score;

        ScoredItinerary {
            itinerary: itinerary.clone(),
            total_score,
            score_breakdown: ScoreBreakdown {
                location_score,
                activity_score,
                group_size_score,
                lodging_score,
                transportation_score,
                trip_pace_score,
            },
        }
    }

    /// Score location matching
    fn score_location(&self, itinerary: &FeaturedVacation, search: &SearchItinerary) -> f32 {
        if let Some(locations) = &search.locations {
            if locations.is_empty() {
                return 0.0;
            }

            let mut best_score: f32 = 0.0;

            for search_location in locations {
                // Parse search location (assuming "City, State" format)
                let search_parts: Vec<&str> =
                    search_location.split(',').map(|s| s.trim()).collect();
                let search_city = search_parts.get(0).unwrap_or(&"").to_lowercase();
                let search_state = search_parts.get(1).unwrap_or(&"").to_lowercase();

                // Check start location
                let start_city = itinerary.start_location.city().to_lowercase();
                let start_state = itinerary.start_location.state().to_lowercase();

                // Check end location
                let end_city = itinerary.end_location.city().to_lowercase();
                let end_state = itinerary.end_location.state().to_lowercase();

                // Calculate match scores
                let start_match_score = self.calculate_location_match_score(
                    &search_city,
                    &search_state,
                    &start_city,
                    &start_state,
                );
                let end_match_score = self.calculate_location_match_score(
                    &search_city,
                    &search_state,
                    &end_city,
                    &end_state,
                );

                // Take the better of start or end location match
                let location_match_score = start_match_score.max(end_match_score);
                best_score = best_score.max(location_match_score);
            }

            best_score * self.weights.location_weight
        } else {
            0.0
        }
    }

    fn calculate_location_match_score(
        &self,
        search_city: &str,
        search_state: &str,
        itinerary_city: &str,
        itinerary_state: &str,
    ) -> f32 {
        // Exact city and state match
        if search_city == itinerary_city && search_state == itinerary_state {
            return 1.0;
        }

        // Exact city match, different state
        if search_city == itinerary_city {
            return 0.7;
        }

        // State match only
        if search_state == itinerary_state && !search_state.is_empty() {
            return 0.3;
        }

        // Partial city name match (contains)
        if itinerary_city.contains(search_city) || search_city.contains(itinerary_city) {
            return 0.5;
        }

        0.0
    }

    /// Score activity matching with detailed activity lookup
    fn score_activities(&self, itinerary: &FeaturedVacation, search: &SearchItinerary) -> f32 {
        if let Some(search_activities) = &search.activities {
            if search_activities.is_empty() {
                return 0.0;
            }

            // Extract activity IDs from itinerary
            let mut activity_ids = Vec::new();
            for day_items in itinerary.days.days.values() {
                for item in day_items {
                    if let crate::models::itinerary::base::DayItem::Activity { activity_id, .. } = item {
                        activity_ids.push(*activity_id);
                    }
                }
            }

            if activity_ids.is_empty() {
                return 0.0;
            }

            // Since we can't easily make async calls here, use activity type matching
            // from the itinerary description and any available metadata
            let mut matched_activities = 0;
            let total_search_activities = search_activities.len();

            for search_activity in search_activities {
                let search_term = search_activity.to_lowercase();
                
                // Check if any part of the itinerary mentions this activity type
                let mut found_match = false;
                
                // Check trip name
                if itinerary.trip_name.to_lowercase().contains(&search_term) {
                    found_match = true;
                }
                
                // Check description
                if itinerary.description.to_lowercase().contains(&search_term) {
                    found_match = true;
                }
                
                // Check for common activity synonyms and variations
                if self.matches_activity_synonyms(&search_term, &itinerary.trip_name.to_lowercase()) ||
                   self.matches_activity_synonyms(&search_term, &itinerary.description.to_lowercase()) {
                    found_match = true;
                }

                if found_match {
                    matched_activities += 1;
                    println!("Found match for activity '{}' in itinerary '{}'", search_activity, itinerary.trip_name);
                }
            }

            // Calculate match percentage
            let match_percentage = matched_activities as f32 / total_search_activities as f32;
            
            // Give full weight for perfect matches, scaled down for partial matches
            let activity_score = match_percentage * self.weights.activity_weight;
            
            println!("Activity scoring: {}/{} activities matched = {:.1}% = {:.1} points", 
                matched_activities, total_search_activities, match_percentage * 100.0, activity_score);
            
            activity_score
        } else {
            // No activity preference specified, give partial credit for having activities
            let mut activity_count = 0;
            for day_items in itinerary.days.days.values() {
                for item in day_items {
                    if matches!(item, crate::models::itinerary::base::DayItem::Activity { .. }) {
                        activity_count += 1;
                    }
                }
            }
            
            if activity_count > 0 {
                self.weights.activity_weight * 0.5 // 50% for having any activities when no preference
            } else {
                0.0
            }
        }
    }

    /// Check for activity synonyms and common variations
    fn matches_activity_synonyms(&self, search_term: &str, text: &str) -> bool {
        let synonyms = match search_term {
            "atving" | "atv" | "atvs" => vec!["quad", "four wheeler", "off road", "off-road", "4x4", "all terrain vehicle", "dirt bike", "trail riding"],
            "hotsprings" | "hot springs" | "hot spring" => vec!["thermal", "spa", "mineral springs", "geothermal", "springs", "natural springs", "thermal baths"],
            "goldminetours" | "gold mine tours" | "gold mine" | "goldmine" => vec!["mining", "mine tour", "mining tour", "historical mine", "gold rush", "underground tour", "mine exploration", "mining history"],
            "hiking" | "hike" | "hikes" => vec!["trail", "trek", "walking", "nature walk", "mountain", "wilderness"],
            "skiing" | "ski" => vec!["slope", "mountain resort", "powder", "alpine"],
            "rafting" | "raft" => vec!["river", "whitewater", "rapids", "float"],
            "climbing" | "climb" => vec!["rock climbing", "bouldering", "mountaineering"],
            "fishing" | "fish" => vec!["angling", "fly fishing", "catch"],
            "biking" | "bike" | "cycling" => vec!["bicycle", "mountain bike", "trail ride"],
            "kayaking" | "kayak" => vec!["paddle", "paddling", "water sports"],
            "camping" | "camp" => vec!["campground", "outdoor", "tent", "rv"],
            "wildlife" => vec!["animals", "safari", "nature viewing", "bird watching"],
            _ => vec![],
        };

        for synonym in synonyms {
            if text.contains(synonym) {
                return true;
            }
        }
        
        false
    }

    /// Score group size compatibility
    fn score_group_size(&self, itinerary: &FeaturedVacation, search: &SearchItinerary) -> f32 {
        if let Some(adults) = search.adults {
            let total_people =
                adults + search.children.unwrap_or_default() + search.infants.unwrap_or_default();

            // Perfect fit
            if total_people >= itinerary.min_group && total_people <= itinerary.max_group {
                return self.weights.group_size_weight;
            }

            // Close to range
            if total_people == itinerary.min_group - 1 || total_people == itinerary.max_group + 1 {
                return self.weights.group_size_weight * 0.7;
            }

            // Moderately close
            if total_people >= itinerary.min_group - 2 && total_people <= itinerary.max_group + 2 {
                return self.weights.group_size_weight * 0.4;
            }

            0.0
        } else {
            0.0
        }
    }

    /// Score lodging/accommodation matching
    fn score_lodging(&self, itinerary: &FeaturedVacation, search: &SearchItinerary) -> f32 {
        if let Some(search_lodging) = &search.lodging {
            if search_lodging.is_empty() {
                return 0.0;
            }

            // Check if itinerary has accommodation items
            let mut has_accommodations = false;
            for day_items in itinerary.days.days.values() {
                for item in day_items {
                    if matches!(
                        item,
                        crate::models::itinerary::base::DayItem::Accommodation { .. }
                    ) {
                        has_accommodations = true;
                        break;
                    }
                }
                if has_accommodations {
                    break;
                }
            }

            if has_accommodations {
                // For now, give partial points for having any accommodations
                // In a full implementation, you'd match specific lodging types
                self.weights.lodging_weight * 0.6
            } else {
                0.0
            }
        } else {
            0.0
        }
    }

    /// Score transportation matching
    fn score_transportation(&self, itinerary: &FeaturedVacation, search: &SearchItinerary) -> f32 {
        if let Some(search_transport) = &search.transportation {
            // Check if itinerary has transportation items
            for day_items in itinerary.days.days.values() {
                for item in day_items {
                    if let crate::models::itinerary::base::DayItem::Transportation {
                        name, ..
                    } = item
                    {
                        if name
                            .to_lowercase()
                            .contains(&search_transport.to_lowercase())
                        {
                            return self.weights.transportation_weight;
                        }
                    }
                }
            }

            // Partial match for having any transportation
            for day_items in itinerary.days.days.values() {
                for item in day_items {
                    if matches!(
                        item,
                        crate::models::itinerary::base::DayItem::Transportation { .. }
                    ) {
                        return self.weights.transportation_weight * 0.3;
                    }
                }
            }

            0.0
        } else {
            0.0
        }
    }
    
    /// Score trip pace matching
    fn score_trip_pace(&self, itinerary: &FeaturedVacation, search: &SearchItinerary) -> f32 {
        if let Some(search_pace) = &search.trip_pace {
            // Count activities per day in the itinerary
            let mut total_activities = 0;
            let mut total_activity_hours = 0.0;
            let num_days = itinerary.days.days.len() as f32;
            
            for day_items in itinerary.days.days.values() {
                let mut day_activities = 0;
                let mut day_hours = 0.0;
                
                for item in day_items {
                    if let crate::models::itinerary::base::DayItem::Activity { .. } = item {
                        day_activities += 1;
                        // Assume 2 hours per activity if we don't have duration info
                        day_hours += 2.0;
                    }
                }
                
                total_activities += day_activities;
                total_activity_hours += day_hours;
            }
            
            let avg_activities_per_day = if num_days > 0.0 { total_activities as f32 / num_days } else { 0.0 };
            let avg_hours_per_day = if num_days > 0.0 { total_activity_hours / num_days } else { 0.0 };
            
            // Score based on how well the itinerary matches the desired pace
            let expected_activities = search_pace.typical_activities_per_day() as f32;
            let expected_hours = search_pace.max_activity_hours_per_day();
            
            // Calculate activity count match (50% of pace score)
            let activity_diff = (avg_activities_per_day - expected_activities).abs();
            let activity_match = if activity_diff <= 0.5 {
                1.0
            } else if activity_diff <= 1.0 {
                0.8
            } else if activity_diff <= 2.0 {
                0.5
            } else {
                0.2
            };
            
            // Calculate hours match (50% of pace score)
            let hours_diff = (avg_hours_per_day - expected_hours).abs();
            let hours_match = if hours_diff <= 1.0 {
                1.0
            } else if hours_diff <= 2.0 {
                0.8
            } else if hours_diff <= 3.0 {
                0.5
            } else {
                0.2
            };
            
            // Combined score
            let pace_match = (activity_match + hours_match) / 2.0;
            
            pace_match * self.weights.trip_pace_weight
        } else {
            // No pace preference, give partial credit
            self.weights.trip_pace_weight * 0.5
        }
    }

    /// Score multiple itineraries and return sorted results above minimum threshold
    pub fn score_and_rank_itineraries(
        &self,
        itineraries: Vec<FeaturedVacation>,
        search: &SearchItinerary,
    ) -> Vec<ScoredItinerary> {
        let mut scored: Vec<ScoredItinerary> = itineraries
            .iter()
            .map(|itinerary| self.score_itinerary(itinerary, search))
            .filter(|scored| scored.total_score >= self.weights.minimum_score)
            .collect();

        // Sort by score descending
        scored.sort_by(|a, b| {
            b.total_score
                .partial_cmp(&a.total_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        scored
    }
}

impl AsyncSearchScorer {
    pub fn new(client: Arc<Client>) -> Self {
        let weights = SearchWeights::from_env();
        println!("AsyncSearchScorer initialized with weights: {:?}", weights);
        Self { weights, client }
    }

    pub fn with_weights(client: Arc<Client>, weights: SearchWeights) -> Self {
        Self { weights, client }
    }

    /// Score an itinerary against search criteria with full activity lookup
    pub async fn score_itinerary(
        &self,
        itinerary: &FeaturedVacation,
        search: &SearchItinerary,
    ) -> ScoredItinerary {
        let location_score = self.score_location(itinerary, search);
        let activity_score = self.score_activities_async(itinerary, search).await;
        let group_size_score = self.score_group_size(itinerary, search);
        let lodging_score = self.score_lodging(itinerary, search);
        let transportation_score = self.score_transportation(itinerary, search);
        let trip_pace_score = self.score_trip_pace(itinerary, search);

        let total_score = location_score
            + activity_score
            + group_size_score
            + lodging_score
            + transportation_score
            + trip_pace_score;

        ScoredItinerary {
            itinerary: itinerary.clone(),
            total_score,
            score_breakdown: ScoreBreakdown {
                location_score,
                activity_score,
                group_size_score,
                lodging_score,
                transportation_score,
                trip_pace_score,
            },
        }
    }

    /// Score activity matching with full database lookup
    async fn score_activities_async(&self, itinerary: &FeaturedVacation, search: &SearchItinerary) -> f32 {
        if let Some(search_activities) = &search.activities {
            if search_activities.is_empty() {
                return 0.0;
            }

            // Extract activity IDs from itinerary
            let mut activity_ids = Vec::new();
            for day_items in itinerary.days.days.values() {
                for item in day_items {
                    if let crate::models::itinerary::base::DayItem::Activity { activity_id, .. } = item {
                        activity_ids.push(*activity_id);
                    }
                }
            }

            if activity_ids.is_empty() {
                return 0.0;
            }

            // Fetch activities from database
            let activities = match self.fetch_activities(activity_ids).await {
                Ok(activities) => activities,
                Err(e) => {
                    eprintln!("Failed to fetch activities for scoring: {}", e);
                    // Fallback to basic text matching
                    return self.score_activities_fallback(itinerary, search);
                }
            };

            let mut matched_activities = 0;
            let total_search_activities = search_activities.len();

            for search_activity in search_activities {
                let search_term = search_activity.to_lowercase();
                
                let mut found_match = false;
                
                // Check against actual activity data
                for activity in &activities {
                    // Check activity types
                    for activity_type in &activity.activity_types {
                        if activity_type.to_lowercase().contains(&search_term) ||
                           self.matches_activity_synonyms(&search_term, &activity_type.to_lowercase()) {
                            found_match = true;
                            break;
                        }
                    }
                    
                    // Check tags
                    if !found_match {
                        for tag in &activity.tags {
                            if tag.to_lowercase().contains(&search_term) ||
                               self.matches_activity_synonyms(&search_term, &tag.to_lowercase()) {
                                found_match = true;
                                break;
                            }
                        }
                    }
                    
                    // Check title and description
                    if !found_match {
                        if activity.title.to_lowercase().contains(&search_term) ||
                           activity.description.to_lowercase().contains(&search_term) ||
                           self.matches_activity_synonyms(&search_term, &activity.title.to_lowercase()) ||
                           self.matches_activity_synonyms(&search_term, &activity.description.to_lowercase()) {
                            found_match = true;
                        }
                    }
                    
                    if found_match {
                        break;
                    }
                }

                if found_match {
                    matched_activities += 1;
                    println!("Found match for activity '{}' in itinerary '{}' (database lookup)", 
                        search_activity, itinerary.trip_name);
                }
            }

            // Calculate match percentage
            let match_percentage = matched_activities as f32 / total_search_activities as f32;
            
            // Give full weight for perfect matches, scaled down for partial matches
            let activity_score = match_percentage * self.weights.activity_weight;
            
            println!("Activity scoring (database): {}/{} activities matched = {:.1}% = {:.1} points", 
                matched_activities, total_search_activities, match_percentage * 100.0, activity_score);
            
            activity_score
        } else {
            // No activity preference specified, give partial credit for having activities
            let mut activity_count = 0;
            for day_items in itinerary.days.days.values() {
                for item in day_items {
                    if matches!(item, crate::models::itinerary::base::DayItem::Activity { .. }) {
                        activity_count += 1;
                    }
                }
            }
            
            if activity_count > 0 {
                self.weights.activity_weight * 0.5 // 50% for having any activities when no preference
            } else {
                0.0
            }
        }
    }

    /// Fetch activities from database by IDs
    async fn fetch_activities(&self, activity_ids: Vec<ObjectId>) -> Result<Vec<Activity>, mongodb::error::Error> {
        let collection: mongodb::Collection<Activity> = self
            .client
            .database("Itineraries")
            .collection("Activities");

        let filter = mongodb::bson::doc! {
            "_id": { "$in": activity_ids }
        };

        let cursor = collection.find(filter).await?;
        let activities: Vec<Activity> = cursor.try_collect().await?;
        
        println!("Fetched {} activities from database for scoring", activities.len());
        Ok(activities)
    }

    /// Fallback to basic text matching if database lookup fails
    fn score_activities_fallback(&self, itinerary: &FeaturedVacation, search: &SearchItinerary) -> f32 {
        if let Some(search_activities) = &search.activities {
            if search_activities.is_empty() {
                return 0.0;
            }

            let mut matched_activities = 0;
            let total_search_activities = search_activities.len();

            for search_activity in search_activities {
                let search_term = search_activity.to_lowercase();
                
                // Check if any part of the itinerary mentions this activity type
                let mut found_match = false;
                
                // Check trip name
                if itinerary.trip_name.to_lowercase().contains(&search_term) {
                    found_match = true;
                }
                
                // Check description
                if itinerary.description.to_lowercase().contains(&search_term) {
                    found_match = true;
                }
                
                // Check for common activity synonyms and variations
                if self.matches_activity_synonyms(&search_term, &itinerary.trip_name.to_lowercase()) ||
                   self.matches_activity_synonyms(&search_term, &itinerary.description.to_lowercase()) {
                    found_match = true;
                }

                if found_match {
                    matched_activities += 1;
                }
            }

            // Calculate match percentage
            let match_percentage = matched_activities as f32 / total_search_activities as f32;
            match_percentage * self.weights.activity_weight
        } else {
            0.0
        }
    }

    /// Score multiple itineraries and return sorted results above minimum threshold
    pub async fn score_and_rank_itineraries(
        &self,
        itineraries: Vec<FeaturedVacation>,
        search: &SearchItinerary,
    ) -> Vec<ScoredItinerary> {
        let mut scored = Vec::new();
        
        for itinerary in itineraries {
            let scored_itinerary = self.score_itinerary(&itinerary, search).await;
            if scored_itinerary.total_score >= self.weights.minimum_score {
                scored.push(scored_itinerary);
            }
        }

        // Sort by score descending
        scored.sort_by(|a, b| {
            b.total_score
                .partial_cmp(&a.total_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        scored
    }

    // Delegate methods from SearchScorer for compatibility
    fn score_location(&self, itinerary: &FeaturedVacation, search: &SearchItinerary) -> f32 {
        let scorer = SearchScorer { weights: self.weights.clone() };
        scorer.score_location(itinerary, search)
    }

    fn score_group_size(&self, itinerary: &FeaturedVacation, search: &SearchItinerary) -> f32 {
        let scorer = SearchScorer { weights: self.weights.clone() };
        scorer.score_group_size(itinerary, search)
    }

    fn score_lodging(&self, itinerary: &FeaturedVacation, search: &SearchItinerary) -> f32 {
        let scorer = SearchScorer { weights: self.weights.clone() };
        scorer.score_lodging(itinerary, search)
    }

    fn score_transportation(&self, itinerary: &FeaturedVacation, search: &SearchItinerary) -> f32 {
        let scorer = SearchScorer { weights: self.weights.clone() };
        scorer.score_transportation(itinerary, search)
    }

    fn matches_activity_synonyms(&self, search_term: &str, text: &str) -> bool {
        let scorer = SearchScorer { weights: self.weights.clone() };
        scorer.matches_activity_synonyms(search_term, text)
    }
    
    fn score_trip_pace(&self, itinerary: &FeaturedVacation, search: &SearchItinerary) -> f32 {
        let scorer = SearchScorer { weights: self.weights.clone() };
        scorer.score_trip_pace(itinerary, search)
    }
}
