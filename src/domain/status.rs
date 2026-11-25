use super::server::ServerId;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConnState {
    Connected,
    Connecting,
    AuthFailed,
    Error,
}

/// Health of the authenticated NATS connection for one server.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NatsStatus {
    pub state: ConnState,
    /// Why the connection isn't up. Always `None` when connected.
    pub error: Option<String>,
}

impl NatsStatus {
    /// Derives the user-facing state from the transport state and the last connect error.
    pub fn derive(connected: bool, last_error: Option<String>) -> Self {
        if connected {
            return NatsStatus {
                state: ConnState::Connected,
                error: None,
            };
        }
        let state = match &last_error {
            Some(e) if e.to_lowercase().contains("authorization") => ConnState::AuthFailed,
            Some(_) => ConnState::Error,
            None => ConnState::Connecting,
        };
        NatsStatus {
            state,
            error: last_error,
        }
    }

    pub fn is_connected(&self) -> bool {
        self.state == ConnState::Connected
    }
}

/// One tick of the status feed for one server.
#[derive(Clone, Debug)]
pub struct ServerStatus {
    pub server_id: ServerId,
    pub nats: NatsStatus,
    /// Raw `/varz` document. Deliberately untyped: the backend only relays it, and a typed struct
    /// broke on nats-server 2.15 (single nodes return `cluster: {}`).
    pub varz: Option<serde_json::Value>,
    pub varz_error: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn derives_state() {
        assert_eq!(
            NatsStatus::derive(true, Some("old".into())).state,
            ConnState::Connected
        );
        assert_eq!(NatsStatus::derive(true, Some("old".into())).error, None);
        assert_eq!(NatsStatus::derive(false, None).state, ConnState::Connecting);
        assert_eq!(
            NatsStatus::derive(false, Some("nats: authorization violation".into())).state,
            ConnState::AuthFailed
        );
        assert_eq!(
            NatsStatus::derive(false, Some("tls: unknown issuer".into())).state,
            ConnState::Error
        );
    }
}
