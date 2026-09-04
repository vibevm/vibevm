//! The engine-owned third mechanism source.
//!
//! §3 of the build/package/deploy architecture rejects a privileged branch for
//! what vibe already implements: the built-in Cargo adapter "is represented by
//! the reserved provider key `org.vibevm/vibe#cargo`, not by a privileged
//! branch outside the registry". This cell is that representation — sixteen
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
/// The first five shipped rows are the ones §§5–7 name by key: `#cargo`
/// (§5, the Cargo commissioning backend), `#static-skill` (§6.1),
/// `#agent-plugin` (§6.2), `#vibe-bin` (§7.1) and `#windows-zip` (§7.0.8).
/// The nine that follow are §6.3.0.2's commissioning matrix. The two
/// final rows are §13.1's opaque static-file packager and receipt-owned
/// opt-launcher destination. Each is
/// `protocol = 1`, because the provider protocol starts at 1 and nothing has
/// revised it.
struct BuiltinDescriptor {
    /// The reserved provider id — the `#…` half of this row's identity, and
    /// the `handler = { kind = "builtin", name = … }` spelling an executor
    /// dispatches on.
    id: &'static str,
    role: MechanismRole,
    /// The LOGICAL capability this row defaults for — the `name` half of its
    /// `<role>:<name>` key, which is a vocabulary word and never an identity.
    ///
    /// It is NOT `id`. §6.3.0.2 ships the three client-plugin projections
    /// "deliberately to prove that provider id and logical name are separate
    /// fields": `package:claude-plugin` is serviced by
    /// `org.vibevm/vibe#claude-plugin-projection`, because the deploy row that
    /// INSTALLS a projected plugin already owns the bare id `claude-plugin`,
    /// and one reserved owner cannot key two rows under one `#id`.
    name: &'static str,
    freshness: MechanismFreshness,
    config_schema: &'static str,
}

/// The shipped table.
///
/// `freshness` is read from the architecture, not chosen here: §4.1 states
/// outright that "Cargo is provider-fresh", and a deploy target reconciles
/// state outside the workspace that no engine-side census can hash, so
/// `deploy:vibe-bin` is provider-fresh for the same reason. The three
/// historical packaging rows are engine-fresh because their input set is closed
/// and hashable by construction — §6.1 produces exactly one file from declared
/// textual resources, §6.2 a directory of declared files, and §7.0.8's
/// archive is exactly the declared inputs and nothing else.
/// §13.1 gives the appended pair the same split: the closed static-file
/// input is engine-fresh; the external opt destination is provider-fresh.
///
/// §6.3.0.2 rules the nine client rows the same way and in the same words:
/// "Projection rows are engine-fresh; destination rows are provider-fresh."
/// A projection consumes one recorded `agent-plugin` directory and emits one
/// recorded directory — a closed, hashable input set; a client destination
/// reconciles a private install state (a marketplace, a client's own plugin
/// registry, a shared config document) that no engine census can hash.
///
/// ORDER is part of the table: the first five rows keep their historical
/// positions, so a reader of `vibe extensions`, a candidate list in a refusal
/// and every selection result stay byte-compatible with the pre-R8-CLIENTS
/// engine. New rows are appended, never interleaved.
//
// REVIEW: confirm each `config_schema` path below when the provider protocol
// materialises the JTD files these names point at. Nothing reads them yet —
// selection is pure — so the spelling is an engine-owned schema identity and
// not yet a file on disk. The family is `<role>_<provider id>.jtd.json` in
// snake_case: the schema describes ONE PROVIDER's config, so it is keyed by
// the provider's id and not by the logical capability it defaults for —
// otherwise two providers of one key would have to share one schema identity.
const BUILTINS: [BuiltinDescriptor; 16] = [
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
    BuiltinDescriptor {
        id: "windows-zip",
        role: MechanismRole::Package,
        name: "windows-zip",
        freshness: MechanismFreshness::Engine,
        config_schema: "schemas/mechanism/package_windows_zip.jtd.json",
    },
    // §6.3.0.2's three client-plugin PROJECTIONS. Package role, engine-fresh,
    // and the rows whose `id` deliberately differs from their `name`.
    BuiltinDescriptor {
        id: "claude-plugin-projection",
        role: MechanismRole::Package,
        name: "claude-plugin",
        freshness: MechanismFreshness::Engine,
        config_schema: "schemas/mechanism/package_claude_plugin_projection.jtd.json",
    },
    BuiltinDescriptor {
        id: "codex-plugin-projection",
        role: MechanismRole::Package,
        name: "codex-plugin",
        freshness: MechanismFreshness::Engine,
        config_schema: "schemas/mechanism/package_codex_plugin_projection.jtd.json",
    },
    BuiltinDescriptor {
        id: "opencode-plugin-projection",
        role: MechanismRole::Package,
        name: "opencode-plugin",
        freshness: MechanismFreshness::Engine,
        config_schema: "schemas/mechanism/package_opencode_plugin_projection.jtd.json",
    },
    // §6.3.0.5's three standalone-skill destinations, then §6.3.0.7–8's three
    // plugin destinations. Deploy role, provider-fresh.
    BuiltinDescriptor {
        id: "claude-skill",
        role: MechanismRole::Deploy,
        name: "claude-skill",
        freshness: MechanismFreshness::Provider,
        config_schema: "schemas/mechanism/deploy_claude_skill.jtd.json",
    },
    BuiltinDescriptor {
        id: "codex-skill",
        role: MechanismRole::Deploy,
        name: "codex-skill",
        freshness: MechanismFreshness::Provider,
        config_schema: "schemas/mechanism/deploy_codex_skill.jtd.json",
    },
    BuiltinDescriptor {
        id: "opencode-skill",
        role: MechanismRole::Deploy,
        name: "opencode-skill",
        freshness: MechanismFreshness::Provider,
        config_schema: "schemas/mechanism/deploy_opencode_skill.jtd.json",
    },
    BuiltinDescriptor {
        id: "claude-plugin",
        role: MechanismRole::Deploy,
        name: "claude-plugin",
        freshness: MechanismFreshness::Provider,
        config_schema: "schemas/mechanism/deploy_claude_plugin.jtd.json",
    },
    BuiltinDescriptor {
        id: "codex-plugin",
        role: MechanismRole::Deploy,
        name: "codex-plugin",
        freshness: MechanismFreshness::Provider,
        config_schema: "schemas/mechanism/deploy_codex_plugin.jtd.json",
    },
    BuiltinDescriptor {
        id: "opencode-plugin",
        role: MechanismRole::Deploy,
        name: "opencode-plugin",
        freshness: MechanismFreshness::Provider,
        config_schema: "schemas/mechanism/deploy_opencode_plugin.jtd.json",
    },
    // §13.1's launcher-delivery pair, appended after every historical row.
    BuiltinDescriptor {
        id: "static-file",
        role: MechanismRole::Package,
        name: "static-file",
        freshness: MechanismFreshness::Engine,
        config_schema: "schemas/mechanism/package_static_file.jtd.json",
    },
    BuiltinDescriptor {
        id: "vibe-opt-launcher",
        role: MechanismRole::Deploy,
        name: "vibe-opt-launcher",
        freshness: MechanismFreshness::Provider,
        config_schema: "schemas/mechanism/deploy_vibe_opt_launcher.jtd.json",
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
/// assert_eq!(source.declarations().len(), 16);
/// assert_eq!(source.declarations()[0].id, "cargo");
///
/// // A projection row's provider id is NOT its logical name.
/// let projection = &source.declarations()[5];
/// assert_eq!(projection.id, "claude-plugin-projection");
/// assert_eq!(projection.name, "claude-plugin");
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
