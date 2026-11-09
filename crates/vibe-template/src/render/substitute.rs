//! Substitution inside string values only — never in a key, never in a
//! comment, never in a non-string scalar (PROP-075
//! `#TEMPLATE-VARIABLE-GRAMMAR`).
//!
//! The one type rule: a string holding exactly one variable and nothing
//! else renders as that variable's own type, so a numeric or boolean field
//! can vary. A string that mixes a variable with other text always renders
//! as a string. There is no cast syntax.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-075#TEMPLATE-VARIABLE-GRAMMAR");

use toml_edit::{Item, Table, Value};
use vibe_core::manifest::{Scalar, TemplateSection};

use crate::error::{RenderError, RenderErrorKind};
use crate::source::{DigestTable, VersionRecord};

use super::SKELETON_DIGEST;
use super::grammar::{Piece, Var, scan};

/// What a substitution can see: the version record, the digests (absent in
/// a skeleton render), the declaration, and the installer-option row's own
/// `platform` / `arch` when there is one.
#[derive(Clone, Copy)]
pub(crate) struct Context<'a> {
    pub record: &'a VersionRecord,
    pub digests: Option<&'a DigestTable>,
    pub section: &'a TemplateSection,
    pub platform: Option<&'a str>,
    pub arch: Option<&'a str>,
}

/// One rendered variable, with the type the coercion rule may adopt.
enum Resolved {
    Text(String),
    Integer(i64),
    Bool(bool),
}

impl Resolved {
    fn as_text(&self) -> String {
        match self {
            Self::Text(text) => text.clone(),
            Self::Integer(value) => value.to_string(),
            Self::Bool(value) => value.to_string(),
        }
    }
}

pub(crate) fn substitute_item(
    item: &mut Item,
    path: &str,
    context: &Context<'_>,
) -> Result<(), RenderError> {
    match item {
        Item::Value(value) => substitute_value(value, path, context),
        Item::Table(table) => substitute_table(table, path, context),
        Item::ArrayOfTables(rows) => {
            for (index, table) in rows.iter_mut().enumerate() {
                substitute_table(table, &format!("{path}[{index}]"), context)?;
            }
            Ok(())
        }
        Item::None => Ok(()),
    }
}

pub(crate) fn substitute_table(
    table: &mut Table,
    path: &str,
    context: &Context<'_>,
) -> Result<(), RenderError> {
    for (key, item) in table.iter_mut() {
        let child = join(path, key.get());
        substitute_item(item, &child, context)?;
    }
    Ok(())
}

fn substitute_value(
    value: &mut Value,
    path: &str,
    context: &Context<'_>,
) -> Result<(), RenderError> {
    match value {
        Value::String(text) => {
            let source = text.value().clone();
            if !source.contains(['{', '}']) {
                return Ok(());
            }
            let pieces =
                scan(&source, false).map_err(|reason| RenderError::grammar(path, reason))?;
            let mut replacement = render_pieces(&pieces, path, context)?;
            let Some(replacement) = replacement.take() else {
                // Only literal braces: the scan unescaped them.
                let decor = value.decor().clone();
                let mut plain = Value::from(literal_only(&pieces));
                *plain.decor_mut() = decor;
                *value = plain;
                return Ok(());
            };
            let decor = value.decor().clone();
            let mut rendered = match replacement {
                Resolved::Text(text) => Value::from(text),
                Resolved::Integer(number) => Value::from(number),
                Resolved::Bool(flag) => Value::from(flag),
            };
            *rendered.decor_mut() = decor;
            *value = rendered;
            Ok(())
        }
        Value::Array(array) => {
            for (index, element) in array.iter_mut().enumerate() {
                substitute_value(element, &format!("{path}[{index}]"), context)?;
            }
            Ok(())
        }
        Value::InlineTable(table) => {
            for (key, element) in table.iter_mut() {
                let child = join(path, key.get());
                substitute_value(element, &child, context)?;
            }
            Ok(())
        }
        _ => Ok(()),
    }
}

/// The whole string, with the one-variable coercion applied. `None` means
/// the string held no variable at all.
fn render_pieces(
    pieces: &[Piece],
    path: &str,
    context: &Context<'_>,
) -> Result<Option<Resolved>, RenderError> {
    if !pieces.iter().any(|piece| matches!(piece, Piece::Var(_))) {
        return Ok(None);
    }
    if let [Piece::Var(var)] = pieces {
        return Ok(Some(resolve(var, path, context)?));
    }
    let mut text = String::new();
    for piece in pieces {
        match piece {
            Piece::Literal(literal) => text.push_str(literal),
            Piece::Var(var) => text.push_str(&resolve(var, path, context)?.as_text()),
        }
    }
    Ok(Some(Resolved::Text(text)))
}

fn literal_only(pieces: &[Piece]) -> String {
    pieces
        .iter()
        .map(|piece| match piece {
            Piece::Literal(literal) => literal.as_str(),
            Piece::Var(_) => "",
        })
        .collect()
}

fn resolve(var: &Var, path: &str, context: &Context<'_>) -> Result<Resolved, RenderError> {
    let record = context.record;
    match var {
        Var::Version => Ok(Resolved::Text(record.version.to_string())),
        Var::VersionRaw => Ok(Resolved::Text(record.raw.clone())),
        Var::Major => Ok(Resolved::Text(record.version.major.to_string())),
        Var::Minor => Ok(Resolved::Text(record.version.minor.to_string())),
        Var::Patch => Ok(Resolved::Text(record.version.patch.to_string())),
        Var::Prerelease => Ok(Resolved::Text(record.version.pre.to_string())),
        Var::Platform | Var::Arch => {
            let (bound, name) = match var {
                Var::Platform => (context.platform, "platform"),
                _ => (context.arch, "arch"),
            };
            bound.map(|value| Resolved::Text(value.to_string())).ok_or_else(|| {
                RenderError::grammar(
                    path,
                    format!(
                        "`{{{name}}}` is bound to an [[installer_options]] row's own `{name}` and \
                         there is nothing to bind it to here"
                    ),
                )
            })
        }
        Var::Meta(dotted) => match record.meta.get(dotted.as_str()) {
            Some(Scalar::Bool(flag)) => Ok(Resolved::Bool(*flag)),
            Some(Scalar::Integer(number)) => Ok(Resolved::Integer(*number)),
            Some(Scalar::String(text)) => Ok(Resolved::Text(text.clone())),
            None if record.is_composite(dotted) => Err(RenderError::grammar(
                path,
                format!(
                    "`{{meta.{dotted}}}` resolves to an array or a table in the version record of \
                     `{}`, and a variable renders a string, an integer or a boolean",
                    record.raw
                ),
            )),
            None => Err(RenderError::grammar(
                path,
                format!(
                    "`{{meta.{dotted}}}` names no field of the version record of `{}`",
                    record.raw
                ),
            )),
        },
        Var::Sha256(argument) | Var::Size(argument) => resolve_digest(var, argument, path, context),
    }
}

fn resolve_digest(
    var: &Var,
    argument: &[Piece],
    path: &str,
    context: &Context<'_>,
) -> Result<Resolved, RenderError> {
    let wants_size = matches!(var, Var::Size(_));
    if !is_digest_field(path) {
        return Err(RenderError::grammar(
            path,
            format!(
                "`{{{}:…}}` is a digest variable, legal only in the digest-bearing fields of an \
                 installer option — `sha256` and `size` of a step, of `installation` and of a \
                 `sources` row",
                var.name()
            ),
        ));
    }
    if wants_size && !context.section.has_sizes() {
        return Err(RenderError::grammar(
            path,
            "`{size:…}` needs a digest source that carries sizes — only \
             `[template.digests] kind = \"json\"` with a `size_field` does"
                .to_string(),
        ));
    }
    let mut file = String::new();
    for piece in argument {
        match piece {
            Piece::Literal(literal) => file.push_str(literal),
            Piece::Var(inner) => file.push_str(&resolve(inner, path, context)?.as_text()),
        }
    }
    let Some(digests) = context.digests else {
        return Ok(if wants_size {
            Resolved::Integer(0)
        } else {
            Resolved::Text(SKELETON_DIGEST.to_string())
        });
    };
    let missing = |what: &str| {
        let mut error = RenderError::new(
            RenderErrorKind::MissingDigest,
            path,
            format!(
                "the digest document of `{}` names no {what} for `{file}`",
                context.record.raw
            ),
        );
        error.subject = Some(file.clone());
        error
    };
    let entry = digests.get(&file).ok_or_else(|| missing("digest"))?;
    if wants_size {
        entry
            .size
            .map(|size| Resolved::Integer(i64::try_from(size).unwrap_or(i64::MAX)))
            .ok_or_else(|| missing("size"))
    } else {
        Ok(Resolved::Text(entry.sha256.clone()))
    }
}

/// The digest-bearing fields: the `sha256` and `size` keys inside an
/// `[[installer_options]]` row. Every other field refuses a digest
/// variable, which is what keeps a skeleton render valid (PROP-075
/// `#TEMPLATE-TWO-STAGE`).
fn is_digest_field(path: &str) -> bool {
    path.starts_with("installer_options[")
        && matches!(
            path.rsplit('.').next().unwrap_or_default(),
            "sha256" | "size"
        )
}

fn join(path: &str, key: &str) -> String {
    if path.is_empty() {
        key.to_string()
    } else {
        format!("{path}.{key}")
    }
}
