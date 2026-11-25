//! SQLite persistence. rusqlite is synchronous, so every query runs on tokio's blocking pool
//! through [`Db::call`] instead of stalling the async runtime.

mod client_repo;
mod migrations;
mod server_repo;

pub use client_repo::SqliteClientRepo;
pub use server_repo::SqliteServerRepo;

use crate::domain::error::{AppError, AppResult};
use rusqlite::Connection;
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// One shared connection. SQLite serializes writers anyway, and this app's load is tiny.
#[derive(Clone)]
pub struct Db {
    conn: Arc<Mutex<Connection>>,
}

impl Db {
    /// Opens the database file, caps its size at `max_mb` MiB and brings the schema up to date.
    ///
    /// The cap is SQLite's `max_page_count`: once reached, writes fail with SQLITE_FULL
    /// (reported as "database is full") instead of filling the disk. It is a per-connection
    /// setting, so it's applied every time the database is opened.
    pub fn open(path: &str, max_mb: u64) -> AppResult<Self> {
        let conn = Connection::open(path).map_err(db_error)?;
        conn.busy_timeout(Duration::from_secs(5))
            .map_err(db_error)?;
        set_size_cap(&conn, max_mb).map_err(db_error)?;
        Self::init(conn)
    }

    #[cfg(test)]
    pub fn open_in_memory() -> AppResult<Self> {
        Self::init(Connection::open_in_memory().map_err(db_error)?)
    }

    #[cfg(test)]
    pub(crate) fn from_connection(conn: Connection) -> AppResult<Self> {
        Self::init(conn)
    }

    fn init(conn: Connection) -> AppResult<Self> {
        migrations::run(&conn).map_err(db_error)?;
        Ok(Db {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    pub async fn call<T, F>(&self, f: F) -> AppResult<T>
    where
        T: Send + 'static,
        F: FnOnce(&Connection) -> rusqlite::Result<T> + Send + 'static,
    {
        let conn = self.conn.clone();
        tokio::task::spawn_blocking(move || {
            let guard = conn
                .lock()
                .map_err(|_| AppError::Internal("database lock poisoned".into()))?;
            f(&guard).map_err(db_error)
        })
        .await
        .map_err(|e| AppError::Internal(format!("database task failed: {e}")))?
    }
}

fn set_size_cap(conn: &Connection, max_mb: u64) -> rusqlite::Result<()> {
    let page_size: u64 = conn.query_row("PRAGMA page_size", [], |r| r.get(0))?;
    let pages = (max_mb * 1024 * 1024 / page_size).max(1);
    // The pragma returns the new limit; query_row consumes that row.
    conn.query_row(&format!("PRAGMA max_page_count = {pages}"), [], |_| Ok(()))
}

fn db_error(e: rusqlite::Error) -> AppError {
    if e.sqlite_error_code() == Some(rusqlite::ErrorCode::DiskFull) {
        return AppError::Internal(
            "database is full: DATABASE_MAX_MB reached (raise it or delete unused servers and clients)"
                .into(),
        );
    }
    AppError::Internal(format!("database: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Gatekeeper for the disk cap: writes past DATABASE_MAX_MB fail cleanly with a clear message.
    #[tokio::test]
    async fn size_cap_stops_writes_with_a_clear_error() {
        let dir = std::env::temp_dir().join(format!("nats-webui-cap-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("cap.db");
        let _ = std::fs::remove_file(&path);

        let db = Db::open(path.to_str().unwrap(), 1).unwrap(); // 1 MiB
        let result = db
            .call(|c| {
                c.execute("CREATE TABLE fill (b BLOB)", [])?;
                for _ in 0..64 {
                    c.execute("INSERT INTO fill VALUES (zeroblob(65536))", [])?;
                    // 4 MiB total
                }
                Ok(())
            })
            .await;
        let size = std::fs::metadata(&path).unwrap().len();
        std::fs::remove_dir_all(&dir).unwrap();

        match result {
            Err(AppError::Internal(msg)) => assert!(msg.contains("database is full"), "{msg}"),
            other => panic!("expected the size cap to stop writes, got {other:?}"),
        }
        assert!(size <= 1024 * 1024, "file grew past the cap: {size} bytes");
    }
}
