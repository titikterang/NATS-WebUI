//! Versioned schema migrations, tracked in `PRAGMA user_version`.
//!
//! Rules: never edit a released migration, only append. Migrations only add things, so an
//! existing data volume always stays readable. Each one runs in its own transaction.

use rusqlite::{Connection, Result};
use serde::Deserialize;

type Migration = fn(&Connection) -> Result<()>;

/// Index + 1 is the schema version the migration produces.
const MIGRATIONS: &[Migration] = &[v1_original_schema, v2_credentials_and_patterns];

pub fn run(conn: &Connection) -> Result<()> {
    let current: usize =
        conn.query_row("PRAGMA user_version", [], |r| r.get::<_, i64>(0))? as usize;
    for (i, migrate) in MIGRATIONS.iter().enumerate().skip(current) {
        let tx = conn.unchecked_transaction()?;
        migrate(&tx)?;
        tx.pragma_update(None, "user_version", (i + 1) as i64)?;
        tx.commit()?;
    }
    Ok(())
}

/// The v0.1 schema, verbatim (`IF NOT EXISTS`: v0.1/v0.2 databases are at user_version 0).
fn v1_original_schema(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS servers (
            id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL,
            host TEXT NOT NULL,
            port INTEGER NOT NULL,
            monitoring_port INTEGER NOT NULL,
            subjects TEXT NOT NULL,
            publications TEXT NOT NULL
        );
        CREATE TABLE IF NOT EXISTS clients (
            id INTEGER NOT NULL UNIQUE,
            name TEXT NOT NULL,
            server_id INTEGER NOT NULL,
            subjects TEXT NOT NULL,
            info INTEGER NOT NULL, ping INTEGER NOT NULL, pong INTEGER NOT NULL,
            ok INTEGER NOT NULL, err INTEGER NOT NULL, publ INTEGER NOT NULL,
            sub INTEGER NOT NULL, unsub INTEGER NOT NULL, connect INTEGER NOT NULL,
            msg INTEGER NOT NULL,
            FOREIGN KEY(server_id) REFERENCES servers(id),
            PRIMARY KEY(id)
        );",
    )
}

/// Credentials on servers, and subject patterns on clients (converted from the v0.1 subject
/// tree). Guarded by column checks because v0.2 databases already have the columns.
fn v2_credentials_and_patterns(conn: &Connection) -> Result<()> {
    for (table, column) in [
        ("servers", "username"),
        ("servers", "password"),
        ("clients", "patterns"),
    ] {
        if !has_column(conn, table, column)? {
            conn.execute(&format!("ALTER TABLE {table} ADD COLUMN {column} TEXT"), [])?;
        }
    }

    #[derive(Deserialize)]
    struct Node {
        subject_str: String,
        #[serde(default)]
        subjects: Vec<Node>,
        #[serde(default)]
        selected: bool,
    }
    fn walk(node: &Node, prefix: &mut Vec<String>, out: &mut Vec<String>) {
        prefix.push(node.subject_str.clone());
        if node.selected {
            out.push(prefix.join("."));
        }
        for child in &node.subjects {
            walk(child, prefix, out);
        }
        prefix.pop();
    }

    let rows: Vec<(i64, String)> = conn
        .prepare("SELECT id, subjects FROM clients WHERE patterns IS NULL")?
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?
        .collect::<Result<_>>()?;
    for (id, tree) in rows {
        let roots: Vec<Node> = serde_json::from_str(&tree).unwrap_or_default();
        let mut patterns = Vec::new();
        for root in &roots {
            walk(root, &mut Vec::new(), &mut patterns);
        }
        conn.execute(
            "UPDATE clients SET patterns = ?1 WHERE id = ?2",
            rusqlite::params![
                serde_json::to_string(&patterns).unwrap_or_else(|_| "[]".into()),
                id
            ],
        )?;
    }
    Ok(())
}

fn has_column(conn: &Connection, table: &str, column: &str) -> Result<bool> {
    let mut stmt = conn.prepare(&format!("PRAGMA table_info({table})"))?;
    let names = stmt.query_map([], |r| r.get::<_, String>(1))?;
    for name in names {
        if name? == column {
            return Ok(true);
        }
    }
    Ok(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn version(c: &Connection) -> i64 {
        c.query_row("PRAGMA user_version", [], |r| r.get(0))
            .unwrap()
    }

    #[test]
    fn fresh_database_reaches_latest_and_is_idempotent() {
        let c = Connection::open_in_memory().unwrap();
        run(&c).unwrap();
        run(&c).unwrap();
        assert_eq!(version(&c), MIGRATIONS.len() as i64);
        assert!(has_column(&c, "clients", "patterns").unwrap());
    }

    #[test]
    fn v02_database_without_version_is_adopted() {
        let c = Connection::open_in_memory().unwrap();
        v1_original_schema(&c).unwrap();
        v2_credentials_and_patterns(&c).unwrap(); // columns exist, user_version still 0
        run(&c).unwrap();
        assert_eq!(version(&c), MIGRATIONS.len() as i64);
    }
}
