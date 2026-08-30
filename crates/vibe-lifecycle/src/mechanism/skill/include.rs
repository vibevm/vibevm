//! `vibe:include` — §6.1's one door from a multi-file source to a static
//! distributable.
//!
//! > "A multi-file source is static-buildable only through explicit
//! > `vibe:include` directives in `SKILL.md`; every directive names one
//! > declared textual resource and is replaced deterministically with
//! > visible origin/hash framing. Every declared extra resource must be
//! > consumed exactly once or the build refuses."
//!
//! Four laws follow from that paragraph and all four are here:
//!
//! 1. a directive is a WHOLE line — `<!-- vibe:include NAME -->` — and a
//!    line that mentions `vibe:include` in any other shape refuses rather
//!    than surviving into the output as text. A malformed directive left
//!    in place is exactly the failure §6.1 forbids by name: a skill that
//!    claims to be static while a resource was silently dropped;
//! 2. `NAME` must be one DECLARED resource. An unknown name is the
//!    "unresolved sibling reference" refusal;
//! 3. each declared resource is consumed EXACTLY once — twice refuses,
//!    zero times refuses;
//! 4. the replacement is framed VISIBLY with the origin and the digest of
//!    the bytes that were really inlined, and the framing is fixed text,
//!    so two runs over one source produce identical bytes.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE");

use std::collections::BTreeMap;

use crate::mechanism::MechanismError;
use crate::mechanism::error::preview;

/// The token a directive line is recognised by.
const TOKEN: &str = "vibe:include";

/// One resource as the include pass needs it: the identity a directive
/// names, where it came from, and the exact bytes.
pub(crate) struct Inlinable<'a> {
    pub(crate) name: &'a str,
    pub(crate) origin: &'a str,
    pub(crate) digest: &'a str,
    pub(crate) text: &'a str,
}

/// Replace every directive in one body, proving the exactly-once law.
///
/// Returns the rendered body; the caller already holds the resources and
/// therefore already knows what was consumed.
pub(crate) fn render(
    target: &str,
    body: &str,
    resources: &[Inlinable<'_>],
) -> Result<String, MechanismError> {
    let mut consumed: BTreeMap<&str, usize> = resources
        .iter()
        .map(|resource| (resource.name, 0_usize))
        .collect();
    let mut rendered = String::with_capacity(body.len());
    for (number, line) in body.split_inclusive('\n').enumerate() {
        let bare = line.trim_end_matches(['\n', '\r']);
        let trimmed = bare.trim();
        if !trimmed.contains(TOKEN) {
            rendered.push_str(line);
            continue;
        }
        let name = directive_name(trimmed).ok_or_else(|| MechanismError::IncludeMalformed {
            target: target.to_owned(),
            line: number + 1,
            value: preview(trimmed),
        })?;
        let resource = resources
            .iter()
            .find(|resource| resource.name == name)
            .ok_or_else(|| MechanismError::IncludeUnknown {
                target: target.to_owned(),
                name: preview(name),
                declared: declared_list(resources),
            })?;
        let count = consumed.entry(resource.name).or_insert(0);
        *count += 1;
        if *count > 1 {
            return Err(MechanismError::IncludeDuplicate {
                target: target.to_owned(),
                name: resource.name.to_owned(),
                line: number + 1,
            });
        }
        frame(&mut rendered, resource);
    }
    let unconsumed: Vec<&str> = resources
        .iter()
        .filter(|resource| consumed.get(resource.name).copied().unwrap_or(0) == 0)
        .map(|resource| resource.name)
        .collect();
    if !unconsumed.is_empty() {
        return Err(MechanismError::ResourceUnconsumed {
            target: target.to_owned(),
            names: unconsumed.join(", "),
        });
    }
    Ok(rendered)
}

/// The name one well-formed directive line carries.
///
/// Strict on purpose: the opening and closing comment markers, the token,
/// exactly one argument, nothing else on the line.
fn directive_name(trimmed: &str) -> Option<&str> {
    let inner = trimmed.strip_prefix("<!--")?.strip_suffix("-->")?.trim();
    let rest = inner.strip_prefix(TOKEN)?;
    if !rest.starts_with(char::is_whitespace) {
        return None;
    }
    let mut words = rest.split_whitespace();
    let name = words.next()?;
    if words.next().is_some() || name.is_empty() {
        return None;
    }
    Some(name)
}

/// The visible origin/hash framing one inclusion is replaced with.
///
/// Deterministic by construction: fixed words, the declared identity, the
/// project-relative origin and the digest of the inlined bytes. No clock,
/// no absolute path, no counter — two runs over one source are byte-equal.
fn frame(rendered: &mut String, resource: &Inlinable<'_>) {
    rendered.push_str(&format!(
        "<!-- vibe:included name=\"{}\" origin=\"{}\" sha256=\"{}\" -->\n",
        resource.name, resource.origin, resource.digest,
    ));
    rendered.push_str(resource.text);
    if !resource.text.ends_with('\n') {
        rendered.push('\n');
    }
    rendered.push_str(&format!(
        "<!-- vibe:end name=\"{}\" sha256=\"{}\" -->\n",
        resource.name, resource.digest,
    ));
}

/// The declared names a refusal lists, bounded.
fn declared_list(resources: &[Inlinable<'_>]) -> String {
    if resources.is_empty() {
        return "none declared".to_owned();
    }
    resources
        .iter()
        .map(|resource| resource.name)
        .collect::<Vec<_>>()
        .join(", ")
}
