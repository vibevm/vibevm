//! The projection model — what one client projection of one canonical
//! Agent Plugin WOULD emit, computed by reading and nothing else.
//!
//! One function ([`read_projection`]) answers the three source-dependent
//! operation questions, each for its own reason: `plan` needs the census and the
//! parsed identity for its summary and its capability refusals, `fingerprint`
//! needs the identity, and `apply` needs the file list to stage. Computing it
//! once per operation rather than threading it through the shared
//! [`PackagePlan`] is the incumbent idiom — §6.2's provider reads its source
//! in both `fingerprint` and `apply` — and it keeps the shared protocol from
//! growing a member only one provider family reads.
//!
//! **The canonical tree is revalidated here, not trusted.** The input
//! arrived through the engine's own record, whose digest was re-proven
//! against the bytes on disk; that says the tree is the recorded one, not
//! that it is a well-formed Agent Plugin. So the projection walks it through
//! §6.2's OWN cells — the fixed shape, the containment law across links and
//! junctions, the local 1.0.0 manifest validation — and an adapter therefore
//! cannot admit a tree the canonical provider would refuse.
//!
//! **Reverse-domain client-extension directories are not projected.**
//! §6.3.0.3 is explicit that they "remain unrequested unless a later
//! client-specific ruling admits one", and the config vocabulary has no word
//! for them. They are legal in the canonical source, they are counted in the
//! evidence, and they are absent from every projection by contract rather
//! than by oversight.
//!
//! [`PackagePlan`]: crate::mechanism::package::protocol::PackagePlan

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE");

use std::path::{Path, PathBuf};

use super::client::{
    DOT_MCP_MANIFEST, McpShape, OPENCODE_CONFIG, PLUGIN_MANIFEST, ProjectionClient,
};
use super::config::{ClientProjectionConfig, PortableComponent};
use super::error::ClientProjectionError;
use super::opencode;
use crate::mechanism::MechanismError;
use crate::mechanism::plugin::shape::{PluginSource, SKILLS_DIR, read_source};

/// Where one emitted file's bytes come from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum EmittedBytes {
    /// Copied byte-for-byte out of the canonical tree. The canonical
    /// relative path is carried so evidence can say what became what.
    Canonical {
        absolute: PathBuf,
        canonical: String,
    },
    /// Rendered by this engine — the OpenCode configuration fragment, the
    /// one file in a projection that is not a copy.
    Rendered(Vec<u8>),
}

/// One file the projection emits.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Emitted {
    /// Projection-relative, forward-slashed.
    pub(crate) relative: String,
    pub(crate) bytes: EmittedBytes,
}

/// One client's projection of one canonical Agent Plugin, resolved.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Projection {
    /// The `name` member of the canonical `plugin.json`.
    pub(crate) name: String,
    /// The `version` member of the same document.
    pub(crate) version: String,
    /// Every emitted file, in canonical (sorted) order.
    pub(crate) emitted: Vec<Emitted>,
    /// How many canonical files the projection deliberately does NOT emit —
    /// the reverse-domain client-extension files and any unselected
    /// component's. Counted so the evidence can state the absence as a
    /// contract rather than leave it invisible.
    pub(crate) withheld: usize,
}

impl Projection {
    /// The emitted census a summary and an evidence line quote: the file
    /// count and the sorted top-level entries it lands under.
    ///
    /// Top-level rather than every path, because the shape vocabulary is
    /// fixed and small (`.claude-plugin`, `.codex-plugin`, `.mcp.json`,
    /// `opencode.json`, `skills`) while the file count is not — a census
    /// that grew with the plugin would make one target's record unreadable.
    pub(crate) fn census(&self) -> String {
        let mut roots: Vec<&str> = Vec::new();
        for file in &self.emitted {
            let head = file.relative.split('/').next().unwrap_or_default();
            if !roots.contains(&head) {
                roots.push(head);
            }
        }
        roots.sort_unstable();
        format!(
            "{} file(s) under [{}], {} canonical file(s) withheld by contract",
            self.emitted.len(),
            roots.join(", "),
            self.withheld,
        )
    }
}

/// Resolve what one client projection would emit from one canonical tree.
///
/// Read-only — §6.3.0.11's "Plan and verify use read-only probes only". It
/// opens the canonical source, judges it, and creates nothing.
pub(crate) fn read_projection(
    target: &str,
    project_root: &Path,
    client: ProjectionClient,
    config: &ClientProjectionConfig,
    canonical: &str,
) -> Result<Projection, MechanismError> {
    let source = read_source(target, project_root, canonical)?;
    let mut emitted: Vec<Emitted> = Vec::new();
    let mut projected_canonical = 0_usize;

    // The manifest. Unconditional for a client that keeps one, because it
    // is the plugin's IDENTITY and not a portable component: §6.3.0.4 moves
    // "the canonical manifest" to the hidden directory, and §6.3's frozen
    // shape keeps its FULL bytes — this projection renames its placement,
    // never its content.
    if let Some(directory) = client.manifest_dir() {
        emitted.push(Emitted {
            relative: format!("{directory}/{PLUGIN_MANIFEST}"),
            bytes: canonical_bytes(target, client, &source, PLUGIN_MANIFEST)?,
        });
        projected_canonical += 1;
    }

    if config.wants(PortableComponent::Skills) {
        projected_canonical += skills(target, client, &source, &mut emitted)?;
    }
    if config.wants(PortableComponent::Mcp) {
        projected_canonical += mcp(target, client, &source, &mut emitted)?;
    }

    // A projection is addressed by path, so two emitted files may never
    // claim one. Unreachable through the shapes above — a refusal rather
    // than an overwrite for exactly the reason §6.2 gives about dropping.
    emitted.sort_by(|left, right| left.relative.cmp(&right.relative));
    if let Some(collision) = duplicate(&emitted) {
        return Err(ClientProjectionError::Unrepresentable {
            target: target.to_owned(),
            client: client.as_str(),
            member: collision.clone(),
            reason: "two emitted files claim one projection path".to_owned(),
        }
        .into());
    }

    Ok(Projection {
        name: source.identity.name.clone(),
        version: source.identity.version.clone(),
        emitted,
        withheld: source.files.len().saturating_sub(projected_canonical),
    })
}

/// The selected `skills` component — §6.3's "selected `skills/**` retained".
fn skills(
    target: &str,
    client: ProjectionClient,
    source: &PluginSource,
    emitted: &mut Vec<Emitted>,
) -> Result<usize, MechanismError> {
    if source.skills.is_empty() {
        return Err(ClientProjectionError::ComponentMissing {
            target: target.to_owned(),
            client: client.as_str(),
            component: PortableComponent::Skills.as_str(),
            reason: format!(
                "the canonical plugin declares no `{SKILLS_DIR}/<name>/SKILL.md` tree, so there \
                 is no skill to project"
            ),
        }
        .into());
    }
    let prefix = format!("{SKILLS_DIR}/");
    let mut counted = 0_usize;
    for file in &source.files {
        if !file.relative.starts_with(&prefix) {
            continue;
        }
        // The skills tree keeps its canonical placement in every client:
        // §6.3's three shapes move the MANIFEST and the MCP declaration and
        // leave `skills/**` exactly where the plugin put it.
        emitted.push(Emitted {
            relative: file.relative.clone(),
            bytes: EmittedBytes::Canonical {
                absolute: file.absolute.clone(),
                canonical: file.relative.clone(),
            },
        });
        counted += 1;
    }
    Ok(counted)
}

/// The selected `mcp` component, in whichever shape the client takes.
fn mcp(
    target: &str,
    client: ProjectionClient,
    source: &PluginSource,
    emitted: &mut Vec<Emitted>,
) -> Result<usize, MechanismError> {
    let Some(servers) = source.mcp.as_ref() else {
        return Err(ClientProjectionError::ComponentMissing {
            target: target.to_owned(),
            client: client.as_str(),
            component: PortableComponent::Mcp.as_str(),
            reason: "the canonical plugin declares no root `mcp.json`, so there is no MCP server \
                     to project"
                .to_owned(),
        }
        .into());
    };
    match client.mcp_shape() {
        McpShape::CanonicalCopy => {
            emitted.push(Emitted {
                relative: DOT_MCP_MANIFEST.to_owned(),
                bytes: canonical_bytes(
                    target,
                    client,
                    source,
                    crate::mechanism::plugin::shape::MCP_MANIFEST,
                )?,
            });
            // One canonical file consumed: the copy is byte-for-byte, which
            // is what makes the projected declaration provably the authored
            // one rather than a re-encoding of it.
            Ok(1)
        }
        McpShape::OpenCodeFragment => {
            emitted.push(Emitted {
                relative: OPENCODE_CONFIG.to_owned(),
                bytes: EmittedBytes::Rendered(opencode::render(target, servers)?),
            });
            Ok(1)
        }
    }
}

/// One canonical file, addressed by its tree-relative name.
fn canonical_bytes(
    target: &str,
    client: ProjectionClient,
    source: &PluginSource,
    relative: &str,
) -> Result<EmittedBytes, MechanismError> {
    let file = source.file(relative).ok_or_else(|| {
        // `read_source` proves the root manifest exists and only reports an
        // `mcp.json` it listed, so this names an engine defect rather than
        // an authored one — and it refuses instead of emitting nothing.
        MechanismError::from(ClientProjectionError::Unrepresentable {
            target: target.to_owned(),
            client: client.as_str(),
            member: relative.to_owned(),
            reason: "the validated canonical census does not carry it".to_owned(),
        })
    })?;
    Ok(EmittedBytes::Canonical {
        absolute: file.absolute.clone(),
        canonical: file.relative.clone(),
    })
}

/// The first repeated projection path of a sorted census, if any.
fn duplicate(emitted: &[Emitted]) -> Option<&String> {
    emitted
        .windows(2)
        .find(|pair| pair[0].relative == pair[1].relative)
        .map(|pair| &pair[0].relative)
}
