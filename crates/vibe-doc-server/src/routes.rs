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
//! | `<base><coordinate>/<version>/<document>/` | the page: the island in the shell |
//! | `<base><coordinate>/<version>/<document>.md` | `text/markdown` |
//! | `<base><coordinate>/<version>/<document>.xml` | `application/xml` |
//! | `<base><coordinate>/<version>/` | the package's own page: the card, the shelf, the agent surfaces |
//! | `<base>media/<name>`, `<base><coordinate>/<version>/media/<name>` | the card's pictures |
//! | `<base>manifest.json`, `<base><coordinate>/<version>/manifest.json` | the page manifest |
//! | `<base>llms.txt`, `llms-small.txt`, `llms-medium.txt`, `llms-full.txt`, and the same under the edition | the agent files |
//! | `<base>resolve?uri=spec://…` | a citation, followed |
//! | `<base>assets/…`, `<base>build/…` | the shell's own files |
//! | `/healthz` | `{"status":"ok"}` |
//!
//! A page address without its trailing slash is a 308 to the one with
//! it, on the web and locally alike.
//!
//! **Every one of those addresses answers under `latest` as well**, and
//! answers the same thing rather than a redirect to the number: the site
//! publishes both spellings of a version as real pages, and a reader
//! pointed at one package can only mean this version by `latest`. The
//! translation is made once, at the top of [`answer`], so no lane below
//! knows a version has two names.
//!
//! The order of those lanes is not arbitrary. The machine files and the
//! resolver are the mount's own names; the package's pages live under a
//! prefix nothing else can match; and the shell's files are tried LAST,
//! so a shell can never shadow a document — the shell is a build product
//! and the documentation is the point.

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

mod media;
mod resolve;
mod statics;

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
    match answer(&reader, uri.path(), uri.query()) {
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
fn answer(reader: &Reader, raw_path: &str, query: Option<&str>) -> Result<Response, ApiError> {
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
    // `latest` is read into the number ONCE, before any lane looks at the
    // address, so every route below answers under both spellings without
    // knowing there are two.
    let numbered = as_numbered(reader, rest);
    let rest = numbered.as_ref();

    if let Some(response) = machine_file(reader, rest)? {
        return Ok(response);
    }
    if let Some(response) = media::answer(reader, rest)? {
        return Ok(response);
    }
    if rest == resolve::ROUTE || rest == concat_slash(resolve::ROUTE) {
        return resolve::answer(reader, query);
    }
    // The package's own page.
    if rest == reader.prefix {
        return package_page(reader);
    }
    // The door above the mount. It means «the documentation», and the
    // documentation now has a page of its own to mean — so the door leads
    // there rather than past it into the first chapter.
    if rest.is_empty() {
        return Ok(found(&format!("{}{}", reader.base, reader.prefix)));
    }

    if let Some(address) = rest.strip_prefix(&reader.prefix) {
        return page(reader, address);
    }

    // Last, and only after nothing about the documentation matched: the
    // shell's own files. The shell checks the name again before it
    // becomes a path.
    if let Some(response) = statics::asset(reader, rest) {
        return Ok(response);
    }
    Err(ApiError::not_found(format!(
        "`{rest}` is not under `{}`, which is the one package this reader was pointed at, \
         and is no file of the reader's shell",
        reader.prefix
    )))
}

/// `resolve/` beside `resolve` — the trailing slash a link may carry.
fn concat_slash(route: &str) -> String {
    format!("{route}/")
}

/// One address with `latest` read as the version this reader serves
/// (`##SITE-MOUNT`).
///
/// The site publishes every page at BOTH spellings of one version, and a
/// link — the shell's version switcher, a citation followed to `latest`,
/// an address somebody kept — arrives here under either. A reader is
/// pointed at ONE package, so `latest` can only mean this version: the
/// translation is total, and it is made in one place before any lane
/// reads the address, so no route has to know that a version has two
/// names.
///
/// The answer is the page itself and not a redirect to the other
/// spelling. Three reasons, in the order they matter: the site answers
/// `200` at both, so a reader sees locally what a reader sees on the web;
/// the shell's version switcher OFFERS `latest`, and a redirect would put
/// the number back in the address bar and read as a control that does
/// nothing; and every relative address inside an island is written to be
/// right under either spelling, so moving the reader out of the one they
/// chose buys nothing and costs a round trip per asset.
fn as_numbered<'a>(reader: &Reader, rest: &'a str) -> std::borrow::Cow<'a, str> {
    match rest.strip_prefix(&reader.latest_prefix()) {
        Some(tail) => std::borrow::Cow::Owned(format!("{}{tail}", reader.prefix)),
        None => std::borrow::Cow::Borrowed(rest),
    }
}

/// The package's own page: the card, the shelf of what this
/// documentation holds, and the surfaces an agent reads.
///
/// The island is EMPTY, and that is the whole design rather than a gap.
/// A package page has no document behind it — there is no `.xml` in the
/// package that says «this is the package» — so the pipeline renders no
/// island for it, and the values it shows are the manifest's: the card,
/// the pages in reading order, `llms.txt`. The shell already builds that
/// page out of `<base>manifest.json` (it has been building it and had
/// nowhere to show it), so what this address owes it is the template and
/// the page's title, and nothing else.
fn package_page(reader: &Reader) -> Result<Response, ApiError> {
    let built = manifest::build(&reader.package_dir, &reader.sources, &options(reader))
        .map_err(|e| ApiError::internal(e.to_string()))?;
    let title = built.manifest.package.title.clone();
    let llms = format!("{}llms.txt", reader.base);
    let dressed = wearing(
        reader,
        Some(&title),
        &[vibe_doc_shell::template::Alternate {
            media_type: "text/plain",
            href: &llms,
            title: "llms.txt of this documentation",
        }],
        "",
    );
    Ok(text(dressed, "text/html; charset=utf-8"))
}

/// The files that describe the documentation rather than one page.
///
/// They answer at the mount's root AND under the edition, because that is
/// where the site publishes them: `copySurfaces` copies the pipeline's
/// four `llms` tiers into `<coordinate>/<version>/`, and the build
/// re-serialises the page manifest beside them (`##SEO-LLMS-FILES`,
/// A4.5). A reader is pointed at one package, so the two places hold the
/// same five files, and an agent that learned an address from the site
/// finds it here.
///
/// The `latest` spelling costs nothing: the address arrives already read
/// into the number ([`as_numbered`]).
fn machine_file(reader: &Reader, rest: &str) -> Result<Option<Response>, ApiError> {
    let rest = rest.strip_prefix(&reader.prefix).unwrap_or(rest);
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
    let set = vibe_doc::pages::read_package(&reader.package_dir)
        .map_err(|e| ApiError::internal(e.to_string()))?;
    let carries = |rel: &str| set.pages.iter().any(|page| page.rel == rel);
    let (rel, format) = match address.strip_suffix('/') {
        Some(stem) => (format!("{stem}.xml"), Format::Html),
        None => match address.rsplit_once('.') {
            Some((stem, "md")) => (format!("{stem}.xml"), Format::Md),
            Some((stem, "xml")) => (format!("{stem}.xml"), Format::Xml),
            // A page address without its slash is the same page
            // (`##SITE-TRAILING-SLASH`); the redirect is permanent so a
            // link that lost the slash is repaired once and not on every
            // visit.
            //
            // Only for an address that IS a page, though. The repair used
            // to fire on any name with a dot in it, which sent a request
            // for a file this reader does not serve — `llms.txt` under the
            // edition, before it was served there — off to `…/llms.txt/`
            // and a 404 one hop further on. A name nobody carries is a
            // refusal where it was asked for.
            _ if carries(&format!("{address}.xml")) => {
                let target = format!("{}{}{address}/", reader.base, reader.prefix);
                return Ok(redirect(&target));
            }
            _ => {
                return Err(ApiError::not_found(format!(
                    "this documentation carries neither a page `{address}` nor a file of that \
                     name — a page's address ends in a slash and its projections are `.md` and \
                     `.xml`"
                )));
            }
        },
    };
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
    match format {
        // The island goes into the shell, and the two things in the
        // template that name a PAGE rather than the shell are corrected:
        // its title, and the projections it offers. Everything else — the
        // head, the chunks, the layout — is the site's own bytes,
        // untouched, which is what makes the parity test meaningful.
        Format::Html => Ok(text(
            in_shell(reader, found, &body),
            "text/html; charset=utf-8",
        )),
        Format::Md => Ok(text(body, "text/markdown; charset=utf-8")),
        Format::Xml => Ok(text(body, "application/xml; charset=utf-8")),
    }
}

/// One rendered island, dressed in the reader's shell.
pub(crate) fn in_shell(reader: &Reader, page: &vibe_doc::pages::Page, island: &str) -> String {
    let stem = page.rel.strip_suffix(".xml").unwrap_or(&page.rel);
    let at = format!("{}{}{stem}", reader.base, reader.prefix);
    let md = format!("{at}.md");
    let xml = format!("{at}.xml");
    let llms = format!("{}llms.txt", reader.base);
    wearing(
        reader,
        title_of(page).as_deref(),
        &[
            vibe_doc_shell::template::Alternate {
                media_type: "text/markdown",
                href: &md,
                title: "This page as Markdown",
            },
            vibe_doc_shell::template::Alternate {
                media_type: "application/xml",
                href: &xml,
                title: "This page as the dialect XML",
            },
            vibe_doc_shell::template::Alternate {
                media_type: "text/plain",
                href: &llms,
                title: "llms.txt of this documentation",
            },
        ],
        island,
    )
}

/// The reader's template, corrected for the thing being served and
/// filled.
///
/// The two corrections are the two places a prerendered route names a
/// PAGE rather than the shell — the tab's title and the projections it
/// offers — and both are made IN PLACE, on elements the template already
/// declares. Nothing is inserted and nothing is moved: an element added
/// to the head of a resumable document stops the framework from resuming
/// it, and a page that does not resume has no behaviour at all.
fn wearing(
    reader: &Reader,
    title: Option<&str>,
    alternates: &[vibe_doc_shell::template::Alternate<'_>],
    island: &str,
) -> String {
    let dressed = vibe_doc_shell::template::relink(&reader.template, alternates);
    let dressed = match title {
        Some(title) => vibe_doc_shell::template::retitle(&dressed, title),
        None => dressed,
    };
    vibe_doc_shell::template::glue(&dressed, &reader.shell.index().island_marker, island)
}

/// The page's own heading, for the browser tab.
///
/// A page with no H1 leaves the template's title alone rather than
/// blanking it: the pivot admits a document without one, and an empty tab
/// is worse than a stale one.
fn title_of(page: &vibe_doc::pages::Page) -> Option<String> {
    page.doc.title.as_ref().map(|title| title.text.clone())
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

/// A resolver's answer: `302`, not `308`.
///
/// A followed citation is a lookup that can come out differently
/// tomorrow — a page renamed, a version moved on — and a permanent
/// redirect would be cached by the browser and never asked again.
fn found(target: &str) -> Response {
    match HeaderValue::from_str(target) {
        Ok(value) => (StatusCode::FOUND, [(header::LOCATION, value)]).into_response(),
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
