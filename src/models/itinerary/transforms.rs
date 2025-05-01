use crate::models::itinerary::populated::{ActivitySummary, Address, Capacity};

use super::{
    base::{DayItem, FeaturedVacation},
    populated::{AccommodationModel, ActivityModel, PopulatedDayItem, PopulatedFeaturedVacation},
};
use bson::{doc, oid::ObjectId};
use futures::stream::TryStreamExt;
use google_cloud_storage::client::{Client as GcsClient, ClientConfig};
use google_cloud_storage::http::objects::list::ListObjectsRequest;
use mongodb::{error::Error, Client, Collection};
use std::collections::{HashMap, HashSet};
use std::env;

// Helper function to fetch activity images from GCS bucket
async fn fetch_activity_images(activity_id: &str) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    // Get bucket name from environment variable
    let bucket_name = env::var("ACTIVITY_BUCKET").map_err(|_| "ACTIVITY_BUCKET not set")?;
    let base_url = env::var("CLOUD_STORAGE_URL").unwrap_or(format!("https://storage.googleapis.com/{}", bucket_name));
    
    // Initialize GCS client
    let client_config = ClientConfig::default().with_auth().await?;
    let gcs_client = GcsClient::new(client_config);
    
    // Create the folder path for this activity
    let prefix = format!("{}/", activity_id);
    
    // Create a list request for the activity's folder
    let list_request = ListObjectsRequest {
        bucket: bucket_name.clone(),
        prefix: Some(prefix),
        ..Default::default()
    };
    
    // List objects in the activity's folder
    let objects = gcs_client.list_objects(&list_request).await?;
    
    // Generate public URLs for each image
    let images = objects.items.unwrap_or_default()
        .into_iter()
        .map(|object| {
            format!("{}/{}", base_url, object.name)
        })
        .collect();
    
    Ok(images)
}

impl FeaturedVacation {
    pub async fn populate(self, client: &Client) -> Result<PopulatedFeaturedVacation, Error> {
        // 1. Extract all activity and accommodation IDs
        let mut activity_ids = HashSet::new();
        let mut accommodation_ids = HashSet::new();

        println!("Days.days: {:?}", &self.days.days);

        for (_, day_items) in &self.days.days {
            for item in day_items {
                match item {
                    DayItem::Activity { activity_id, .. } => {
                        activity_ids.insert(*activity_id);
                    }
                    DayItem::Accommodation {
                        accommodation_id, ..
                    } => {
                        accommodation_ids.insert(*accommodation_id);
                    }
                    _ => {}
                }
            }
        }

        println!("\n\nActivities: {:?}", activity_ids);

        // 2. Fetch activities
        let activities_collection: Collection<ActivityModel> =
            client.database("Options").collection("Activity");

        let activities_vec: Vec<ObjectId> = activity_ids.into_iter().collect();
        let mut activities_map = HashMap::new();

        if !activities_vec.is_empty() {
            let cursor = activities_collection
                .find(doc! { "_id": { "$in": activities_vec } })
                .await?;

            let activities: Vec<ActivityModel> = cursor.try_collect().await?;

            for activity in activities {
                if let Some(id) = activity.id {
                    activities_map.insert(id, activity);
                }
            }
        }

        // 3. Fetch accommodations
        let accommodations_collection: Collection<AccommodationModel> =
            client.database("Options").collection("Lodging");

        let accommodations_vec: Vec<ObjectId> = accommodation_ids.into_iter().collect();
        let mut accommodations_map = HashMap::new();

        if !accommodations_vec.is_empty() {
            let cursor = accommodations_collection
                .find(doc! { "_id": { "$in": accommodations_vec } })
                .await?;

            let accommodations: Vec<AccommodationModel> = cursor.try_collect().await?;

            for accommodation in accommodations {
                if let Some(id) = accommodation.id {
                    accommodations_map.insert(id, accommodation);
                }
            }
        }

        // 4. Populate days with fetched data
        let mut populated_days = HashMap::new();
        let mut activities = Vec::new();

        for (day_key, day_items) in self.clone().days.days {
            let mut populated_items = Vec::new();

            for item in day_items {
                let populated_item = match item {
                    DayItem::Transportation {
                        time,
                        location,
                        name,
                    } => PopulatedDayItem::Transportation {
                        time,
                        location,
                        name,
                    },

                    DayItem::Activity { time, activity_id } => {
                        // Get activity or create a placeholder if not found
                        if let Some(activity) = activities_map.get(&activity_id) {
                            let mut activity_with_images = activity.clone();
                            
                            // Get images from ACTIVITY_BUCKET for this activity
                            if let Some(id) = activity.id {
                                let activity_id_str = id.to_string();
                                if let Ok(images) = fetch_activity_images(&activity_id_str).await {
                                    activity_with_images.images = Some(images);
                                }
                            }
                            
                            activities.push(ActivitySummary {
                                time,
                                label: activity_with_images.title.clone(),
                                tags: activity_with_images.tags.clone(),
                            });
                            
                            PopulatedDayItem::Activity {
                                time,
                                activity_id: Some(activity_id), // Include the activity_id for backward compatibility
                                activity: activity_with_images,
                            }
                        } else {
                            // Create a placeholder activity instead of failing
                            println!(
                                "Warning: Activity not found: {}, creating placeholder",
                                activity_id
                            );
                            PopulatedDayItem::Activity {
                                time,
                                activity_id: Some(activity_id), // Include the activity_id for backward compatibility
                                activity: ActivityModel {
                                    id: Some(activity_id),
                                    company: "Unknown Company".to_string(),
                                    company_id: "unknown".to_string(),
                                    booking_link: "#".to_string(),
                                    online_booking_status: "unavailable".to_string(),
                                    title: format!("Unknown Activity (ID: {})", activity_id),
                                    description: "This activity information could not be found"
                                        .to_string(),
                                    activity_types: vec!["unknown".to_string()],
                                    tags: vec![],
                                    price_per_person: 0,
                                    duration_minutes: 60,
                                    daily_time_slots: vec![],
                                    address: Address {
                                        street: "Unknown".to_string(),
                                        unit: None,
                                        city: "Unknown".to_string(),
                                        state: "Unknown".to_string(),
                                        zip: "00000".to_string(),
                                        country: "Unknown".to_string(),
                                    },
                                    whats_included: vec![],
                                    weight_limit_lbs: None,
                                    age_requirement: None,
                                    height_requirement: None,
                                    capacity: Capacity {
                                        minimum: 1,
                                        maximum: 10,
                                    },
                                    activities: None,
                                    images: None,
                                },
                            }
                        }
                    }

                    DayItem::Accommodation {
                        time,
                        accommodation_id,
                    } => {
                        // Get accommodation or create a placeholder if not found
                        if let Some(accommodation) = accommodations_map.get(&accommodation_id) {
                            PopulatedDayItem::Accommodation {
                                time,
                                accommodation: accommodation.clone(),
                            }
                        } else {
                            // Create a placeholder accommodation instead of failing
                            println!(
                                "Warning: Accommodation not found: {}, creating placeholder",
                                accommodation_id
                            );
                            PopulatedDayItem::Accommodation {
                                time,
                                accommodation: AccommodationModel {
                                    id: Some(accommodation_id),
                                    name: format!(
                                        "Unknown Accommodation (ID: {})",
                                        accommodation_id
                                    ),
                                    address: Some("Address information unavailable".to_string()),
                                    location: None,
                                    price_per_night: None,
                                    amenities: Some(vec!["Information unavailable".to_string()]),
                                    images: None,
                                    created_at: Some(chrono::Utc::now()),
                                    updated_at: Some(chrono::Utc::now()),
                                },
                            }
                        }
                    }
                };

                populated_items.push(populated_item);
            }

            populated_days.insert(day_key, populated_items);
        }

        // 5. Return populated vacation
        Ok(PopulatedFeaturedVacation {
            base: self,
            populated_days,
            activities,
        })
    }
}
