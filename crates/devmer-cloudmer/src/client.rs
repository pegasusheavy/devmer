//! Cloudmer API client.

use std::time::Duration;

use reqwest::{Client, StatusCode};
use secrecy::{ExposeSecret, SecretString};
use serde::de::DeserializeOwned;

use crate::config::CloudmerConfig;
use crate::error::{CloudmerError, Result};
use crate::types::*;

/// Client for interacting with the Cloudmer API.
#[derive(Debug, Clone)]
pub struct CloudmerClient {
    config: CloudmerConfig,
    http: Client,
    token: Option<SecretString>,
}

impl CloudmerClient {
    /// Create a new Cloudmer client.
    pub fn new(config: CloudmerConfig) -> Result<Self> {
        config.validate()?;

        let http = Client::builder()
            .timeout(Duration::from_secs(config.timeout_secs))
            .user_agent(format!("devmer-cloudmer/{}", env!("CARGO_PKG_VERSION")))
            .build()
            .map_err(CloudmerError::Network)?;

        let token = config.api_token.clone().map(SecretString::from);

        Ok(Self { config, http, token })
    }

    /// Create a client from an API token.
    pub fn from_token(token: impl Into<String>) -> Result<Self> {
        Self::new(CloudmerConfig::with_token(token))
    }

    /// Set the project ID.
    pub fn with_project(mut self, project_id: impl Into<String>) -> Self {
        self.config.project_id = Some(project_id.into());
        self
    }

    /// Get the current configuration.
    pub fn config(&self) -> &CloudmerConfig {
        &self.config
    }

    /// Check if the client is authenticated.
    pub fn is_authenticated(&self) -> bool {
        self.token.is_some()
    }

    /// Verify the API token is valid.
    pub async fn verify_token(&self) -> Result<User> {
        self.get("/v1/auth/me").await
    }

    /// List projects the user has access to.
    pub async fn list_projects(&self) -> Result<Vec<Project>> {
        self.get("/v1/projects").await
    }

    /// Get a specific project.
    pub async fn get_project(&self, project_id: &str) -> Result<Project> {
        self.get(&format!("/v1/projects/{}", project_id)).await
    }

    /// Sync infrastructure state to Cloudmer.
    pub async fn sync_state(&self, state: &InfrastructureState) -> Result<SyncResponse> {
        let project_id = self.config.project_id.as_ref()
            .ok_or(CloudmerError::ProjectNotLinked)?;

        self.post(&format!("/v1/projects/{}/state", project_id), state).await
    }

    /// Send a deployment notification.
    pub async fn notify_deployment(&self, notification: &DeploymentNotification) -> Result<DeploymentResponse> {
        let project_id = self.config.project_id.as_ref()
            .ok_or(CloudmerError::ProjectNotLinked)?;

        self.post(&format!("/v1/projects/{}/deployments", project_id), notification).await
    }

    /// Get cost insights for a project.
    pub async fn get_cost_insights(&self) -> Result<CostInsights> {
        let project_id = self.config.project_id.as_ref()
            .ok_or(CloudmerError::ProjectNotLinked)?;

        self.get(&format!("/v1/projects/{}/costs", project_id)).await
    }

    /// Link a Devmer stack to a Cloudmer project.
    pub async fn link_project(&self, project_id: &str, stack_name: &str) -> Result<()> {
        #[derive(serde::Serialize)]
        #[serde(rename_all = "camelCase")]
        struct LinkRequest<'a> {
            stack_name: &'a str,
            tool: &'static str,
        }

        let _: serde_json::Value = self.post(
            &format!("/v1/projects/{}/link", project_id),
            &LinkRequest {
                stack_name,
                tool: "devmer",
            },
        ).await?;

        Ok(())
    }

    /// Generate an API token via device flow (for CLI login).
    pub async fn start_device_auth(&self) -> Result<DeviceAuthResponse> {
        #[derive(serde::Serialize)]
        struct DeviceAuthRequest {
            client_id: &'static str,
            scope: &'static str,
        }

        self.post_unauthenticated(
            "/v1/auth/device",
            &DeviceAuthRequest {
                client_id: "devmer-cli",
                scope: "read write",
            },
        ).await
    }

    /// Poll for device auth completion.
    pub async fn poll_device_auth(&self, device_code: &str) -> Result<Option<TokenResponse>> {
        #[derive(serde::Serialize)]
        struct PollRequest<'a> {
            device_code: &'a str,
            client_id: &'static str,
        }

        let response: DeviceAuthPollResponse = self.post_unauthenticated(
            "/v1/auth/device/token",
            &PollRequest {
                device_code,
                client_id: "devmer-cli",
            },
        ).await?;

        match response {
            DeviceAuthPollResponse::Pending => Ok(None),
            DeviceAuthPollResponse::Complete(token) => Ok(Some(token)),
        }
    }

    // Internal HTTP methods

    async fn get<T: DeserializeOwned>(&self, path: &str) -> Result<T> {
        let url = self.config.api_endpoint(path);
        let token = self.token.as_ref().ok_or(CloudmerError::InvalidToken)?;

        let response = self.http
            .get(&url)
            .bearer_auth(token.expose_secret())
            .send()
            .await
            .map_err(CloudmerError::Network)?;

        self.handle_response(response).await
    }

    async fn post<T: DeserializeOwned, B: serde::Serialize>(
        &self,
        path: &str,
        body: &B,
    ) -> Result<T> {
        let url = self.config.api_endpoint(path);
        let token = self.token.as_ref().ok_or(CloudmerError::InvalidToken)?;

        let response = self.http
            .post(&url)
            .bearer_auth(token.expose_secret())
            .json(body)
            .send()
            .await
            .map_err(CloudmerError::Network)?;

        self.handle_response(response).await
    }

    async fn post_unauthenticated<T: DeserializeOwned, B: serde::Serialize>(
        &self,
        path: &str,
        body: &B,
    ) -> Result<T> {
        let url = self.config.api_endpoint(path);

        let response = self.http
            .post(&url)
            .json(body)
            .send()
            .await
            .map_err(CloudmerError::Network)?;

        self.handle_response(response).await
    }

    async fn handle_response<T: DeserializeOwned>(&self, response: reqwest::Response) -> Result<T> {
        let status = response.status();

        match status {
            StatusCode::OK | StatusCode::CREATED => {
                response.json().await.map_err(|e| {
                    CloudmerError::InvalidResponse(format!("failed to parse response: {}", e))
                })
            }
            StatusCode::UNAUTHORIZED => {
                Err(CloudmerError::AuthenticationFailed("invalid or expired token".to_string()))
            }
            StatusCode::FORBIDDEN => {
                Err(CloudmerError::AuthenticationFailed("access denied".to_string()))
            }
            StatusCode::NOT_FOUND => {
                let body = response.text().await.unwrap_or_default();
                Err(CloudmerError::NotFound(body))
            }
            StatusCode::TOO_MANY_REQUESTS => {
                let retry_after = response
                    .headers()
                    .get("retry-after")
                    .and_then(|v| v.to_str().ok())
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(60);
                Err(CloudmerError::RateLimited { retry_after_secs: retry_after })
            }
            _ => {
                let body = response.text().await.unwrap_or_default();
                Err(CloudmerError::RequestFailed(format!("{}: {}", status, body)))
            }
        }
    }
}

/// Response from device auth initiation.
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceAuthResponse {
    /// Device code for polling.
    pub device_code: String,
    /// User code to display.
    pub user_code: String,
    /// URL for user to visit.
    pub verification_url: String,
    /// Complete URL with code embedded.
    pub verification_url_complete: String,
    /// Expiry time in seconds.
    pub expires_in: u64,
    /// Polling interval in seconds.
    pub interval: u64,
}

/// Response from device auth polling.
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(untagged)]
pub enum DeviceAuthPollResponse {
    /// Authorization is still pending.
    Pending,
    /// Authorization is complete.
    Complete(TokenResponse),
}

/// Token response from successful auth.
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TokenResponse {
    /// Access token.
    pub access_token: String,
    /// Token type (usually "Bearer").
    pub token_type: String,
    /// Expiry time in seconds.
    pub expires_in: u64,
    /// Refresh token (if provided).
    pub refresh_token: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_creation() {
        let client = CloudmerClient::from_token("test-token");
        assert!(client.is_ok());
        assert!(client.unwrap().is_authenticated());
    }

    #[test]
    fn test_client_without_token() {
        let client = CloudmerClient::new(CloudmerConfig::default());
        assert!(client.is_ok());
        assert!(!client.unwrap().is_authenticated());
    }
}
