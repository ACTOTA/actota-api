use reqwest;
use serde::{Deserialize, Serialize};
use std::env;
use mongodb::{Client, Collection, bson::{doc, oid::ObjectId, DateTime}};
use rand::{distributions::Alphanumeric, Rng};

#[derive(Debug, Serialize, Deserialize)]
pub struct SendGridEmail {
    pub email: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SendGridPersonalization {
    pub to: Vec<SendGridEmail>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SendGridContent {
    #[serde(rename = "type")]
    pub content_type: String,
    pub value: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SendGridRequest {
    pub personalizations: Vec<SendGridPersonalization>,
    pub from: SendGridEmail,
    pub subject: String,
    pub content: Vec<SendGridContent>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct EmailVerification {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,
    pub email: String,
    pub user_id: Option<ObjectId>,
    pub verification_code: String,
    pub expires_at: DateTime,
    pub verified: bool,
    pub created_at: DateTime,
}

#[derive(Debug)]
pub enum EmailError {
    EnvironmentError(String),
    RequestError(String),
    ApiError(String),
    DatabaseError(String),
    CodeExpired,
    InvalidCode,
}

impl std::fmt::Display for EmailError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EmailError::EnvironmentError(err) => write!(f, "Environment error: {}", err),
            EmailError::RequestError(err) => write!(f, "Request error: {}", err),
            EmailError::ApiError(err) => write!(f, "API error: {}", err),
            EmailError::DatabaseError(err) => write!(f, "Database error: {}", err),
            EmailError::CodeExpired => write!(f, "Verification code has expired"),
            EmailError::InvalidCode => write!(f, "Invalid verification code"),
        }
    }
}

impl std::error::Error for EmailError {}

pub struct EmailService {
    api_key: String,
    client: reqwest::Client,
}

impl EmailService {
    pub fn new() -> Result<Self, EmailError> {
        let api_key = env::var("SENDGRID_API_KEY")
            .map_err(|_| EmailError::EnvironmentError("SENDGRID_API_KEY not set".to_string()))?;

        let client = reqwest::Client::new();

        Ok(Self { api_key, client })
    }

    pub async fn send_email(
        &self,
        to_email: &str,
        from_email: &str,
        subject: &str,
        content: &str,
    ) -> Result<(), EmailError> {
        let url = "https://api.sendgrid.com/v3/mail/send";

        let request = SendGridRequest {
            personalizations: vec![SendGridPersonalization {
                to: vec![SendGridEmail {
                    email: to_email.to_string(),
                }],
            }],
            from: SendGridEmail {
                email: from_email.to_string(),
            },
            subject: subject.to_string(),
            content: vec![SendGridContent {
                content_type: "text/plain".to_string(),
                value: content.to_string(),
            }],
        };

        let response = self
            .client
            .post(url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .map_err(|e| EmailError::RequestError(e.to_string()))?;

        if response.status().is_success() {
            Ok(())
        } else {
            let status = response.status();
            let body = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            Err(EmailError::ApiError(format!(
                "Status: {}, Body: {}",
                status, body
            )))
        }
    }

    pub async fn send_html_email(
        &self,
        to_email: &str,
        from_email: &str,
        subject: &str,
        html_content: &str,
    ) -> Result<(), EmailError> {
        let url = "https://api.sendgrid.com/v3/mail/send";

        let request = SendGridRequest {
            personalizations: vec![SendGridPersonalization {
                to: vec![SendGridEmail {
                    email: to_email.to_string(),
                }],
            }],
            from: SendGridEmail {
                email: from_email.to_string(),
            },
            subject: subject.to_string(),
            content: vec![SendGridContent {
                content_type: "text/html".to_string(),
                value: html_content.to_string(),
            }],
        };

        let response = self
            .client
            .post(url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .map_err(|e| EmailError::RequestError(e.to_string()))?;

        if response.status().is_success() {
            Ok(())
        } else {
            let status = response.status();
            let body = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            Err(EmailError::ApiError(format!(
                "Status: {}, Body: {}",
                status, body
            )))
        }
    }

    pub async fn send_verification_email(
        &self,
        email: &str,
        user_id: Option<ObjectId>,
        db_client: &Client,
    ) -> Result<String, EmailError> {
        // Generate 6-digit verification code
        let verification_code: String = rand::thread_rng()
            .sample_iter(&Alphanumeric)
            .take(6)
            .map(char::from)
            .collect::<String>()
            .to_uppercase();

        let now = DateTime::now();
        let expires_at = DateTime::from_millis(now.timestamp_millis() + (15 * 60 * 1000)); // 15 minutes

        // Store verification in database
        let verification = EmailVerification {
            id: None,
            email: email.to_string(),
            user_id,
            verification_code: verification_code.clone(),
            expires_at,
            verified: false,
            created_at: now,
        };

        let collection: Collection<EmailVerification> = db_client
            .database("actota")
            .collection("email_verifications");

        // Remove any existing unverified codes for this email
        let _ = collection
            .delete_many(doc! {
                "email": email,
                "verified": false
            })
            .await;

        collection
            .insert_one(&verification)
            .await
            .map_err(|e| EmailError::DatabaseError(e.to_string()))?;

        // Send verification email
        let from_email = env::var("FROM_EMAIL")
            .unwrap_or_else(|_| "noreply@actota.com".to_string());

        let subject = "Verify Your Email Address";
        let content = format!(
            "Hi there!\n\nYour verification code is: {}\n\nThis code will expire in 15 minutes.\n\nIf you didn't request this verification, please ignore this email.\n\nBest regards,\nThe ACTOTA Team",
            verification_code
        );

        self.send_email(email, &from_email, subject, &content)
            .await?;

        Ok(verification_code)
    }

    pub async fn send_verification_html_email(
        &self,
        email: &str,
        user_id: Option<ObjectId>,
        db_client: &Client,
    ) -> Result<String, EmailError> {
        // Generate 6-digit verification code
        let verification_code: String = rand::thread_rng()
            .sample_iter(&Alphanumeric)
            .take(6)
            .map(char::from)
            .collect::<String>()
            .to_uppercase();

        let now = DateTime::now();
        let expires_at = DateTime::from_millis(now.timestamp_millis() + (15 * 60 * 1000)); // 15 minutes

        // Store verification in database
        let verification = EmailVerification {
            id: None,
            email: email.to_string(),
            user_id,
            verification_code: verification_code.clone(),
            expires_at,
            verified: false,
            created_at: now,
        };

        let collection: Collection<EmailVerification> = db_client
            .database("actota")
            .collection("email_verifications");

        // Remove any existing unverified codes for this email
        let _ = collection
            .delete_many(doc! {
                "email": email,
                "verified": false
            })
            .await;

        collection
            .insert_one(&verification)
            .await
            .map_err(|e| EmailError::DatabaseError(e.to_string()))?;

        // Send HTML verification email
        let from_email = env::var("FROM_EMAIL")
            .unwrap_or_else(|_| "noreply@actota.com".to_string());

        let subject = "Verify Your Email Address";
        let html_content = format!(
            r#"
            <!DOCTYPE html>
            <html>
            <head>
                <meta charset="utf-8">
                <title>Email Verification</title>
                <style>
                    body {{ font-family: Arial, sans-serif; line-height: 1.6; color: #333; }}
                    .container {{ max-width: 600px; margin: 0 auto; padding: 20px; }}
                    .header {{ background-color: #f8f9fa; padding: 20px; border-radius: 5px; text-align: center; }}
                    .code {{ font-size: 32px; font-weight: bold; color: #007bff; letter-spacing: 3px; margin: 20px 0; }}
                    .footer {{ margin-top: 30px; font-size: 14px; color: #666; }}
                </style>
            </head>
            <body>
                <div class="container">
                    <div class="header">
                        <h1>Verify Your Email Address</h1>
                    </div>
                    <p>Hi there!</p>
                    <p>Your verification code is:</p>
                    <div class="code">{}</div>
                    <p>This code will expire in 15 minutes.</p>
                    <p>If you didn't request this verification, please ignore this email.</p>
                    <div class="footer">
                        <p>Best regards,<br>The ACTOTA Team</p>
                    </div>
                </div>
            </body>
            </html>
            "#,
            verification_code
        );

        self.send_html_email(email, &from_email, subject, &html_content)
            .await?;

        Ok(verification_code)
    }

    pub async fn verify_email_code(
        email: &str,
        code: &str,
        db_client: &Client,
    ) -> Result<bool, EmailError> {
        let collection: Collection<EmailVerification> = db_client
            .database("actota")
            .collection("email_verifications");

        let now = DateTime::now();

        // Find the verification record
        let verification = collection
            .find_one(doc! {
                "email": email,
                "verification_code": code,
                "verified": false
            })
            .await
            .map_err(|e| EmailError::DatabaseError(e.to_string()))?;

        match verification {
            Some(v) => {
                // Check if code has expired
                if v.expires_at.timestamp_millis() < now.timestamp_millis() {
                    // Clean up expired code
                    let _ = collection
                        .delete_one(doc! { "_id": v.id })
                        .await;
                    return Err(EmailError::CodeExpired);
                }

                // Mark as verified
                collection
                    .update_one(
                        doc! { "_id": v.id },
                        doc! { "$set": { "verified": true } }
                    )
                    .await
                    .map_err(|e| EmailError::DatabaseError(e.to_string()))?;

                Ok(true)
            }
            None => Err(EmailError::InvalidCode),
        }
    }

    pub async fn cleanup_expired_codes(db_client: &Client) -> Result<u64, EmailError> {
        let collection: Collection<EmailVerification> = db_client
            .database("actota")
            .collection("email_verifications");

        let now = DateTime::now();

        let result = collection
            .delete_many(doc! {
                "expires_at": { "$lt": now },
                "verified": false
            })
            .await
            .map_err(|e| EmailError::DatabaseError(e.to_string()))?;

        Ok(result.deleted_count)
    }
}
