//! Composition root: the only place that knows every layer. It reads config, builds the
//! infrastructure adapters, injects them into the application services and starts the HTTP server.
//!
//! Layers (dependencies point inward):
//!   presentation (warp HTTP/WS) → application (use cases + ports) → domain (entities, rules)
//!   infrastructure (SQLite, async-nats, reqwest) implements the application ports.

mod application;
mod config;
mod domain;
mod infrastructure;
mod logging;
mod presentation;

use application::{
    clients::ClientService, inspect::InspectService, ports::Broker, publish::PublishService,
    servers::ServerService, status::StatusService, subscriptions::SubscriptionService,
};
use infrastructure::{
    monitoring::HttpMonitoring,
    nats::NatsBroker,
    sqlite::{Db, SqliteClientRepo, SqliteServerRepo},
};
use presentation::http::{self, AppState};
use std::process::ExitCode;
use std::sync::Arc;
use tracing::{error, info, warn};

fn main() -> ExitCode {
    // `nats-webui healthcheck`: used by the container HEALTHCHECK (distroless has no curl).
    if std::env::args().nth(1).as_deref() == Some("healthcheck") {
        return healthcheck();
    }

    match logging::LogFormat::from_env() {
        Ok(format) => logging::init(format),
        Err(e) => {
            eprintln!("{e}");
            return ExitCode::FAILURE;
        }
    }

    let runtime = tokio::runtime::Runtime::new().expect("tokio runtime");
    match runtime.block_on(run()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            error!("{e}");
            ExitCode::FAILURE
        }
    }
}

async fn run() -> Result<(), String> {
    let cfg = config::Config::from_env()?;

    match &cfg.ca_file {
        Some(ca) if !ca.is_file() => warn!(path = %ca.display(), "NATS_CA_FILE does not exist"),
        Some(ca) => info!(path = %ca.display(), "NATS TLS CA loaded"),
        None => warn!("NATS_CA_FILE not set: servers with a private-CA certificate will fail TLS verification"),
    }

    // Infrastructure
    let db = Db::open(&cfg.db_path, cfg.db_max_mb)
        .map_err(|e| format!("open database {}: {e}", cfg.db_path))?;
    info!(path = %cfg.db_path, max_mb = cfg.db_max_mb, "database ready");
    let server_repo = Arc::new(SqliteServerRepo::new(db.clone()));
    let client_repo = Arc::new(SqliteClientRepo::new(db));
    let broker: Arc<dyn Broker> = Arc::new(NatsBroker::new(cfg.ca_file.clone()));
    let monitoring = Arc::new(HttpMonitoring::new().map_err(|e| e.to_string())?);

    // Application
    let servers = ServerService::new(server_repo.clone(), client_repo.clone(), broker.clone());
    let connected = servers.connect_all().await.map_err(|e| e.to_string())?;
    info!(servers = connected, "nats connections started");

    let status = Arc::new(StatusService::new(
        server_repo.clone(),
        broker.clone(),
        monitoring.clone(),
    ));
    status.clone().spawn();

    let state = Arc::new(AppState {
        servers,
        clients: ClientService::new(client_repo.clone(), server_repo.clone()),
        publish: PublishService::new(broker.clone()),
        inspect: InspectService::new(server_repo, monitoring),
        status,
        subscriptions: SubscriptionService::new(client_repo, broker),
    });

    // Presentation
    info!(addr = %cfg.bind_addr, "listening");
    warp::serve(http::routes(state, cfg.static_dir.clone()))
        .run(cfg.bind_addr)
        .await;
    Ok(())
}

/// Exit 0 if `GET /healthz` on the configured port answers 200.
fn healthcheck() -> ExitCode {
    use std::io::{Read, Write};
    use std::net::{Ipv4Addr, SocketAddr, TcpStream};
    use std::time::Duration;

    let port = config::Config::from_env()
        .map(|c| c.bind_addr.port())
        .unwrap_or(8080);
    let addr = SocketAddr::from((Ipv4Addr::LOCALHOST, port));
    let ok = (|| -> std::io::Result<bool> {
        let mut s = TcpStream::connect_timeout(&addr, Duration::from_secs(2))?;
        s.set_read_timeout(Some(Duration::from_secs(2)))?;
        s.write_all(b"GET /healthz HTTP/1.0\r\nHost: localhost\r\n\r\n")?;
        let mut buf = [0u8; 32];
        let n = s.read(&mut buf)?;
        Ok(String::from_utf8_lossy(&buf[..n]).contains(" 200 "))
    })()
    .unwrap_or(false);
    if ok {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}
