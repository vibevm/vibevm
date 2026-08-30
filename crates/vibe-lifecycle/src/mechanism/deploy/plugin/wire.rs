//! Exact bounded process and list wire for Claude/Codex plugin state.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS");

use std::ffi::OsString;
use std::path::Path;

use serde_json::Value;

use super::client::PluginClient;
use crate::mechanism::deploy::protocol::DeployTargetRequest;
use crate::mechanism::error::{DeployProviderError, preview};
use crate::process::{ProcessRunner, ProcessSpec, StreamMode, client_environment};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ClientVersion {
    pub(crate) rendered: String,
    pub(crate) major: u64,
    pub(crate) minor: u64,
    pub(crate) patch: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct InstalledPlugin {
    pub(crate) version: String,
    pub(crate) enabled: bool,
    pub(crate) user_scope: bool,
}

impl InstalledPlugin {
    pub(crate) const fn active_user(&self) -> bool {
        self.enabled && self.user_scope
    }
}

pub(crate) fn probe_version(
    runner: &dyn ProcessRunner,
    client: PluginClient,
    request: &DeployTargetRequest<'_>,
) -> Result<ClientVersion, DeployProviderError> {
    let bytes = run(runner, client, request, "version probe", &["--version"])?;
    let text = std::str::from_utf8(&bytes).map_err(|_| {
        command_error(
            client,
            request,
            "version probe",
            "stdout is not UTF-8".to_owned(),
        )
    })?;
    let parsed = parse_version(text).ok_or_else(|| DeployProviderError::ClientVersion {
        target: request.target.id.clone(),
        client: client.as_str(),
        found: preview(text.trim()),
        supported: client.supported_version(),
    })?;
    let expected = client.version_pair();
    if (parsed.major, parsed.minor) != expected {
        return Err(DeployProviderError::ClientVersion {
            target: request.target.id.clone(),
            client: client.as_str(),
            found: preview(text.trim()),
            supported: client.supported_version(),
        });
    }
    Ok(parsed)
}

pub(crate) fn list(
    runner: &dyn ProcessRunner,
    client: PluginClient,
    request: &DeployTargetRequest<'_>,
    plugin: &str,
    marketplace: &str,
    coordinate: &str,
) -> Result<Option<InstalledPlugin>, DeployProviderError> {
    let bytes = run(
        runner,
        client,
        request,
        "plugin list",
        &["plugin", "list", "--json"],
    )?;
    let document: Value = serde_json::from_slice(&bytes).map_err(|error| {
        command_error(
            client,
            request,
            "plugin list",
            format!("stdout is not the documented JSON shape: {error}"),
        )
    })?;
    match client {
        PluginClient::Claude => claude_list(request, &document, coordinate),
        PluginClient::Codex => codex_list(request, &document, plugin, marketplace, coordinate),
        PluginClient::OpenCode => Err(command_error(
            client,
            request,
            "plugin list",
            "OpenCode never invokes a plugin subcommand".to_owned(),
        )),
    }
}

pub(crate) fn marketplace_add(
    runner: &dyn ProcessRunner,
    client: PluginClient,
    request: &DeployTargetRequest<'_>,
    root: &Path,
) -> Result<(), DeployProviderError> {
    let root = root.as_os_str().to_owned();
    let args = match client {
        PluginClient::Claude => vec![
            "plugin".into(),
            "marketplace".into(),
            "add".into(),
            "--scope".into(),
            "user".into(),
            root,
        ],
        PluginClient::Codex => vec![
            "plugin".into(),
            "marketplace".into(),
            "add".into(),
            "--json".into(),
            root,
        ],
        PluginClient::OpenCode => unreachable!("OpenCode has no marketplace"),
    };
    run_os(runner, client, request, "marketplace add", args).map(drop)
}

pub(crate) fn install(
    runner: &dyn ProcessRunner,
    client: PluginClient,
    request: &DeployTargetRequest<'_>,
    coordinate: &str,
) -> Result<(), DeployProviderError> {
    let args = match client {
        PluginClient::Claude => vec![
            "plugin".into(),
            "install".into(),
            "--scope".into(),
            "user".into(),
            coordinate.into(),
        ],
        PluginClient::Codex => vec![
            "plugin".into(),
            "add".into(),
            "--json".into(),
            coordinate.into(),
        ],
        PluginClient::OpenCode => unreachable!("OpenCode has no plugin command"),
    };
    run_os(runner, client, request, "plugin install", args).map(drop)
}

pub(crate) fn remove(
    runner: &dyn ProcessRunner,
    client: PluginClient,
    request: &DeployTargetRequest<'_>,
    coordinate: &str,
) -> Result<(), DeployProviderError> {
    let args = match client {
        PluginClient::Claude => vec![
            "plugin".into(),
            "uninstall".into(),
            "--scope".into(),
            "user".into(),
            coordinate.into(),
        ],
        PluginClient::Codex => vec![
            "plugin".into(),
            "remove".into(),
            "--json".into(),
            coordinate.into(),
        ],
        PluginClient::OpenCode => unreachable!("OpenCode has no plugin command"),
    };
    run_os(runner, client, request, "plugin remove", args).map(drop)
}

fn run(
    runner: &dyn ProcessRunner,
    client: PluginClient,
    request: &DeployTargetRequest<'_>,
    operation: &'static str,
    args: &[&str],
) -> Result<Vec<u8>, DeployProviderError> {
    run_os(
        runner,
        client,
        request,
        operation,
        args.iter().map(OsString::from).collect(),
    )
}

fn run_os(
    runner: &dyn ProcessRunner,
    client: PluginClient,
    request: &DeployTargetRequest<'_>,
    operation: &'static str,
    args: Vec<OsString>,
) -> Result<Vec<u8>, DeployProviderError> {
    let executable = client.executable(request.clients);
    let program =
        executable
            .resolved_path()
            .ok_or_else(|| DeployProviderError::ClientExecutable {
                target: request.target.id.clone(),
                client: client.as_str(),
                command: executable.command().to_owned(),
                reason: "the command surface reported it missing".to_owned(),
            })?;
    if !program.is_absolute() {
        return Err(DeployProviderError::ClientExecutable {
            target: request.target.id.clone(),
            client: client.as_str(),
            command: executable.command().to_owned(),
            reason: format!("`{}` is not absolute", program.display()),
        });
    }
    let private = client.private_root(request.user_home);
    let env = client_environment(
        request.user_home,
        private.as_ref().map(|(key, path)| (*key, path.as_path())),
    );
    let output = runner
        .run(&ProcessSpec {
            program: program.to_path_buf(),
            args,
            cwd: request.project_root.to_path_buf(),
            env,
            stdin: None,
            stdout: StreamMode::Capture,
            stderr: StreamMode::Capture,
            scratch: request
                .staging
                .unwrap_or(request.project_root)
                .to_path_buf(),
        })
        .map_err(|error| command_error(client, request, operation, error.to_string()))?;
    if output.stdout_truncated || output.stderr_truncated {
        return Err(command_error(
            client,
            request,
            operation,
            "bounded stdout/stderr was truncated".to_owned(),
        ));
    }
    if output.code != Some(0) {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(command_error(
            client,
            request,
            operation,
            format!("exit {:?}: {}", output.code, preview(stderr.trim())),
        ));
    }
    Ok(output.stdout)
}

fn parse_version(text: &str) -> Option<ClientVersion> {
    for token in text.split_whitespace() {
        let trimmed = token.trim_matches(|character: char| !character.is_ascii_digit());
        let core = trimmed
            .trim_end_matches(|character: char| !character.is_ascii_digit() && character != '.');
        let pieces: Vec<&str> = core.split('.').collect();
        if pieces.len() == 3
            && pieces
                .iter()
                .all(|piece| !piece.is_empty() && piece.bytes().all(|byte| byte.is_ascii_digit()))
        {
            return Some(ClientVersion {
                rendered: core.to_owned(),
                major: pieces[0].parse().ok()?,
                minor: pieces[1].parse().ok()?,
                patch: pieces[2].parse().ok()?,
            });
        }
    }
    None
}

fn claude_list(
    request: &DeployTargetRequest<'_>,
    document: &Value,
    coordinate: &str,
) -> Result<Option<InstalledPlugin>, DeployProviderError> {
    let array = document.as_array().ok_or_else(|| {
        list_shape(
            PluginClient::Claude,
            request,
            "root must be the installed array".to_owned(),
        )
    })?;
    let mut matching = Vec::new();
    for value in array {
        let Some(object) = value.as_object() else {
            continue;
        };
        if object.get("id") != Some(&Value::String(coordinate.to_owned())) {
            continue;
        }
        let version = required_string(object, "version", PluginClient::Claude, request)?;
        let scope = required_string(object, "scope", PluginClient::Claude, request)?;
        let enabled = required_bool(object, "enabled", PluginClient::Claude, request)?;
        matching.push(InstalledPlugin {
            version,
            enabled,
            user_scope: scope == "user",
        });
    }
    unique(matching, PluginClient::Claude, request, coordinate)
}

fn codex_list(
    request: &DeployTargetRequest<'_>,
    document: &Value,
    plugin: &str,
    marketplace: &str,
    coordinate: &str,
) -> Result<Option<InstalledPlugin>, DeployProviderError> {
    let root = document.as_object().ok_or_else(|| {
        list_shape(
            PluginClient::Codex,
            request,
            "root must be `{installed,available}`".to_owned(),
        )
    })?;
    if root.len() != 2 || !root.contains_key("installed") || !root.contains_key("available") {
        return Err(list_shape(
            PluginClient::Codex,
            request,
            "root carries exactly `installed` and `available`".to_owned(),
        ));
    }
    let installed = root
        .get("installed")
        .and_then(Value::as_array)
        .ok_or_else(|| {
            list_shape(
                PluginClient::Codex,
                request,
                "`installed` must be an array".to_owned(),
            )
        })?;
    let available = root
        .get("available")
        .and_then(Value::as_array)
        .ok_or_else(|| {
            list_shape(
                PluginClient::Codex,
                request,
                "`available` must be an array".to_owned(),
            )
        })?;
    let matching = codex_matches(request, installed, plugin, marketplace, coordinate)?;
    let available_matching = codex_matches(request, available, plugin, marketplace, coordinate)?;
    if available_matching.len() > 1 {
        return Err(list_shape(
            PluginClient::Codex,
            request,
            format!("duplicate/ambiguous available coordinate `{coordinate}`"),
        ));
    }
    unique(matching, PluginClient::Codex, request, coordinate)
}

fn codex_matches(
    request: &DeployTargetRequest<'_>,
    entries: &[Value],
    plugin: &str,
    marketplace: &str,
    coordinate: &str,
) -> Result<Vec<InstalledPlugin>, DeployProviderError> {
    let mut matching = Vec::new();
    for value in entries {
        let Some(object) = value.as_object() else {
            continue;
        };
        let by_id = object.get("pluginId") == Some(&Value::String(coordinate.to_owned()));
        let by_parts = object.get("name") == Some(&Value::String(plugin.to_owned()))
            && object.get("marketplaceName") == Some(&Value::String(marketplace.to_owned()));
        if !by_id && !by_parts {
            continue;
        }
        for (member, expected) in [
            ("pluginId", coordinate),
            ("name", plugin),
            ("marketplaceName", marketplace),
        ] {
            if required_string(object, member, PluginClient::Codex, request)? != expected {
                return Err(list_shape(
                    PluginClient::Codex,
                    request,
                    format!("matching entry has wrong `{member}`"),
                ));
            }
        }
        let version = required_string(object, "version", PluginClient::Codex, request)?;
        let installed = required_bool(object, "installed", PluginClient::Codex, request)?;
        let enabled = required_bool(object, "enabled", PluginClient::Codex, request)?;
        matching.push(InstalledPlugin {
            version,
            enabled: installed && enabled,
            user_scope: true,
        });
    }
    Ok(matching)
}

fn unique(
    mut found: Vec<InstalledPlugin>,
    client: PluginClient,
    request: &DeployTargetRequest<'_>,
    coordinate: &str,
) -> Result<Option<InstalledPlugin>, DeployProviderError> {
    if found.len() > 1 {
        return Err(list_shape(
            client,
            request,
            format!("duplicate/ambiguous matching coordinate `{coordinate}`"),
        ));
    }
    Ok(found.pop())
}

fn required_string(
    object: &serde_json::Map<String, Value>,
    member: &str,
    client: PluginClient,
    request: &DeployTargetRequest<'_>,
) -> Result<String, DeployProviderError> {
    object
        .get(member)
        .and_then(Value::as_str)
        .map(str::to_owned)
        .ok_or_else(|| {
            list_shape(
                client,
                request,
                format!("matching entry member `{member}` must be a string"),
            )
        })
}

fn required_bool(
    object: &serde_json::Map<String, Value>,
    member: &str,
    client: PluginClient,
    request: &DeployTargetRequest<'_>,
) -> Result<bool, DeployProviderError> {
    object.get(member).and_then(Value::as_bool).ok_or_else(|| {
        list_shape(
            client,
            request,
            format!("matching entry member `{member}` must be a boolean"),
        )
    })
}

fn list_shape(
    client: PluginClient,
    request: &DeployTargetRequest<'_>,
    reason: String,
) -> DeployProviderError {
    command_error(client, request, "plugin list", reason)
}

fn command_error(
    client: PluginClient,
    request: &DeployTargetRequest<'_>,
    operation: &'static str,
    reason: String,
) -> DeployProviderError {
    DeployProviderError::ClientCommand {
        target: request.target.id.clone(),
        client: client.as_str(),
        operation,
        reason,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_parser_finds_one_semver_token() {
        assert_eq!(
            parse_version("Claude Code 2.1.7\n").unwrap().rendered,
            "2.1.7"
        );
        assert_eq!(parse_version("codex-cli 0.148.2").unwrap().minor, 148);
        assert!(parse_version("version unknown").is_none());
        assert!(parse_version("1.17").is_none());
    }
}
