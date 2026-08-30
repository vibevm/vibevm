//! The `[[binary]]` lowering — §5.0.6 of the packages-2026-09 architecture,
//! as a *projection* rather than a branch.
//!
//! §4.1 keeps `[[binary]]` compatible and lowers it into a Cargo build
//! target; §5.0.6 fixes the shape of that lowering: "one pure function
//! projects a legacy `[[binary]]` entry into the equivalent build target
//! (mechanism `build:cargo`, one executable output selected by package/bin),
//! so the graph executor sees ONE target shape". The executor therefore has
//! no legacy case: it walks build targets, and a `[[binary]]` simply *is*
//! one after this call.
//!
//! It lives beside the artifact grammar because the value it mints is that
//! grammar's, and it is deliberately **total and unvalidating**: the target
//! it returns answers to the incumbent
//! [`ArtifactBuildTarget::validate`](super::ArtifactBuildTarget::validate)
//! like any authored row. A second grammar check here would be a second
//! place for the portable-token law to drift, which is exactly what §12's
//! one-grammar freeze refuses.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#ARTIFACT-REGISTRY");

use specmark::spec;

use super::super::mechanism::MechanismKey;
use super::super::package::BinaryDecl;
use super::{ArtifactBuildTarget, ArtifactKind, ArtifactOutput};
use crate::manifest::ExtensionConfig;

/// The projected key's one canonical spelling — the literal the test cell
/// round-trips through `FromStr` to pin that the validated-parts seam and
/// the parser agree byte for byte.
#[cfg(test)]
const BUILD_CARGO: &str = "build:cargo";

/// The `select` member that carries the Cargo `[[bin]]` target name.
const SELECT_BIN: &str = "bin";

/// Project one `[[binary]]` declaration into its equivalent build target.
///
/// Field for field: the PATH-facing `name` becomes both the target id and
/// the single executable output's id (the id grammar is one grammar, and a
/// binary's name is already a portable token when the manifest is legal);
/// `crate` becomes the target's `workdir`, because that directory *is* the
/// Cargo package whose manifest the build runs against; the output's
/// `select` names the `[[bin]]` target by name; `mechanism` is
/// `build:cargo`; and nothing pins a provider, so the shipped builtin
/// default answers unless the host routes the key elsewhere.
///
/// `description` is deliberately dropped: it is `vibe bin list` prose, not
/// a producer fact, and the build target grammar has no member for it.
/// `inputs` and `config` are `None` — a Cargo target is provider-fresh
/// (§4.1), so an engine-side input census would be a claim the lowering
/// cannot honestly make, and `[[binary]]` authors no build configuration.
///
/// ```
/// use vibe_core::manifest::{ArtifactKind, BinaryDecl, build_target_for_binary};
///
/// let binary: BinaryDecl = toml::from_str(
///     "name = \"rust-ai-native\"\ncrate = \"crates/rust-ai-native-cli\"",
/// )
/// .unwrap();
/// let target = build_target_for_binary(&binary);
///
/// assert_eq!(target.id, "rust-ai-native");
/// assert_eq!(target.mechanism.to_string(), "build:cargo");
/// assert_eq!(target.provider, None);
/// assert_eq!(target.workdir, "crates/rust-ai-native-cli");
/// assert_eq!(target.outputs.len(), 1);
/// assert_eq!(target.outputs[0].kind, ArtifactKind::Executable);
/// assert!(target.validate().is_ok());
/// ```
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-054#ARTIFACT-REGISTRY")]
#[must_use]
pub fn build_target_for_binary(binary: &BinaryDecl) -> ArtifactBuildTarget {
    let mut select = toml::Table::new();
    select.insert(
        SELECT_BIN.to_owned(),
        toml::Value::String(binary.name.clone()),
    );
    ArtifactBuildTarget {
        id: binary.name.clone(),
        mechanism: cargo_key(),
        provider: None,
        workdir: forward_slashed(&binary.crate_dir),
        inputs: None,
        outputs: vec![ArtifactOutput {
            id: binary.name.clone(),
            kind: ArtifactKind::Executable,
            select: Some(ExtensionConfig::from_table(select)),
        }],
        config: None,
    }
}

/// `build:cargo` in its one typed spelling. `[[binary]]` is a Cargo tool
/// by construction (PROP-025 §2: `name` matches a `[[bin]]` target inside
/// `crate`), so the lowering names the Cargo capability and lets §3.1's
/// four-step law pick the provider — a host that routes `build:cargo`
/// away routes projected targets with it.
fn cargo_key() -> MechanismKey {
    // The engine's own literal enters through the crate-internal
    // validated-parts seam, so a total projection needs neither a Result
    // on an impossible parse nor an excused `expect`;
    // `role_and_name_are_the_literal` pins the spelling.
    MechanismKey::from_validated_parts(super::super::MechanismRole::Build, "cargo")
}

/// The declarant-relative directory in the one forward-slashed spelling the
/// `workdir` law reads. An authored manifest already carries forward
/// slashes; a programmatically built `BinaryDecl` on Windows may not, and
/// the projection normalises rather than emitting a spelling the incumbent
/// validator would refuse for a reason the author never wrote.
fn forward_slashed(path: &std::path::Path) -> String {
    let rendered = path.to_string_lossy().replace('\\', "/");
    if rendered.is_empty() {
        ".".to_owned()
    } else {
        rendered
    }
}

#[cfg(test)]
#[path = "binary_projection_tests.rs"]
mod tests;
