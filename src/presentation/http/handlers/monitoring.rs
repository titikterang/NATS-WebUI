use super::super::{route, with_state, Route, Shared};
use super::reply;
use crate::application::ports::MonitorEndpoint;
use warp::{Filter, Rejection};

pub fn routes(state: Shared) -> Route {
    let jsz = warp::path!("server" / i64 / "jsz")
        .and(warp::get())
        .and(with_state(state.clone()))
        .and_then(|id, s| fetch(id, s, MonitorEndpoint::Jsz));
    let subsz = warp::path!("server" / i64 / "subsz")
        .and(warp::get())
        .and(with_state(state))
        .and_then(|id, s| fetch(id, s, MonitorEndpoint::Subsz));
    route(jsz).or(route(subsz)).unify().boxed()
}

async fn fetch(
    id: i64,
    state: Shared,
    endpoint: MonitorEndpoint,
) -> Result<warp::reply::Json, Rejection> {
    reply(state.inspect.fetch(id, endpoint).await)
}
