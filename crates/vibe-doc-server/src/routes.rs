//! The reader's addresses (PROP-057 `##SITE-MOUNT`,
//! `##SITE-TRAILING-SLASH`, `##LOCAL-STATIC`).
//!
//! One handler answers everything but `/healthz`, and it parses the path
//! itself. That is deliberate: a router matches the RAW path and an
//! extractor percent-decodes the capture AFTERWARDS, so an escape that
//! decodes to a separator becomes one only once the match is already
//! made — which is how `data_dir.join(<capture>)` walks out of its root.
//! `##LOCAL-STATIC` names that form and says the reader must not repeat
//! it. Here the address is decoded first and then split by this module,
//! and a segment that is not exactly one ordinary name never reaches the
//! filesystem.
//!
//! The addresses are the site's own, so a link written on a page works
//! in both worlds:
//!
//! | address | what comes back |
//! |---|---|
//! | `<base><coordinate>/<version>/<document>/` | the island, `text/html` |
//! | `<base><coordinate>/<version>/<document>.md` | `text/markdown` |
//! | `<base><coordinate>/<version>/<document>.xml` | `application/xml` |
//! | `<base>manifest.json` | the page manifest |
//! | `<base>llms.txt`, `llms-small.txt`, `llms-medium.txt`, `llms-full.txt` | the agent files |
//! | `/healthz` | `{"status":"ok"}` |
//!
//! A page address without its trailing slash is a 308 to the one with
//! it, on the web and locally alike.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-057#SITE-MOUNT");

use std::collections::BTreeMap;
use std::sync::Arc;

use axum::Router;
use axum::http::{HeaderValue, StatusCode, Uri, header};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use vibe_doc::build::{self, Format};
use vibe_doc::{llms, manifest};

use crate::Reader;
use crate::error::ApiError;

/// Build the router.
pub fn router(reader: Arc<Reader>) -> Router {
    Router::new()
        .route("/healthz", get(healthz))
        .fallback(dispatch)
        .with_state(reader)
}

/// Is the reader up. The form is `vibe-index`'s, so an operator who
/// knows one knows the other.
async fn healthz() -> Response {
    axum::Json(serde_json::json!({ "status": "ok" })).into_response()
}

/// Everything else.
async fn dispatch(
    axum::extract::State(reader): axum::extract::State<Arc<Reader>>,
    uri: Uri,
) -> Response {
    let csp = reader.csp.clone();
    match answer(&reader, uri.path()) {
        Ok(response) => with_guards(response, &csp),
        Err(error) => with_guards(error.into_response(), &csp),
    }
}

/// The content policy and the sniff guard ride on EVERY response,
/// including refusals and redirects (`##LOCAL-CSP`). A policy that is
/// only on the happy path is a policy with a hole in it.
fn with_guards(mut response: Response, csp: &str) -> Response {
    let headers = response.headers_mut();
    if let Ok(value) = HeaderValue::from_str(csp) {
        headers.insert(header::CONTENT_SECURITY_POLICY, value);
    }
    headers.insert(
        header::X_CONTENT_TYPE_OPTIONS,
        HeaderValue::from_static("nosniff"),
    );
    response
}

/// Resolve one address.
fn answer(reader: &Reader, raw_path: &str) -> Result<Response, ApiError> {
    let path = decode(raw_path).ok_or_else(|| {
        ApiError::bad_request(
            "the address carries an escape this reader cannot decode; a documentation \
             address is a path of ordinary names",
        )
    })?;
    let Some(rest) = path.strip_prefix(&reader.base) else {
        return Err(ApiError::not_found(format!(
            "this reader serves `{}` and nothing else",
            reader.mount()
        )));
    };

    if let Some(response) = machine_file(reader, rest)? {
        return Ok(response);
    }

    let Some(address) = rest.strip_prefix(&reader.prefix) else {
        return Err(ApiError::not_found(format!(
            "`{rest}` is not under `{}`, which is the one package this reader was pointed at",
            reader.prefix
        )));
    };
    page(reader, address)
}

/// The files that describe the whole mount rather than one page.
fn machine_file(reader: &Reader, rest: &str) -> Result<Option<Response>, ApiError> {
    if rest == "manifest.json" {
        let built = manifest::build(&reader.package_dir, &reader.sources, &options(reader))
            .map_err(|e| ApiError::internal(e.to_string()))?;
        return Ok(Some(text(
            manifest::to_json(&built.manifest),
            "application/json; charset=utf-8",
        )));
    }
    for tier in llms::Tier::ALL.iter().copied() {
        if rest != tier.file_name() {
            continue;
        }
        let built = manifest::build(&reader.package_dir, &reader.sources, &options(reader))
            .map_err(|e| ApiError::internal(e.to_string()))?;
        let (set, content) = build::content(
            &reader.package_dir,
            &reader.sources,
            &reader.base,
            reader.derived.clone(),
        )
        .map_err(|e| ApiError::internal(e.to_string()))?;
        let bodies = llms::bodies(&set, &content);
        return Ok(Some(text(
            llms::render(tier, &built.manifest, &bodies, &reader.base),
            "text/plain; charset=utf-8",
        )));
    }
    Ok(None)
}

/// One page, in whichever projection its address asked for.
fn page(reader: &Reader, address: &str) -> Result<Response, ApiError> {
    // Every segment is checked BEFORE the address is read for its shape.
    // An escape that decoded to a separator has already become one by
    // now, so this is the line the traversal has to cross — and it runs
    // on the decoded path, which is the whole point of `##LOCAL-STATIC`.
    for segment in address.trim_end_matches('/').split('/') {
        if segment.is_empty()
            || segment == "."
            || segment == ".."
            || segment.contains('\\')
            || segment.contains('\0')
        {
            return Err(ApiError::bad_request(format!(
                "`{segment}` is not a document name — an address this reader serves is a \
                 path of ordinary names under the package's own tree"
            )));
        }
    }
    let (rel, format) = match address.strip_suffix('/') {
        Some(stem) => (format!("{stem}.xml"), Format::Html),
        None => match address.rsplit_once('.') {
            Some((stem, "md")) => (format!("{stem}.xml"), Format::Md),
            Some((stem, "xml")) => (format!("{stem}.xml"), Format::Xml),
            // A page address without its slash is the same page
            // (`##SITE-TRAILING-SLASH`); the redirect is permanent so a
            // link that lost the slash is repaired once and not on every
            // visit.
            _ => {
                let target = format!("{}{}{address}/", reader.base, reader.prefix);
                return Ok(redirect(&target));
            }
        },
    };
    let set = vibe_doc::pages::read_package(&reader.package_dir)
        .map_err(|e| ApiError::internal(e.to_string()))?;
    let Some(found) = set.pages.iter().find(|p| p.rel == rel) else {
        if let Some(bad) = set.unreadable.iter().find(|p| p.rel == rel) {
            return Err(ApiError::internal(format!(
                "`{rel}` is not readable as a documentation page: {}",
                bad.message
            )));
        }
        return Err(ApiError::not_found(format!(
            "this documentation carries no page `{rel}`"
        )));
    };

    // The island is rendered on THIS request (`##LOCAL-SERVE`), so an
    // author editing a page sees the edit on reload rather than on a
    // restart. Only the page's own citations are resolved: a manual
    // cites hundreds of rules and a reader that resolved all of them to
    // show one page would pay for the whole corpus per view.
    let one = vibe_doc::pages::PageSet {
        pages: vec![found.clone()],
        unreadable: Vec::new(),
    };
    let content = vibe_doc::content::Content {
        rules: vibe_doc::citations::resolve_rules(&one, &reader.sources),
        derived: reader.derived.clone(),
        examples: BTreeMap::new(),
        base: reader.base.clone(),
    };
    let body = build::render_page(found, &content, format);
    Ok(text(
        body,
        match format {
            Format::Html => "text/html; charset=utf-8",
            Format::Md => "text/markdown; charset=utf-8",
            Format::Xml => "application/xml; charset=utf-8",
        },
    ))
}

/// The two instants a manifest carries. The reader supplies the one it
/// started with rather than calling the clock per request: two views of
/// one unchanged page must be one document.
fn options(reader: &Reader) -> manifest::Options {
    manifest::Options::at(reader.rendered_at)
}

/// A document response. Pages are `no-store`: the reader re-renders on
/// every request by contract, and a cached copy would defeat the point
/// of that.
fn text(body: String, content_type: &'static str) -> Response {
    (
        StatusCode::OK,
        [
            (header::CONTENT_TYPE, HeaderValue::from_static(content_type)),
            (header::CACHE_CONTROL, HeaderValue::from_static("no-store")),
        ],
        body,
    )
        .into_response()
}

fn redirect(target: &str) -> Response {
    match HeaderValue::from_str(target) {
        Ok(value) => (StatusCode::PERMANENT_REDIRECT, [(header::LOCATION, value)]).into_response(),
        Err(_) => ApiError::bad_request("the address cannot be spelled as a location header")
            .into_response(),
    }
}

/// Percent-decode a path, or `None` when an escape is malformed or the
/// result is not UTF-8.
///
/// Written here rather than taken from a crate because it is fifteen
/// lines and because the REFUSAL matters more than the decoding: an
/// address this function cannot read is an address the reader answers
/// `400` to, and a lenient decoder would quietly pass on something it
/// guessed at.
fn decode(path: &str) -> Option<String> {
    let bytes = path.as_bytes();
    let mut out: Vec<u8> = Vec::with_capacity(bytes.len());
    let mut at = 0usize;
    while at < bytes.len() {
        if bytes[at] != b'%' {
            out.push(bytes[at]);
            at += 1;
            continue;
        }
        let hex = std::str::from_utf8(bytes.get(at + 1..at + 3)?).ok()?;
        out.push(u8::from_str_radix(hex, 16).ok()?);
        at += 3;
    }
    String::from_utf8(out).ok()
}

#[cfg(test)]
mod tests;
