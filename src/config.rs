use std::net::SocketAddr;
use std::path::PathBuf;

/// The default listen address. Never port 80: the app runs as non-root, and production maps its
/// own host port onto this one.
pub const DEFAULT_BIND: &str = "0.0.0.0:8080";

/// Default cap on the SQLite database size, in MiB.
pub const DEFAULT_DB_MAX_MB: u64 = 256;

/// Runtime configuration, read from environment variables (see `.env.example`).
#[derive(Debug, Clone)]
pub struct Config {
    /// `BIND_ADDR`.
    pub bind_addr: SocketAddr,
    /// `DATABASE_URL`, a SQLite file path. Default `./sqlite.db`.
    pub db_path: String,
    /// `STATIC_DIR`, the built frontend. Default `web/dist`.
    pub static_dir: PathBuf,
    /// `NATS_CA_FILE`, a PEM CA bundle used to verify NATS server TLS certificates.
    pub ca_file: Option<PathBuf>,
    /// `DATABASE_MAX_MB`: hard cap on the SQLite file size (the app's only disk writes).
    /// Writes beyond it fail with "database is full". Default 256.
    pub db_max_mb: u64,
}

impl Config {
    pub fn from_env() -> Result<Self, String> {
        let var = |k: &str| std::env::var(k).ok().filter(|v| !v.trim().is_empty());
        let bind = var("BIND_ADDR").unwrap_or_else(|| DEFAULT_BIND.into());
        Ok(Config {
            bind_addr: bind.parse().map_err(|_| {
                format!("BIND_ADDR must be host:port (e.g. 127.0.0.1:8080), got {bind:?}")
            })?,
            db_path: var("DATABASE_URL").unwrap_or_else(|| "./sqlite.db".into()),
            static_dir: var("STATIC_DIR")
                .unwrap_or_else(|| "web/dist".into())
                .into(),
            ca_file: var("NATS_CA_FILE").map(PathBuf::from),
            db_max_mb: match var("DATABASE_MAX_MB") {
                None => DEFAULT_DB_MAX_MB,
                Some(v) => v.trim().parse().ok().filter(|n| *n > 0).ok_or_else(|| {
                    format!("DATABASE_MAX_MB must be a positive number, got {v:?}")
                })?,
            },
        })
    }
}
