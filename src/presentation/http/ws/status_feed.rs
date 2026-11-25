//! `WS /api/state/ws`: one JSON `ServerStatus` per server per second.

use super::super::dto::ServerStatusDto;
use super::super::{route as erase, with_state, Route, Shared};
use futures::{SinkExt, StreamExt};
use tokio::sync::broadcast;
use warp::ws::{Message, WebSocket, Ws};
use warp::Filter;

pub fn route(state: Shared) -> Route {
    erase(
        warp::path!("ws")
            .and(warp::ws())
            .and(with_state(state))
            .map(|ws: Ws, state: Shared| {
                let rx = state.status.subscribe();
                ws.on_upgrade(move |socket| run(socket, rx))
            }),
    )
}

async fn run(socket: WebSocket, mut rx: broadcast::Receiver<crate::domain::status::ServerStatus>) {
    let (mut tx, mut incoming) = socket.split();
    loop {
        tokio::select! {
            msg = rx.recv() => match msg {
                Ok(s) => {
                    let text = serde_json::to_string(&ServerStatusDto::from(&s)).unwrap_or_default();
                    if tx.send(Message::text(text)).await.is_err() { break; }
                }
                Err(broadcast::error::RecvError::Lagged(_)) => continue,
                Err(broadcast::error::RecvError::Closed) => break,
            },
            m = incoming.next() => match m {
                Some(Ok(m)) if !m.is_close() => continue,
                _ => break,
            },
        }
    }
}
