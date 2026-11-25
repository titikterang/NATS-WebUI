use super::events::{self, LastError};
use crate::application::ports::{Broker, Subscription};
use crate::domain::client::SubjectPattern;
use crate::domain::error::{AppError, AppResult};
use crate::domain::message::{InboundMessage, OutboundMessage};
use crate::domain::server::{Server, ServerId};
use crate::domain::status::NatsStatus;
use async_nats::connection::State;
use async_nats::{ConnectOptions, HeaderMap};
use async_trait::async_trait;
use futures::StreamExt;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{broadcast, RwLock};
use tracing::warn;

const CONNECT_TIMEOUT: Duration = Duration::from_secs(5);
const FLUSH_TIMEOUT: Duration = Duration::from_secs(3);

struct Connection {
    /// `None` only if the connect options were invalid (e.g. an unreadable CA file).
    client: Option<async_nats::Client>,
    last_error: LastError,
    errors: broadcast::Sender<String>,
}

impl Connection {
    fn status(&self) -> NatsStatus {
        let connected = self
            .client
            .as_ref()
            .is_some_and(|c| c.connection_state() == State::Connected);
        NatsStatus::derive(connected, self.last_error.lock().unwrap().clone())
    }

    fn client(&self) -> AppResult<&async_nats::Client> {
        self.client.as_ref().ok_or_else(|| {
            AppError::Unavailable(
                self.last_error
                    .lock()
                    .unwrap()
                    .clone()
                    .unwrap_or_else(|| "not connected".into()),
            )
        })
    }
}

pub struct NatsBroker {
    ca_file: Option<PathBuf>,
    conns: RwLock<HashMap<ServerId, Arc<Connection>>>,
}

impl NatsBroker {
    pub fn new(ca_file: Option<PathBuf>) -> Self {
        NatsBroker {
            ca_file,
            conns: RwLock::new(HashMap::new()),
        }
    }

    async fn get(&self, id: ServerId) -> AppResult<Arc<Connection>> {
        self.conns
            .read()
            .await
            .get(&id)
            .cloned()
            .ok_or_else(|| AppError::not_found("server", id))
    }

    async fn open(&self, server: &Server) -> Connection {
        let last_error: LastError = Arc::default();
        let (errors, _) = broadcast::channel(64);

        let mut opts = ConnectOptions::new()
            .name("nats-webui")
            .connection_timeout(CONNECT_TIMEOUT)
            .retry_on_initial_connect()
            .max_reconnects(None)
            .reconnect_delay_callback(|attempts| {
                Duration::from_secs((attempts as u64).clamp(1, 15))
            })
            .event_callback(events::handler(
                server.id,
                server.name.clone(),
                last_error.clone(),
                errors.clone(),
            ));
        if let Some(c) = &server.credentials {
            opts = opts.user_and_password(c.username.clone(), c.password.expose().to_string());
        }
        if let Some(ca) = &self.ca_file {
            // In async-nats, adding a CA alone makes every server TLS-only, which breaks plain
            // `nats://` servers. `tls_optional` keeps TLS per server: used when the server's INFO
            // requires it (or the URL is tls://), verified against this CA.
            opts = opts.add_root_certificates(ca.clone()).tls_optional();
        }

        // With retry_on_initial_connect this returns at once and connects in the background;
        // only invalid options fail here.
        let client = match opts.connect(server.nats_url()).await {
            Ok(c) => Some(c),
            Err(e) => {
                warn!(server_id = server.id, error = %e, "nats connect options rejected");
                *last_error.lock().unwrap() = Some(e.to_string());
                None
            }
        };
        Connection {
            client,
            last_error,
            errors,
        }
    }
}

fn close(conn: Arc<Connection>) {
    tokio::spawn(async move {
        if let Some(c) = &conn.client {
            let _ = c.drain().await;
        }
    });
}

fn to_inbound(m: async_nats::Message) -> InboundMessage {
    let mut headers = Vec::new();
    if let Some(h) = &m.headers {
        for (name, values) in h.iter() {
            for v in values {
                headers.push((name.to_string(), v.to_string()));
            }
        }
    }
    InboundMessage {
        subject: m.subject.to_string(),
        reply: m.reply.map(|r| r.to_string()),
        headers,
        payload: m.payload,
    }
}

#[async_trait]
impl Broker for NatsBroker {
    async fn connect(&self, server: &Server) {
        let conn = Arc::new(self.open(server).await);
        if let Some(old) = self.conns.write().await.insert(server.id, conn) {
            close(old);
        }
    }

    async fn disconnect(&self, id: ServerId) {
        if let Some(old) = self.conns.write().await.remove(&id) {
            close(old);
        }
    }

    async fn status(&self, id: ServerId) -> Option<NatsStatus> {
        self.conns.read().await.get(&id).map(|c| c.status())
    }

    async fn server_errors(&self, id: ServerId) -> Option<broadcast::Receiver<String>> {
        self.conns
            .read()
            .await
            .get(&id)
            .map(|c| c.errors.subscribe())
    }

    async fn publish(&self, id: ServerId, m: OutboundMessage) -> AppResult<()> {
        let conn = self.get(id).await?;
        let client = conn.client()?;
        let upstream = |e: &dyn std::fmt::Display| AppError::Upstream(e.to_string());
        if m.headers.is_empty() {
            client
                .publish(m.subject, m.payload)
                .await
                .map_err(|e| upstream(&e))?;
        } else {
            let mut headers = HeaderMap::new();
            for (name, value) in &m.headers {
                headers.append(name.as_str(), value.as_str());
            }
            client
                .publish_with_headers(m.subject, headers, m.payload)
                .await
                .map_err(|e| upstream(&e))?;
        }
        tokio::time::timeout(FLUSH_TIMEOUT, client.flush())
            .await
            .map_err(|_| AppError::Upstream("flush timed out".into()))?
            .map_err(|e| upstream(&e))
    }

    async fn subscribe(
        &self,
        id: ServerId,
        patterns: &[SubjectPattern],
    ) -> AppResult<Subscription> {
        let conn = self.get(id).await?;
        let client = conn.client()?;
        let (mut subs, mut failures) = (Vec::new(), Vec::new());
        for p in patterns {
            match client.subscribe(p.as_str().to_string()).await {
                Ok(s) => subs.push(s),
                Err(e) => failures.push((p.clone(), e.to_string())),
            }
        }
        let subscribed = subs.len();
        // Dropping the stream drops the subscribers, which unsubscribes them.
        let messages = if subs.is_empty() {
            futures::stream::pending().boxed()
        } else {
            futures::stream::select_all(subs).map(to_inbound).boxed()
        };
        Ok(Subscription {
            messages,
            subscribed,
            failures,
        })
    }
}
