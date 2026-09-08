//! Kind-specific validation for the full compiler-pass manifest declaration.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#PASS-TIER-LAW");

use super::{ExtensionPass, ExtensionPassKind, PASS_TIER_LAW};

impl ExtensionPass {
    /// Validate the executable field matrix without resolving runtime names.
    pub(super) fn validate(&self, extension: &str) -> Result<(), String> {
        let Self {
            kind,
            level,
            from,
            to,
            after,
            before,
            replace,
            formats,
            artifact,
        } = self;
        match kind {
            ExtensionPassKind::Transform => {
                if level.is_none() {
                    return Err(error(extension, "transform", "requires field `level`"));
                }
                forbid(
                    extension,
                    "transform",
                    &[
                        ("from", from.is_some()),
                        ("to", to.is_some()),
                        ("replace", replace.is_some()),
                        ("formats", formats.is_some()),
                        ("artifact", artifact.is_some()),
                    ],
                )?;
                exactly_one(extension, "transform", after, before)
            }
            ExtensionPassKind::Lowering => {
                forbid(
                    extension,
                    "lowering",
                    &[("level", level.is_some()), ("formats", formats.is_some())],
                )?;
                for (field, value) in [
                    ("after", after.as_deref()),
                    ("before", before.as_deref()),
                    ("replace", replace.as_deref()),
                    ("artifact", artifact.as_deref()),
                ] {
                    if let Some(value) = value {
                        nonblank(extension, "lowering", field, value)?;
                    }
                }
                if replace.is_some() {
                    forbid(
                        extension,
                        "lowering replacement",
                        &[
                            ("from", from.is_some()),
                            ("to", to.is_some()),
                            ("after", after.is_some()),
                            ("before", before.is_some()),
                        ],
                    )?;
                    if artifact.is_none() {
                        return Err(error(
                            extension,
                            "lowering replacement",
                            "requires field `artifact`",
                        ));
                    }
                    return Ok(());
                }
                if from.is_none() || to.is_none() {
                    return Err(error(
                        extension,
                        "lowering augment",
                        "requires fields `from` and `to`",
                    ));
                }
                exactly_one(extension, "lowering augment", after, before)
            }
            ExtensionPassKind::Frontend => {
                forbid(
                    extension,
                    "frontend",
                    &[
                        ("level", level.is_some()),
                        ("from", from.is_some()),
                        ("to", to.is_some()),
                        ("after", after.is_some()),
                        ("before", before.is_some()),
                        ("replace", replace.is_some()),
                        ("artifact", artifact.is_some()),
                    ],
                )?;
                let formats = formats
                    .as_ref()
                    .ok_or_else(|| error(extension, "frontend", "requires field `formats`"))?;
                if formats.is_empty() {
                    return Err(error(extension, "frontend", "field `formats` is empty"));
                }
                for (index, format) in formats.iter().enumerate() {
                    if format.trim().is_empty() {
                        return Err(error(
                            extension,
                            "frontend",
                            &format!("field `formats[{index}]` must not be blank"),
                        ));
                    }
                }
                Ok(())
            }
            ExtensionPassKind::Backend => {
                forbid(
                    extension,
                    "backend",
                    &[
                        ("level", level.is_some()),
                        ("from", from.is_some()),
                        ("to", to.is_some()),
                        ("after", after.is_some()),
                        ("before", before.is_some()),
                        ("replace", replace.is_some()),
                        ("formats", formats.is_some()),
                    ],
                )?;
                let artifact = artifact
                    .as_deref()
                    .ok_or_else(|| error(extension, "backend", "requires field `artifact`"))?;
                nonblank(extension, "backend", "artifact", artifact)
            }
        }
    }
}

fn exactly_one(
    extension: &str,
    kind: &str,
    after: &Option<String>,
    before: &Option<String>,
) -> Result<(), String> {
    for (field, value) in [("after", after), ("before", before)] {
        if let Some(value) = value.as_deref() {
            nonblank(extension, kind, field, value)?;
        }
    }
    if after.is_some() ^ before.is_some() {
        Ok(())
    } else {
        Err(error(
            extension,
            kind,
            "requires exactly one of fields `after` or `before`",
        ))
    }
}

fn forbid(extension: &str, kind: &str, fields: &[(&str, bool)]) -> Result<(), String> {
    if let Some((field, _)) = fields.iter().find(|(_, present)| *present) {
        Err(error(extension, kind, &format!("forbids field `{field}`")))
    } else {
        Ok(())
    }
}

fn nonblank(extension: &str, kind: &str, field: &str, value: &str) -> Result<(), String> {
    if value.trim().is_empty() {
        Err(error(
            extension,
            kind,
            &format!("field `{field}` must not be blank"),
        ))
    } else {
        Ok(())
    }
}

fn error(extension: &str, kind: &str, detail: &str) -> String {
    format!("[[extension]] `{extension}` pass kind `{kind}` {detail} ({PASS_TIER_LAW})")
}
