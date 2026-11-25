use super::super::dto::PublishInput;
use super::super::{route, with_state, Route, Shared};
use super::{json_body, ok};
use warp::{Filter, Rejection};

pub fn routes(state: Shared) -> Route {
    let publish = warp::path!("server" / i64 / "publish")
        .and(warp::post())
        .and(with_state(state))
        .and(json_body())
        .and_then(publish);
    route(publish)
}

async fn publish(
    id: i64,
    state: Shared,
    input: PublishInput,
) -> Result<warp::reply::Json, Rejection> {
    let r = match input.into_message() {
        Ok(m) => state.publish.publish(id, m).await,
        Err(e) => Err(e),
    };
    ok(r)
}
