//! Handlers stay thin: parse the request, call one service, map the result.

pub mod clients;
pub mod health;
pub mod monitoring;
pub mod publish;
pub mod servers;

use super::error::reject;
use crate::domain::error::AppResult;
use serde::de::DeserializeOwned;
use serde::Serialize;
use warp::{Filter, Rejection};

/// JSON request body, capped at 1 MiB.
fn json_body<T: DeserializeOwned + Send>() -> impl Filter<Extract = (T,), Error = Rejection> + Clone
{
    warp::body::content_length_limit(1024 * 1024).and(warp::body::json())
}

/// Maps a service result to a JSON reply or an API rejection.
fn reply<T: Serialize>(r: AppResult<T>) -> Result<warp::reply::Json, Rejection> {
    r.map(|v| warp::reply::json(&v)).map_err(reject)
}

/// For endpoints whose success body is `{"ok": true}`.
fn ok(r: AppResult<()>) -> Result<warp::reply::Json, Rejection> {
    reply(r.map(|_| serde_json::json!({ "ok": true })))
}
