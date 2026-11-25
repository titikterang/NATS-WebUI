use super::Db;
use crate::application::ports::ServerRepo;
use crate::domain::error::AppResult;
use crate::domain::secret::Secret;
use crate::domain::server::{Credentials, Server, ServerId};
use async_trait::async_trait;
use rusqlite::{params, OptionalExtension, Row};

pub struct SqliteServerRepo {
    db: Db,
}

impl SqliteServerRepo {
    pub fn new(db: Db) -> Self {
        SqliteServerRepo { db }
    }
}

const COLUMNS: &str = "id, name, host, port, monitoring_port, username, password";

fn from_row(r: &Row) -> rusqlite::Result<Server> {
    let username: Option<String> = r.get(5)?;
    let password: Option<String> = r.get(6)?;
    Ok(Server {
        id: r.get(0)?,
        name: r.get(1)?,
        host: r.get(2)?,
        port: r.get(3)?,
        monitoring_port: r.get(4)?,
        credentials: username
            .filter(|u| !u.is_empty())
            .map(|username| Credentials {
                username,
                password: Secret::new(password.unwrap_or_default()),
            }),
    })
}

/// (username, password) columns. Passwords are stored only alongside a username.
fn credential_columns(s: &Server) -> (Option<String>, Option<String>) {
    match &s.credentials {
        Some(c) => (
            Some(c.username.clone()),
            Some(c.password.expose().to_string()).filter(|p| !p.is_empty()),
        ),
        None => (None, None),
    }
}

#[async_trait]
impl ServerRepo for SqliteServerRepo {
    async fn list(&self) -> AppResult<Vec<Server>> {
        self.db
            .call(|c| {
                c.prepare(&format!("SELECT {COLUMNS} FROM servers ORDER BY id"))?
                    .query_map([], from_row)?
                    .collect()
            })
            .await
    }

    async fn get(&self, id: ServerId) -> AppResult<Option<Server>> {
        self.db
            .call(move |c| {
                c.query_row(
                    &format!("SELECT {COLUMNS} FROM servers WHERE id = ?1"),
                    [id],
                    from_row,
                )
                .optional()
            })
            .await
    }

    async fn insert(&self, s: &Server) -> AppResult<ServerId> {
        let s = s.clone();
        self.db
            .call(move |c| {
                let (username, password) = credential_columns(&s);
                // `subjects`/`publications` are v0.1 NOT NULL columns that are no longer used.
                c.execute(
                    "INSERT INTO servers (name, host, port, monitoring_port, subjects, publications, username, password)
                     VALUES (?1, ?2, ?3, ?4, '[]', '[]', ?5, ?6)",
                    params![s.name, s.host, s.port, s.monitoring_port, username, password],
                )?;
                Ok(c.last_insert_rowid())
            })
            .await
    }

    async fn update(&self, s: &Server) -> AppResult<()> {
        let s = s.clone();
        self.db
            .call(move |c| {
                let (username, password) = credential_columns(&s);
                c.execute(
                    "UPDATE servers SET name = ?1, host = ?2, port = ?3, monitoring_port = ?4, username = ?5, password = ?6
                     WHERE id = ?7",
                    params![s.name, s.host, s.port, s.monitoring_port, username, password, s.id],
                )?;
                Ok(())
            })
            .await
    }

    async fn delete(&self, id: ServerId) -> AppResult<()> {
        self.db
            .call(move |c| {
                c.execute("DELETE FROM servers WHERE id = ?1", [id])
                    .map(|_| ())
            })
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn server(creds: Option<(&str, &str)>) -> Server {
        Server {
            id: 0,
            name: "sbx".into(),
            host: "10.0.0.1".into(),
            port: 4222,
            monitoring_port: 8222,
            credentials: creds.map(|(u, p)| Credentials {
                username: u.into(),
                password: Secret::new(p),
            }),
        }
    }

    #[tokio::test]
    async fn round_trips_servers_and_credentials() {
        let repo = SqliteServerRepo::new(Db::open_in_memory().unwrap());
        let id = repo.insert(&server(Some(("user", "pw")))).await.unwrap();
        let s = repo.get(id).await.unwrap().unwrap();
        assert_eq!(s.credentials.as_ref().unwrap().password.expose(), "pw");

        repo.update(&Server { id, ..server(None) }).await.unwrap();
        assert!(repo.get(id).await.unwrap().unwrap().credentials.is_none());

        repo.delete(id).await.unwrap();
        assert!(repo.list().await.unwrap().is_empty());
        assert!(repo.get(id).await.unwrap().is_none());
    }
}
