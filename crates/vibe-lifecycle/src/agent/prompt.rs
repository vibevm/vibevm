//! Provider-scoped prompt addressing and the envelope's prose projection.
//!
//! Two laws live here. First, `handler.prompt` is resolved **only inside the
//! contributing provider's own identity, and inside the exact instance that is
//! executing**: the address authority must equal the declaring package's
//! coordinate, and the request carries that provider's own materialised root,
//! so resolution can never fall back to a slot search that would serve the
//! freshest *installed* version rather than the *selected* one. No authoring
//! directory is ever scanned.
//!
//! Second, an agent handler receives the envelope as prose (PROP-054
//! `##ENVELOPE-LAW`) rather than as the machine document script and binary
//! handlers get. That projection is built here, and it deliberately carries no
//! credential, endpoint or token path: the provider sees the work, never the
//! means of paying for it.

use std::collections::BTreeMap;
use std::path::PathBuf;

use specmark::spec;
use vibe_spec::{Authority, SelectedPackage, SpecAddress};
use vibe_wire::generated::lifecycle::e1::context::Context;

use crate::{ExtensionProvider, ExtensionRegistryRow, HostIdentity};

use super::AgentError;
use super::contract::{OUTPUT_ACCEPT_NON_EMPTY, OUTPUT_KIND_FILE, OutputContract};

/// An address proven to name its own provider, together with the exact
/// instance of that provider the resolver must read from and the exact set of
/// other instances its composition may reach.
#[derive(Debug, Clone, PartialEq, Eq)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-054#H-AGENT")]
pub struct PromptRequest {
    /// The address exactly as declared.
    pub address: String,
    /// The executing provider's own materialised root — the host node's
    /// directory or the selected dependency slot, never a re-derived one.
    pub provider_root: PathBuf,
    /// The executing provider's coordinate halves, so the resolver treats this
    /// root as that coordinate's self root.
    pub provider_group: String,
    pub provider_name: String,
    /// The lock-selected world: `(group, name) -> {version, root}`, taken from
    /// the effective lifecycle world this run was built from. A cross-package
    /// `#embed` resolves **only** through this map, so a coordinate the lock
    /// did not select cannot be reached and no slot directory is ever scanned
    /// for a "freshest installed" answer. The version travels with the root so
    /// an explicit `@version` in an embedded address is checked against what
    /// the lock actually chose instead of being dropped.
    pub selected_world: BTreeMap<(String, String), SelectedPackage>,
}

/// What a backend resolved, and what it found that this handler does not
/// execute.
#[derive(Debug, Clone, PartialEq, Eq)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-054#H-AGENT")]
pub struct ResolvedPrompt {
    /// The embed-expanded bytes. These feed the fingerprint and the one paid
    /// request — the same value, never resolved twice.
    pub text: String,
    /// Composition directives found anywhere in the expanded closure that this
    /// handler does not perform, rendered as `#keyword address`, in document
    /// order. Non-empty refuses before spend.
    pub unsupported: Vec<String>,
}

/// Parse `handler.prompt` and refuse any address that leaves the declaring
/// provider's identity; bind the executing provider's root to it.
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-054#H-AGENT")]
pub fn provider_scoped(
    prompt: &str,
    row: &ExtensionRegistryRow,
    context: &Context,
) -> Result<PromptRequest, AgentError> {
    let address = SpecAddress::parse(prompt).map_err(|error| AgentError::PromptAddress {
        address: prompt.to_string(),
        reason: error.to_string(),
    })?;
    let Authority::Package {
        group,
        name,
        version,
    } = &address.authority
    else {
        return Err(AgentError::PromptAddress {
            address: prompt.to_string(),
            reason: "the authority is undotted; a prompt names its own package coordinate \
                     `spec://<group>/<name>/…`"
                .into(),
        });
    };
    if version.is_some() {
        return Err(AgentError::PromptAddress {
            address: prompt.to_string(),
            reason: "the authority carries an `@version`; a prompt is resolved inside the exact \
                     provider instance already selected into this world, so a pin could only \
                     disagree with it"
                .into(),
        });
    }
    let Some(owner) = provider_identity(row.provider()) else {
        return Err(AgentError::PromptProvider {
            address: prompt.to_string(),
            provider: row.provider().to_string(),
            reason: "the contributing host declares no `<group>/<name>` coordinate, so it can \
                     address no prompt document"
                .into(),
        });
    };
    if (group.as_str(), name.as_str()) != (owner.group.as_str(), owner.name.as_str()) {
        return Err(AgentError::PromptProvider {
            address: prompt.to_string(),
            provider: row.provider().to_string(),
            reason: format!(
                "it names `{group}/{name}`, but a prompt is resolved only inside its own \
                 provider `{}/{}`",
                owner.group, owner.name
            ),
        });
    }
    Ok(PromptRequest {
        address: address.raw.clone(),
        provider_root: owner.root,
        provider_group: owner.group,
        provider_name: owner.name,
        selected_world: selected_world(context),
    })
}

/// The lock-selected world, exactly as the effective lifecycle envelope
/// records it. Nothing here re-derives a root from a coordinate.
fn selected_world(context: &Context) -> BTreeMap<(String, String), SelectedPackage> {
    context
        .world
        .packages
        .iter()
        .map(|package| {
            (
                (package.group.clone(), package.name.clone()),
                SelectedPackage::new(package.version.clone(), &package.slot),
            )
        })
        .collect()
}

struct ProviderIdentity {
    group: String,
    name: String,
    root: PathBuf,
}

fn provider_identity(provider: &ExtensionProvider) -> Option<ProviderIdentity> {
    match provider {
        ExtensionProvider::Dependency(provider) => Some(ProviderIdentity {
            group: provider.id.group().to_string(),
            name: provider.id.name().to_string(),
            // The selected slot, carried by the registry row itself. Nothing
            // downstream re-derives it from the coordinate, so the executing
            // version and the resolved bytes cannot disagree.
            root: provider.root.clone(),
        }),
        ExtensionProvider::Host(provider) => match &provider.identity {
            HostIdentity::Coordinate(id) => Some(ProviderIdentity {
                group: id.group().to_string(),
                name: id.name().to_string(),
                // The selected node, which in a multi-node workspace is the
                // member, not the workspace root.
                root: provider.root.clone(),
            }),
            HostIdentity::UngroupedProject(_) | HostIdentity::VirtualWorkspace => None,
        },
    }
}

/// This atom's prompt composition law.
///
/// An agent prompt is **one addressed section plus recursive `#embed`
/// expansion** — nothing more. `#use` and `#source` are real PROP-035
/// composition mechanisms with their own semantics (alias binding and source
/// gathering), and this handler performs neither. Calling the result a "full
/// spec closure" while quietly dropping them would put text in front of a
/// paid model that the author believed had been assembled, so they are refused
/// before spend with the remediation that names what IS supported.
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-054#H-AGENT")]
pub fn refuse_unsupported_composition(
    address: &str,
    unsupported: &[String],
) -> Result<(), AgentError> {
    if unsupported.is_empty() {
        return Ok(());
    }
    Err(AgentError::PromptComposition {
        address: address.to_string(),
        found: unsupported.join(", "),
    })
}

/// The invariant discipline every agent execution imposes on the provider.
#[must_use]
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-054#REPLY-SHAPE")]
pub fn system_prose() -> String {
    "You are producing project files for a VibeVM `create`-phase execution.\n\
     Answer with exactly one JSON document and nothing else: no prose before or after it, \
     no Markdown code fence, no second document.\n\
     The document shape is {\"outputs\":[{\"path\":\"…\",\"content\":\"…\"}]}.\n\
     `outputs` repeats the declared output contract exactly: the same paths, each exactly \
     once, in the declared order, with no path added, dropped or rewritten.\n\
     Each `content` is the complete UTF-8 body of that file and is never empty.\n"
        .to_string()
}

/// The resolved instruction prose, the envelope projection and the exact
/// output contract — the whole of what the provider is told.
#[must_use]
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-054#ENVELOPE-LAW")]
pub fn user_prose(instructions: &str, context: &Context, contract: &OutputContract) -> String {
    let mut prose = String::new();
    prose.push_str("# Instructions\n\n");
    prose.push_str(instructions.trim_end());
    prose.push_str("\n\n# Lifecycle context\n\n");
    prose.push_str(&envelope_prose(context));
    prose.push_str("\n# Output contract\n\n");
    prose.push_str(&contract.prose());
    prose.push_str(&format!(
        "\nEvery row is written to the project root as a `{OUTPUT_KIND_FILE}` and is accepted \
         only as a `{OUTPUT_ACCEPT_NON_EMPTY}`. Return the rows in this exact order.\n"
    ));
    prose
}

/// The prose projection of the epoch-1 envelope. Deliberately partial: the
/// scratch directory, the credential layer and the raw config table are not
/// the provider's business, while the project, the moment in the ritual and
/// the already-produced artifacts are.
fn envelope_prose(context: &Context) -> String {
    let mut prose = String::new();
    prose.push_str(&format!(
        "- project: `{}` version `{}` (kind {})\n",
        context.project.name, context.project.version, context.project.kind,
    ));
    prose.push_str(&format!(
        "- lifecycle: requested `{}`, chain [{}], now at phase `{}`\n",
        context.run.requested,
        context.run.chain.join(", "),
        context.run.phase,
    ));
    prose.push_str(&format!(
        "- execution: `{}` contributed by `{}` at point `{}`\n",
        context.execution.id, context.execution.package, context.point,
    ));
    prose.push_str(&format!(
        "- installed packages: {}\n",
        if context.world.packages.is_empty() {
            "none".to_string()
        } else {
            context
                .world
                .packages
                .iter()
                .map(|package| format!("{}/{}@{}", package.group, package.name, package.version))
                .collect::<Vec<_>>()
                .join(", ")
        },
    ));
    prose.push_str(&format!(
        "- artifacts already produced this run: {}\n",
        if context.artifacts.is_empty() {
            "none".to_string()
        } else {
            context
                .artifacts
                .iter()
                .map(|artifact| format!("{} ({})", artifact.id, artifact.kind))
                .collect::<Vec<_>>()
                .join(", ")
        },
    ));
    prose
}
