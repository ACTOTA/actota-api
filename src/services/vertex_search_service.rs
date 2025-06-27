use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::env;
use std::error::Error;
use std::fmt;

#[derive(Debug, Serialize)]
pub struct VertexSearchRequest {
    pub query: String,
    #[serde(rename = "pageSize")]
    pub page_size: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filter: Option<String>,
    #[serde(rename = "queryExpansionSpec")]
    pub query_expansion_spec: QueryExpansionSpec,
    #[serde(rename = "spellCorrectionSpec")]
    pub spell_correction_spec: SpellCorrectionSpec,
}

#[derive(Debug, Serialize)]
pub struct QueryExpansionSpec {
    pub condition: String,
}

#[derive(Debug, Serialize)]
pub struct SpellCorrectionSpec {
    pub mode: String,
}

#[derive(Debug, Deserialize)]
pub struct VertexSearchResponse {
    pub results: Vec<SearchResult>,
    #[serde(rename = "totalSize")]
    pub total_size: Option<i32>,
    #[serde(rename = "nextPageToken")]
    pub next_page_token: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct SearchResult {
    pub id: String,
    pub document: Document,
    #[serde(rename = "modelScores")]
    pub model_scores: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
pub struct Document {
    pub id: String,
    #[serde(rename = "structData")]
    pub struct_data: serde_json::Value,
    #[serde(rename = "jsonData")]
    pub json_data: Option<String>,
}

#[derive(Debug)]
pub enum VertexSearchError {
    EnvironmentError(String),
    HttpError(reqwest::Error),
    AuthError(String),
    ResponseError(String),
}

impl fmt::Display for VertexSearchError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            VertexSearchError::EnvironmentError(msg) => write!(f, "Environment error: {}", msg),
            VertexSearchError::HttpError(err) => write!(f, "HTTP error: {}", err),
            VertexSearchError::AuthError(msg) => write!(f, "Authentication error: {}", msg),
            VertexSearchError::ResponseError(msg) => write!(f, "Response error: {}", msg),
        }
    }
}

impl Error for VertexSearchError {}

impl From<reqwest::Error> for VertexSearchError {
    fn from(err: reqwest::Error) -> Self {
        VertexSearchError::HttpError(err)
    }
}

#[derive(Clone)]
pub struct VertexSearchService {
    client: Client,
    project_id: String,
    location: String,
    data_store_id: String,
    serving_config: String,
}

impl VertexSearchService {
    pub fn new() -> Result<Self, VertexSearchError> {
        let project_id = env::var("GOOGLE_CLOUD_PROJECT_ID")
            .map_err(|_| VertexSearchError::EnvironmentError("GOOGLE_CLOUD_PROJECT_ID not set".to_string()))?;
        
        let location = env::var("VERTEX_SEARCH_LOCATION")
            .unwrap_or_else(|_| "global".to_string());
        
        let data_store_id = env::var("VERTEX_SEARCH_DATA_STORE_ID")
            .map_err(|_| VertexSearchError::EnvironmentError("VERTEX_SEARCH_DATA_STORE_ID not set".to_string()))?;
        
        let serving_config = env::var("VERTEX_SEARCH_SERVING_CONFIG")
            .unwrap_or_else(|_| "default_config".to_string());

        Ok(Self {
            client: Client::new(),
            project_id,
            location,
            data_store_id,
            serving_config,
        })
    }

    pub async fn search_activities(&self, activity_types: &[String], query: &str) -> Result<VertexSearchResponse, VertexSearchError> {
        let search_query = self.build_search_query(activity_types, query);
        
        let request = VertexSearchRequest {
            query: search_query,
            page_size: 20,
            filter: None, // Filtering removed for simplicity, rely on query matching
            query_expansion_spec: QueryExpansionSpec {
                condition: "AUTO".to_string(),
            },
            spell_correction_spec: SpellCorrectionSpec {
                mode: "AUTO".to_string(),
            },
        };

        self.execute_search(request).await
    }


    async fn execute_search(&self, request: VertexSearchRequest) -> Result<VertexSearchResponse, VertexSearchError> {
        let access_token = self.get_access_token().await?;
        
        let url = format!(
            "https://discoveryengine.googleapis.com/v1/projects/{}/locations/{}/dataStores/{}/servingConfigs/{}:search",
            self.project_id, self.location, self.data_store_id, self.serving_config
        );

        let response = self.client
            .post(&url)
            .header("Authorization", format!("Bearer {}", access_token))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await?;

        let status = response.status();
        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            return Err(VertexSearchError::ResponseError(format!(
                "Search request failed with status {}: {}",
                status,
                error_text
            )));
        }

        let search_response: VertexSearchResponse = response.json().await
            .map_err(|e| VertexSearchError::ResponseError(format!("Failed to parse response: {}", e)))?;

        Ok(search_response)
    }

    fn build_search_query(&self, activity_types: &[String], additional_query: &str) -> String {
        let mut query_parts = Vec::new();
        
        println!("Building Vertex AI Search query with activity_types: {:?}, location: '{}'", activity_types, additional_query);
        
        // Add activity types as search terms
        if !activity_types.is_empty() {
            query_parts.extend(activity_types.iter().cloned());
        }
        
        // Add location information if provided
        if !additional_query.is_empty() {
            query_parts.push(additional_query.to_string());
        }
        
        // Join all search terms with spaces for natural language search
        let final_query = if query_parts.is_empty() {
            "activities".to_string() // Default search term
        } else {
            query_parts.join(" ")
        };
        
        println!("🔍 Final Vertex AI Search Query: '{}'", final_query);
        final_query
    }


    async fn get_access_token(&self) -> Result<String, VertexSearchError> {
        // For now, return a placeholder token or use environment variable
        // In production, you would implement proper OAuth2 flow or service account authentication
        let token = env::var("GOOGLE_CLOUD_ACCESS_TOKEN")
            .or_else(|_| {
                // Try getting token from gcloud CLI
                std::process::Command::new("gcloud")
                    .args(&["auth", "print-access-token"])
                    .output()
                    .map_err(|e| format!("Failed to get gcloud token: {}", e))
                    .and_then(|output| {
                        if output.status.success() {
                            String::from_utf8(output.stdout)
                                .map_err(|e| format!("Invalid UTF-8 in token: {}", e))
                                .map(|s| s.trim().to_string())
                        } else {
                            Err(format!("gcloud command failed: {}", String::from_utf8_lossy(&output.stderr)))
                        }
                    })
            })
            .map_err(|e| VertexSearchError::AuthError(format!("Failed to get access token: {}", e)))?;

        Ok(token)
    }
}

impl Default for VertexSearchService {
    fn default() -> Self {
        Self::new().expect("Failed to create VertexSearchService")
    }
}