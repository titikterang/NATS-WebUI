//! Ports: what the application needs from the outside world. Implemented in `infrastructure`,
//! faked in tests (`application::fakes`).

use crate::domain::client::{ClientId, ClientView, SubjectPattern};
use crate::domain::error::AppResult;
use crate::domain::message::{InboundMessage, OutboundMessage};
use crate::domain::server::{Server, ServerId};
use crate::domain::status::NatsStatus;
use async_trait::async_trait;
use futures::stream::BoxStream;
use std::time::Duration;
use tokio::sync::broadcast;

#[async_trait]
pub trait ServerRepo: Send + Sync {
    async fn list(&self) -> AppResult<Vec<Server>>;
    async fn get(&self, id: ServerId) -> AppResult<Option<Server>>;
    /// Stores a new server and returns its id (`server.id` is ignored).
    async fn insert(&self, server: &Server) -> AppResult<ServerId>;
    async fn update(&self, server: &Server) -> AppResult<()>;
    async fn delete(&self, id: ServerId) -> AppResult<()>;
}

#[async_trait]
pub trait ClientRepo: Send + Sync {
    async fn list(&self) -> AppResult<Vec<ClientView>>;
    async fn get(&self, id: ClientId) -> AppResult<Option<ClientView>>;
    /// Stores a new client and returns its id (`client.id` is ignored).
    async fn insert(&self, client: &ClientView) -> AppResult<ClientId>;
    async fn update(&self, client: &ClientView) -> AppResult<()>;
    async fn delete(&self, id: ClientId) -> AppResult<()>;
    async fn delete_by_server(&self, server_id: ServerId) -> AppResult<()>;
}

pub type MessageStream = BoxStream<'static, InboundMessage>;

/// Result of subscribing a set of patterns: one merged stream, plus the patterns that failed.
pub struct Subscription {
    pub messages: MessageStream,
    pub subscribed: usize,
    pub failures: Vec<(SubjectPattern, String)>,
}

/// The NATS side: one managed connection per server.
#[async_trait]
pub trait Broker: Send + Sync {
    /// Opens (or replaces) the connection for `server`.
    async fn connect(&self, server: &Server);
    async fn disconnect(&self, id: ServerId);
    /// `None` if the server has no managed connection.
    async fn status(&self, id: ServerId) -> Option<NatsStatus>;
    /// Errors the server reports asynchronously while connected (e.g. permission violations).
    async fn server_errors(&self, id: ServerId) -> Option<broadcast::Receiver<String>>;
    /// Publishes and flushes.
    async fn publish(&self, id: ServerId, message: OutboundMessage) -> AppResult<()>;
    async fn subscribe(&self, id: ServerId, patterns: &[SubjectPattern])
        -> AppResult<Subscription>;
}

/// Endpoints of the nats-server HTTP monitoring port that the UI uses.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MonitorEndpoint {
    Varz,
    Jsz,
    Subsz,
}

impl MonitorEndpoint {
    pub fn path(self) -> &'static str {
        match self {
            MonitorEndpoint::Varz => "varz",
            MonitorEndpoint::Jsz => "jsz?streams=true&consumers=true&config=true",
            MonitorEndpoint::Subsz => "subsz?subs=1&limit=1024",
        }
    }

    pub fn timeout(self) -> Duration {
        match self {
            // Polled every second, so it must finish within the tick.
            MonitorEndpoint::Varz => Duration::from_millis(900),
            _ => Duration::from_secs(5),
        }
    }
}

#[async_trait]
pub trait Monitoring: Send + Sync {
    async fn fetch(
        &self,
        server: &Server,
        endpoint: MonitorEndpoint,
    ) -> AppResult<serde_json::Value>;
}
