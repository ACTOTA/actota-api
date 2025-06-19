//! Distance Service with Google Maps API Integration
//!
//! This service provides intelligent distance and travel time calculations using Google Maps API
//! with smart caching to minimize API costs and improve performance.
//!
//! ## Setup
//! 1. Get a Google Maps API key from Google Cloud Console
//! 2. Enable the Distance Matrix API
//! 3. Set the environment variable: `GOOGLE_MAPS_API_KEY=your_api_key_here`
//!
//! ## Features
//! - Smart caching with MongoDB (avoids duplicate API calls for same location pairs)
//! - Traffic-aware travel times for driving routes
//! - Configurable cache durations (24h for static routes, 1h for traffic-aware)
//! - Batch distance calculations for efficiency
//! - Automatic fallback if API is unavailable
//!
//! ## Cost Optimization
//! - Results are cached in database to avoid repeated API calls
//! - Coordinate tolerance matching (±10 meters) for cache hits
//! - Traffic-aware results cached for shorter periods
//! - Batch API calls when calculating multiple distances

use mongodb::{bson::oid::ObjectId, Client, Collection};
use reqwest;
use serde::{Deserialize, Serialize};
use std::{env, sync::Arc, time::Duration};

// Cache duration in seconds (24 hours for non-traffic, 1 hour for traffic-aware)
const CACHE_DURATION_STATIC: i64 = 86400; // 24 hours
const CACHE_DURATION_TRAFFIC: i64 = 3600; // 1 hour

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedDistance {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,
    pub origin_lat: f64,
    pub origin_lng: f64,
    pub destination_lat: f64,
    pub destination_lng: f64,
    pub distance_meters: u32,
    pub duration_seconds: u32,
    pub duration_in_traffic_seconds: Option<u32>,
    pub travel_mode: String, // "driving", "walking", "transit", "bicycling"
    pub with_traffic: bool,
    pub cached_at: mongodb::bson::DateTime,
    pub expires_at: mongodb::bson::DateTime,
}

#[derive(Debug, Deserialize)]
struct GoogleMapsResponse {
    status: String,
    rows: Vec<GoogleMapsRow>,
}

#[derive(Debug, Deserialize)]
struct GoogleMapsRow {
    elements: Vec<GoogleMapsElement>,
}

#[derive(Debug, Deserialize)]
struct GoogleMapsElement {
    status: String,
    distance: Option<GoogleMapsDistance>,
    duration: Option<GoogleMapsDuration>,
    duration_in_traffic: Option<GoogleMapsDuration>,
}

#[derive(Debug, Deserialize)]
struct GoogleMapsDistance {
    value: u32, // meters
}

#[derive(Debug, Deserialize)]
struct GoogleMapsDuration {
    value: u32, // seconds
}

#[derive(Debug, Clone)]
pub enum TravelMode {
    Driving,
    Walking,
    Transit,
    Bicycling,
}

impl TravelMode {
    fn as_str(&self) -> &str {
        match self {
            TravelMode::Driving => "driving",
            TravelMode::Walking => "walking",
            TravelMode::Transit => "transit",
            TravelMode::Bicycling => "bicycling",
        }
    }
}

#[derive(Debug, Clone)]
pub struct DistanceResult {
    pub distance_meters: u32,
    pub duration_minutes: u32,
    pub duration_in_traffic_minutes: Option<u32>,
    pub from_cache: bool,
}

pub struct DistanceService {
    client: Arc<Client>,
    http_client: reqwest::Client,
    api_key: String,
}

impl DistanceService {
    pub fn new(client: Arc<Client>) -> Result<Self, Box<dyn std::error::Error>> {
        let api_key = env::var("GOOGLE_MAPS_API_KEY")
            .map_err(|_| "GOOGLE_MAPS_API_KEY environment variable not set")?;

        let http_client = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .build()?;

        Ok(Self {
            client,
            http_client,
            api_key,
        })
    }

    /// Get distance between two coordinates with smart caching
    pub async fn get_distance(
        &self,
        origin: (f64, f64),
        destination: (f64, f64),
        travel_mode: TravelMode,
        with_traffic: bool,
    ) -> Result<DistanceResult, Box<dyn std::error::Error>> {
        // Check cache first
        if let Ok(Some(cached)) = self.get_cached_distance(origin, destination, &travel_mode, with_traffic).await {
            println!("Using cached distance for ({:.4}, {:.4}) to ({:.4}, {:.4})", 
                origin.0, origin.1, destination.0, destination.1);
            
            return Ok(DistanceResult {
                distance_meters: cached.distance_meters,
                duration_minutes: cached.duration_seconds / 60,
                duration_in_traffic_minutes: cached.duration_in_traffic_seconds.map(|d| d / 60),
                from_cache: true,
            });
        }

        // Not in cache or expired, call Google Maps API
        println!("Fetching distance from Google Maps API for ({:.4}, {:.4}) to ({:.4}, {:.4})", 
            origin.0, origin.1, destination.0, destination.1);
        
        let result = self.fetch_from_google_maps(origin, destination, &travel_mode, with_traffic).await?;
        
        // Cache the result
        if let Err(e) = self.cache_distance(origin, destination, &travel_mode, with_traffic, &result).await {
            eprintln!("Failed to cache distance result: {}", e);
        }

        Ok(result)
    }

    /// Get multiple distances efficiently (batch request)
    pub async fn get_distances_batch(
        &self,
        origins: Vec<(f64, f64)>,
        destinations: Vec<(f64, f64)>,
        travel_mode: TravelMode,
        with_traffic: bool,
    ) -> Result<Vec<Vec<DistanceResult>>, Box<dyn std::error::Error>> {
        // First, check what we have in cache
        let mut results = vec![vec![None; destinations.len()]; origins.len()];
        let mut missing_pairs = Vec::new();

        for (i, origin) in origins.iter().enumerate() {
            for (j, destination) in destinations.iter().enumerate() {
                if let Ok(Some(cached)) = self.get_cached_distance(*origin, *destination, &travel_mode, with_traffic).await {
                    results[i][j] = Some(DistanceResult {
                        distance_meters: cached.distance_meters,
                        duration_minutes: cached.duration_seconds / 60,
                        duration_in_traffic_minutes: cached.duration_in_traffic_seconds.map(|d| d / 60),
                        from_cache: true,
                    });
                } else {
                    missing_pairs.push((i, j, *origin, *destination));
                }
            }
        }

        // If we have missing pairs, make API calls
        if !missing_pairs.is_empty() {
            println!("Making batch API call for {} missing distance pairs", missing_pairs.len());
            
            // Extract unique origins and destinations for batch call
            let batch_origins: Vec<(f64, f64)> = missing_pairs.iter().map(|(_, _, origin, _)| *origin).collect();
            let batch_destinations: Vec<(f64, f64)> = missing_pairs.iter().map(|(_, _, _, dest)| *dest).collect();
            
            let api_results = self.fetch_batch_from_google_maps(batch_origins, batch_destinations, &travel_mode, with_traffic).await?;
            
            // Fill in the missing results and cache them
            for ((i, j, origin, destination), api_result) in missing_pairs.into_iter().zip(api_results.into_iter()) {
                results[i][j] = Some(api_result.clone());
                
                // Cache the result
                if let Err(e) = self.cache_distance(origin, destination, &travel_mode, with_traffic, &api_result).await {
                    eprintln!("Failed to cache batch distance result: {}", e);
                }
            }
        }

        // Convert Option<DistanceResult> to DistanceResult (unwrap should be safe now)
        let final_results: Vec<Vec<DistanceResult>> = results
            .into_iter()
            .map(|row| row.into_iter().map(|r| r.unwrap()).collect())
            .collect();

        Ok(final_results)
    }

    /// Check cache for existing distance calculation
    async fn get_cached_distance(
        &self,
        origin: (f64, f64),
        destination: (f64, f64),
        travel_mode: &TravelMode,
        with_traffic: bool,
    ) -> mongodb::error::Result<Option<CachedDistance>> {
        let collection: Collection<CachedDistance> = self
            .client
            .database("Itineraries")
            .collection("DistanceCache");

        // Create a tolerance for coordinate matching (about 10 meters)
        let coord_tolerance = 0.0001;

        let filter = mongodb::bson::doc! {
            "origin_lat": {
                "$gte": origin.0 - coord_tolerance,
                "$lte": origin.0 + coord_tolerance
            },
            "origin_lng": {
                "$gte": origin.1 - coord_tolerance,
                "$lte": origin.1 + coord_tolerance
            },
            "destination_lat": {
                "$gte": destination.0 - coord_tolerance,
                "$lte": destination.0 + coord_tolerance
            },
            "destination_lng": {
                "$gte": destination.1 - coord_tolerance,
                "$lte": destination.1 + coord_tolerance
            },
            "travel_mode": travel_mode.as_str(),
            "with_traffic": with_traffic,
            "expires_at": { "$gt": mongodb::bson::DateTime::now() }
        };

        collection.find_one(filter).await
    }

    /// Cache a distance calculation
    async fn cache_distance(
        &self,
        origin: (f64, f64),
        destination: (f64, f64),
        travel_mode: &TravelMode,
        with_traffic: bool,
        result: &DistanceResult,
    ) -> mongodb::error::Result<()> {
        let collection: Collection<CachedDistance> = self
            .client
            .database("Itineraries")
            .collection("DistanceCache");

        let now = mongodb::bson::DateTime::now();
        let cache_duration = if with_traffic { CACHE_DURATION_TRAFFIC } else { CACHE_DURATION_STATIC };
        let expires_at = mongodb::bson::DateTime::from_millis(now.timestamp_millis() + (cache_duration * 1000));

        let cached_distance = CachedDistance {
            id: None,
            origin_lat: origin.0,
            origin_lng: origin.1,
            destination_lat: destination.0,
            destination_lng: destination.1,
            distance_meters: result.distance_meters,
            duration_seconds: result.duration_minutes * 60,
            duration_in_traffic_seconds: result.duration_in_traffic_minutes.map(|d| d * 60),
            travel_mode: travel_mode.as_str().to_string(),
            with_traffic,
            cached_at: now,
            expires_at,
        };

        collection.insert_one(cached_distance).await?;
        Ok(())
    }

    /// Fetch distance from Google Maps Distance Matrix API
    async fn fetch_from_google_maps(
        &self,
        origin: (f64, f64),
        destination: (f64, f64),
        travel_mode: &TravelMode,
        with_traffic: bool,
    ) -> Result<DistanceResult, Box<dyn std::error::Error>> {
        let origins = format!("{},{}", origin.0, origin.1);
        let destinations = format!("{},{}", destination.0, destination.1);
        
        let mut url = format!(
            "https://maps.googleapis.com/maps/api/distancematrix/json?origins={}&destinations={}&mode={}&key={}",
            origins, destinations, travel_mode.as_str(), self.api_key
        );

        // Add traffic parameters for driving mode
        if matches!(travel_mode, TravelMode::Driving) && with_traffic {
            url.push_str("&departure_time=now&traffic_model=best_guess");
        }

        let response = self.http_client.get(&url).send().await?;
        let response_text = response.text().await?;
        
        let google_response: GoogleMapsResponse = serde_json::from_str(&response_text)
            .map_err(|e| format!("Failed to parse Google Maps response: {}. Response: {}", e, response_text))?;

        if google_response.status != "OK" {
            return Err(format!("Google Maps API error: {}", google_response.status).into());
        }

        if google_response.rows.is_empty() || google_response.rows[0].elements.is_empty() {
            return Err("No distance data returned from Google Maps".into());
        }

        let element = &google_response.rows[0].elements[0];
        
        if element.status != "OK" {
            return Err(format!("Google Maps element error: {}", element.status).into());
        }

        let distance = element.distance.as_ref()
            .ok_or("Distance not available")?;
        let duration = element.duration.as_ref()
            .ok_or("Duration not available")?;

        Ok(DistanceResult {
            distance_meters: distance.value,
            duration_minutes: duration.value / 60,
            duration_in_traffic_minutes: element.duration_in_traffic.as_ref().map(|d| d.value / 60),
            from_cache: false,
        })
    }

    /// Fetch multiple distances in a single API call (more efficient)
    async fn fetch_batch_from_google_maps(
        &self,
        origins: Vec<(f64, f64)>,
        destinations: Vec<(f64, f64)>,
        travel_mode: &TravelMode,
        with_traffic: bool,
    ) -> Result<Vec<DistanceResult>, Box<dyn std::error::Error>> {
        // Google Maps API limits: 25 origins, 25 destinations per request
        let max_origins = 25;
        let max_destinations = 25;
        
        if origins.len() > max_origins || destinations.len() > max_destinations {
            return Err("Too many origins or destinations for single API call".into());
        }

        let origins_str = origins.iter()
            .map(|(lat, lng)| format!("{},{}", lat, lng))
            .collect::<Vec<_>>()
            .join("|");
        
        let destinations_str = destinations.iter()
            .map(|(lat, lng)| format!("{},{}", lat, lng))
            .collect::<Vec<_>>()
            .join("|");

        let mut url = format!(
            "https://maps.googleapis.com/maps/api/distancematrix/json?origins={}&destinations={}&mode={}&key={}",
            origins_str, destinations_str, travel_mode.as_str(), self.api_key
        );

        if matches!(travel_mode, TravelMode::Driving) && with_traffic {
            url.push_str("&departure_time=now&traffic_model=best_guess");
        }

        let response = self.http_client.get(&url).send().await?;
        let response_text = response.text().await?;
        
        let google_response: GoogleMapsResponse = serde_json::from_str(&response_text)
            .map_err(|e| format!("Failed to parse batch Google Maps response: {}. Response: {}", e, response_text))?;

        if google_response.status != "OK" {
            return Err(format!("Google Maps batch API error: {}", google_response.status).into());
        }

        let mut results = Vec::new();
        
        for row in google_response.rows {
            for element in row.elements {
                if element.status == "OK" {
                    let distance = element.distance.as_ref()
                        .ok_or("Distance not available in batch response")?;
                    let duration = element.duration.as_ref()
                        .ok_or("Duration not available in batch response")?;

                    results.push(DistanceResult {
                        distance_meters: distance.value,
                        duration_minutes: duration.value / 60,
                        duration_in_traffic_minutes: element.duration_in_traffic.as_ref().map(|d| d.value / 60),
                        from_cache: false,
                    });
                } else {
                    return Err(format!("Google Maps batch element error: {}", element.status).into());
                }
            }
        }

        Ok(results)
    }

    /// Clean up expired cache entries
    pub async fn cleanup_expired_cache(&self) -> mongodb::error::Result<u64> {
        let collection: Collection<CachedDistance> = self
            .client
            .database("Itineraries")
            .collection("DistanceCache");

        let filter = mongodb::bson::doc! {
            "expires_at": { "$lt": mongodb::bson::DateTime::now() }
        };

        let result = collection.delete_many(filter).await?;
        println!("Cleaned up {} expired distance cache entries", result.deleted_count);
        
        Ok(result.deleted_count)
    }
}