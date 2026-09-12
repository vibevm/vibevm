//! The reader's one error layer (the new-crate checklist, PROP-057
//! `##PIPE-CRATES`).
//!
//! Two kinds of failure live here and they reach a person differently.
//! A **start-up** failure — the package cannot be read, the port is
//! taken — ends the command with a message naming the rule it broke, the
//! shape every refusal in this pipeline takes. A **request** failure is
//! an HTTP status and an RFC 7807 body, because the reader is a server
//! and a browser is the one reading it.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-057#LOCAL-SERVE");

use std::path::PathBuf;

use axum::Json;
use axum::http::{StatusCode, header};
use axum::response::{IntoResponse, Response};
use specmark::spec;

/// Everything the reader can refuse to do at start-up.
#[derive(Debug, thiserror::Error)]
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-057#LOCAL-SERVE")]
#[non_exhaustive]
pub enum ServerError {
    /// The package the reader was pointed at cannot answer for itself.
    #[error(
        "`{path}` {message} \
         (violates spec://org.vibevm.core/vibevm/common/PROP-057#LOCAL-SERVE; \
         fix: point the reader at a documentation package, or warm one with `vibe cache add`)"
    )]
    Package { path: PathBuf, message: String },

    /// The loopback address could not be listened on.
    #[error(
        "the local reader cannot start: {message} \
         (violates spec://org.vibevm.core/vibevm/common/PROP-057#LOCAL-SERVE; \
         fix: choose another `--port`; the host is not a choice — the reader binds the \
         loopback and nothing else)"
    )]
    Bind { message: String },

    /// The pipeline refused: an unreadable page, an unreadable manifest,
    /// a language the package is not written in.
    #[error(transparent)]
    Doc(#[from] vibe_doc::DocError),
}

/// The crate's result type.
pub type ServerResult<T> = std::result::Result<T, ServerError>;

/// One refused request, as a browser receives it: a status and an RFC
/// 7807 problem document.
///
/// The body carries `title` and `detail` under a machine-readable
/// `kind`, the same shape `vibe-index` answers with — repeated rather
/// than imported, for the reason in the crate header.
#[derive(Debug, Clone)]
pub struct ApiError {
    pub status: StatusCode,
    pub kind: &'static str,
    pub title: &'static str,
    pub detail: String,
}

impl ApiError {
    /// Nothing here answers to that address.
    ///
    /// ```
    /// use vibe_doc_server::error::ApiError;
    /// let e = ApiError::not_found("no such page");
    /// assert_eq!(e.status.as_u16(), 404);
    /// ```
    pub fn not_found(detail: impl Into<String>) -> ApiError {
        ApiError {
            status: StatusCode::NOT_FOUND,
            kind: "not-found",
            title: "No such document",
            detail: detail.into(),
        }
    }

    /// The address is not one this reader could ever answer — a segment
    /// that is not a plain name, an escape that decodes to nothing.
    pub fn bad_request(detail: impl Into<String>) -> ApiError {
        ApiError {
            status: StatusCode::BAD_REQUEST,
            kind: "bad-address",
            title: "Not an address this reader serves",
            detail: detail.into(),
        }
    }

    /// The package is there and something about reading it failed.
    pub fn internal(detail: impl Into<String>) -> ApiError {
        ApiError {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            kind: "unreadable",
            title: "The documentation could not be read",
            detail: detail.into(),
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let body = Json(serde_json::json!({
            "kind": self.kind,
            "title": self.title,
            "status": self.status.as_u16(),
            "detail": self.detail,
        }));
        // A refusal carries the same policy and sniff headers a page
        // does: an error body is a document a browser renders too.
        (self.status, [(header::CACHE_CONTROL, "no-store")], body).into_response()
    }
}
