use super::ports::Broker;
use crate::domain::error::{AppError, AppResult};
use crate::domain::message::OutboundMessage;
use crate::domain::server::ServerId;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::broadcast;
use tracing::{info, instrument, warn};

/// How long to wait for the server to report a permission violation after a flush.
const VIOLATION_WINDOW: Duration = Duration::from_millis(500);

pub struct PublishService {
    broker: Arc<dyn Broker>,
}

impl PublishService {
    pub fn new(broker: Arc<dyn Broker>) -> Self {
        PublishService { broker }
    }

    /// Publishes as the server's configured user.
    ///
    /// NATS reports permission violations asynchronously (`-ERR 'Permissions Violation for
    /// Publish to "x"'`), not as a reply to PUB. So this listens for server errors before
    /// publishing and waits briefly after the flush for one that names the subject.
    #[instrument(skip(self, message), fields(subject = %message.subject))]
    pub async fn publish(&self, server_id: ServerId, message: OutboundMessage) -> AppResult<()> {
        let status = self
            .broker
            .status(server_id)
            .await
            .ok_or_else(|| AppError::not_found("server", server_id))?;
        if !status.is_connected() {
            return Err(AppError::Unavailable(
                status.error.unwrap_or_else(|| "not connected".into()),
            ));
        }
        let mut errors = self
            .broker
            .server_errors(server_id)
            .await
            .ok_or_else(|| AppError::not_found("server", server_id))?;

        let subject = message.subject.clone();
        self.broker.publish(server_id, message).await?;

        if let Some(violation) = wait_for_violation(&mut errors, &subject, VIOLATION_WINDOW).await {
            warn!(%violation, "publish rejected");
            return Err(AppError::PermissionDenied(violation));
        }
        info!("published");
        Ok(())
    }
}

async fn wait_for_violation(
    errors: &mut broadcast::Receiver<String>,
    subject: &str,
    within: Duration,
) -> Option<String> {
    let needle = format!("\"{subject}\"");
    tokio::time::timeout(within, async {
        loop {
            match errors.recv().await {
                Ok(e) if e.contains(&needle) => return Some(e),
                Ok(_) | Err(broadcast::error::RecvError::Lagged(_)) => continue,
                Err(broadcast::error::RecvError::Closed) => return None,
            }
        }
    })
    .await
    .ok()
    .flatten()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::fakes::FakeBroker;
    use crate::domain::server::Server;

    async fn setup(connected: bool) -> (PublishService, Arc<FakeBroker>) {
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
        broker.set_connected(1, connected);
        (PublishService::new(broker.clone()), broker)
    }

    fn msg(subject: &str) -> OutboundMessage {
        OutboundMessage::new(subject, "x", vec![]).unwrap()
    }

    #[tokio::test]
    async fn publishes_when_connected() {
        let (svc, broker) = setup(true).await;
        svc.publish(1, msg("test.ok")).await.unwrap();
        assert_eq!(broker.published(), vec!["test.ok".to_string()]);
    }

    #[tokio::test]
    async fn permission_violation_becomes_permission_denied() {
        let (svc, broker) = setup(true).await;
        broker.deny_publish("secret.x");
        let r = svc.publish(1, msg("secret.x")).await;
        assert!(matches!(r, Err(AppError::PermissionDenied(e)) if e.contains("secret.x")));
    }

    #[tokio::test]
    async fn unavailable_when_not_connected_and_not_found_when_unknown() {
        let (svc, _) = setup(false).await;
        assert!(matches!(
            svc.publish(1, msg("a")).await,
            Err(AppError::Unavailable(_))
        ));
        assert!(matches!(
            svc.publish(9, msg("a")).await,
            Err(AppError::NotFound(_))
        ));
    }
}
