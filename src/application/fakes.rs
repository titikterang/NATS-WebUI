//! In-memory implementations of the ports, for testing services without I/O.

use super::ports::{Broker, ClientRepo, ServerRepo, Subscription};
use crate::domain::client::{ClientId, ClientView, SubjectPattern};
use crate::domain::error::{AppError, AppResult};
use crate::domain::message::OutboundMessage;
use crate::domain::server::{Server, ServerId};
use crate::domain::status::NatsStatus;
use async_trait::async_trait;
use std::collections::{BTreeMap, HashMap};
use std::sync::Mutex;
use tokio::sync::broadcast;

#[derive(Default)]
pub struct MemServerRepo(Mutex<BTreeMap<ServerId, Server>>);

#[async_trait]
impl ServerRepo for MemServerRepo {
    async fn list(&self) -> AppResult<Vec<Server>> {
        Ok(self.0.lock().unwrap().values().cloned().collect())
    }
    async fn get(&self, id: ServerId) -> AppResult<Option<Server>> {
        Ok(self.0.lock().unwrap().get(&id).cloned())
    }
    async fn insert(&self, s: &Server) -> AppResult<ServerId> {
        let mut m = self.0.lock().unwrap();
        let id = m.keys().last().copied().unwrap_or(0) + 1;
        m.insert(id, Server { id, ..s.clone() });
        Ok(id)
    }
    async fn update(&self, s: &Server) -> AppResult<()> {
        self.0.lock().unwrap().insert(s.id, s.clone());
        Ok(())
    }
    async fn delete(&self, id: ServerId) -> AppResult<()> {
        self.0.lock().unwrap().remove(&id);
        Ok(())
    }
}

#[derive(Default)]
pub struct MemClientRepo(Mutex<BTreeMap<ClientId, ClientView>>);

#[async_trait]
impl ClientRepo for MemClientRepo {
    async fn list(&self) -> AppResult<Vec<ClientView>> {
        Ok(self.0.lock().unwrap().values().cloned().collect())
    }
    async fn get(&self, id: ClientId) -> AppResult<Option<ClientView>> {
        Ok(self.0.lock().unwrap().get(&id).cloned())
    }
    async fn insert(&self, c: &ClientView) -> AppResult<ClientId> {
        let mut m = self.0.lock().unwrap();
        let id = m.keys().last().copied().unwrap_or(0) + 1;
        m.insert(id, ClientView { id, ..c.clone() });
        Ok(id)
    }
    async fn update(&self, c: &ClientView) -> AppResult<()> {
        self.0.lock().unwrap().insert(c.id, c.clone());
        Ok(())
    }
    async fn delete(&self, id: ClientId) -> AppResult<()> {
        self.0.lock().unwrap().remove(&id);
        Ok(())
    }
    async fn delete_by_server(&self, server_id: ServerId) -> AppResult<()> {
        self.0
            .lock()
            .unwrap()
            .retain(|_, c| c.server_id != server_id);
        Ok(())
    }
}

struct FakeConn {
    connected: bool,
    connects: usize,
    errors: broadcast::Sender<String>,
}

/// A broker that records calls and can simulate a denied publish.
#[derive(Default)]
pub struct FakeBroker {
    conns: Mutex<HashMap<ServerId, FakeConn>>,
    published: Mutex<Vec<String>>,
    denied: Mutex<Vec<String>>,
}

impl FakeBroker {
    pub fn connected(&self, id: ServerId) -> bool {
        self.conns.lock().unwrap().contains_key(&id)
    }
    pub fn connects(&self, id: ServerId) -> usize {
        self.conns
            .lock()
            .unwrap()
            .get(&id)
            .map_or(0, |c| c.connects)
    }
    pub fn set_connected(&self, id: ServerId, up: bool) {
        if let Some(c) = self.conns.lock().unwrap().get_mut(&id) {
            c.connected = up;
        }
    }
    pub fn deny_publish(&self, subject: &str) {
        self.denied.lock().unwrap().push(subject.to_string());
    }
    pub fn published(&self) -> Vec<String> {
        self.published.lock().unwrap().clone()
    }
}

#[async_trait]
impl Broker for FakeBroker {
    async fn connect(&self, s: &Server) {
        let mut m = self.conns.lock().unwrap();
        let connects = m.get(&s.id).map_or(0, |c| c.connects) + 1;
        m.insert(
            s.id,
            FakeConn {
                connected: true,
                connects,
                errors: broadcast::channel(8).0,
            },
        );
    }
    async fn disconnect(&self, id: ServerId) {
        self.conns.lock().unwrap().remove(&id);
    }
    async fn status(&self, id: ServerId) -> Option<NatsStatus> {
        self.conns
            .lock()
            .unwrap()
            .get(&id)
            .map(|c| NatsStatus::derive(c.connected, None))
    }
    async fn server_errors(&self, id: ServerId) -> Option<broadcast::Receiver<String>> {
        self.conns
            .lock()
            .unwrap()
            .get(&id)
            .map(|c| c.errors.subscribe())
    }
    async fn publish(&self, id: ServerId, m: OutboundMessage) -> AppResult<()> {
        let conns = self.conns.lock().unwrap();
        let conn = conns
            .get(&id)
            .ok_or_else(|| AppError::not_found("server", id))?;
        if self.denied.lock().unwrap().contains(&m.subject) {
            let _ = conn.errors.send(format!(
                "nats: Permissions Violation for Publish to \"{}\"",
                m.subject
            ));
        } else {
            self.published.lock().unwrap().push(m.subject);
        }
        Ok(())
    }
    async fn subscribe(
        &self,
        _id: ServerId,
        patterns: &[SubjectPattern],
    ) -> AppResult<Subscription> {
        Ok(Subscription {
            messages: Box::pin(futures::stream::empty()),
            subscribed: patterns.len(),
            failures: vec![],
        })
    }
}
