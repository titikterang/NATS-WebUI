use super::ports::{Broker, MonitorEndpoint, Monitoring, ServerRepo};
use crate::domain::error::AppResult;
use crate::domain::status::{NatsStatus, ServerStatus};
use futures::future::join_all;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::broadcast;
use tracing::warn;

const TICK: Duration = Duration::from_secs(1);

/// The live status feed: every second, one `ServerStatus` per server.
pub struct StatusService {
    servers: Arc<dyn ServerRepo>,
    broker: Arc<dyn Broker>,
    monitoring: Arc<dyn Monitoring>,
    tx: broadcast::Sender<ServerStatus>,
}

impl StatusService {
    pub fn new(
        servers: Arc<dyn ServerRepo>,
        broker: Arc<dyn Broker>,
        monitoring: Arc<dyn Monitoring>,
    ) -> Self {
        let (tx, _) = broadcast::channel(256);
        StatusService {
            servers,
            broker,
            monitoring,
            tx,
        }
    }

    pub fn subscribe(&self) -> broadcast::Receiver<ServerStatus> {
        self.tx.subscribe()
    }

    /// The current status of every server.
    pub async fn snapshot(&self) -> AppResult<Vec<ServerStatus>> {
        let servers = self.servers.list().await?;
        Ok(join_all(servers.iter().map(|s| async move {
            let varz = self.monitoring.fetch(s, MonitorEndpoint::Varz).await;
            ServerStatus {
                server_id: s.id,
                nats: self
                    .broker
                    .status(s.id)
                    .await
                    .unwrap_or_else(|| NatsStatus::derive(false, None)),
                varz_error: varz.as_ref().err().map(|e| e.to_string()),
                varz: varz.ok(),
            }
        }))
        .await)
    }

    /// Starts the background poller. It only does work while someone is listening.
    pub fn spawn(self: Arc<Self>) {
        tokio::spawn(async move {
            let mut tick = tokio::time::interval(TICK);
            tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
            loop {
                tick.tick().await;
                if self.tx.receiver_count() == 0 {
                    continue;
                }
                match self.snapshot().await {
                    Ok(statuses) => statuses.into_iter().for_each(|s| {
                        let _ = self.tx.send(s);
                    }),
                    Err(e) => warn!(error = %e, "status poll failed"),
                }
            }
        });
    }
}
