//! nats-server HTTP monitoring endpoint client (`/varz`, `/jsz`, `/subsz`). No auth.

use crate::application::ports::{MonitorEndpoint, Monitoring};
use crate::domain::error::{AppError, AppResult};
use crate::domain::server::Server;
use async_trait::async_trait;
use std::time::Duration;

pub struct HttpMonitoring {
    http: reqwest::Client,
}

impl HttpMonitoring {
    pub fn new() -> AppResult<Self> {
        let http = reqwest::Client::builder()
            .connect_timeout(Duration::from_millis(500))
            .build()
            .map_err(|e| AppError::Internal(format!("http client: {e}")))?;
        Ok(HttpMonitoring { http })
    }
}

#[async_trait]
impl Monitoring for HttpMonitoring {
    async fn fetch(
        &self,
        server: &Server,
        endpoint: MonitorEndpoint,
    ) -> AppResult<serde_json::Value> {
        let url = server.monitoring_url(endpoint.path());
        let resp = self
            .http
            .get(&url)
            .timeout(endpoint.timeout())
            .send()
            .await
            .and_then(|r| r.error_for_status())
            .map_err(describe)?;
        resp.json().await.map_err(describe)
    }
}

fn describe(e: reqwest::Error) -> AppError {
    let msg = if e.is_timeout() {
        "monitoring endpoint timed out".to_string()
    } else if e.is_connect() {
        "monitoring endpoint unreachable".to_string()
    } else if e.is_decode() {
        "monitoring endpoint returned invalid JSON".to_string()
    } else {
        e.without_url().to_string()
    };
    AppError::Upstream(msg)
}
