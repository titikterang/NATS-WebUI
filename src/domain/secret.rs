use std::fmt;

/// A credential value that can't leak by accident.
///
/// `Debug` and `Display` print `***`, and there is deliberately no `Serialize` impl, so a secret
/// can't end up in logs, error messages or API responses. Call [`Secret::expose`] only at the
/// point where the raw value is required (the NATS CONNECT and the database row).
#[derive(Clone, Default, PartialEq, Eq)]
pub struct Secret(String);

impl Secret {
    pub fn new(value: impl Into<String>) -> Self {
        Secret(value.into())
    }

    pub fn expose(&self) -> &str {
        &self.0
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl fmt::Debug for Secret {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Secret(***)")
    }
}

impl fmt::Display for Secret {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("***")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn never_formats_the_value() {
        let s = Secret::new("hunter2");
        assert!(!format!("{s:?} {s} {s:#?}").contains("hunter2"));
        assert_eq!(s.expose(), "hunter2");
    }
}
