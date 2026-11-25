use super::client::SubjectPattern;
use super::error::{AppError, AppResult};
use bytes::Bytes;

/// Header name/value pairs, in order. A name may repeat.
pub type Headers = Vec<(String, String)>;

/// A message received from NATS.
#[derive(Clone, Debug)]
pub struct InboundMessage {
    pub subject: String,
    pub reply: Option<String>,
    pub headers: Headers,
    pub payload: Bytes,
}

/// A validated message to publish.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OutboundMessage {
    pub subject: String,
    pub payload: Bytes,
    pub headers: Headers,
}

impl OutboundMessage {
    pub fn new(subject: &str, payload: impl Into<Bytes>, headers: Headers) -> AppResult<Self> {
        let subject = SubjectPattern::parse(subject)?;
        if subject.is_wildcard() {
            return Err(AppError::validation("cannot publish to a wildcard subject"));
        }
        let mut clean = Vec::with_capacity(headers.len());
        for (name, value) in headers {
            let name = name.trim().to_string();
            if name.is_empty() {
                continue; // blank rows from the UI
            }
            if name.contains(':') || name.chars().any(|c| c.is_whitespace() || c.is_control()) {
                return Err(AppError::validation(format!(
                    "invalid header name \"{name}\""
                )));
            }
            if value.contains(['\r', '\n']) {
                return Err(AppError::validation(format!(
                    "header \"{name}\" value must be one line"
                )));
            }
            clean.push((name, value));
        }
        Ok(OutboundMessage {
            subject: subject.as_str().to_string(),
            payload: payload.into(),
            headers: clean,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_subject_and_headers() {
        let m = OutboundMessage::new(
            " test.x ",
            "hi",
            vec![("".into(), "".into()), ("X-A".into(), "1".into())],
        )
        .unwrap();
        assert_eq!(m.subject, "test.x");
        assert_eq!(m.headers, vec![("X-A".to_string(), "1".to_string())]);

        assert!(OutboundMessage::new("test.>", "", vec![]).is_err());
        assert!(OutboundMessage::new("test.x", "", vec![("bad name".into(), "v".into())]).is_err());
        assert!(OutboundMessage::new("test.x", "", vec![("X".into(), "a\r\nb".into())]).is_err());
    }
}
