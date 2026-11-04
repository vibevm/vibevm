//! What a `remap`-kind package may carry, and what every manifest's
//! `[[remap]]` rows must look like (`PROP-066#package`).
//!
//! Two rules, and they pull in opposite directions on purpose. Any
//! vibepack may carry rows, so the row grammar is checked on every
//! manifest — `vibe check` validates them exactly as the Locator parses
//! them, which is why the check calls the same `into_row` and not a second
//! reading of the same TOML. And a package that declares
//! `kind = "remap"` carries **nothing else**: the offer is policy, it is
//! installed into a user's home, and a package that could also ship a
//! binary or a boot snippet would be a way to run code by being accepted
//! as a redirection.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-066#package");

use crate::PackageKind;
use crate::error::{Error, Result};
use crate::remap::{Layer, Provenance};

use super::super::Manifest;

/// Every table a `remap`-kind package is allowed to carry, as a reader of
/// the manifest spells it. Anything else is named in the refusal.
const ALLOWED: [&str; 6] = [
    "[package]",
    "[compatibility]",
    "[configuration] (data only)",
    "[remap]",
    "[[remap]]",
    "[policy]",
];

impl Manifest {
    /// The remap laws: the row grammar on every manifest, and the
    /// closed table set of a `remap`-kind package.
    pub(in crate::manifest::document) fn validate_remap(&self) -> Result<()> {
        self.validate_remap_rows()?;
        let is_remap = self
            .package
            .as_ref()
            .is_some_and(|meta| meta.kind == PackageKind::Remap);
        if !is_remap {
            return Ok(());
        }
        if let Some(offender) = self.first_forbidden_in_a_remap_package() {
            return Err(Error::InvalidManifest {
                reason: format!(
                    "a `remap`-kind package may not declare {offender} — a remap package \
                     carries where artifacts may come from and nothing executable, because \
                     accepting a redirection must never be a way to run someone's code \
                     (violates spec://org.vibevm.core/vibevm/common/PROP-066#package; \
                      fix: drop {offender}, or publish the rest as its own package of the \
                      kind it really is; a `remap` package may carry only {})",
                    ALLOWED.join(", ")
                ),
            });
        }
        if self.remap.rows.is_empty() {
            return Err(Error::InvalidManifest {
                reason: "a `remap`-kind package must declare at least one `[[remap]]` row — \
                         the rows ARE the offer, and a package that offers nothing has \
                         nothing for a user to accept \
                         (violates spec://org.vibevm.core/vibevm/common/PROP-066#package; \
                          fix: add `[[remap]]` rows, or set [package] kind to what this \
                          package really is)"
                    .to_string(),
            });
        }
        Ok(())
    }

    /// Validate the `[[remap]]` rows of any manifest exactly as the
    /// Locator parses them (`PROP-066#row`): one key, one target, and a
    /// position-naming error otherwise.
    fn validate_remap_rows(&self) -> Result<()> {
        for (index, row) in self.remap.rows.iter().enumerate() {
            row.clone()
                .into_row(Provenance::new(Layer::Local, Manifest::FILENAME, index))
                .map_err(|error| Error::InvalidManifest {
                    reason: error.to_string(),
                })?;
        }
        Ok(())
    }

    /// The first table a `remap`-kind package declared and may not, named
    /// as the manifest spells it.
    ///
    /// The list is the manifest's own table set minus [`ALLOWED`], so a
    /// table added to the document in a later revision is refused here
    /// until someone decides it belongs in the offer — the safe direction
    /// for a kind whose whole point is carrying nothing executable.
    fn first_forbidden_in_a_remap_package(&self) -> Option<&'static str> {
        let declared: [(&'static str, bool); 29] = [
            ("[project]", self.project.is_some()),
            ("[application]", self.application.is_some()),
            ("[[installer_options]]", !self.installer_options.is_empty()),
            ("[workspace]", self.workspace.is_some()),
            ("[requires]", !self.requires.is_empty()),
            ("[[requires_any]]", !self.requires_any.is_empty()),
            ("[provides]", !self.provides.is_empty()),
            ("[obsoletes]", !self.obsoletes.is_empty()),
            ("[conflicts]", !self.conflicts.is_empty()),
            ("[recommends]", !self.recommends.is_empty()),
            ("[suggests]", !self.suggests.is_empty()),
            ("[[embedded_source]]", !self.embedded_sources.is_empty()),
            ("[[skill]]", !self.skills.is_empty()),
            ("[[binary]]", !self.binaries.is_empty()),
            ("[[mcp_server]]", !self.mcp_servers.is_empty()),
            ("[[documents]]", !self.documents.is_empty()),
            ("[documentation]", self.documentation.is_some()),
            ("[translates]", self.translates.is_some()),
            ("[navigation]", self.navigation.is_some()),
            ("[glossary]", self.glossary.is_some()),
            ("[hooks]", !self.hooks.is_empty()),
            ("[[extension]]", !self.extensions.is_empty()),
            ("[[command]]", !self.run_commands.is_empty()),
            ("[[mechanism]]", !self.mechanism_decls.is_empty()),
            ("[artifacts]", self.artifacts.is_some()),
            ("[deploy]", self.deploy.is_some()),
            ("[boot_snippet]", self.boot_snippet.is_some()),
            ("[features]", !self.features.is_empty()),
            ("[target]", !self.conditional_deps.is_empty()),
        ];
        declared
            .into_iter()
            .find_map(|(name, present)| present.then_some(name))
    }
}
