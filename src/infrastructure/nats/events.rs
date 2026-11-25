//! Turns async-nats connection events into connection state and server-error broadcasts.

use async_nats::{Event, ServerError};
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use tokio::sync::broadcast;
use tracing::{info, warn};

/// The most recent connect error. Cleared on every successful (re)connect.
pub type LastError = Arc<Mutex<Option<String>>>;

type Handler = Box<dyn Fn(Event) -> Pin<Box<dyn Future<Output = ()> + Send + Sync>> + Send + Sync>;

/// Builds the `event_callback` for one connection. Logs carry `server_id` but never credentials.
pub fn handler(
    server_id: i64,
    server: String,
    last_error: LastError,
    errors: broadcast::Sender<String>,
) -> Handler {
    Box::new(move |event| {
        let (server, last_error, errors) = (server.clone(), last_error.clone(), errors.clone());
        Box::pin(async move {
            match event {
                Event::Connected => {
                    info!(server_id, %server, "nats connected");
                    *last_error.lock().unwrap() = None;
                }
                Event::ServerError(ServerError::AuthorizationViolation) => {
                    warn!(server_id, %server, "nats authorization violation");
                    *last_error.lock().unwrap() =
                        Some("nats: authorization violation (check username/password)".into());
                }
                // Permission violations and similar arrive while connected; subscribers and the
                // publish use case listen for them.
                Event::ServerError(e) => {
                    warn!(server_id, %server, error = %e, "nats server error");
                    let _ = errors.send(e.to_string());
                }
                Event::ClientError(e) => {
                    warn!(server_id, %server, error = %e, "nats client error");
                    *last_error.lock().unwrap() = Some(e.to_string());
                }
                other => info!(server_id, %server, event = %other, "nats event"),
            }
        })
    })
}
