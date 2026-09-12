//! Where an island's links point (PROP-057 `##SITE-MOUNT`,
//! `##SITE-TRAILING-SLASH`, `##SEO-MANIFEST-AND-RESOLVER`).
//!
//! A page's SOURCE addresses its neighbours as files: `../glossary/index.xml`
//! is a path from one `.xml` to another inside the package directory. The
//! SITE addresses them as places: a page is a directory that ends in a
//! slash, and the `.xml` lies beside it as a projection. An island that
//! published the source spelling would publish a link that is right in a
//! checkout and dead everywhere the documentation is actually read.
//!
//! ## The conversion needs no page address, and that is the point
//!
//! A page whose source is `<document>.xml` is served from the directory
//! `<document>/`, so the served page sits EXACTLY one level deeper than
//! the file its links were written against. Every relative address
//! therefore gains exactly one `../`, and a target naming a page trades
//! its `.xml` for a slash. Nothing else is needed — not the page's own
//! address, not the base, not which of the two version spellings the
//! reader arrived by, and not the language segment the site puts in front
//! of an adaptation (D-06). One rule, right under every mount, which is
//! strictly stronger than computing the answer from the page's own path:
//! that path does not know whether a language segment sits in front of
//! it, and under an adaptation it would be wrong by one level.
//!
//! ## A quoted document's links are not the page's
//!
//! A `rule` shows the CURRENT text of a fact, fetched from a
//! specification, and that text carries relative links of its own —
//! `../modules/vibe-mcp/PROP-027-mcp-packages.xml` means something beside
//! the file that says it and nothing at all beside the page that quotes
//! it. Those are read against the CITED document's address and written as
//! citations, which is the only honest reading of them: what the site
//! carries is documentation, and a specification is reached by asking the
//! resolver.
//!
//! ## Why a citation asks the resolver instead of writing the map itself
//!
//! The address map of `##SITE-MOUNT` is deterministic, so an island COULD
//! spell `spec://<group>/<name>/<document>` straight out as a path — and
//! did, into documentation the mount in front of it does not carry. The
//! pipeline cannot know what any particular mount holds: the public site
//! carries a library, a local reader carries exactly one package. The
//! resolver is the one address that knows, on both
//! (`##SEO-MANIFEST-AND-RESOLVER`, the plan's fork F-15), so the island
//! names it and hands it the citation whole.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-057#SITE-MOUNT");

/// The resolver's address under the base the documentation is served
/// under: a redirect page on the static site, a route in the local
/// reader, the same spelling in both.
pub const RESOLVER: &str = "resolve/";

/// Whose writing a link is — which is the whole of what a relative
/// address inside it means.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Author<'a> {
    /// Nobody's: the five inline conventions and no address map at all.
    /// This is the shape a caller uses to see what the vocabulary does.
    Verbatim,
    /// The page's own prose. A relative address is a path from the page's
    /// source file to another file of the same package.
    Page,
    /// A document quoted on the page, named by its `spec://` address. A
    /// relative address is a path from THAT document's file.
    Cited(&'a str),
}

/// The addresses one island writes.
#[derive(Debug, Clone, Copy)]
pub struct Links<'a> {
    /// The path the documentation is served under (`/doc/` on the site).
    /// Empty means the caller has nowhere to point, and a citation then
    /// gets no address at all rather than a made-up one.
    base: &'a str,
    author: Author<'a>,
}

impl<'a> Links<'a> {
    /// Links exactly as they were written.
    ///
    /// ```
    /// use vibe_doc::html::links::Links;
    ///
    /// assert_eq!(Links::verbatim().href("../glossary/index.xml").as_deref(), Some("../glossary/index.xml"));
    /// ```
    pub fn verbatim() -> Links<'static> {
        Links {
            base: "",
            author: Author::Verbatim,
        }
    }

    /// The links of a page's own prose, under the base it is served at.
    ///
    /// ```
    /// use vibe_doc::html::links::Links;
    ///
    /// let links = Links::page("/doc/");
    /// // A neighbouring page, as the site addresses it.
    /// assert_eq!(links.href("../glossary/index.xml#term").as_deref(), Some("../../glossary/index/#term"));
    /// assert_eq!(links.href("first-project.xml").as_deref(), Some("../first-project/"));
    /// // A file the edition carries beside its pages keeps its name.
    /// assert_eq!(links.href("../media/card.png").as_deref(), Some("../../media/card.png"));
    /// // A citation goes to the resolver, fragment and all.
    /// assert_eq!(
    ///     links.href("spec://org.demo/lib/guide#X").as_deref(),
    ///     Some("/doc/resolve/?uri=spec://org.demo/lib/guide%23X")
    /// );
    /// // The rest is left alone: the web, an anchor, an address already
    /// // written against the site's root.
    /// assert_eq!(links.href("https://vibevm.org/").as_deref(), Some("https://vibevm.org/"));
    /// assert_eq!(links.href("#p07").as_deref(), Some("#p07"));
    /// assert_eq!(links.href("/doc/x/").as_deref(), Some("/doc/x/"));
    /// ```
    pub fn page(base: &'a str) -> Links<'a> {
        Links {
            base,
            author: Author::Page,
        }
    }

    /// The links inside the text of the document at `uri`, quoted on a
    /// page of this documentation.
    ///
    /// ```
    /// use vibe_doc::html::links::Links;
    ///
    /// let links = Links::quoting("/doc/", "spec://org.demo/lib/modules/vibe-mcp/PROP-027#R");
    /// // Read against the CITED file, and written as the citation it is.
    /// assert_eq!(
    ///     links.href("../../common/PROP-024.xml#build").as_deref(),
    ///     Some("/doc/resolve/?uri=spec://org.demo/lib/common/PROP-024%23build")
    /// );
    /// // A file of a repository is not a document and has no address,
    /// // so the island writes none and the caller keeps the spelling.
    /// assert!(links.href("../../crates/vibe-core/src/manifest.rs").is_none());
    /// // Neither has a path that climbs out of the package it started in.
    /// assert!(links.href("../../../elsewhere/PROP-004.xml").is_none());
    /// ```
    pub fn quoting(base: &'a str, uri: &'a str) -> Links<'a> {
        Links {
            base,
            author: Author::Cited(uri),
        }
    }

    /// Where a link points on the site, or `None` when this build cannot
    /// place it.
    ///
    /// `None` is an answer and not a failure: an island writes no address
    /// it cannot stand behind, and the caller keeps the source spelling
    /// beside the text (as `data-address`) so a reader still sees what
    /// the author named.
    pub fn href(&self, target: &str) -> Option<String> {
        let author = match self.author {
            Author::Verbatim => return Some(target.to_owned()),
            other => other,
        };
        if target.starts_with("spec://") {
            return self.citation(target);
        }
        // Anything with a scheme, a network path or the site's own root
        // is already an address; the island is not asked to improve it.
        if has_scheme(target) || target.starts_with("//") || target.starts_with('/') {
            return Some(target.to_owned());
        }
        let (path, fragment) = split_fragment(target);
        match author {
            Author::Verbatim | Author::Page => Some(page_address(path, fragment)),
            Author::Cited(uri) => self.cited(uri, path, fragment),
        }
    }

    /// The address of one `spec://` citation: the resolver, asked.
    ///
    /// ```
    /// use vibe_doc::html::links::Links;
    ///
    /// let links = Links::page("/doc/");
    /// assert_eq!(
    ///     links.citation("spec://org.demo/lib@2.1.0/guide#X").as_deref(),
    ///     Some("/doc/resolve/?uri=spec://org.demo/lib@2.1.0/guide%23X")
    /// );
    /// // An undotted authority names no package, so it addresses no
    /// // document either — and gets no link rather than an invented one.
    /// assert!(links.citation("spec://vibevm/common/PROP-001#X").is_none());
    /// // With no base there is nowhere to point.
    /// assert!(Links::page("").citation("spec://org.demo/lib/guide").is_none());
    /// ```
    pub fn citation(&self, uri: &str) -> Option<String> {
        if self.base.is_empty() {
            return None;
        }
        let address = vibe_spec::SpecAddress::parse(uri).ok()?;
        match address.authority {
            vibe_spec::Authority::Package { .. } => {
                Some(format!("{}{RESOLVER}?uri={}", self.base, encode(uri)))
            }
            vibe_spec::Authority::Host(_) => None,
        }
    }

    /// A link inside a quoted document, read against that document.
    fn cited(&self, uri: &str, path: &str, fragment: &str) -> Option<String> {
        let (head, document) = split_address(uri)?;
        if path.is_empty() {
            // An anchor alone names a place in the document being quoted.
            return self.citation(&format!("{head}/{document}{fragment}"));
        }
        // Only a specification FILE is a document with an address; a path
        // to source code or to a file of the repository is neither.
        let stem = path.strip_suffix(".xml")?;
        let mut directory: Vec<&str> = document.split('/').collect();
        directory.pop();
        let resolved = walk(&directory, stem)?;
        self.citation(&format!("{head}/{resolved}{fragment}"))
    }
}

/// A relative address written beside a page's source file, as the site
/// addresses it: one level deeper, and a page's `.xml` traded for the
/// slash the address map ends in (`##SITE-TRAILING-SLASH`).
fn page_address(path: &str, fragment: &str) -> String {
    if path.is_empty() {
        return fragment.to_owned();
    }
    let placed = match path.strip_suffix(".xml") {
        Some(stem) => format!("{stem}/"),
        None => path.to_owned(),
    };
    format!("{}{fragment}", normalise(&format!("../{placed}")))
}

/// `spec://<group>/<name>[@<version>]` and the document behind it.
fn split_address(uri: &str) -> Option<(&str, &str)> {
    let uri = uri.split_once('#').map_or(uri, |(head, _)| head);
    let rest = uri.strip_prefix("spec://")?;
    let (_, after_group) = rest.split_once('/')?;
    let (_, document) = after_group.split_once('/')?;
    if document.is_empty() {
        return None;
    }
    Some((&uri[..uri.len() - document.len() - 1], document))
}

/// A relative path walked from a directory, or `None` when it climbs out
/// of the package it started in.
fn walk(directory: &[&str], relative: &str) -> Option<String> {
    let mut out: Vec<&str> = directory.to_vec();
    for part in relative.split('/') {
        match part {
            "" | "." => {}
            ".." => {
                out.pop()?;
            }
            name => out.push(name),
        }
    }
    if out.is_empty() {
        return None;
    }
    Some(out.join("/"))
}

/// Fold `.` and `<name>/..` out of a relative path, keeping the leading
/// climb — `../` in front of a path that already climbed is one level
/// more, not a mistake to straighten.
fn normalise(path: &str) -> String {
    let trailing = path.ends_with('/');
    let mut out: Vec<&str> = Vec::new();
    for part in path.split('/') {
        match part {
            "" | "." => {}
            ".." => match out.last() {
                Some(&last) if last != ".." => {
                    out.pop();
                }
                _ => out.push(".."),
            },
            name => out.push(name),
        }
    }
    let mut joined = out.join("/");
    if trailing && !joined.is_empty() {
        joined.push('/');
    }
    joined
}

/// The address and the fragment behind it, the `#` kept with the
/// fragment so an address without one rebuilds byte for byte.
fn split_fragment(target: &str) -> (&str, &str) {
    match target.find('#') {
        Some(at) => (&target[..at], &target[at..]),
        None => (target, ""),
    }
}

/// Does this target already carry a scheme of its own?
fn has_scheme(target: &str) -> bool {
    let Some(at) = target.find(':') else {
        return false;
    };
    let head = &target[..at];
    !head.is_empty()
        && head.starts_with(|c: char| c.is_ascii_alphabetic())
        && head
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '+' || c == '-' || c == '.')
}

/// Percent-encode a citation for the `uri` parameter.
///
/// The unreserved set plus the three characters a `spec://` address is
/// built out of — `:`, `/` and `@` — stay as they are, so the address is
/// still readable in a status bar; everything else is escaped, and the
/// one that must be is `#`, which a browser would otherwise read as the
/// fragment of the resolver's own page.
fn encode(uri: &str) -> String {
    let mut out = String::with_capacity(uri.len());
    for byte in uri.bytes() {
        match byte {
            b'A'..=b'Z'
            | b'a'..=b'z'
            | b'0'..=b'9'
            | b'-'
            | b'.'
            | b'_'
            | b'~'
            | b':'
            | b'/'
            | b'@' => out.push(char::from(byte)),
            _ => out.push_str(&format!("%{byte:02X}")),
        }
    }
    out
}

#[cfg(test)]
mod tests;
