//! Logging setup and conventions.
//!
//! **One pipeline.** Our code and async-nats/hyper emit `tracing` events. Dependencies that use the
//! `log` facade (reqwest, parts of warp) are bridged by tracing-subscriber's `tracing-log` feature,
//! so everything reaches the same subscriber, filter and format. This follows the `log` crate's
//! guidance: libraries emit through a facade, and the executable chooses the implementation.
//!
//! **Levels.**
//! - `error`: a bug or a broken invariant that needs a developer (for example `AppError::Internal`).
//! - `warn`: degraded behaviour caused by something outside the process: auth or permission
//!   violations, unreachable servers. Operators should look.
//! - `info`: lifecycle events: startup, config changes (server created/updated/deleted),
//!   connections up or down, streams opened or closed.
//! - `debug`: per-request detail, off by default.
//! - Never log per NATS message (hot path).
//!
//! **Fields, not prose.** Put identifiers in structured fields (`server_id`, `client_id`,
//! `subject`, `error`) so logs can be filtered. Every HTTP request runs inside an `http` span
//! with `req`, `method` and `path`, so a request's events can be correlated.
//!
//! **Secrets.** Credentials are `domain::secret::Secret`, whose `Debug`/`Display` print `***`.
//! Spans that take user input use `#[instrument(skip(...))]`. The tests below enforce both.
//!
//! **Log or return, not both.** Services return `AppError`; the HTTP error mapper logs
//! `Internal` errors once. Expected failures (`warn`) are logged where they're detected.

use std::io::IsTerminal;
use tracing_subscriber::EnvFilter;

/// Output format, from `LOG_FORMAT`: `text` (default, human-readable) or `json` (one JSON object
/// per line, for log aggregation in production).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LogFormat {
    Text,
    Json,
}

impl LogFormat {
    pub fn from_env() -> Result<Self, String> {
        Self::parse(&std::env::var("LOG_FORMAT").unwrap_or_default())
    }

    pub fn parse(raw: &str) -> Result<Self, String> {
        match raw.trim().to_lowercase().as_str() {
            "" | "text" => Ok(LogFormat::Text),
            "json" => Ok(LogFormat::Json),
            other => Err(format!(
                "LOG_FORMAT must be \"text\" or \"json\", got {other:?}"
            )),
        }
    }
}

/// Installs the global subscriber. Filter comes from `RUST_LOG` (default `info`).
pub fn init(format: LogFormat) {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    let builder = tracing_subscriber::fmt().with_env_filter(filter);
    match format {
        LogFormat::Json => builder
            .json()
            .flatten_event(true)
            .with_current_span(true)
            .init(),
        // Colors only on a terminal: no ANSI escape codes in files or `docker logs`.
        LogFormat::Text => builder.with_ansi(std::io::stdout().is_terminal()).init(),
    }
}

/// Test helper: runs `f` with a subscriber that writes into a buffer, and returns what was logged.
#[cfg(test)]
pub async fn capture<F: std::future::Future>(f: F) -> String {
    use std::sync::{Arc, Mutex};

    #[derive(Clone, Default)]
    struct Buf(Arc<Mutex<Vec<u8>>>);
    impl std::io::Write for Buf {
        fn write(&mut self, b: &[u8]) -> std::io::Result<usize> {
            self.0.lock().unwrap().extend_from_slice(b);
            Ok(b.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    let buf = Buf::default();
    let writer = buf.clone();
    let subscriber = tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::new("trace"))
        .with_ansi(false)
        .with_writer(move || writer.clone())
        .finish();
    let _guard = tracing::subscriber::set_default(subscriber);
    f.await;
    let out = buf.0.lock().unwrap().clone();
    String::from_utf8(out).unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::fakes::{FakeBroker, MemClientRepo, MemServerRepo};
    use crate::application::servers::ServerService;
    use crate::domain::secret::Secret;
    use crate::domain::server::ServerDraft;
    use std::sync::Arc;

    const CANARY: &str = "canary-P4ssw0rd-do-not-log";

    fn draft(password: Option<&str>) -> ServerDraft {
        ServerDraft {
            name: "sbx".into(),
            host: "10.0.0.1".into(),
            port: 4222,
            monitoring_port: 8222,
            username: Some("svc-user".into()),
            password: password.map(Secret::new),
        }
    }

    /// Gatekeeper: config changes are logged at info with ids, and the password never appears,
    /// even at `trace` level with the input draft in scope.
    #[tokio::test(flavor = "current_thread")]
    async fn server_lifecycle_logs_ids_but_never_the_password() {
        let svc = ServerService::new(
            Arc::new(MemServerRepo::default()),
            Arc::new(MemClientRepo::default()),
            Arc::new(FakeBroker::default()),
        );
        let logs = capture(async {
            let s = svc.create(draft(Some(CANARY))).await.unwrap();
            svc.update(s.id, draft(Some(&format!("{CANARY}-2"))))
                .await
                .unwrap();
            svc.delete(s.id).await.unwrap();
            // Debug of the whole entity must be safe too.
            tracing::debug!(server = ?s, "debug dump");
        })
        .await;

        assert!(!logs.contains(CANARY), "password leaked into logs:\n{logs}");
        for expected in [
            "server created",
            "server updated",
            "server deleted",
            "server_id=1",
            "INFO",
        ] {
            assert!(logs.contains(expected), "missing {expected:?} in:\n{logs}");
        }
        assert!(
            logs.contains("Secret(***)"),
            "Debug of a server must redact the password"
        );
    }

    /// Gatekeeper: a rejected publish is a `warn` with the reason, not an `error`.
    #[tokio::test(flavor = "current_thread")]
    async fn permission_denied_is_a_warning_with_the_reason() {
        use crate::application::ports::Broker;
        use crate::application::publish::PublishService;
        use crate::domain::message::OutboundMessage;
        use crate::domain::server::Server;

        let broker = Arc::new(FakeBroker::default());
        broker
            .connect(&Server {
                id: 1,
                name: "s".into(),
                host: "h".into(),
                port: 1,
                monitoring_port: 2,
                credentials: None,
            })
            .await;
        broker.deny_publish("secret.x");
        let svc = PublishService::new(broker);

        let logs = capture(async {
            let _ = svc
                .publish(1, OutboundMessage::new("secret.x", "x", vec![]).unwrap())
                .await;
        })
        .await;
        assert!(
            logs.contains("WARN") && logs.contains("publish rejected"),
            "{logs}"
        );
        assert!(logs.contains("subject=secret.x"), "{logs}");
        assert!(!logs.contains("ERROR"), "{logs}");
    }

    #[test]
    fn log_format_parses() {
        assert_eq!(LogFormat::parse(""), Ok(LogFormat::Text));
        assert_eq!(LogFormat::parse(" TEXT "), Ok(LogFormat::Text));
        assert_eq!(LogFormat::parse("json"), Ok(LogFormat::Json));
        assert!(LogFormat::parse("xml").is_err());
    }
}
