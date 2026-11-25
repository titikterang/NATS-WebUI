//! Wire format of the API. Domain types never derive Serialize/Deserialize; the mapping lives
//! here so the contract with the web UI (web/src/domain/types.ts) is in one place.

use crate::domain::client::{ClientDraft, ClientView};
use crate::domain::error::AppResult;
use crate::domain::message::{InboundMessage, OutboundMessage};
use crate::domain::secret::Secret;
use crate::domain::server::{Server, ServerDraft};
use crate::domain::status::{ConnState, NatsStatus, ServerStatus};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

// ---- servers -------------------------------------------------------------------------------

/// A server as the UI sees it. The password is never included; `has_password` says whether one
/// is stored.
#[derive(Serialize)]
pub struct ServerDto {
    id: i64,
    name: String,
    host: String,
    port: u16,
    monitoring_port: u16,
    username: Option<String>,
    has_password: bool,
}

impl From<&Server> for ServerDto {
    fn from(s: &Server) -> Self {
        ServerDto {
            id: s.id,
            name: s.name.clone(),
            host: s.host.clone(),
            port: s.port,
            monitoring_port: s.monitoring_port,
            username: s.credentials.as_ref().map(|c| c.username.clone()),
            has_password: s.has_password(),
        }
    }
}

/// Create/update body. Unknown fields (e.g. `has_password` echoed back) are ignored.
#[derive(Deserialize)]
pub struct ServerInput {
    #[serde(default)]
    pub id: Option<i64>,
    pub name: String,
    pub host: String,
    pub port: u16,
    pub monitoring_port: u16,
    #[serde(default)]
    pub username: Option<String>,
    /// `null` or `""` on update keeps the stored password.
    #[serde(default)]
    pub password: Option<String>,
}

impl ServerInput {
    pub fn into_draft(self) -> (Option<i64>, ServerDraft) {
        (
            self.id,
            ServerDraft {
                name: self.name,
                host: self.host,
                port: self.port,
                monitoring_port: self.monitoring_port,
                username: self.username,
                password: self.password.map(Secret::new),
            },
        )
    }
}

// ---- clients -------------------------------------------------------------------------------

#[derive(Serialize)]
pub struct ClientDto {
    id: i64,
    name: String,
    server_id: i64,
    subjects: Vec<String>,
}

impl From<&ClientView> for ClientDto {
    fn from(c: &ClientView) -> Self {
        ClientDto {
            id: c.id,
            name: c.name.clone(),
            server_id: c.server_id,
            subjects: c.patterns.iter().map(|p| p.as_str().to_string()).collect(),
        }
    }
}

#[derive(Deserialize)]
pub struct ClientInput {
    #[serde(default)]
    pub id: Option<i64>,
    pub name: String,
    pub server_id: i64,
    #[serde(default)]
    pub subjects: Vec<String>,
}

impl ClientInput {
    pub fn into_draft(self) -> (Option<i64>, ClientDraft) {
        (
            self.id,
            ClientDraft {
                name: self.name,
                server_id: self.server_id,
                patterns: self.subjects,
            },
        )
    }
}

#[derive(Serialize)]
pub struct StateDto {
    pub servers: Vec<ServerDto>,
    pub clients: Vec<ClientDto>,
}

// ---- status --------------------------------------------------------------------------------

#[derive(Serialize)]
pub struct NatsStatusDto {
    state: &'static str,
    error: Option<String>,
}

impl From<&NatsStatus> for NatsStatusDto {
    fn from(s: &NatsStatus) -> Self {
        let state = match s.state {
            ConnState::Connected => "connected",
            ConnState::Connecting => "connecting",
            ConnState::AuthFailed => "auth_failed",
            ConnState::Error => "error",
        };
        NatsStatusDto {
            state,
            error: s.error.clone(),
        }
    }
}

#[derive(Serialize)]
pub struct ServerStatusDto<'a> {
    server_id: i64,
    nats: NatsStatusDto,
    varz: &'a Option<serde_json::Value>,
    varz_error: &'a Option<String>,
}

impl<'a> From<&'a ServerStatus> for ServerStatusDto<'a> {
    fn from(s: &'a ServerStatus) -> Self {
        ServerStatusDto {
            server_id: s.server_id,
            nats: (&s.nats).into(),
            varz: &s.varz,
            varz_error: &s.varz_error,
        }
    }
}

// ---- publish -------------------------------------------------------------------------------

#[derive(Deserialize)]
pub struct HeaderInput {
    name: String,
    value: String,
}

#[derive(Deserialize)]
pub struct PublishInput {
    subject: String,
    #[serde(default)]
    payload: String,
    #[serde(default)]
    headers: Vec<HeaderInput>,
}

impl PublishInput {
    pub fn into_message(self) -> AppResult<OutboundMessage> {
        let headers = self
            .headers
            .into_iter()
            .map(|h| (h.name, h.value))
            .collect();
        OutboundMessage::new(&self.subject, self.payload, headers)
    }
}

// ---- live client stream frames ---------------------------------------------------------------

/// Payloads larger than this are cut in the preview (the full size is still reported).
const MAX_PAYLOAD_PREVIEW: usize = 64 * 1024;

#[derive(Serialize)]
#[serde(tag = "typ")]
pub enum Frame {
    Msg {
        ts: i64,
        subject: String,
        reply: Option<String>,
        headers: BTreeMap<String, Vec<String>>,
        size: usize,
        payload: String,
        truncated: bool,
    },
    Status {
        ts: i64,
        #[serde(flatten)]
        status: NatsStatusDto,
    },
    Err {
        ts: i64,
        message: String,
    },
}

fn now() -> i64 {
    chrono::Utc::now().timestamp_millis()
}

impl Frame {
    pub fn status(s: &NatsStatus) -> Self {
        Frame::Status {
            ts: now(),
            status: s.into(),
        }
    }

    pub fn err(message: impl Into<String>) -> Self {
        Frame::Err {
            ts: now(),
            message: message.into(),
        }
    }

    pub fn message(m: InboundMessage) -> Self {
        let size = m.payload.len();
        let mut headers: BTreeMap<String, Vec<String>> = BTreeMap::new();
        for (k, v) in m.headers {
            headers.entry(k).or_default().push(v);
        }
        Frame::Msg {
            ts: now(),
            subject: m.subject,
            reply: m.reply,
            headers,
            size,
            payload: String::from_utf8_lossy(&m.payload[..size.min(MAX_PAYLOAD_PREVIEW)])
                .into_owned(),
            truncated: size > MAX_PAYLOAD_PREVIEW,
        }
    }

    pub fn to_ws(&self) -> warp::ws::Message {
        warp::ws::Message::text(serde_json::to_string(self).unwrap_or_default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::server::Credentials;

    #[test]
    fn server_dto_never_contains_the_password() {
        let s = Server {
            id: 1,
            name: "n".into(),
            host: "h".into(),
            port: 1,
            monitoring_port: 2,
            credentials: Some(Credentials {
                username: "u".into(),
                password: Secret::new("topsecret"),
            }),
        };
        let json = serde_json::to_string(&ServerDto::from(&s)).unwrap();
        assert!(!json.contains("topsecret") && !json.contains("\"password\""));
        assert!(json.contains("\"has_password\":true"));
    }

    #[test]
    fn message_frame_groups_headers_and_truncates() {
        let m = InboundMessage {
            subject: "a".into(),
            reply: None,
            headers: vec![("X".into(), "1".into()), ("X".into(), "2".into())],
            payload: vec![b'x'; MAX_PAYLOAD_PREVIEW + 1].into(),
        };
        let json = serde_json::to_value(Frame::message(m)).unwrap();
        assert_eq!(json["typ"], "Msg");
        assert_eq!(json["headers"]["X"], serde_json::json!(["1", "2"]));
        assert_eq!(json["truncated"], true);
        assert_eq!(json["size"], MAX_PAYLOAD_PREVIEW + 1);
    }
}
