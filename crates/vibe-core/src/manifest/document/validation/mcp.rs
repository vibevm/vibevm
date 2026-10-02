//! Validation laws for mcp-kind packages.

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-mcp/PROP-027#manifest");

use crate::error::{Error, Result};
use crate::manifest::package::MCP_ARG_VARS;
use crate::package_ref::PackageKind;

use super::Manifest;

impl Manifest {
    /// The `mcp`-kind laws (PROP-027; VIBEVM-SPEC §4.1): `[[mcp_server]]`
    /// is legal only in `mcp`-kind packages and mandatory there; every
    /// declared server names either a local `[[binary]]` or a remote HTTPS
    /// Streamable HTTP endpoint; server names are unique, local launch args
    /// substitute only the closed variable set; and every package
    /// requirement is an exact `=X.Y.Z` pin, so
    /// the served engines and the consumer's gates resolve to one
    /// version set.
    pub(super) fn validate_mcp_kind(&self) -> Result<()> {
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
            match (s.binary.as_deref(), s.url.as_deref()) {
                (Some(binary), None) => {
                    if binary.trim().is_empty() {
                        return Err(Error::InvalidManifest {
                            reason: format!(
                                "[[mcp_server]] `{}` has an empty binary name (violates REQ spec://org.vibevm.core/vibevm/modules/vibe-mcp/PROP-027#manifest; fix surface: [[mcp_server]].binary)",
                                s.name
                            ),
                        });
                    }
                    if s.transport.as_deref().is_some_and(|t| t != "stdio") {
                        return Err(Error::InvalidManifest {
                            reason: format!(
                                "[[mcp_server]] `{}` uses a binary and requires stdio transport (violates REQ spec://org.vibevm.core/vibevm/modules/vibe-mcp/PROP-027#manifest; fix surface: [[mcp_server]].transport)",
                                s.name
                            ),
                        });
                    }
                    if !self.binaries.iter().any(|b| b.name == binary) {
                        return Err(Error::InvalidManifest {
                            reason: format!(
                                "[[mcp_server]] `{}` names binary `{}` but no [[binary]] declares it \
                                 (violates spec://org.vibevm.core/vibevm/modules/vibe-mcp/PROP-027#manifest; \
                                  fix: declare it in [[binary]])",
                                s.name, binary
                            ),
                        });
                    }
                }
                (None, Some(_url)) => {
                    if s.transport.as_deref() != Some("streamable-http") {
                        return Err(Error::InvalidManifest {
                            reason: format!(
                                "[[mcp_server]] `{}` with url requires transport = \"streamable-http\" (violates REQ spec://org.vibevm.core/vibevm/modules/vibe-mcp/PROP-027#manifest; fix surface: [[mcp_server]].transport)",
                                s.name
                            ),
                        });
                    }
                    if !s.args.is_empty() {
                        return Err(Error::InvalidManifest {
                            reason: format!(
                                "[[mcp_server]] `{}` uses a remote URL and cannot declare launch args (violates REQ spec://org.vibevm.core/vibevm/modules/vibe-mcp/PROP-027#manifest; fix surface: [[mcp_server]].args)",
                                s.name
                            ),
                        });
                    }
                    if !s.valid_remote_url() {
                        return Err(Error::InvalidManifest {
                            reason: format!(
                                "[[mcp_server]] `{}` url must be an HTTPS endpoint without credentials, fragment or whitespace (violates REQ spec://org.vibevm.core/vibevm/modules/vibe-mcp/PROP-027#manifest; fix surface: [[mcp_server]].url)",
                                s.name
                            ),
                        });
                    }
                }
                _ => {
                    return Err(Error::InvalidManifest {
                        reason: format!(
                            "[[mcp_server]] `{}` must declare exactly one of binary or url (violates REQ spec://org.vibevm.core/vibevm/modules/vibe-mcp/PROP-027#manifest; fix surface: [[mcp_server]].binary or .url)",
                            s.name
                        ),
                    });
                }
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
