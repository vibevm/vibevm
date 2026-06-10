//! Lightweight RFC-7807 problem-details mapper for HTTP responses.

specmark::scope!("spec://vibevm/modules/vibe-index/PROP-005#root");

use axum::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::Serialize;

#[derive(Debug)]
pub struct ApiError {
    pub status: StatusCode,
    pub kind: &'static str,
    pub title: &'static str,
    pub detail: String,
}

impl ApiError {
    pub fn not_found(detail: impl Into<String>) -> Self {
        ApiError {
            status: StatusCode::NOT_FOUND,
            kind: "vibe-index/error/not-found",
            title: "resource not found",
            detail: detail.into(),
        }
    }

    pub fn bad_request(detail: impl Into<String>) -> Self {
        ApiError {
            status: StatusCode::BAD_REQUEST,
            kind: "vibe-index/error/bad-request",
            title: "bad request",
            detail: detail.into(),
        }
    }

    pub fn internal(detail: impl Into<String>) -> Self {
        ApiError {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            kind: "vibe-index/error/internal",
            title: "internal server error",
            detail: detail.into(),
        }
    }

    pub fn unauthorized() -> Self {
        ApiError {
            status: StatusCode::UNAUTHORIZED,
            kind: "vibe-index/error/unauthorized",
            title: "authentication required",
            detail: "supply a valid bearer token via the Authorization header".into(),
        }
    }

    pub fn forbidden(detail: impl Into<String>) -> Self {
        ApiError {
            status: StatusCode::FORBIDDEN,
            kind: "vibe-index/error/forbidden",
            title: "forbidden",
            detail: detail.into(),
        }
    }
}

#[derive(Serialize)]
struct Body<'a> {
    #[serde(rename = "type")]
    kind: &'a str,
    title: &'a str,
    status: u16,
    detail: &'a str,
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let status = self.status;
        let body = Body {
            kind: self.kind,
            title: self.title,
            status: status.as_u16(),
            detail: &self.detail,
        };
        (status, Json(body)).into_response()
    }
}
