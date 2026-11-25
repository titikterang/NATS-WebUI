use warp::{Filter, Rejection, Reply};

/// `GET /healthz`: the process is up and serving HTTP.
pub fn route() -> impl Filter<Extract = (impl Reply,), Error = Rejection> + Clone {
    warp::path!("healthz").and(warp::get()).map(|| "ok")
}
