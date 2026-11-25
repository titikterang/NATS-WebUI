//! Route table. Paths are part of the public contract with the web UI:
//!
//! REST under `/api/state` (JSON; errors are `{"error": "..."}`):
//!   GET  /                              servers + clients (never passwords)
//!   POST /server/new | /server/update   GET /server/delete/<id>
//!   GET  /server/<id>/jsz | /subsz      monitoring passthrough
//!   POST /server/<id>/publish           {subject, payload, headers:[{name,value}]}
//!   POST /client/new | /client/update   GET /client/delete/<id>
//!   WS   /ws                            status feed
//! WS   /client/<id>                     live messages for a saved client
//! GET  /healthz                         liveness (used by the container healthcheck)
//! GET  /*                               the SPA (falls back to index.html for client-side routes)

mod dto;
mod error;
mod handlers;
mod ws;

use crate::application::{
    clients::ClientService, inspect::InspectService, publish::PublishService,
    servers::ServerService, status::StatusService, subscriptions::SubscriptionService,
};
use std::convert::Infallible;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use warp::filters::BoxedFilter;
use warp::{Filter, Rejection, Reply};

/// Everything the handlers need, built once by `main.rs`.
pub struct AppState {
    pub servers: ServerService,
    pub clients: ClientService,
    pub publish: PublishService,
    pub inspect: InspectService,
    pub status: Arc<StatusService>,
    pub subscriptions: SubscriptionService,
}

pub type Shared = Arc<AppState>;
type Route = BoxedFilter<(Box<dyn Reply>,)>;

fn with_state(state: Shared) -> impl Filter<Extract = (Shared,), Error = Infallible> + Clone {
    warp::any().map(move || state.clone())
}

/// Type-erases a route so route groups can be combined with `or` + `unify`.
fn route<F, R>(f: F) -> Route
where
    F: Filter<Extract = (R,), Error = Rejection> + Clone + Send + Sync + 'static,
    R: Reply + 'static,
{
    f.map(|r: R| Box::new(r) as Box<dyn Reply>).boxed()
}

pub fn routes(
    state: Shared,
    static_dir: PathBuf,
) -> impl Filter<Extract = (impl Reply,), Error = Infallible> + Clone {
    let api = handlers::servers::routes(state.clone())
        .or(handlers::clients::routes(state.clone()))
        .unify()
        .or(handlers::monitoring::routes(state.clone()))
        .unify()
        .or(handlers::publish::routes(state.clone()))
        .unify()
        .or(ws::status_feed::route(state.clone()))
        .unify();

    // Everything under /api is answered here; without this inner recover an API rejection
    // would fall through to the SPA and come back as index.html with 200.
    let api = warp::path("api")
        .and(warp::path("state"))
        .and(api.recover(error::recover));

    let spa = warp::get()
        .and(warp::fs::dir(static_dir.clone()).or(warp::fs::file(static_dir.join("index.html"))));

    let traced = api
        .or(ws::client_stream::route(state))
        .or(spa)
        .recover(error::recover)
        .with(warp::trace(|info| {
            static NEXT: AtomicU64 = AtomicU64::new(1);
            tracing::info_span!("http", req = NEXT.fetch_add(1, Ordering::Relaxed), method = %info.method(), path = %info.path())
        }));

    // /healthz is polled every 30 s by the container healthcheck; it stays out of the request
    // logs so they only show real traffic.
    handlers::health::route().or(traced)
}
