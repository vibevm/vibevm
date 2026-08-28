//! The execution-declaration fingerprint — the evidence sibling of the
//! legacy execution fingerprint (PROP-054 `##DECLARATION-FINGERPRINT`).
//!
//! Where the execution fingerprint answers «may this execution fresh-skip in
//! THIS invocation?», the declaration fingerprint answers «is this the same
//! DECLARED work?». It therefore binds exactly the effective executable
//! declaration — qualified execution, phase, point, portable slot
//! coordinate, exhaustive handler payload, the effective config actually
//! delivered, the provider pin, the authored input pattern list, the pass
//! placement, the `compiler_internals` capability bit and the resolved agent
//! prompt — and excludes every selection/invocation/world member the spec
//! names: requested command/chain, offline, agent mode, project/world,
//! manifest/lock bytes, accumulated artifacts and the current input bytes
//! (those belong to run identity, execution freshness or the independent
//! manifest witness).
//!
//! The recipe is frozen normatively (R7.5 architecture §1.2): one SHA-256
//! domain, the common length frame, ASCII `0|1` presence bytes and canonical
//! decimal counts. The complete `ExtensionDecl` is destructured WITHOUT `..`
//! and every field is explicitly bound or classified, so a future grammar
//! member fails compilation here until an epoch ruling decides it; the
//! handler match is exhaustive for the same reason.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#DECLARATION-FINGERPRINT");

use std::collections::BTreeMap;

use serde_json::Value;
use specmark::spec;
use vibe_core::manifest::{
    ExtensionDecl, ExtensionHandler, ExtensionIrLevel, ExtensionPass, ExtensionPassKind,
};
use vibe_wire::generated::lifecycle::e1::context::SlotTarget;

use crate::ExtensionProvider;
use crate::ExtensionRegistryRow;
use crate::agent::PreparedAgent;

use super::{FingerprintError, FramedHash, machine_path};

/// The declaration identity of one prepared execution under its own epoch.
///
/// `execution_identity` is the exact `HandlerExecution::key()` spelling at
/// the handler level (slot rows carry their `@slot(group/name@version)`
/// qualifier); `slot` is the descriptor's portable coordinate. The absolute
/// machine `slot.root` is deliberately never hashed: it is environment, not
/// declaration, and freezing it would mint `stale` for identical declared
/// work whenever a workspace moves.
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-054#DECLARATION-FINGERPRINT")]
pub(super) fn declaration_fingerprint(
    execution_identity: &str,
    slot: Option<&SlotTarget>,
    row: &ExtensionRegistryRow,
    phase: &str,
    delivered_config: &BTreeMap<String, Option<Value>>,
    prepared_agent: Option<&PreparedAgent>,
) -> Result<String, FingerprintError> {
    let mut hash = FramedHash::declaration();
    hash.field("execution", execution_identity.as_bytes());
    hash.field("phase", phase.as_bytes());
    // The COMPLETE declaration, destructured without `..` so a new field is a
    // compile-time epoch decision. Every member is either bound below into
    // the digest or classified as excluded with the reason.
    let ExtensionDecl {
        // Bound through `execution`: the provider-qualified key embeds the id.
        id: _id,
        point,
        handler,
        // Excluded: authored config replaced by an effective override is not
        // the work delivered — `delivered_config` below is the authority.
        config: _authored_config,
        // Excluded: activation policy decides plan membership, not work.
        auto: _auto,
        inputs,
        // Excluded: a selector decides whether the row enters the plan.
        applies_to: _applies_to,
        compiler_internals,
        pass,
        // Excluded: an opaque pre-selection applicability guard.
        when: _when,
    } = row.declaration();
    hash.field("point", point.to_string().as_bytes());
    hash.presence("slot_present", slot.is_some());
    if let Some(slot) = slot {
        hash.field("slot_group", slot.group.as_bytes());
        hash.field("slot_kind", slot.kind.as_bytes());
        hash.field("slot_name", slot.name.as_bytes());
        hash.field("slot_version", slot.version.as_bytes());
    }
    hash.field("handler_kind", handler.kind().as_bytes());
    handler_payload(&mut hash, handler);
    // Always framed: an absent and an explicitly empty effective config are
    // intentionally ONE executable value (unlike the input pattern list).
    hash.json("effective_config", delivered_config, row.key())?;
    provider_payload(&mut hash, row.provider());
    hash.presence("inputs_present", inputs.is_some());
    if let Some(patterns) = inputs {
        hash.count("pattern_count", patterns.len());
        for pattern in patterns {
            hash.field("pattern", pattern.as_bytes());
        }
    }
    hash.presence("pass_present", pass.is_some());
    if let Some(pass) = pass {
        pass_payload(&mut hash, pass);
    }
    hash.presence("compiler_internals_present", compiler_internals.is_some());
    if let Some(flag) = compiler_internals {
        hash.presence("compiler_internals", *flag);
    }
    hash.presence("prompt_present", prepared_agent.is_some());
    if let Some(prepared) = prepared_agent {
        let (address, bytes) = prepared.fingerprint_material();
        hash.field("prompt_address", address.as_bytes());
        hash.field("prompt_bytes", bytes);
    }
    Ok(hash.finish())
}

/// The exhaustive handler payload: every variant frames its complete
/// work-shaping material, and the native option/map presence is explicit so
/// `None` can never collide with an authored empty value.
fn handler_payload(hash: &mut FramedHash, handler: &ExtensionHandler) {
    match handler {
        ExtensionHandler::Builtin { name } | ExtensionHandler::Binary { name } => {
            hash.field("handler_name", name.as_bytes());
        }
        ExtensionHandler::Script { base } => {
            hash.field("handler_base", machine_path(base).as_bytes());
        }
        ExtensionHandler::Native {
            crate_dir,
            prebuilt,
        } => {
            hash.presence("handler_crate_present", crate_dir.is_some());
            if let Some(crate_dir) = crate_dir {
                hash.field("handler_crate", machine_path(crate_dir).as_bytes());
            }
            hash.presence("handler_prebuilt_present", prebuilt.is_some());
            if let Some(prebuilt) = prebuilt {
                hash.count("handler_prebuilt_count", prebuilt.len());
                // A `BTreeMap`: the pairs enter deterministically platform-sorted.
                for (platform, path) in prebuilt {
                    hash.field("handler_platform", platform.as_bytes());
                    hash.field("handler_prebuilt", machine_path(path).as_bytes());
                }
            }
        }
        ExtensionHandler::Agent { prompt } => {
            hash.field("handler_prompt", prompt.as_bytes());
        }
    }
}

/// The provider pin: identity, exact version and content hash presence. The
/// content hash is what makes «the declaring package's bytes changed» move
/// the declaration even when handler paths and config spell identically.
fn provider_payload(hash: &mut FramedHash, provider: &ExtensionProvider) {
    match provider {
        ExtensionProvider::Dependency(provider) => {
            hash.field("provider_kind", b"dependency");
            hash.field("provider_id", provider.id.to_string().as_bytes());
            hash.field("provider_version", provider.version.as_bytes());
            hash.presence("provider_content_present", true);
            hash.field(
                "provider_content",
                provider.content_hash.to_string().as_bytes(),
            );
        }
        ExtensionProvider::Host(provider) => {
            hash.field("provider_kind", b"host");
            hash.field("provider_id", provider.identity.to_string().as_bytes());
            hash.field("provider_version", provider.version.as_bytes());
            hash.presence("provider_content_present", provider.content_hash.is_some());
            if let Some(content) = &provider.content_hash {
                hash.field("provider_content", content.to_string().as_bytes());
            }
        }
    }
}

/// Every pass field with explicit presence: `level`/`from`/`to` carry their
/// closed lowercase IR-level spelling, `formats` frames presence, count and
/// declaration-order values, and `artifact` is a present/absent string.
fn pass_payload(hash: &mut FramedHash, pass: &ExtensionPass) {
    hash.field("pass_kind", pass_kind(pass.kind).as_bytes());
    level_payload(hash, "level", pass.level);
    level_payload(hash, "from", pass.from);
    level_payload(hash, "to", pass.to);
    string_payload(hash, "after", pass.after.as_deref());
    string_payload(hash, "before", pass.before.as_deref());
    string_payload(hash, "replace", pass.replace.as_deref());
    hash.presence("formats_present", pass.formats.is_some());
    if let Some(formats) = &pass.formats {
        hash.count("formats_count", formats.len());
        for format in formats {
            hash.field("format", format.as_bytes());
        }
    }
    string_payload(hash, "artifact", pass.artifact.as_deref());
}

fn level_payload(hash: &mut FramedHash, label: &str, level: Option<ExtensionIrLevel>) {
    hash.presence(&format!("{label}_present"), level.is_some());
    if let Some(level) = level {
        hash.field(label, ir_level(level).as_bytes());
    }
}

fn string_payload(hash: &mut FramedHash, label: &str, value: Option<&str>) {
    hash.presence(&format!("{label}_present"), value.is_some());
    if let Some(value) = value {
        hash.field(label, value.as_bytes());
    }
}

/// The closed lowercase wire spelling of a pass kind — the same bytes the
/// manifest wire carries, never a Rust debug spelling.
const fn pass_kind(kind: ExtensionPassKind) -> &'static str {
    match kind {
        ExtensionPassKind::Transform => "transform",
        ExtensionPassKind::Lowering => "lowering",
        ExtensionPassKind::Frontend => "frontend",
        ExtensionPassKind::Backend => "backend",
    }
}

/// The closed lowercase wire spelling of an IR level.
const fn ir_level(level: ExtensionIrLevel) -> &'static str {
    match level {
        ExtensionIrLevel::Source => "source",
        ExtensionIrLevel::Document => "document",
        ExtensionIrLevel::Closure => "closure",
        ExtensionIrLevel::Lane => "lane",
        ExtensionIrLevel::Emitted => "emitted",
    }
}
