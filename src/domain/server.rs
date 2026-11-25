use super::error::{AppError, AppResult};
use super::secret::Secret;

pub type ServerId = i64;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Credentials {
    pub username: String,
    pub password: Secret,
}

/// A configured NATS server node. A cluster is configured as one `Server` per node.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Server {
    /// 0 until the server has been stored.
    pub id: ServerId,
    pub name: String,
    pub host: String,
    pub port: u16,
    pub monitoring_port: u16,
    pub credentials: Option<Credentials>,
}

impl Server {
    pub fn nats_url(&self) -> String {
        format!("nats://{}:{}", self.host, self.port)
    }

    /// URL of a path on the plain-HTTP monitoring endpoint, e.g. `varz`.
    pub fn monitoring_url(&self, path: &str) -> String {
        format!("http://{}:{}/{}", self.host, self.monitoring_port, path)
    }

    pub fn has_password(&self) -> bool {
        self.credentials
            .as_ref()
            .is_some_and(|c| !c.password.is_empty())
    }
}

/// User input for creating or editing a server.
#[derive(Clone, Debug, Default)]
pub struct ServerDraft {
    pub name: String,
    pub host: String,
    pub port: u16,
    pub monitoring_port: u16,
    /// Blank or missing means "no authentication".
    pub username: Option<String>,
    /// On edit, blank or missing means "keep the stored password".
    pub password: Option<Secret>,
}

impl ServerDraft {
    /// Builds a new, not yet stored server.
    pub fn into_new_server(self) -> AppResult<Server> {
        let (name, host) = self.validate()?;
        let credentials = trimmed(self.username).map(|username| Credentials {
            username,
            password: self.password.unwrap_or_default(),
        });
        Ok(Server {
            id: 0,
            name,
            host,
            port: self.port,
            monitoring_port: self.monitoring_port,
            credentials,
        })
    }

    /// Applies an edit to a stored server.
    ///
    /// Rules: a blank username removes the credentials. A blank password keeps the stored one,
    /// because the stored password is never sent back to the browser to be re-submitted.
    pub fn apply_to(self, current: &Server) -> AppResult<Server> {
        let (name, host) = self.validate()?;
        let credentials = trimmed(self.username).map(|username| {
            let password = self.password.filter(|p| !p.is_empty()).unwrap_or_else(|| {
                current
                    .credentials
                    .as_ref()
                    .map(|c| c.password.clone())
                    .unwrap_or_default()
            });
            Credentials { username, password }
        });
        Ok(Server {
            id: current.id,
            name,
            host,
            port: self.port,
            monitoring_port: self.monitoring_port,
            credentials,
        })
    }

    fn validate(&self) -> AppResult<(String, String)> {
        let name = self.name.trim();
        let host = self.host.trim();
        if name.is_empty() {
            return Err(AppError::validation("name is required"));
        }
        if host.is_empty() {
            return Err(AppError::validation("host is required"));
        }
        if host.contains("://") || host.contains('/') || host.chars().any(char::is_whitespace) {
            return Err(AppError::validation(
                "host must be a hostname or IP address, without scheme, path or spaces",
            ));
        }
        if self.port == 0 || self.monitoring_port == 0 {
            return Err(AppError::validation(
                "port and monitoring port are required",
            ));
        }
        Ok((name.to_string(), host.to_string()))
    }
}

fn trimmed(s: Option<String>) -> Option<String> {
    s.map(|v| v.trim().to_string()).filter(|v| !v.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn draft() -> ServerDraft {
        ServerDraft {
            name: " sbx ".into(),
            host: " 10.0.0.1 ".into(),
            port: 4222,
            monitoring_port: 8222,
            username: Some("user".into()),
            password: Some(Secret::new("pw1")),
        }
    }

    #[test]
    fn new_server_is_trimmed_and_carries_credentials() {
        let s = draft().into_new_server().unwrap();
        assert_eq!((s.name.as_str(), s.host.as_str()), ("sbx", "10.0.0.1"));
        assert_eq!(s.credentials.unwrap().password.expose(), "pw1");
    }

    #[test]
    fn blank_username_means_no_credentials() {
        let s = ServerDraft {
            username: Some("  ".into()),
            ..draft()
        }
        .into_new_server()
        .unwrap();
        assert!(s.credentials.is_none());
        assert!(!s.has_password());
    }

    #[test]
    fn edit_keeps_password_when_blank_and_rotates_when_given() {
        let mut current = draft().into_new_server().unwrap();
        current.id = 7;

        let kept = ServerDraft {
            password: None,
            ..draft()
        }
        .apply_to(&current)
        .unwrap();
        assert_eq!(kept.credentials.as_ref().unwrap().password.expose(), "pw1");
        assert_eq!(kept.id, 7);

        let kept = ServerDraft {
            password: Some(Secret::new("")),
            ..draft()
        }
        .apply_to(&current)
        .unwrap();
        assert_eq!(kept.credentials.as_ref().unwrap().password.expose(), "pw1");

        let rotated = ServerDraft {
            password: Some(Secret::new("pw2")),
            ..draft()
        }
        .apply_to(&current)
        .unwrap();
        assert_eq!(rotated.credentials.unwrap().password.expose(), "pw2");

        let cleared = ServerDraft {
            username: None,
            ..draft()
        }
        .apply_to(&current)
        .unwrap();
        assert!(cleared.credentials.is_none());
    }

    #[test]
    fn rejects_bad_input() {
        for bad in [
            ServerDraft {
                name: " ".into(),
                ..draft()
            },
            ServerDraft {
                host: "".into(),
                ..draft()
            },
            ServerDraft {
                host: "nats://10.0.0.1".into(),
                ..draft()
            },
            ServerDraft { port: 0, ..draft() },
        ] {
            assert!(matches!(
                bad.into_new_server(),
                Err(AppError::Validation(_))
            ));
        }
    }
}
