use super::ports::{Broker, ClientRepo, ServerRepo};
use crate::domain::error::{AppError, AppResult};
use crate::domain::server::{Server, ServerDraft, ServerId};
use std::sync::Arc;
use tracing::{info, instrument};

/// Server configuration use cases. Keeps the store and the live NATS connections in sync.
pub struct ServerService {
    servers: Arc<dyn ServerRepo>,
    clients: Arc<dyn ClientRepo>,
    broker: Arc<dyn Broker>,
}

impl ServerService {
    pub fn new(
        servers: Arc<dyn ServerRepo>,
        clients: Arc<dyn ClientRepo>,
        broker: Arc<dyn Broker>,
    ) -> Self {
        ServerService {
            servers,
            clients,
            broker,
        }
    }

    /// Opens a connection for every stored server (at startup).
    pub async fn connect_all(&self) -> AppResult<usize> {
        let servers = self.servers.list().await?;
        for s in &servers {
            self.broker.connect(s).await;
        }
        Ok(servers.len())
    }

    pub async fn list(&self) -> AppResult<Vec<Server>> {
        self.servers.list().await
    }

    pub async fn get(&self, id: ServerId) -> AppResult<Server> {
        self.servers
            .get(id)
            .await?
            .ok_or_else(|| AppError::not_found("server", id))
    }

    #[instrument(skip_all)]
    pub async fn create(&self, draft: ServerDraft) -> AppResult<Server> {
        let server = draft.into_new_server()?;
        let id = self.servers.insert(&server).await?;
        let saved = self.get(id).await?;
        self.broker.connect(&saved).await;
        info!(server_id = id, server = %saved.name, "server created");
        Ok(saved)
    }

    #[instrument(skip(self, draft))]
    pub async fn update(&self, id: ServerId, draft: ServerDraft) -> AppResult<Server> {
        let current = self.get(id).await?;
        let server = draft.apply_to(&current)?;
        self.servers.update(&server).await?;
        self.broker.connect(&server).await;
        info!(server = %server.name, "server updated");
        Ok(server)
    }

    /// Deletes a server together with the clients that watch it.
    #[instrument(skip(self))]
    pub async fn delete(&self, id: ServerId) -> AppResult<()> {
        self.get(id).await?;
        self.clients.delete_by_server(id).await?;
        self.servers.delete(id).await?;
        self.broker.disconnect(id).await;
        info!("server deleted");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::fakes::{FakeBroker, MemClientRepo, MemServerRepo};
    use crate::domain::client::ClientView;
    use crate::domain::secret::Secret;

    fn draft(password: Option<&str>) -> ServerDraft {
        ServerDraft {
            name: "sbx".into(),
            host: "10.0.0.1".into(),
            port: 4222,
            monitoring_port: 8222,
            username: Some("u".into()),
            password: password.map(Secret::new),
        }
    }

    fn service() -> (
        ServerService,
        Arc<MemServerRepo>,
        Arc<MemClientRepo>,
        Arc<FakeBroker>,
    ) {
        let (s, c, b) = (
            Arc::new(MemServerRepo::default()),
            Arc::new(MemClientRepo::default()),
            Arc::new(FakeBroker::default()),
        );
        (ServerService::new(s.clone(), c.clone(), b.clone()), s, c, b)
    }

    #[tokio::test]
    async fn create_update_delete_keep_broker_in_sync() {
        let (svc, _, clients, broker) = service();
        let s = svc.create(draft(Some("pw"))).await.unwrap();
        assert!(broker.connected(s.id));

        let s2 = svc.update(s.id, draft(None)).await.unwrap();
        assert_eq!(
            s2.credentials.unwrap().password.expose(),
            "pw",
            "blank password keeps stored"
        );
        assert_eq!(broker.connects(s.id), 2, "update reconnects");

        clients
            .insert(&ClientView {
                id: 0,
                name: "c".into(),
                server_id: s.id,
                patterns: vec![],
            })
            .await
            .unwrap();
        svc.delete(s.id).await.unwrap();
        assert!(!broker.connected(s.id));
        assert!(
            clients.list().await.unwrap().is_empty(),
            "clients are deleted with their server"
        );
    }

    #[tokio::test]
    async fn unknown_server_is_not_found() {
        let (svc, ..) = service();
        assert!(matches!(
            svc.update(42, draft(None)).await,
            Err(AppError::NotFound(_))
        ));
        assert!(matches!(svc.delete(42).await, Err(AppError::NotFound(_))));
    }

    #[tokio::test]
    async fn invalid_input_is_rejected_before_storing() {
        let (svc, repo, ..) = service();
        let bad = ServerDraft {
            host: "".into(),
            ..draft(None)
        };
        assert!(matches!(
            svc.create(bad).await,
            Err(AppError::Validation(_))
        ));
        assert!(repo.list().await.unwrap().is_empty());
    }
}
