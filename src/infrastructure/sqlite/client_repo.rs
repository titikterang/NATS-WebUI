use super::Db;
use crate::application::ports::ClientRepo;
use crate::domain::client::{ClientId, ClientView, SubjectPattern};
use crate::domain::error::AppResult;
use crate::domain::server::ServerId;
use async_trait::async_trait;
use rusqlite::{params, OptionalExtension, Row};

pub struct SqliteClientRepo {
    db: Db,
}

impl SqliteClientRepo {
    pub fn new(db: Db) -> Self {
        SqliteClientRepo { db }
    }
}

const COLUMNS: &str = "id, name, server_id, patterns";

fn from_row(r: &Row) -> rusqlite::Result<ClientView> {
    let raw: Option<String> = r.get(3)?;
    let patterns: Vec<String> = raw
        .and_then(|p| serde_json::from_str(&p).ok())
        .unwrap_or_default();
    Ok(ClientView {
        id: r.get(0)?,
        name: r.get(1)?,
        server_id: r.get(2)?,
        // Lenient on read: a pattern stored by an older version that no longer validates is
        // dropped rather than making the whole client unreadable.
        patterns: patterns
            .iter()
            .filter_map(|p| SubjectPattern::parse(p).ok())
            .collect(),
    })
}

fn patterns_json(c: &ClientView) -> String {
    let p: Vec<&str> = c.patterns.iter().map(|p| p.as_str()).collect();
    serde_json::to_string(&p).unwrap_or_else(|_| "[]".into())
}

#[async_trait]
impl ClientRepo for SqliteClientRepo {
    async fn list(&self) -> AppResult<Vec<ClientView>> {
        self.db
            .call(|c| {
                c.prepare(&format!("SELECT {COLUMNS} FROM clients ORDER BY id"))?
                    .query_map([], from_row)?
                    .collect()
            })
            .await
    }

    async fn get(&self, id: ClientId) -> AppResult<Option<ClientView>> {
        self.db
            .call(move |c| {
                c.query_row(
                    &format!("SELECT {COLUMNS} FROM clients WHERE id = ?1"),
                    [id],
                    from_row,
                )
                .optional()
            })
            .await
    }

    async fn insert(&self, client: &ClientView) -> AppResult<ClientId> {
        let (name, server_id, patterns) =
            (client.name.clone(), client.server_id, patterns_json(client));
        self.db
            .call(move |c| {
                // The v0.1 subject-tree and protocol-flag columns are NOT NULL but unused.
                c.execute(
                    "INSERT INTO clients (name, server_id, subjects, info, ping, pong, ok, err, publ, sub, unsub, connect, msg, patterns)
                     VALUES (?1, ?2, '[]', 0, 0, 0, 0, 1, 0, 0, 0, 0, 1, ?3)",
                    params![name, server_id, patterns],
                )?;
                Ok(c.last_insert_rowid())
            })
            .await
    }

    async fn update(&self, client: &ClientView) -> AppResult<()> {
        let (id, name, server_id, patterns) = (
            client.id,
            client.name.clone(),
            client.server_id,
            patterns_json(client),
        );
        self.db
            .call(move |c| {
                c.execute(
                    "UPDATE clients SET name = ?1, server_id = ?2, patterns = ?3 WHERE id = ?4",
                    params![name, server_id, patterns, id],
                )
                .map(|_| ())
            })
            .await
    }

    async fn delete(&self, id: ClientId) -> AppResult<()> {
        self.db
            .call(move |c| {
                c.execute("DELETE FROM clients WHERE id = ?1", [id])
                    .map(|_| ())
            })
            .await
    }

    async fn delete_by_server(&self, server_id: ServerId) -> AppResult<()> {
        self.db
            .call(move |c| {
                c.execute("DELETE FROM clients WHERE server_id = ?1", [server_id])
                    .map(|_| ())
            })
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::ports::ServerRepo;
    use rusqlite::Connection;

    #[tokio::test]
    async fn round_trips_clients() {
        let db = Db::open_in_memory().unwrap();
        // clients.server_id is a foreign key, and the bundled SQLite enforces it.
        let server_id = super::super::SqliteServerRepo::new(db.clone())
            .insert(&crate::domain::server::Server {
                id: 0,
                name: "s".into(),
                host: "h".into(),
                port: 1,
                monitoring_port: 2,
                credentials: None,
            })
            .await
            .unwrap();
        let repo = SqliteClientRepo::new(db);
        let client = ClientView {
            id: 0,
            name: "c".into(),
            server_id,
            patterns: vec![SubjectPattern::parse("test.>").unwrap()],
        };
        let id = repo.insert(&client).await.unwrap();
        assert_eq!(
            repo.get(id).await.unwrap().unwrap().patterns,
            client.patterns
        );
        repo.delete_by_server(server_id).await.unwrap();
        assert!(repo.list().await.unwrap().is_empty());
    }

    /// A real v0.1 database: old tables only, one client with a selected subtree.
    #[tokio::test]
    async fn migrates_v01_subject_tree_to_patterns() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE servers (id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT, name TEXT NOT NULL,
               host TEXT NOT NULL, port INTEGER NOT NULL, monitoring_port INTEGER NOT NULL,
               subjects TEXT NOT NULL, publications TEXT NOT NULL);
             CREATE TABLE clients (id INTEGER NOT NULL UNIQUE, name TEXT NOT NULL, server_id INTEGER NOT NULL,
               subjects TEXT NOT NULL, info INTEGER NOT NULL, ping INTEGER NOT NULL, pong INTEGER NOT NULL,
               ok INTEGER NOT NULL, err INTEGER NOT NULL, publ INTEGER NOT NULL, sub INTEGER NOT NULL,
               unsub INTEGER NOT NULL, connect INTEGER NOT NULL, msg INTEGER NOT NULL, PRIMARY KEY(id));
             INSERT INTO servers VALUES (1, 'old', 'h', 4222, 8222, '[]', '[]');",
        )
        .unwrap();
        let tree = r#"[{"id":"0","subject_str":"time","selected":false,"subjects":[
            {"id":"0-0","subject_str":"us","selected":true,"subjects":[
              {"id":"0-0-0","subject_str":"east","selected":true,"subjects":[]}]},
            {"id":"0-1","subject_str":"eu","selected":false,"subjects":[]}]}]"#;
        conn.execute(
            "INSERT INTO clients VALUES (1, 'c', 1, ?1, 0,0,0,0,0,0,0,0,0,1)",
            [tree],
        )
        .unwrap();

        let db = Db::from_connection(conn).unwrap();
        let clients = SqliteClientRepo::new(db.clone()).list().await.unwrap();
        let p: Vec<&str> = clients[0].patterns.iter().map(|p| p.as_str()).collect();
        assert_eq!(p, ["time.us", "time.us.east"]);

        let servers = super::super::SqliteServerRepo::new(db)
            .list()
            .await
            .unwrap();
        assert!(servers[0].credentials.is_none());
    }
}
