//! Role and package-kind validation for the unified manifest document.

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-007#unified-manifest");

use crate::error::{Error, Result};
use crate::manifest::package::{MCP_ARG_VARS, validate_visibility};

use super::Manifest;

impl Manifest {
    /// Enforce the role rules: `[project]` ⊕ `[package]`; at least one role
    /// section present; package-role sections require `[package]`.
    pub fn validate(&self) -> Result<()> {
        let has_project = self.project.is_some();
        let has_package = self.package.is_some();
        let has_workspace = self.workspace.is_some();

        if has_project && has_package {
            return Err(Error::InvalidManifest {
                reason: "[project] and [package] are mutually exclusive — a node is \
                         either a plain project or a publishable package, not both"
                    .to_string(),
            });
        }
        if !has_project && !has_package && !has_workspace {
            return Err(Error::InvalidManifest {
                reason: "manifest declares no role — it must carry [project], [package], \
                         or [workspace]"
                    .to_string(),
            });
        }

        if let Some(table) = &self.override_table {
            table
                .targets()
                .map_err(|reason| Error::InvalidManifest { reason })?;
        }
        if let Some(meta) = &self.visibility {
            validate_visibility(meta).map_err(|reason| Error::InvalidManifest { reason })?;
        }

        if !has_package {
            let mut offenders: Vec<&str> = Vec::new();
            if self.boot_snippet.is_some() {
                offenders.push("[boot_snippet]");
            }
            if !self.provides.is_empty() {
                offenders.push("[provides]");
            }
            if !self.requires_any.is_empty() {
                offenders.push("[[requires_any]]");
            }
            if !self.obsoletes.is_empty() {
                offenders.push("[obsoletes]");
            }
            if !self.conflicts.is_empty() {
                offenders.push("[conflicts]");
            }
            if !self.recommends.is_empty() {
                offenders.push("[recommends]");
            }
            if !self.suggests.is_empty() {
                offenders.push("[suggests]");
            }
            if !self.skills.is_empty() {
                offenders.push("[[skill]]");
            }
            if !self.binaries.is_empty() {
                offenders.push("[[binary]]");
            }
            if !self.mcp_servers.is_empty() {
                offenders.push("[[mcp_server]]");
            }
            if !self.hooks.is_empty() {
                offenders.push("[hooks]");
            }
            if !self.compatibility.is_empty() {
                offenders.push("[compatibility]");
            }
            if !self.features.is_empty() {
                offenders.push("[features]");
            }
            if !self.conditional_deps.is_empty() {
                offenders.push("[target]");
            }
            if !offenders.is_empty() {
                return Err(Error::InvalidManifest {
                    reason: format!(
                        "package-role section(s) {} present without a [package] table",
                        offenders.join(", ")
                    ),
                });
            }
        }

        self.validate_mcp_kind()?;
        Ok(())
    }

    /// The `mcp`-kind laws (PROP-027; VIBEVM-SPEC §4.1): `[[mcp_server]]`
    /// is legal only in `mcp`-kind packages and mandatory there; every
    /// declared server names a `[[binary]]` in the same manifest, server
    /// names are unique, launch args substitute only the closed variable
    /// set; and every package requirement is an exact `=X.Y.Z` pin, so
    /// the served engines and the consumer's gates resolve to one
    /// version set.
    fn validate_mcp_kind(&self) -> Result<()> {
        use crate::package_ref::PackageKind;

        let kind = self.package.as_ref().map(|p| p.kind);
        if kind != Some(PackageKind::Mcp) {
            if !self.mcp_servers.is_empty() {
                return Err(Error::InvalidManifest {
                    reason: format!(
                        "[[mcp_server]] is legal only in `mcp`-kind packages (this manifest is {}) \
                         — the kind IS the taxonomy \
                         (violates spec://org.vibevm.core/vibevm/modules/vibe-mcp/PROP-027#manifest; \
                          fix: set [package] kind = \"mcp\", or drop the [[mcp_server]] table)",
                        kind.map_or("not a package".to_string(), |k| format!("kind = \"{k}\"")),
                    ),
                });
            }
            return Ok(());
        }

        if self.mcp_servers.is_empty() {
            return Err(Error::InvalidManifest {
                reason: "an `mcp`-kind package must declare at least one [[mcp_server]] — \
                         the kind promises a server \
                         (violates spec://org.vibevm.core/vibevm/modules/vibe-mcp/PROP-027#manifest; \
                          fix: declare the server, or pick the kind that matches the content)"
                    .to_string(),
            });
        }

        let mut seen: std::collections::BTreeSet<&str> = std::collections::BTreeSet::new();
        for s in &self.mcp_servers {
            if !seen.insert(s.name.as_str()) {
                return Err(Error::InvalidManifest {
                    reason: format!(
                        "duplicate [[mcp_server]] name `{}` \
                         (violates spec://org.vibevm.core/vibevm/modules/vibe-mcp/PROP-027#manifest; \
                          fix: server names are the agent-visible identity — make them unique)",
                        s.name
                    ),
                });
            }
            if !self.binaries.iter().any(|b| b.name == s.binary) {
                return Err(Error::InvalidManifest {
                    reason: format!(
                        "[[mcp_server]] `{}` names binary `{}` but no [[binary]] declares it \
                         (violates spec://org.vibevm.core/vibevm/modules/vibe-mcp/PROP-027#manifest; \
                          fix: the server IS a PROP-025 binary — declare it in [[binary]])",
                        s.name, s.binary
                    ),
                });
            }
            let unknown = s.unknown_arg_vars();
            if !unknown.is_empty() {
                return Err(Error::InvalidManifest {
                    reason: format!(
                        "[[mcp_server]] `{}` args carry unknown substitution variable(s) {} \
                         (violates spec://org.vibevm.core/vibevm/modules/vibe-mcp/PROP-027#manifest; \
                          fix: only {} substitute at registration time)",
                        s.name,
                        unknown.join(", "),
                        MCP_ARG_VARS.join(", "),
                    ),
                });
            }
        }

        for r in &self.requires.packages {
            if !r.version.is_exact_pin() {
                return Err(Error::InvalidManifest {
                    reason: format!(
                        "`mcp`-kind packages pin every package requirement exactly, and \
                         `{r}` does not — the served engines and the consumer's gates must \
                         resolve to ONE version set \
                         (violates spec://org.vibevm.core/vibevm/modules/vibe-mcp/PROP-027#exact-pin; \
                          fix: require `=X.Y.Z`, and bump it in lockstep with the served package)",
                    ),
                });
            }
        }
        Ok(())
    }
}
