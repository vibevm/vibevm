//! Lightweight RFC-7807 problem-details mapper for HTTP responses.

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-index/PROP-005#root");

use axum::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::Serialize;

use crate::index::quarantine::Unavailable;

#[derive(Debug)]
pub struct ApiError {
    pub status: StatusCode,
    pub kind: &'static str,
    pub title: &'static str,
    pub detail: String,
    /// The refusal row an `unavailable` answer carries (R55.3/R55.4):
    /// `None` for every error that is not a quarantine refusal.
    // Boxed to keep `ApiError` (and so every `Result<_, ApiError>` the
    // routes return) under clippy's `result_large_err` threshold — the
    // tree's own remedy (vibe-core `Error::parse_toml`, vibe-registry
    // `VendorError::Refresh`), reached for the same reason.
    pub unavailable: Option<Box<Unavailable>>,
}

impl ApiError {
    pub fn not_found(detail: impl Into<String>) -> Self {
        ApiError {
            status: StatusCode::NOT_FOUND,
            kind: "vibe-index/error/not-found",
            title: "resource not found",
            detail: detail.into(),
            unavailable: None,
        }
    }

    pub fn bad_request(detail: impl Into<String>) -> Self {
        ApiError {
            status: StatusCode::BAD_REQUEST,
            kind: "vibe-index/error/bad-request",
            title: "bad request",
            detail: detail.into(),
            unavailable: None,
        }
    }

    pub fn internal(detail: impl Into<String>) -> Self {
        ApiError {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            kind: "vibe-index/error/internal",
            title: "internal server error",
            detail: detail.into(),
            unavailable: None,
        }
    }

    pub fn unauthorized() -> Self {
        ApiError {
            status: StatusCode::UNAUTHORIZED,
            kind: "vibe-index/error/unauthorized",
            title: "authentication required",
            detail: "supply a valid bearer token via the Authorization header".into(),
            unavailable: None,
        }
    }

    pub fn forbidden(detail: impl Into<String>) -> Self {
        ApiError {
            status: StatusCode::FORBIDDEN,
            kind: "vibe-index/error/forbidden",
            title: "forbidden",
            detail: detail.into(),
            unavailable: None,
        }
    }

    /// A version that STANDS in the index but this build cannot act
    /// on. The status stays 404 (R55.4) — "you did not get the thing"
    /// is preserved — while the `kind`/`title` name the refusal as its
    /// own word, not "resource not found", and the body's extension
    /// member carries the full `Unavailable` row.
    pub fn unavailable(detail: impl Into<String>, row: Unavailable) -> Self {
        ApiError {
            status: StatusCode::NOT_FOUND,
            kind: "vibe-index/error/unavailable",
            title: "version unavailable to this build",
            detail: detail.into(),
            unavailable: Some(Box::new(row)),
        }
    }
}

/// The RFC-7807 problem-details body. `unavailable` is an extension
/// member — RFC 7807 allows extra members in a problem document by
/// design — carrying the refusal row whole (R55.3: the judgement lives
/// in the envelope, never inside a generated `VersionEntry`).
#[derive(Serialize)]
struct Body<'a> {
    #[serde(rename = "type")]
    kind: &'a str,
    title: &'a str,
    status: u16,
    detail: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    unavailable: Option<&'a Unavailable>,
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let status = self.status;
        let body = Body {
            kind: self.kind,
            title: self.title,
            status: status.as_u16(),
            detail: &self.detail,
            unavailable: self.unavailable.as_deref(),
        };
        (status, Json(body)).into_response()
    }
}
