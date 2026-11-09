//! The refusals that keep `[package] template` and the `[template]` table
//! in agreement (PROP-075 `#TEMPLATE-SPELLING`).

use specmark::spec;

use crate::error::Result;
use crate::manifest::{FieldError, FieldPath};

use super::Manifest;

impl Manifest {
    /// Whether this document is a template manifest: `[package] template =
    /// true`, the only thing that sets the property (PROP-075
    /// `#TEMPLATE-FLAG`).
    pub fn is_template(&self) -> bool {
        self.package
            .as_ref()
            .is_some_and(|package| package.template)
    }

    /// The flag and the table must agree, and only an application may be a
    /// template today. Every refusal names its TOML path.
    #[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-075#TEMPLATE-SPELLING")]
    pub(super) fn validate_template(&self) -> Result<()> {
        let at = FieldPath::root().table("template");
        let flag = FieldPath::root().table("package").field("template");
        // `is_template` is `[package].template`, so the flagged arm always
        // has a `[package]`; the pattern says so instead of asserting it.
        match (&self.template, self.package.as_ref()) {
            (Some(section), Some(package)) if package.template => {
                section.validate(&at)?;
                if package.kind != crate::PackageKind::App {
                    return Err(FieldError::new(
                        flag,
                        format!(
                            "[package].template is admitted only on kind = \"app\" (this \
                             manifest is kind = \"{}\") — the surface widens later without a \
                             format change",
                            package.kind
                        ),
                    )
                    .into());
                }
                Ok(())
            }
            (Some(_), _) => Err(FieldError::new(
                at,
                "[template] without `[package] template = true` — the table and the flag cannot \
                 disagree, and a template never infers itself from a version spelling or a \
                 repository name (fix: write `template = true` under [package], or delete \
                 [template])"
                    .to_string(),
            )
            .into()),
            (None, Some(package)) if package.template => Err(FieldError::new(
                flag,
                "`[package] template = true` without a [template] table — a template must say \
                 where its versions and digests come from (fix: add [template.versions] and \
                 [template.digests], or drop the flag)"
                    .to_string(),
            )
            .into()),
            (None, _) => Ok(()),
        }
    }
}

/// `template_optional` is a template-only key: in an ordinary manifest it
/// would silently mean nothing, so it is refused by name.
pub(super) fn refuse_stray_optional(
    option: &crate::manifest::InstallerOptionDecl,
    template: bool,
    at: &FieldPath,
) -> Result<()> {
    if !option.template_optional || template {
        return Ok(());
    }
    Err(FieldError::new(
        at.field("template_optional"),
        format!(
            "[[installer_options]] `{}` declares template_optional, which applies only to a \
             template manifest (fix: write `template = true` under [package] and add a \
             [template] table, or drop the key)",
            option.id
        ),
    )
    .into())
}
