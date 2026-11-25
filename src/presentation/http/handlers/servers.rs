use super::super::dto::{ServerDto, ServerInput, StateDto};
use super::super::{route, with_state, Route, Shared};
use super::{json_body, ok, reply};
use crate::domain::error::{AppError, AppResult};
use warp::{Filter, Rejection};

pub fn routes(state: Shared) -> Route {
    let all = warp::path::end()
        .and(warp::get())
        .and(with_state(state.clone()))
        .and_then(get_state);
    let create = warp::path!("server" / "new")
        .and(warp::post())
        .and(with_state(state.clone()))
        .and(json_body())
        .and_then(create);
    let update = warp::path!("server" / "update")
        .and(warp::post())
        .and(with_state(state.clone()))
        .and(json_body())
        .and_then(update);
    let delete = warp::path!("server" / "delete" / i64)
        .and(warp::get())
        .and(with_state(state))
        .and_then(delete);
    route(all)
        .or(route(create))
        .unify()
        .or(route(update))
        .unify()
        .or(route(delete))
        .unify()
        .boxed()
}

/// Servers and clients in one call (the UI's initial load and refresh).
async fn get_state(state: Shared) -> Result<warp::reply::Json, Rejection> {
    let r: AppResult<StateDto> = async {
        Ok(StateDto {
            servers: state
                .servers
                .list()
                .await?
                .iter()
                .map(ServerDto::from)
                .collect(),
            clients: state.clients.list().await?.iter().map(Into::into).collect(),
        })
    }
    .await;
    reply(r)
}

async fn create(state: Shared, input: ServerInput) -> Result<warp::reply::Json, Rejection> {
    let (_, draft) = input.into_draft();
    reply(
        state
            .servers
            .create(draft)
            .await
            .map(|s| ServerDto::from(&s)),
    )
}

async fn update(state: Shared, input: ServerInput) -> Result<warp::reply::Json, Rejection> {
    let (id, draft) = input.into_draft();
    let r = match id {
        Some(id) => state
            .servers
            .update(id, draft)
            .await
            .map(|s| ServerDto::from(&s)),
        None => Err(AppError::validation("id is required")),
    };
    reply(r)
}

async fn delete(id: i64, state: Shared) -> Result<warp::reply::Json, Rejection> {
    ok(state.servers.delete(id).await)
}
