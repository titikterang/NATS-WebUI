use super::error::{AppError, AppResult};
use super::server::ServerId;

pub type ClientId = i64;

/// A NATS subject or subscription pattern, validated against the NATS subject rules:
/// dot-separated non-empty tokens, no whitespace, `*` only as a whole token and `>` only as the
/// whole last token.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct SubjectPattern(String);

impl SubjectPattern {
    pub fn parse(raw: &str) -> AppResult<Self> {
        let s = raw.trim();
        if s.is_empty() {
            return Err(AppError::validation("subject must not be empty"));
        }
        if s.chars().any(char::is_whitespace) {
            return Err(AppError::validation(format!(
                "subject \"{s}\" must not contain spaces"
            )));
        }
        let tokens: Vec<&str> = s.split('.').collect();
        for (i, t) in tokens.iter().enumerate() {
            if t.is_empty() {
                return Err(AppError::validation(format!(
                    "subject \"{s}\" has an empty token"
                )));
            }
            if *t == ">" && i + 1 != tokens.len() {
                return Err(AppError::validation(format!(
                    "subject \"{s}\": '>' must be the last token"
                )));
            }
            if t.len() > 1 && (t.contains('*') || t.contains('>')) {
                return Err(AppError::validation(format!(
                    "subject \"{s}\": wildcards must be whole tokens"
                )));
            }
        }
        Ok(SubjectPattern(s.to_string()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn is_wildcard(&self) -> bool {
        self.0.split('.').any(|t| t == "*" || t == ">")
    }
}

impl std::fmt::Display for SubjectPattern {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// A saved subscription view: a server plus the subject patterns to watch on it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ClientView {
    /// 0 until the client has been stored.
    pub id: ClientId,
    pub name: String,
    pub server_id: ServerId,
    pub patterns: Vec<SubjectPattern>,
}

/// User input for creating or editing a client.
#[derive(Clone, Debug, Default)]
pub struct ClientDraft {
    pub name: String,
    pub server_id: ServerId,
    pub patterns: Vec<String>,
}

impl ClientDraft {
    /// Validates the draft. Blank patterns are dropped and duplicates are removed, keeping order.
    pub fn into_client(self, id: ClientId) -> AppResult<ClientView> {
        let name = self.name.trim();
        if name.is_empty() {
            return Err(AppError::validation("name is required"));
        }
        let mut patterns: Vec<SubjectPattern> = Vec::new();
        for raw in self.patterns.iter().filter(|p| !p.trim().is_empty()) {
            let p = SubjectPattern::parse(raw)?;
            if !patterns.contains(&p) {
                patterns.push(p);
            }
        }
        Ok(ClientView {
            id,
            name: name.to_string(),
            server_id: self.server_id,
            patterns,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_valid_subjects() {
        for s in ["a", "a.b.c", "a.*.c", "a.>", ">", "*", "$SYS.>", "_INBOX.x"] {
            assert!(SubjectPattern::parse(s).is_ok(), "{s}");
        }
        assert!(SubjectPattern::parse("test.>").unwrap().is_wildcard());
        assert!(!SubjectPattern::parse("test.x").unwrap().is_wildcard());
    }

    #[test]
    fn rejects_invalid_subjects() {
        for s in ["", " ", "a..b", ".a", "a.", "a b", "a.>.b", "a.b*", "a.>x"] {
            assert!(SubjectPattern::parse(s).is_err(), "{s:?}");
        }
    }

    #[test]
    fn draft_cleans_patterns() {
        let c = ClientDraft {
            name: " watch ".into(),
            server_id: 1,
            patterns: vec!["test.>".into(), " ".into(), "test.>".into(), "a.*".into()],
        }
        .into_client(0)
        .unwrap();
        assert_eq!(c.name, "watch");
        let p: Vec<&str> = c.patterns.iter().map(|p| p.as_str()).collect();
        assert_eq!(p, ["test.>", "a.*"]);
    }
}
