//! Exact, no-follow admission for one recorded client projection tree.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS");

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use serde_json::{Map, Value};
use vibe_core::manifest::{ArtifactKind, SkillDecl};
use vibe_wire::generated::artifact_record::ArtifactShape;

use super::client::PluginClient;
use crate::mechanism::contain::{digest_file, prove_directory, tree_digest, walk_tree};
use crate::mechanism::deploy::protocol::ResolvedDeployArtifact;
use crate::mechanism::error::DeployProviderError;
use crate::mechanism::plugin::manifest::{
    PluginIdentity, validate_mcp_manifest_at, validate_plugin_manifest_at,
};

const OPENCODE_CONFIG: &str = "opencode.json";

#[derive(Debug, Clone)]
pub(crate) struct ProjectionFile {
    pub(crate) relative: String,
    pub(crate) absolute: PathBuf,
    pub(crate) digest: String,
}

#[derive(Debug, Clone)]
pub(crate) struct AdmittedProjection {
    pub(crate) identity: Option<PluginIdentity>,
    pub(crate) files: Vec<ProjectionFile>,
    pub(crate) opencode_mcp: Map<String, Value>,
}

pub(crate) fn admit(
    target: &str,
    provider: &'static str,
    client: PluginClient,
    artifact: &ResolvedDeployArtifact,
) -> Result<AdmittedProjection, DeployProviderError> {
    if artifact.kind != ArtifactKind::Directory || artifact.shape != ArtifactShape::Directory {
        return Err(refuse(
            target,
            artifact,
            provider,
            "expected the recorded kind=directory, shape=directory client projection".to_owned(),
        ));
    }
    let tree = tree_digest(&artifact.absolute).map_err(|fault| {
        refuse(
            target,
            artifact,
            provider,
            format!(
                "entry `{}` is not a closed no-link tree: {}",
                fault.path, fault.reason
            ),
        )
    })?;
    if tree.digest != artifact.digest {
        return Err(refuse(
            target,
            artifact,
            provider,
            format!(
                "canonical tree digest is `{}` but the proven record carried `{}`",
                tree.digest, artifact.digest
            ),
        ));
    }
    let walked = walk_tree(&artifact.absolute).map_err(|fault| {
        refuse(
            target,
            artifact,
            provider,
            format!("entry `{}` is not admissible: {}", fault.path, fault.reason),
        )
    })?;
    let relatives: BTreeSet<&str> = walked.iter().map(|(path, _)| path.as_str()).collect();
    exact_files(target, provider, client, artifact, &relatives)?;
    exact_directories(target, provider, client, artifact, &relatives)?;

    let identity = if let Some(directory) = client.manifest_dir() {
        let file = format!("{directory}/plugin.json");
        Some(
            validate_plugin_manifest_at(target, &artifact.absolute, &file)
                .map_err(|error| refuse(target, artifact, provider, error.to_string()))?,
        )
    } else {
        None
    };
    if client != PluginClient::OpenCode && relatives.contains(".mcp.json") {
        validate_mcp_manifest_at(target, &artifact.absolute, ".mcp.json")
            .map_err(|error| refuse(target, artifact, provider, error.to_string()))?;
    }
    let opencode_mcp = if client == PluginClient::OpenCode && relatives.contains(OPENCODE_CONFIG) {
        parse_opencode(target, provider, artifact)?
    } else {
        Map::new()
    };
    let mut files = Vec::with_capacity(walked.len());
    for (relative, absolute) in walked {
        let (digest, _) = digest_file(&absolute)
            .map_err(|fault| refuse(target, artifact, provider, fault.reason()))?;
        files.push(ProjectionFile {
            relative,
            absolute,
            digest,
        });
    }
    Ok(AdmittedProjection {
        identity,
        files,
        opencode_mcp,
    })
}

fn exact_files(
    target: &str,
    provider: &'static str,
    client: PluginClient,
    artifact: &ResolvedDeployArtifact,
    files: &BTreeSet<&str>,
) -> Result<(), DeployProviderError> {
    if let Some(manifest_dir) = client.manifest_dir() {
        let manifest = format!("{manifest_dir}/plugin.json");
        if !files.contains(manifest.as_str()) {
            return Err(refuse(
                target,
                artifact,
                provider,
                format!("required `{manifest}` is missing"),
            ));
        }
        for file in files {
            if *file != manifest && *file != ".mcp.json" && !file.starts_with("skills/") {
                return Err(refuse(
                    target,
                    artifact,
                    provider,
                    format!("unexpected projection entry `{file}`"),
                ));
            }
            validate_skill_file(target, artifact, provider, file)?;
        }
    } else {
        if files.is_empty() {
            return Err(refuse(
                target,
                artifact,
                provider,
                "OpenCode projection carries at least one of `skills/` or `opencode.json`"
                    .to_owned(),
            ));
        }
        for file in files {
            if *file != OPENCODE_CONFIG && !file.starts_with("skills/") {
                return Err(refuse(
                    target,
                    artifact,
                    provider,
                    format!("unexpected projection entry `{file}`"),
                ));
            }
            validate_skill_file(target, artifact, provider, file)?;
        }
    }
    Ok(())
}

fn validate_skill_file(
    target: &str,
    artifact: &ResolvedDeployArtifact,
    provider: &'static str,
    file: &str,
) -> Result<(), DeployProviderError> {
    let Some(tail) = file.strip_prefix("skills/") else {
        return Ok(());
    };
    let mut parts = tail.split('/');
    let name = parts.next().unwrap_or_default();
    if !SkillDecl::valid_name(name) {
        return Err(refuse(
            target,
            artifact,
            provider,
            format!("skill directory `{name}` is not portable lowercase-kebab"),
        ));
    }
    if parts.next().is_none() {
        return Err(refuse(
            target,
            artifact,
            provider,
            format!("`{file}` is not below one named skill directory"),
        ));
    }
    Ok(())
}

fn exact_directories(
    target: &str,
    provider: &'static str,
    client: PluginClient,
    artifact: &ResolvedDeployArtifact,
    files: &BTreeSet<&str>,
) -> Result<(), DeployProviderError> {
    let mut directories = Vec::new();
    collect_directories(&artifact.absolute, &artifact.absolute, &mut directories)
        .map_err(|reason| refuse(target, artifact, provider, reason))?;
    for directory in directories {
        let allowed = client
            .manifest_dir()
            .is_some_and(|manifest| directory == manifest)
            || directory == "skills"
            || directory.starts_with("skills/");
        if !allowed {
            return Err(refuse(
                target,
                artifact,
                provider,
                format!("unexpected projection directory `{directory}`"),
            ));
        }
        if !files
            .iter()
            .any(|file| file.starts_with(&format!("{directory}/")))
        {
            return Err(refuse(
                target,
                artifact,
                provider,
                format!(
                    "empty projection directory `{directory}` is not part of the recorded file tree"
                ),
            ));
        }
    }
    Ok(())
}

fn collect_directories(root: &Path, at: &Path, found: &mut Vec<String>) -> Result<(), String> {
    prove_directory(at).map_err(|fault| fault.reason())?;
    for entry in std::fs::read_dir(at).map_err(|error| error.to_string())? {
        let path = entry.map_err(|error| error.to_string())?.path();
        let metadata = std::fs::symlink_metadata(&path).map_err(|error| error.to_string())?;
        if metadata.file_type().is_symlink() {
            return Err(format!("`{}` is a link/reparse entry", path.display()));
        }
        if metadata.is_dir() {
            let relative = path
                .strip_prefix(root)
                .map_err(|error| error.to_string())?
                .components()
                .map(|part| part.as_os_str().to_string_lossy())
                .collect::<Vec<_>>()
                .join("/");
            found.push(relative);
            collect_directories(root, &path, found)?;
        }
    }
    Ok(())
}

fn parse_opencode(
    target: &str,
    provider: &'static str,
    artifact: &ResolvedDeployArtifact,
) -> Result<Map<String, Value>, DeployProviderError> {
    let bytes = crate::mechanism::contain::read_file_bounded(
        &artifact.absolute.join(OPENCODE_CONFIG),
        4 * 1024 * 1024,
    )
    .map_err(|fault| refuse(target, artifact, provider, fault.reason()))?;
    let value: Value = serde_json::from_slice(&bytes).map_err(|error| {
        refuse(
            target,
            artifact,
            provider,
            format!("`opencode.json` is not JSON: {error}"),
        )
    })?;
    let root = value.as_object().ok_or_else(|| {
        refuse(
            target,
            artifact,
            provider,
            "`opencode.json` must be an object".to_owned(),
        )
    })?;
    if root.len() != 1 || !root.contains_key("mcp") {
        return Err(refuse(
            target,
            artifact,
            provider,
            "`opencode.json` carries exactly one root member, `mcp`".to_owned(),
        ));
    }
    let mcp = root["mcp"].as_object().ok_or_else(|| {
        refuse(
            target,
            artifact,
            provider,
            "`opencode.json.mcp` must be an object".to_owned(),
        )
    })?;
    for (name, entry) in mcp {
        if !SkillDecl::valid_name(name) {
            return Err(refuse(
                target,
                artifact,
                provider,
                format!("OpenCode MCP name `{name}` is not portable lowercase-kebab"),
            ));
        }
        validate_opencode_entry(target, provider, artifact, name, entry)?;
    }
    Ok(mcp.clone())
}

fn validate_opencode_entry(
    target: &str,
    provider: &'static str,
    artifact: &ResolvedDeployArtifact,
    name: &str,
    value: &Value,
) -> Result<(), DeployProviderError> {
    let entry = value.as_object().ok_or_else(|| {
        refuse(
            target,
            artifact,
            provider,
            format!("OpenCode MCP `{name}` must be an object"),
        )
    })?;
    let kind = entry.get("type").and_then(Value::as_str).unwrap_or("");
    let allowed: &[&str] = match kind {
        "local" => &["type", "command", "enabled", "environment"],
        "remote" => &["type", "url", "enabled", "headers"],
        _ => {
            return Err(refuse(
                target,
                artifact,
                provider,
                format!("OpenCode MCP `{name}` has invalid `type`"),
            ));
        }
    };
    if entry.keys().any(|key| !allowed.contains(&key.as_str()))
        || entry.get("enabled") != Some(&Value::Bool(true))
    {
        return Err(refuse(
            target,
            artifact,
            provider,
            format!("OpenCode MCP `{name}` has an unknown member or is not enabled"),
        ));
    }
    if kind == "local" {
        let argv = entry
            .get("command")
            .and_then(Value::as_array)
            .ok_or_else(|| {
                refuse(
                    target,
                    artifact,
                    provider,
                    format!("OpenCode MCP `{name}` local command must be an argv array"),
                )
            })?;
        if argv.is_empty()
            || argv
                .iter()
                .any(|item| item.as_str().is_none_or(str::is_empty))
        {
            return Err(refuse(
                target,
                artifact,
                provider,
                format!("OpenCode MCP `{name}` command must contain non-empty strings"),
            ));
        }
        validate_string_map(entry.get("environment"), target, provider, artifact, name)?;
    } else {
        if entry
            .get("url")
            .and_then(Value::as_str)
            .is_none_or(str::is_empty)
        {
            return Err(refuse(
                target,
                artifact,
                provider,
                format!("OpenCode MCP `{name}` remote URL must be a non-empty string"),
            ));
        }
        validate_string_map(entry.get("headers"), target, provider, artifact, name)?;
    }
    Ok(())
}

fn validate_string_map(
    value: Option<&Value>,
    target: &str,
    provider: &'static str,
    artifact: &ResolvedDeployArtifact,
    name: &str,
) -> Result<(), DeployProviderError> {
    if let Some(value) = value {
        let map = value.as_object().ok_or_else(|| {
            refuse(
                target,
                artifact,
                provider,
                format!("OpenCode MCP `{name}` map member must be an object"),
            )
        })?;
        if map.values().any(|item| item.as_str().is_none()) {
            return Err(refuse(
                target,
                artifact,
                provider,
                format!("OpenCode MCP `{name}` map values must be strings"),
            ));
        }
    }
    Ok(())
}

fn refuse(
    target: &str,
    artifact: &ResolvedDeployArtifact,
    provider: &'static str,
    reason: String,
) -> DeployProviderError {
    DeployProviderError::PluginArtifact {
        target: target.to_owned(),
        artifact: artifact.id.clone(),
        provider,
        reason,
    }
}
