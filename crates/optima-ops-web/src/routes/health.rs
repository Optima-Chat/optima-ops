//! Health check related routes and utilities

use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Health check result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheckResult {
    pub name: String,
    pub status: HealthStatus,
    pub response_time_ms: Option<u64>,
    pub error: Option<String>,
}

/// Health status enum
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum HealthStatus {
    Healthy,
    Unhealthy,
    Unknown,
}

/// HTTP client for health checks
pub struct HealthChecker {
    client: Client,
}

impl HealthChecker {
    pub fn new() -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(5))
            .build()
            .expect("Failed to create HTTP client");

        Self { client }
    }

    /// Check health of a single endpoint
    pub async fn check(&self, name: &str, endpoint: &str) -> HealthCheckResult {
        let start = std::time::Instant::now();

        match self.client.get(endpoint).send().await {
            Ok(response) => {
                let response_time = start.elapsed().as_millis() as u64;
                let status = if response.status().is_success() {
                    HealthStatus::Healthy
                } else {
                    HealthStatus::Unhealthy
                };

                HealthCheckResult {
                    name: name.to_string(),
                    status,
                    response_time_ms: Some(response_time),
                    error: None,
                }
            }
            Err(e) => HealthCheckResult {
                name: name.to_string(),
                status: HealthStatus::Unhealthy,
                response_time_ms: None,
                error: Some(e.to_string()),
            },
        }
    }
}

impl Default for HealthChecker {
    fn default() -> Self {
        Self::new()
    }
}
