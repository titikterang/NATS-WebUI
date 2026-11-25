use super::super::dto::{ClientDto, ClientInput};
use super::super::{route, with_state, Route, Shared};
use super::{json_body, ok, reply};
use crate::domain::error::AppError;
use warp::{Filter, Rejection};

pub fn routes(state: Shared) -> Route {
    let create = warp::path!("client" / "new")
        .and(warp::post())
        .and(with_state(state.clone()))
        .and(json_body())
        .and_then(create);
    let update = warp::path!("client" / "update")
        .and(warp::post())
        .and(with_state(state.clone()))
        .and(json_body())
        .and_then(update);
    let delete = warp::path!("client" / "delete" / i64)
        .and(warp::get())
        .and(with_state(state))
        .and_then(delete);
    route(create)
        .or(route(update))
        .unify()
        .or(route(delete))
        .unify()
        .boxed()
}

async fn create(state: Shared, input: ClientInput) -> Result<warp::reply::Json, Rejection> {
    let (_, draft) = input.into_draft();
    reply(
        state
            .clients
            .create(draft)
            .await
            .map(|c| ClientDto::from(&c)),
    )
}

async fn update(state: Shared, input: ClientInput) -> Result<warp::reply::Json, Rejection> {
    let (id, draft) = input.into_draft();
    let r = match id {
        Some(id) => state
            .clients
            .update(id, draft)
            .await
            .map(|c| ClientDto::from(&c)),
        None => Err(AppError::validation("id is required")),
    };
    reply(r)
}

async fn delete(id: i64, state: Shared) -> Result<warp::reply::Json, Rejection> {
    ok(state.clients.delete(id).await)
}
