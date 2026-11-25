//! The single mapping from `AppError` to HTTP. Every API error body is `{"error": "<message>"}`.

use crate::domain::error::AppError;
use std::convert::Infallible;
use warp::http::StatusCode;
use warp::{Rejection, Reply};

#[derive(Debug)]
pub struct ApiError(pub AppError);

impl warp::reject::Reject for ApiError {}

pub fn reject(e: AppError) -> Rejection {
    warp::reject::custom(ApiError(e))
}

fn status_of(e: &AppError) -> StatusCode {
    match e {
        AppError::NotFound(_) => StatusCode::NOT_FOUND,
        AppError::Validation(_) => StatusCode::BAD_REQUEST,
        AppError::Unavailable(_) => StatusCode::SERVICE_UNAVAILABLE,
        AppError::PermissionDenied(_) => StatusCode::FORBIDDEN,
        AppError::Upstream(_) => StatusCode::BAD_GATEWAY,
        AppError::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
    }
}

pub async fn recover(r: Rejection) -> Result<impl Reply, Infallible> {
    let (code, msg) = if let Some(ApiError(e)) = r.find() {
        if let AppError::Internal(_) = e {
            tracing::error!(error = %e, "internal error");
        }
        (status_of(e), e.to_string())
    } else if r.is_not_found() {
        (StatusCode::NOT_FOUND, "not found".into())
    } else if r.find::<warp::reject::MethodNotAllowed>().is_some() {
        (StatusCode::METHOD_NOT_ALLOWED, "method not allowed".into())
    } else if let Some(e) = r.find::<warp::filters::body::BodyDeserializeError>() {
        (StatusCode::BAD_REQUEST, e.to_string())
    } else if let Some(e) = r.find::<warp::reject::PayloadTooLarge>() {
        (StatusCode::PAYLOAD_TOO_LARGE, e.to_string())
    } else {
        (StatusCode::BAD_REQUEST, format!("{r:?}"))
    };
    Ok(warp::reply::with_status(
        warp::reply::json(&serde_json::json!({ "error": msg })),
        code,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_error_kind_has_a_distinct_status() {
        let kinds = [
            AppError::NotFound(String::new()),
            AppError::Validation(String::new()),
            AppError::Unavailable(String::new()),
            AppError::PermissionDenied(String::new()),
            AppError::Upstream(String::new()),
            AppError::Internal(String::new()),
        ];
        let mut codes: Vec<u16> = kinds.iter().map(|k| status_of(k).as_u16()).collect();
        codes.sort();
        codes.dedup();
        assert_eq!(codes, vec![400, 403, 404, 500, 502, 503]);
    }
}
