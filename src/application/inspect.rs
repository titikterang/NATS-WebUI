use super::ports::{MonitorEndpoint, Monitoring, ServerRepo};
use crate::domain::error::{AppError, AppResult};
use crate::domain::server::ServerId;
use std::sync::Arc;

/// Read-only monitoring data (JetStream, subscriptions) for one server.
pub struct InspectService {
    servers: Arc<dyn ServerRepo>,
    monitoring: Arc<dyn Monitoring>,
}

impl InspectService {
    pub fn new(servers: Arc<dyn ServerRepo>, monitoring: Arc<dyn Monitoring>) -> Self {
        InspectService {
            servers,
            monitoring,
        }
    }

    pub async fn fetch(
        &self,
        id: ServerId,
        endpoint: MonitorEndpoint,
    ) -> AppResult<serde_json::Value> {
        let server = self
            .servers
            .get(id)
            .await?
            .ok_or_else(|| AppError::not_found("server", id))?;
        self.monitoring.fetch(&server, endpoint).await
    }
}
