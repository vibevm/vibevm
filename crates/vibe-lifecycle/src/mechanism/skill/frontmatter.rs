//! Agent Skills frontmatter — read and validated LOCALLY and
//! structurally, which is the whole claim §6.1 makes for it.
//!
//! Nothing here fetches a schema, consults a client, or pretends to be a
//! YAML implementation. It reads the SUBSET real skill frontmatter is
//! written in — a fenced block of `key: value` entries, inline or block
//! sequences, and one level of nested mapping — and refuses anything
//! outside it by name, so a document this cell cannot fully understand is
//! never half-understood. The exact member laws it enforces are listed on
//! [`validate`], and that list is the honest answer to "what does it
//! validate".
//!
//! The block is never rewritten. §6.1 asks for one UTF-8 `SKILL.md` whose
//! includes are replaced deterministically; re-emitting an author's YAML
//! through a serializer would change bytes nobody asked to change, so the
//! output carries the original block verbatim.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE");

use crate::mechanism::MechanismError;
use crate::mechanism::error::preview;

/// The fence that opens and closes the block.
const FENCE: &str = "---";

/// The longest a `description` may be before it stops being one.
const DESCRIPTION_CAP: usize = 1024;

/// The longest a `name` may be — the same 64-character bound the portable
/// id grammar uses everywhere else in this system.
const NAME_CAP: usize = 64;

/// One frontmatter value, in the subset this reader speaks.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Value {
    Scalar(String),
    Sequence(Vec<String>),
    Mapping(Vec<(String, String)>),
}

impl Value {
    fn kind(&self) -> &'static str {
        match self {
            Self::Scalar(_) => "a scalar",
            Self::Sequence(_) => "a sequence",
            Self::Mapping(_) => "a mapping",
        }
    }
}

/// One document split at its frontmatter fence, with the members §6.1
/// names already proven.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Frontmatter<'a> {
    /// The block between the fences, byte-for-byte as authored.
    pub(crate) block: &'a str,
    /// Everything after the closing fence.
    pub(crate) body: &'a str,
    /// The validated `name` member.
    pub(crate) name: String,
}

/// Split one document at its frontmatter fence and validate the block.
///
/// The member laws, in full — this is what "validates Agent Skills
/// frontmatter" means in this engine:
///
/// * the document OPENS with a `---` fence line and the block is closed by
///   another one;
/// * every entry is unique, and its key is a non-blank token without
///   whitespace;
/// * `name` — REQUIRED scalar, `[a-z0-9]([a-z0-9-]*[a-z0-9])?`, at most 64
///   characters;
/// * `description` — REQUIRED scalar, non-blank once trimmed, free of
///   control bytes, at most 1024 characters;
/// * `license` — OPTIONAL scalar, non-blank;
/// * `allowed-tools` — OPTIONAL scalar or sequence of non-blank scalars;
/// * `metadata` — OPTIONAL mapping of non-blank scalars;
/// * any other member is PRESERVED and not judged: the Agent Skills
///   vocabulary is not this engine's to close.
pub(crate) fn parse<'a>(
    target: &str,
    document: &'a str,
) -> Result<Frontmatter<'a>, MechanismError> {
    let refuse = |member: &str, reason: String| MechanismError::Frontmatter {
        target: target.to_owned(),
        member: member.to_owned(),
        reason,
    };
    let rest = document
        .strip_prefix("---\n")
        .or_else(|| document.strip_prefix("---\r\n"))
        .ok_or_else(|| {
            refuse(
                "<block>",
                "the document does not open with a `---` frontmatter fence".to_owned(),
            )
        })?;
    let (block, body) = split_at_close(rest).ok_or_else(|| {
        refuse(
            "<block>",
            "the frontmatter block is never closed by a `---` line".to_owned(),
        )
    })?;
    let entries = read_entries(target, block)?;
    // The validated identity is RETURNED rather than looked up again: a
    // second lookup would be a second place that has to agree about which
    // member the identity comes from.
    let name = validate(target, &entries)?;
    Ok(Frontmatter { block, body, name })
}

/// The block up to the closing fence, and the body after it.
fn split_at_close(rest: &str) -> Option<(&str, &str)> {
    let mut offset = 0;
    for line in rest.split_inclusive('\n') {
        if line.trim_end_matches(['\r', '\n']).trim_end() == FENCE {
            let block = &rest[..offset];
            let body = &rest[offset + line.len()..];
            return Some((block, body));
        }
        offset += line.len();
    }
    None
}

/// Read the block into entries, refusing every shape outside the subset.
fn read_entries(target: &str, block: &str) -> Result<Vec<(String, Value)>, MechanismError> {
    let refuse = |member: &str, reason: String| MechanismError::Frontmatter {
        target: target.to_owned(),
        member: member.to_owned(),
        reason,
    };
    let mut entries: Vec<(String, Value)> = Vec::new();
    let mut lines = block.lines().peekable();
    while let Some(line) = lines.next() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        if line.starts_with(char::is_whitespace) {
            return Err(refuse(
                "<block>",
                format!("`{}` is indented under no member", preview(trimmed)),
            ));
        }
        let (key, rest) = line.split_once(':').ok_or_else(|| {
            refuse(
                "<block>",
                format!("`{}` is not a `key: value` entry", preview(trimmed)),
            )
        })?;
        let key = key.trim();
        if key.is_empty() || key.split_whitespace().count() != 1 {
            return Err(refuse(
                "<block>",
                format!("`{}` is not a usable member name", preview(key)),
            ));
        }
        if entries.iter().any(|(known, _)| known == key) {
            return Err(refuse(key, "declared twice in one block".to_owned()));
        }
        let inline = rest.trim();
        let value = if inline.is_empty() {
            nested(target, key, &mut lines)?
        } else if let Some(items) = inline_sequence(inline) {
            Value::Sequence(items)
        } else {
            Value::Scalar(scalar(inline))
        };
        entries.push((key.to_owned(), value));
    }
    Ok(entries)
}

/// The indented block under one member: a sequence or a one-level mapping.
fn nested(
    target: &str,
    key: &str,
    lines: &mut std::iter::Peekable<std::str::Lines<'_>>,
) -> Result<Value, MechanismError> {
    let refuse = |reason: String| MechanismError::Frontmatter {
        target: target.to_owned(),
        member: key.to_owned(),
        reason,
    };
    let mut items: Vec<String> = Vec::new();
    let mut pairs: Vec<(String, String)> = Vec::new();
    while let Some(peeked) = lines.peek() {
        let trimmed = peeked.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            lines.next();
            continue;
        }
        if !peeked.starts_with(char::is_whitespace) {
            break;
        }
        let line = match lines.next() {
            Some(line) => line.trim(),
            None => break,
        };
        if let Some(item) = line.strip_prefix("- ") {
            if !pairs.is_empty() {
                return Err(refuse(
                    "mixes sequence items and mapping entries".to_owned(),
                ));
            }
            items.push(scalar(item.trim()));
            continue;
        }
        let (nested_key, rest) = line.split_once(':').ok_or_else(|| {
            refuse(format!(
                "`{}` is neither a `- item` nor a `key: value` entry",
                preview(line)
            ))
        })?;
        if !items.is_empty() {
            return Err(refuse(
                "mixes sequence items and mapping entries".to_owned(),
            ));
        }
        if rest.trim().is_empty() {
            return Err(refuse(format!(
                "`{}` nests deeper than this reader speaks; keep frontmatter to one level of \
                 mapping",
                preview(nested_key.trim())
            )));
        }
        pairs.push((nested_key.trim().to_owned(), scalar(rest.trim())));
    }
    if !items.is_empty() {
        return Ok(Value::Sequence(items));
    }
    if !pairs.is_empty() {
        return Ok(Value::Mapping(pairs));
    }
    Ok(Value::Scalar(String::new()))
}

/// `[a, b]`, or nothing.
fn inline_sequence(value: &str) -> Option<Vec<String>> {
    let inner = value.strip_prefix('[')?.strip_suffix(']')?;
    if inner.trim().is_empty() {
        return Some(Vec::new());
    }
    Some(inner.split(',').map(|item| scalar(item.trim())).collect())
}

/// One scalar, unquoted in the two spellings YAML admits.
fn scalar(value: &str) -> String {
    if let Some(inner) = value.strip_prefix('"').and_then(|v| v.strip_suffix('"')) {
        return inner.replace("\\\"", "\"").replace("\\\\", "\\");
    }
    if let Some(inner) = value.strip_prefix('\'').and_then(|v| v.strip_suffix('\'')) {
        return inner.replace("''", "'");
    }
    value.to_owned()
}

/// The member laws listed on [`parse`], returning the validated identity.
fn validate(target: &str, entries: &[(String, Value)]) -> Result<String, MechanismError> {
    let refuse = |member: &str, reason: String| MechanismError::Frontmatter {
        target: target.to_owned(),
        member: member.to_owned(),
        reason,
    };
    let scalar_of = |member: &str| -> Result<Option<&String>, MechanismError> {
        match entries.iter().find(|(key, _)| key == member) {
            None => Ok(None),
            Some((_, Value::Scalar(value))) => Ok(Some(value)),
            Some((_, other)) => Err(refuse(
                member,
                format!("expected a scalar, found {}", other.kind()),
            )),
        }
    };
    let name = scalar_of("name")?.ok_or_else(|| {
        refuse(
            "name",
            "required; an Agent Skill names itself in its frontmatter".to_owned(),
        )
    })?;
    if !is_skill_name(name) {
        return Err(refuse(
            "name",
            format!(
                "`{}` is not a skill name; use lowercase letters, digits and inner hyphens, at \
                 most {NAME_CAP} characters",
                preview(name)
            ),
        ));
    }
    let description = scalar_of("description")?.ok_or_else(|| {
        refuse(
            "description",
            "required; an Agent Skill describes when it applies".to_owned(),
        )
    })?;
    if description.trim().is_empty() || description.chars().any(char::is_control) {
        return Err(refuse(
            "description",
            "must be non-blank and free of control bytes".to_owned(),
        ));
    }
    if description.chars().count() > DESCRIPTION_CAP {
        return Err(refuse(
            "description",
            format!("longer than the {DESCRIPTION_CAP}-character bound"),
        ));
    }
    if let Some(license) = scalar_of("license")?
        && license.trim().is_empty()
    {
        return Err(refuse("license", "present and blank".to_owned()));
    }
    match entries.iter().find(|(key, _)| key == "allowed-tools") {
        None => {}
        Some((_, Value::Scalar(value))) if !value.trim().is_empty() => {}
        Some((_, Value::Sequence(items))) if items.iter().all(|item| !item.trim().is_empty()) => {}
        Some((_, other)) => {
            return Err(refuse(
                "allowed-tools",
                format!(
                    "expected a non-blank scalar or a sequence of non-blank scalars, found {}",
                    other.kind()
                ),
            ));
        }
    }
    match entries.iter().find(|(key, _)| key == "metadata") {
        None => {}
        Some((_, Value::Mapping(pairs)))
            if pairs
                .iter()
                .all(|(key, value)| !key.is_empty() && !value.trim().is_empty()) => {}
        Some((_, other)) => {
            return Err(refuse(
                "metadata",
                format!(
                    "expected a mapping of non-blank scalars, found {}",
                    other.kind()
                ),
            ));
        }
    }
    Ok(name.clone())
}

/// The Agent Skills name grammar, which is also the directory name.
fn is_skill_name(value: &str) -> bool {
    let bytes = value.as_bytes();
    if bytes.is_empty() || bytes.len() > NAME_CAP {
        return false;
    }
    if !bytes[0].is_ascii_lowercase() && !bytes[0].is_ascii_digit() {
        return false;
    }
    let last = bytes[bytes.len() - 1];
    if !last.is_ascii_lowercase() && !last.is_ascii_digit() {
        return false;
    }
    bytes
        .iter()
        .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || *byte == b'-')
}
