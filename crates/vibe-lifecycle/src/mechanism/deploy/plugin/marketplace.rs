//! Deterministic immutable local marketplaces for Claude and Codex.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS");

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::Serialize;
use sha2::{Digest, Sha256};
use vibe_safefs::Project;

use super::artifact::AdmittedProjection;
use super::client::PluginClient;
use crate::mechanism::contain::{digest_file, prove_directory, walk_tree};
use crate::mechanism::deploy::protocol::DeployTargetRequest;
use crate::mechanism::error::DeployProviderError;

const DOMAIN: &str = "client-plugin-marketplace/1";

pub(crate) fn marketplace_name(client: PluginClient, target: &str, plugin: &str) -> String {
    let mut hash = Sha256::new();
    for value in [DOMAIN, client.as_str(), target, plugin] {
        hash.update(value.as_bytes());
        hash.update(b"\x00");
    }
    format!("vibevm-{}", &format!("{:x}", hash.finalize())[..32])
}

pub(crate) fn support_root(
    settings: &Path,
    client: PluginClient,
    target: &str,
    artifact_digest: &str,
) -> PathBuf {
    settings
        .join("client-marketplaces")
        .join(client.as_str())
        .join(target)
        .join(artifact_digest)
}

pub(crate) fn manifest_bytes(
    client: PluginClient,
    marketplace: &str,
    plugin: &str,
    version: &str,
) -> Result<(&'static str, Vec<u8>), String> {
    let (relative, value) = match client {
        PluginClient::Claude => (
            ".claude-plugin/marketplace.json",
            serde_json::to_value(ClaudeMarketplace {
                name: marketplace,
                owner: ClaudeOwner { name: "VibeVM" },
                plugins: [ClaudePlugin {
                    name: plugin,
                    source: format!("./plugins/{plugin}"),
                    version,
                }],
            }),
        ),
        PluginClient::Codex => (
            "marketplace.json",
            serde_json::to_value(CodexMarketplace {
                name: marketplace,
                interface: CodexInterface {
                    display_name: codex_display_name(marketplace),
                },
                plugins: [CodexPlugin {
                    name: plugin,
                    source: CodexSource {
                        source: "local",
                        path: format!("./plugins/{plugin}"),
                    },
                    policy: CodexPolicy {
                        installation: "AVAILABLE",
                        authentication: "ON_INSTALL",
                    },
                    category: "Productivity",
                }],
            }),
        ),
        PluginClient::OpenCode => return Err("OpenCode has no local marketplace".to_owned()),
    };
    let mut bytes = serde_json::to_vec_pretty(&value.map_err(|error| error.to_string())?)
        .map_err(|error| error.to_string())?;
    bytes.push(b'\n');
    Ok((relative, bytes))
}

pub(crate) fn materialize(
    request: &DeployTargetRequest<'_>,
    client: PluginClient,
    projection: &AdmittedProjection,
    marketplace: &str,
    plugin: &str,
    version: &str,
    artifact_digest: &str,
) -> Result<PathBuf, DeployProviderError> {
    let root = support_root(
        request.settings_root,
        client,
        &request.target.id,
        artifact_digest,
    );
    let (manifest_relative, manifest) = manifest_bytes(client, marketplace, plugin, version)
        .map_err(|reason| refusal(request, &root, reason))?;
    let expected = expected_files(projection, plugin, manifest_relative, &manifest);
    match std::fs::symlink_metadata(&root) {
        Ok(_) => {
            validate_existing(request, &root, &expected)?;
            return Ok(root);
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(refusal(request, &root, error.to_string())),
    }
    let staging = request.staging.ok_or_else(|| {
        refusal(
            request,
            &root,
            "the engine supplied no staging directory for immutable support".to_owned(),
        )
    })?;
    let staged = staging.join(format!("{}-marketplace", client.as_str()));
    create_directory(&staged).map_err(|reason| refusal(request, &root, reason))?;
    for (relative, source) in &expected {
        let destination = join(&staged, relative);
        ensure_staged_parent(&staged, &destination)
            .map_err(|reason| refusal(request, &root, reason))?;
        match source {
            Expected::Bytes(bytes) => std::fs::write(&destination, bytes)
                .map_err(|error| refusal(request, &root, error.to_string()))?,
            Expected::File { absolute, digest } => {
                let (found, _) = digest_file(absolute)
                    .map_err(|fault| refusal(request, &root, fault.reason()))?;
                if &found != digest {
                    return Err(refusal(
                        request,
                        &root,
                        format!(
                            "projection member `{relative}` changed during support materialization"
                        ),
                    ));
                }
                std::fs::copy(absolute, &destination)
                    .map_err(|error| refusal(request, &root, error.to_string()))?;
            }
        }
    }
    validate_existing(request, &staged, &expected)?;
    let project = Project::open(request.settings_root)
        .map_err(|error| refusal(request, &root, format!("{error:#}")))?;
    let parent = project
        .dir(
            &["client-marketplaces", client.as_str(), &request.target.id],
            true,
        )
        .map_err(|error| refusal(request, &root, format!("{error:#}")))?;
    let destination = parent.join(artifact_digest);
    if destination != root {
        return Err(refusal(
            request,
            &root,
            "pinned support parent disagrees with the deterministic root".to_owned(),
        ));
    }
    std::fs::rename(&staged, &destination)
        .map_err(|error| refusal(request, &root, error.to_string()))?;
    validate_existing(request, &destination, &expected)?;
    Ok(destination)
}

enum Expected<'a> {
    Bytes(&'a [u8]),
    File { absolute: &'a Path, digest: &'a str },
}

fn expected_files<'a>(
    projection: &'a AdmittedProjection,
    plugin: &str,
    manifest_relative: &'a str,
    manifest: &'a [u8],
) -> BTreeMap<String, Expected<'a>> {
    let mut expected = BTreeMap::new();
    expected.insert(manifest_relative.to_owned(), Expected::Bytes(manifest));
    for file in &projection.files {
        expected.insert(
            format!("plugins/{plugin}/{}", file.relative),
            Expected::File {
                absolute: &file.absolute,
                digest: &file.digest,
            },
        );
    }
    expected
}

fn validate_existing(
    request: &DeployTargetRequest<'_>,
    root: &Path,
    expected: &BTreeMap<String, Expected<'_>>,
) -> Result<(), DeployProviderError> {
    let walked = walk_tree(root).map_err(|fault| {
        refusal(
            request,
            root,
            format!("entry `{}`: {}", fault.path, fault.reason),
        )
    })?;
    let actual: Vec<&str> = walked
        .iter()
        .map(|(relative, _)| relative.as_str())
        .collect();
    let wanted: Vec<&str> = expected.keys().map(String::as_str).collect();
    if actual != wanted {
        return Err(refusal(
            request,
            root,
            format!("file census differs; expected {wanted:?}, found {actual:?}"),
        ));
    }
    validate_directory_census(root, &wanted).map_err(|reason| refusal(request, root, reason))?;
    for (relative, source) in expected {
        let path = join(root, relative);
        let expected_digest = match source {
            Expected::Bytes(bytes) => format!("{:x}", Sha256::digest(bytes)),
            Expected::File { digest, .. } => (*digest).to_owned(),
        };
        let (found, _) = digest_file(&path)
            .map_err(|fault| refusal(request, root, format!("`{relative}`: {}", fault.reason())))?;
        if found != expected_digest {
            return Err(refusal(
                request,
                root,
                format!("`{relative}` drifted: expected `{expected_digest}`, found `{found}`"),
            ));
        }
    }
    Ok(())
}

fn validate_directory_census(root: &Path, files: &[&str]) -> Result<(), String> {
    fn descend(root: &Path, at: &Path, files: &[&str]) -> Result<(), String> {
        prove_directory(at).map_err(|fault| fault.reason())?;
        for entry in std::fs::read_dir(at).map_err(|error| error.to_string())? {
            let path = entry.map_err(|error| error.to_string())?.path();
            let metadata = std::fs::symlink_metadata(&path).map_err(|error| error.to_string())?;
            if metadata.file_type().is_symlink() {
                return Err(format!("`{}` is a link", path.display()));
            }
            if metadata.is_dir() {
                let relative = path
                    .strip_prefix(root)
                    .map_err(|error| error.to_string())?
                    .components()
                    .map(|part| part.as_os_str().to_string_lossy())
                    .collect::<Vec<_>>()
                    .join("/");
                if !files
                    .iter()
                    .any(|file| file.starts_with(&format!("{relative}/")))
                {
                    return Err(format!("unexpected or empty directory `{relative}`"));
                }
                descend(root, &path, files)?;
            }
        }
        Ok(())
    }
    descend(root, root, files)
}

fn ensure_staged_chain(base: &Path, destination: &Path) -> Result<(), String> {
    let relative = destination
        .strip_prefix(base)
        .map_err(|_| "support parent escapes settings root".to_owned())?;
    prove_directory(base).map_err(|fault| fault.reason())?;
    let mut current = base.to_path_buf();
    for component in relative.components() {
        current.push(component.as_os_str());
        match std::fs::symlink_metadata(&current) {
            Ok(metadata) if metadata.file_type().is_symlink() => {
                return Err(format!("`{}` is a link", current.display()));
            }
            Ok(metadata) if metadata.is_dir() => {}
            Ok(_) => return Err(format!("`{}` is not a directory", current.display())),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                std::fs::create_dir(&current).map_err(|error| error.to_string())?
            }
            Err(error) => return Err(error.to_string()),
        }
    }
    Ok(())
}

fn create_directory(path: &Path) -> Result<(), String> {
    match std::fs::create_dir(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
            prove_directory(path).map_err(|fault| fault.reason())
        }
        Err(error) => Err(error.to_string()),
    }
}

fn ensure_staged_parent(root: &Path, path: &Path) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or_else(|| "support member has no parent".to_owned())?;
    ensure_staged_chain(root, parent)
}

fn join(root: &Path, relative: &str) -> PathBuf {
    let mut result = root.to_path_buf();
    for component in relative.split('/') {
        result.push(component);
    }
    result
}

/// Exact `display_name_from_plugin_name` translation used by the installed
/// canonical Codex marketplace producer.
fn codex_display_name(name: &str) -> String {
    name.split(['-', '_'])
        .filter(|part| !part.is_empty())
        .map(|part| {
            let mut characters = part.chars();
            characters.next().map_or_else(String::new, |first| {
                format!(
                    "{}{}",
                    first.to_ascii_uppercase(),
                    characters.as_str().to_ascii_lowercase()
                )
            })
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn refusal(request: &DeployTargetRequest<'_>, root: &Path, reason: String) -> DeployProviderError {
    DeployProviderError::MarketplaceSupport {
        target: request.target.id.clone(),
        path: root.display().to_string(),
        reason,
    }
}

#[derive(Serialize)]
struct ClaudeMarketplace<'a> {
    name: &'a str,
    owner: ClaudeOwner<'a>,
    plugins: [ClaudePlugin<'a>; 1],
}
#[derive(Serialize)]
struct ClaudeOwner<'a> {
    name: &'a str,
}
#[derive(Serialize)]
struct ClaudePlugin<'a> {
    name: &'a str,
    source: String,
    version: &'a str,
}

#[derive(Serialize)]
struct CodexMarketplace<'a> {
    name: &'a str,
    interface: CodexInterface,
    plugins: [CodexPlugin<'a>; 1],
}
#[derive(Serialize)]
struct CodexInterface {
    #[serde(rename = "displayName")]
    display_name: String,
}
#[derive(Serialize)]
struct CodexPlugin<'a> {
    name: &'a str,
    source: CodexSource<'a>,
    policy: CodexPolicy<'a>,
    category: &'a str,
}
#[derive(Serialize)]
struct CodexSource<'a> {
    source: &'a str,
    path: String,
}
#[derive(Serialize)]
struct CodexPolicy<'a> {
    installation: &'a str,
    authentication: &'a str,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn marketplace_name_is_domain_separated_and_stable() {
        let name = marketplace_name(PluginClient::Claude, "target", "demo-plugin");
        assert!(name.starts_with("vibevm-"));
        assert_eq!(name.len(), 39);
        assert_ne!(
            name,
            marketplace_name(PluginClient::Codex, "target", "demo-plugin")
        );
    }

    #[test]
    fn codex_manifest_is_the_official_local_entry_shape() {
        let (path, bytes) =
            manifest_bytes(PluginClient::Codex, "vibevm-a", "demo", "1.2.3").unwrap();
        assert_eq!(path, "marketplace.json");
        let value: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(value["plugins"][0]["source"]["source"], "local");
        assert_eq!(value["plugins"][0]["policy"]["installation"], "AVAILABLE");
        assert_eq!(value["interface"]["displayName"], "Vibevm A");
    }
}
