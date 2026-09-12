//! `<base>resolve?uri=spec://…` — a citation, followed (PROP-057
//! `##SEO-MANIFEST-AND-RESOLVER`; the plan's fork F-15).
//!
//! An agent quotes a rule by its `spec://` address, and a person handed
//! that address wants the page. On a static host there is no server to
//! ask, so the site publishes a redirect table and a page that follows it
//! in the browser; here there IS a server, and F-15 says the local reader
//! keeps the route. Same answer, computed instead of looked up.
//!
//! It invents nothing. The coordinate must be the one package this reader
//! was pointed at, the document must be a page that package actually
//! holds, and a fragment travels only when the page carries that anchor —
//! a citation that has gone stale lands the reader on the page rather
//! than on nothing, and an address this reader does not serve is reported
//! rather than guessed at.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-057#SEO-MANIFEST-AND-RESOLVER");

use axum::response::Response;

use crate::Reader;
use crate::error::ApiError;

/// The address the resolver answers at, under the reader's base.
pub(crate) const ROUTE: &str = "resolve";

/// One parsed citation.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Citation {
    /// `<group>/<name>`.
    pub(crate) coordinate: String,
    /// The version the citation named, when it named one. Absent means
    /// «whichever is newest», which is what the `latest` address is.
    pub(crate) version: Option<String>,
    /// The document path inside the package, without an extension.
    pub(crate) document: String,
    /// The anchor, without its `#`.
    pub(crate) anchor: Option<String>,
}

/// Read a `spec://` address, or `None` when it is not one.
///
/// ```text
/// spec://org.vibevm.core/vibevm-docs/guide/start#install
/// spec://org.vibevm.core/vibevm-docs@0.1.0/guide/start
/// ```
pub(crate) fn parse(uri: &str) -> Option<Citation> {
    let rest = uri.strip_prefix("spec://")?;
    let (rest, anchor) = match rest.split_once('#') {
        Some((head, tail)) if !tail.is_empty() => (head, Some(tail.to_string())),
        Some((head, _)) => (head, None),
        None => (rest, None),
    };
    let mut segments = rest.split('/');
    let group = segments.next()?;
    let name = segments.next()?;
    let document: Vec<&str> = segments.collect();
    if group.is_empty() || name.is_empty() || document.is_empty() {
        return None;
    }
    if document
        .iter()
        .any(|part| part.is_empty() || *part == "." || *part == "..")
    {
        return None;
    }
    let (name, version) = match name.split_once('@') {
        Some((name, version)) if !name.is_empty() && !version.is_empty() => {
            (name, Some(version.to_string()))
        }
        Some(_) => return None,
        None => (name, None),
    };
    Some(Citation {
        coordinate: format!("{group}/{name}"),
        version,
        document: document.join("/"),
        anchor,
    })
}

/// Answer one `resolve` request.
pub(crate) fn answer(reader: &Reader, query: Option<&str>) -> Result<Response, ApiError> {
    let Some(uri) = parameter(query.unwrap_or(""), "uri") else {
        return Err(ApiError::bad_request(
            "`resolve` takes the citation to follow: `?uri=spec://<group>/<name>/<document>`",
        ));
    };
    let Some(citation) = parse(&uri) else {
        return Err(ApiError::bad_request(format!(
            "`{uri}` is not a `spec://` address — a citation names a group, a documentation \
             package and a document inside it"
        )));
    };

    // `prefix` is `<group>/<name>/<version>/`; a citation is followed
    // only inside the one package this reader was pointed at.
    let Some((coordinate, version)) = reader.prefix.trim_end_matches('/').rsplit_once('/') else {
        return Err(ApiError::internal("this reader has no coordinate"));
    };
    if citation.coordinate != coordinate {
        return Err(ApiError::not_found(format!(
            "this reader serves `{coordinate}` and the citation names `{}`",
            citation.coordinate
        )));
    }
    if let Some(asked) = &citation.version
        && asked != version
    {
        return Err(ApiError::not_found(format!(
            "this reader serves version {version} and the citation names {asked} — a reader \
             is pointed at one package, and a version is a different one"
        )));
    }

    let set = vibe_doc::pages::read_package(&reader.package_dir)
        .map_err(|e| ApiError::internal(e.to_string()))?;
    let wanted = format!("{}.xml", citation.document);
    let Some(page) = set.pages.iter().find(|page| page.rel == wanted) else {
        return Err(ApiError::not_found(format!(
            "this documentation carries no page `{}`",
            citation.document
        )));
    };

    let mut target = format!("{}{}{}/", reader.base, reader.prefix, citation.document);
    if let Some(anchor) = &citation.anchor
        && vibe_doc::manifest::page::anchors_of(&page.doc)
            .iter()
            .any(|held| held == anchor)
    {
        target.push('#');
        target.push_str(anchor);
    }
    Ok(super::found(&target))
}

/// One query parameter's value, percent-decoded.
///
/// Written here for the reason the path decoder is: the REFUSAL matters
/// more than the decoding, and a lenient reader would pass on something
/// it guessed at. A parameter that repeats answers with its first
/// spelling, which is what every server that does not want two answers
/// does.
fn parameter(query: &str, name: &str) -> Option<String> {
    for pair in query.split('&') {
        let Some((key, value)) = pair.split_once('=') else {
            continue;
        };
        if key != name {
            continue;
        }
        return super::decode(&value.replace('+', " "));
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_citation_is_read_into_its_four_parts() {
        assert_eq!(
            parse("spec://org.example/a-docs/guide/start#install"),
            Some(Citation {
                coordinate: "org.example/a-docs".to_string(),
                version: None,
                document: "guide/start".to_string(),
                anchor: Some("install".to_string()),
            })
        );
        assert_eq!(
            parse("spec://org.example/a-docs@0.1.0/start"),
            Some(Citation {
                coordinate: "org.example/a-docs".to_string(),
                version: Some("0.1.0".to_string()),
                document: "start".to_string(),
                anchor: None,
            })
        );
    }

    #[test]
    fn anything_that_is_not_a_citation_is_refused_rather_than_repaired() {
        for spelling in [
            "https://example.com/",
            "spec://org.example",
            "spec://org.example/a-docs",
            "spec://org.example/a-docs/",
            "spec://org.example/a-docs/../etc",
            "spec:///a-docs/start",
            "spec://org.example/@1.0/start",
        ] {
            assert!(parse(spelling).is_none(), "`{spelling}` must not resolve");
        }
    }

    #[test]
    fn a_parameter_is_decoded_and_the_first_spelling_wins() {
        assert_eq!(
            parameter("uri=spec%3A%2F%2Fa%2Fb%2Fc", "uri").as_deref(),
            Some("spec://a/b/c")
        );
        assert!(parameter("other=1", "uri").is_none());
    }
}
