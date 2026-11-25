//! `WS /client/<id>`: live messages for a saved client, plus Status and Err frames.

use super::super::dto::Frame;
use super::super::error::reject;
use super::super::{with_state, Shared};
use futures::{SinkExt, StreamExt};
use std::time::Duration;
use tokio::sync::broadcast::error::RecvError;
use tracing::{info, info_span, Instrument};
use warp::ws::{WebSocket, Ws};
use warp::{Filter, Rejection, Reply};

const STATUS_EVERY: Duration = Duration::from_secs(5);

/// Under `/client/` everything is answered here (errors as JSON) so a rejected upgrade, e.g. for an
/// unknown client id, can't fall through to the SPA fallback and return index.html with 200.
pub fn route(state: Shared) -> impl Filter<Extract = (impl Reply,), Error = Rejection> + Clone {
    let stream = warp::path!(i64)
        .and(warp::ws())
        .and(with_state(state))
        .and_then(upgrade);
    warp::path("client").and(stream.recover(super::super::error::recover))
}

async fn upgrade(client_id: i64, ws: Ws, state: Shared) -> Result<impl Reply, Rejection> {
    // Reject unknown clients with a 404 before upgrading.
    state
        .subscriptions
        .ensure_client(client_id)
        .await
        .map_err(reject)?;
    Ok(ws.on_upgrade(move |socket| {
        run(socket, state, client_id).instrument(info_span!("client_stream", client_id))
    }))
}

async fn run(socket: WebSocket, state: Shared, client_id: i64) {
    let (mut tx, mut incoming) = socket.split();
    macro_rules! send {
        ($frame:expr) => {
            if tx.send($frame.to_ws()).await.is_err() {
                return;
            }
        };
    }

    let live = match state.subscriptions.open(client_id).await {
        Ok(l) => l,
        Err(e) => {
            send!(Frame::err(e.to_string()));
            return;
        }
    };
    let server_id = live.server_id;
    send!(Frame::status(&state.subscriptions.status(server_id).await));
    for (pattern, e) in &live.subscription.failures {
        send!(Frame::err(format!("subscribe {pattern}: {e}")));
    }
    if live.subscription.subscribed == 0 {
        send!(Frame::err("no subjects to subscribe to"));
    }

    let mut messages = live.subscription.messages;
    let mut errors = live.server_errors;
    let mut tick = tokio::time::interval(STATUS_EVERY);
    let mut delivered: u64 = 0;

    loop {
        let frame = tokio::select! {
            m = messages.next() => match m {
                Some(m) => { delivered += 1; Frame::message(m) }
                // The server connection was replaced (config edit) or closed. The UI reconnects.
                None => { send!(Frame::err("subscription closed: the server connection was replaced or closed")); break; }
            },
            e = errors.recv() => match e {
                Ok(e) => Frame::err(e),
                Err(RecvError::Lagged(n)) => Frame::err(format!("{n} server errors dropped")),
                Err(RecvError::Closed) => break,
            },
            _ = tick.tick() => Frame::status(&state.subscriptions.status(server_id).await),
            m = incoming.next() => match m {
                Some(Ok(m)) if !m.is_close() => continue,
                _ => break,
            },
        };
        send!(frame);
    }
    info!(server_id, delivered, "client stream closed");
}
