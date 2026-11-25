use super::ports::{Broker, ClientRepo, Subscription};
use crate::domain::client::ClientId;
use crate::domain::error::{AppError, AppResult};
use crate::domain::server::ServerId;
use crate::domain::status::NatsStatus;
use std::sync::Arc;
use tokio::sync::broadcast;
use tracing::{info, instrument};

/// A running subscription for one saved client.
pub struct LiveSubscription {
    pub server_id: ServerId,
    pub subscription: Subscription,
    /// Asynchronous server errors (permission violations for these subscriptions arrive here).
    pub server_errors: broadcast::Receiver<String>,
}

/// Opens the live message stream for a saved client.
pub struct SubscriptionService {
    clients: Arc<dyn ClientRepo>,
    broker: Arc<dyn Broker>,
}

impl SubscriptionService {
    pub fn new(clients: Arc<dyn ClientRepo>, broker: Arc<dyn Broker>) -> Self {
        SubscriptionService { clients, broker }
    }

    /// Fails with `NotFound` for an unknown client, so the caller can reject before upgrading.
    pub async fn ensure_client(&self, id: ClientId) -> AppResult<()> {
        self.clients
            .get(id)
            .await?
            .map(|_| ())
            .ok_or_else(|| AppError::not_found("client", id))
    }

    #[instrument(skip(self))]
    pub async fn open(&self, client_id: ClientId) -> AppResult<LiveSubscription> {
        let client = self
            .clients
            .get(client_id)
            .await?
            .ok_or_else(|| AppError::not_found("client", client_id))?;
        let server_errors = self
            .broker
            .server_errors(client.server_id)
            .await
            .ok_or_else(|| {
                AppError::Unavailable(format!("server {} is not configured", client.server_id))
            })?;
        let subscription = self
            .broker
            .subscribe(client.server_id, &client.patterns)
            .await?;
        info!(
            server_id = client.server_id,
            subscribed = subscription.subscribed,
            failed = subscription.failures.len(),
            "subscription opened"
        );
        Ok(LiveSubscription {
            server_id: client.server_id,
            subscription,
            server_errors,
        })
    }

    pub async fn status(&self, server_id: ServerId) -> NatsStatus {
        self.broker
            .status(server_id)
            .await
            .unwrap_or_else(|| NatsStatus::derive(false, None))
    }
}
