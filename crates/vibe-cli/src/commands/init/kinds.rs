//! The placeholder tables of the kinds whose validators demand a row:
//! `mcp` (PROP-027 `#manifest`: at least one `[[mcp_server]]`) and
//! `remap` (PROP-066 `#package`: at least one `[[remap]]` row), beside
//! `app`, whose tables live in [`super::app`].
//!
//! Without them `vibe init --kind mcp` and `--kind remap` produced a
//! manifest their own validator refuses, so a client that creates a
//! package of either kind (the desktop shell's wizard) got a refusal and
//! nothing it could open. The rows written here are the smallest ones the
//! validators accept, spelled so that a reader sees what to replace.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-024#INIT-ROOT-ROLE");

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use vibe_core::PackageKind;
use vibe_core::manifest::Manifest;

use super::helpers::*;
use crate::output;

/// The tables a fresh manifest of `kind` needs before its validator
/// accepts it, as TOML text that follows every other table. Empty for the
/// kinds whose `[package]` alone is valid.
pub(super) fn manifest_tail(kind: PackageKind, group: &str, name: &str) -> String {
    match kind {
        PackageKind::App => super::app::manifest_tail(name),
        PackageKind::Mcp => mcp_tail(name),
        PackageKind::Remap => remap_tail(group),
        _ => String::new(),
    }
}

/// Add the kind's tables to a typed root manifest, so the root form and
/// the nested form declare exactly the same tables.
///
/// The work happens on the TOML document, not on the typed manifest:
/// serializing a `Manifest` validates it, and an `mcp` or `remap` package
/// without its rows is exactly what the validator refuses. The manifest
/// is serialized under the neutral `tool` kind, given its real kind and
/// the tail back as TOML, and parsed once whole, which validates it.
pub(super) fn declare_in_root(
    manifest: &mut Manifest,
    kind: PackageKind,
    group: &str,
    name: &str,
) -> Result<()> {
    let tail = manifest_tail(kind, group, name);
    if tail.is_empty() {
        return Ok(());
    }
    let mut neutral = manifest.clone();
    neutral
        .package
        .as_mut()
        .context("a package scaffold has a [package] table")?
        .kind = PackageKind::Tool;
    let mut document: toml::Table = toml::from_str(&toml::to_string_pretty(&neutral)?)?;
    if let Some(toml::Value::Table(package)) = document.get_mut("package") {
        package.insert("kind".into(), toml::Value::String(kind.as_str().into()));
    }
    document.extend(toml::from_str::<toml::Table>(&tail)?);
    *manifest = Manifest::parse_str(&toml::to_string_pretty(&document)?)?;
    Ok(())
}

/// The package-relative directory of the server's placeholder crate.
fn server_crate(name: &str) -> PathBuf {
    Path::new("crates").join(name)
}

/// A local stdio server is the common case, so the row names a
/// `[[binary]]` rather than a remote `url`; the validator requires that
/// binary to be declared, and `vibe bin build` needs a crate behind it.
/// `transport` is left out: a binary server is stdio, and an explicit
/// spelling would add nothing the validator asks for.
fn mcp_tail(name: &str) -> String {
    format!(
        "\n[[binary]]\nname = \"{name}\"\ncrate = \"crates/{name}\"\n\
         description = \"The executable the MCP server runs as\"\n\n\
         [[mcp_server]]\nname = \"{name}\"\nbinary = \"{name}\"\n\
         description = \"The {name} MCP server\"\n"
    )
}

/// The host a reverse-DNS group names: `org.example` is `example.org`.
fn group_host(group: &str) -> String {
    group.rsplit('.').collect::<Vec<_>>().join(".")
}

/// A `prefix` row needs no digest, so it is valid before any artifact
/// exists; the `sha256` key names one artifact's content and could only
/// be invented here. The example rewrites the package's own group's host
/// to a mirror under it, which a reader recognises as theirs to replace.
fn remap_tail(group: &str) -> String {
    let host = group_host(group);
    format!(
        "\n[[remap]]\nprefix = \"https://{host}/\"\nurl = \"https://mirror.{host}/\"\n\
         reason = \"An example row: downloads from {host} come from its mirror\"\n"
    )
}

/// True when the manifest declares the scaffold's server crate.
/// Re-initialising a hand-written server package must not add a crate
/// its manifest never names.
pub(super) fn declares_mcp_scaffold(manifest: &Manifest, name: &str) -> bool {
    let crate_dir = server_crate(name);
    manifest
        .binaries
        .iter()
        .any(|binary| binary.name == name && binary.crate_dir == crate_dir)
}

/// Write the placeholder crate the `[[binary]]` points at, under `pkg_dir`.
pub(super) fn create_mcp_files(
    ctx: &output::Context,
    project_root: &Path,
    pkg_dir: &Path,
    name: &str,
) -> Result<Vec<Outcome>> {
    let crate_dir = pkg_dir.join(server_crate(name));
    // The empty `[workspace]` keeps the crate its own: a project that is
    // itself a Cargo workspace would otherwise claim it and refuse to build.
    let cargo = format!(
        "[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[workspace]\n"
    );
    let main = format!(
        "//! The `{name}` MCP server, spoken to over stdio. A placeholder that exits\n\
         //! at once: replace it with the server before an agent registers it.\n\
         fn main() {{}}\n"
    );
    Ok(vec![
        ensure_file(
            ctx,
            project_root,
            &crate_dir.join("Cargo.toml"),
            &cargo,
            "server crate",
        )?,
        ensure_file(
            ctx,
            project_root,
            &crate_dir.join("src").join("main.rs"),
            &main,
            "server source",
        )?,
    ])
}

/// A free-text field as a TOML string literal — quoted and escaped by the
/// TOML writer, so a quote or a backslash in a description, a licence or an
/// author's name lands in the manifest instead of breaking it.
pub(super) fn toml_literal(text: &str) -> String {
    toml::Value::String(text.to_owned()).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn group_host_reverses_the_group() {
        assert_eq!(group_host("org.example"), "example.org");
        assert_eq!(group_host("org.vibevm.apple"), "apple.vibevm.org");
    }
}
