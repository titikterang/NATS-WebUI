use super::ports::{ClientRepo, ServerRepo};
use crate::domain::client::{ClientDraft, ClientId, ClientView};
use crate::domain::error::{AppError, AppResult};
use std::sync::Arc;
use tracing::{info, instrument};

/// Saved subscription views ("clients").
pub struct ClientService {
    clients: Arc<dyn ClientRepo>,
    servers: Arc<dyn ServerRepo>,
}

impl ClientService {
    pub fn new(clients: Arc<dyn ClientRepo>, servers: Arc<dyn ServerRepo>) -> Self {
        ClientService { clients, servers }
    }

    pub async fn list(&self) -> AppResult<Vec<ClientView>> {
        self.clients.list().await
    }

    pub async fn get(&self, id: ClientId) -> AppResult<ClientView> {
        self.clients
            .get(id)
            .await?
            .ok_or_else(|| AppError::not_found("client", id))
    }

    #[instrument(skip_all)]
    pub async fn create(&self, draft: ClientDraft) -> AppResult<ClientView> {
        self.ensure_server(draft.server_id).await?;
        let client = draft.into_client(0)?;
        let id = self.clients.insert(&client).await?;
        info!(client_id = id, "client created");
        self.get(id).await
    }

    #[instrument(skip(self, draft))]
    pub async fn update(&self, id: ClientId, draft: ClientDraft) -> AppResult<ClientView> {
        self.get(id).await?;
        self.ensure_server(draft.server_id).await?;
        let client = draft.into_client(id)?;
        self.clients.update(&client).await?;
        Ok(client)
    }

    #[instrument(skip(self))]
    pub async fn delete(&self, id: ClientId) -> AppResult<()> {
        self.get(id).await?;
        self.clients.delete(id).await
    }

    async fn ensure_server(&self, server_id: i64) -> AppResult<()> {
        match self.servers.get(server_id).await? {
            Some(_) => Ok(()),
            None => Err(AppError::validation(format!(
                "server {server_id} does not exist"
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::fakes::{MemClientRepo, MemServerRepo};
    use crate::domain::server::Server;

    #[tokio::test]
    async fn create_requires_existing_server_and_valid_patterns() {
        let servers = Arc::new(MemServerRepo::default());
        let sid = servers
            .insert(&Server {
                id: 0,
                name: "s".into(),
                host: "h".into(),
                port: 1,
                monitoring_port: 2,
                credentials: None,
            })
            .await
            .unwrap();
        let svc = ClientService::new(Arc::new(MemClientRepo::default()), servers);

        let draft = |server_id, p: &str| ClientDraft {
            name: "c".into(),
            server_id,
            patterns: vec![p.into()],
        };
        assert!(matches!(
            svc.create(draft(99, "a.b")).await,
            Err(AppError::Validation(_))
        ));
        assert!(matches!(
            svc.create(draft(sid, "a..b")).await,
            Err(AppError::Validation(_))
        ));

        let c = svc.create(draft(sid, "test.>")).await.unwrap();
        assert_eq!(c.patterns[0].as_str(), "test.>");
        assert!(matches!(
            svc.update(77, draft(sid, "x")).await,
            Err(AppError::NotFound(_))
        ));
    }
}
