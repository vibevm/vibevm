//! The engine-owned third mechanism source.
//!
//! §3 of the build/package/deploy architecture rejects a privileged branch for
//! what vibe already implements: the built-in Cargo adapter "is represented by
//! the reserved provider key `org.vibevm/vibe#cargo`, not by a privileged
//! branch outside the registry". This cell is that representation — four
//! ordinary declarations under one reserved identity, which the collector
//! ALWAYS appends ahead of every collected manifest. Selection then has no
//! builtin case to special-case: step 3 of §3.1 is a lookup in the same vector
//! steps 1 and 2 read.
//!
//! These descriptors are engine-synthetic, which is why they carry the
//! `builtin` handler kind that `MechanismDecl::validate` refuses to any
//! authored manifest: a package that could spell one would be naming an engine
//! internal by string.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE");

use std::path::PathBuf;

use specmark::spec;
use vibe_core::manifest::{ExtensionHandler, MechanismDecl, MechanismFreshness, MechanismRole};

/// The engine's own provider owner, spelled once.
///
/// It is a `&str` rather than a typed [`ProviderOwner`](vibe_core::manifest::ProviderOwner)
/// on purpose: a typed value would have to be *parsed* from these bytes at
/// runtime, and a parse that cannot fail still has to answer what happens when
/// it does — either an `expect` in the kernel or an unreachable variant on a
/// public enum. Comparing and joining the canonical spelling instead is the
/// same move `ProviderPin::parse` itself makes with `HOST_OWNER`, and the
/// collector's own `?` covers the one place these bytes become a typed
/// identity.
pub(super) const RESERVED_OWNER: &str = "org.vibevm/vibe";

/// One engine-minted provider, in the exact order collection appends it.
///
/// The four shipped rows are the ones the architecture names by key: `#cargo`
/// (§5, the Cargo commissioning backend), `#static-skill` (§6.1),
/// `#agent-plugin` (§6.2) and `#vibe-bin` (§7.1). Each is `protocol = 1`,
/// because the provider protocol starts at 1 and nothing has revised it.
struct BuiltinDescriptor {
    id: &'static str,
    role: MechanismRole,
    /// Always equal to `id` today: the engine ships exactly one provider per
    /// logical key, so the reserved id IS the logical name it defaults for.
    /// They stay separate fields because a second engine provider for one key
    /// would need distinct spellings, and conflating them now would make that
    /// a grammar change instead of a table row.
    name: &'static str,
    freshness: MechanismFreshness,
    config_schema: &'static str,
}

/// The shipped table.
///
/// `freshness` is read from the architecture, not chosen here: §4.1 states
/// outright that "Cargo is provider-fresh", and a deploy target reconciles
/// state outside the workspace that no engine-side census can hash, so
/// `deploy:vibe-bin` is provider-fresh for the same reason. The two packaging
/// rows are engine-fresh because their input set is closed and hashable by
/// construction — §6.1 produces exactly one file from declared textual
/// resources, §6.2 a directory of declared files.
//
// REVIEW: confirm each `config_schema` path below when R8-CARGO lands the
// provider protocol, because it materialises the JTD files these names point
// at. Nothing reads them at this atom — selection is pure — so the spelling is
// an engine-owned schema identity and not yet a file on disk.
const BUILTINS: [BuiltinDescriptor; 4] = [
    BuiltinDescriptor {
        id: "cargo",
        role: MechanismRole::Build,
        name: "cargo",
        freshness: MechanismFreshness::Provider,
        config_schema: "schemas/mechanism/build_cargo.jtd.json",
    },
    BuiltinDescriptor {
        id: "static-skill",
        role: MechanismRole::Package,
        name: "static-skill",
        freshness: MechanismFreshness::Engine,
        config_schema: "schemas/mechanism/package_static_skill.jtd.json",
    },
    BuiltinDescriptor {
        id: "agent-plugin",
        role: MechanismRole::Package,
        name: "agent-plugin",
        freshness: MechanismFreshness::Engine,
        config_schema: "schemas/mechanism/package_agent_plugin.jtd.json",
    },
    BuiltinDescriptor {
        id: "vibe-bin",
        role: MechanismRole::Deploy,
        name: "vibe-bin",
        freshness: MechanismFreshness::Provider,
        config_schema: "schemas/mechanism/deploy_vibe_bin.jtd.json",
    },
];

/// The engine's own mechanism source — one owner and its declarations.
///
/// Shaped like the two world source kinds on purpose: the collector walks all
/// three the same way, and the only thing that distinguishes this one is that
/// nobody supplies it.
///
/// ```
/// use vibe_extension_registry::builtin_mechanism_source;
///
/// let source = builtin_mechanism_source();
/// assert_eq!(source.owner(), "org.vibevm/vibe");
/// assert_eq!(source.declarations().len(), 4);
/// assert_eq!(source.declarations()[0].id, "cargo");
/// ```
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuiltinMechanismSource {
    declarations: Vec<MechanismDecl>,
}

impl BuiltinMechanismSource {
    /// The canonical spelling of the reserved provider owner every row of this
    /// source is keyed under.
    #[must_use]
    pub const fn owner(&self) -> &'static str {
        RESERVED_OWNER
    }

    /// The engine's declarations, in the order collection appends them.
    #[must_use]
    pub fn declarations(&self) -> &[MechanismDecl] {
        &self.declarations
    }
}

/// The engine's shipped mechanism rows.
///
/// Pure and allocation-only: it reads no configuration, consults no world, and
/// returns the same value on every call, so the collector can append it
/// unconditionally.
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE")]
#[must_use]
pub fn builtin_mechanism_source() -> BuiltinMechanismSource {
    BuiltinMechanismSource {
        declarations: BUILTINS
            .iter()
            .map(|builtin| MechanismDecl {
                id: builtin.id.to_owned(),
                role: builtin.role,
                name: builtin.name.to_owned(),
                handler: ExtensionHandler::Builtin {
                    name: builtin.id.to_owned(),
                },
                protocol: 1,
                config_schema: PathBuf::from(builtin.config_schema),
                freshness: builtin.freshness,
            })
            .collect(),
    }
}
