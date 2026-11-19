//! Explicit local override bindings; ordinary project initialization and SafeFS own writes.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-074#OVERRIDE-CONTROLS");
use crate::{cli::UserOverrideCommand, output};
use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use vibe_core::cancellation::CancellationToken;
use vibe_safefs::{ExternalStore, Project};

const FILE: &str = "override-bindings.toml";
const CAP: usize = 1024 * 1024;
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Identity {
    kind: String,
    root: String,
    node: String,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Binding {
    alias: String,
    id: String,
    enabled: bool,
    identities: Vec<Identity>,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Mapping {
    schema: u32,
    bindings: Vec<Binding>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    user_controls: Vec<UserControl>,
}
impl Default for Mapping {
    fn default() -> Self {
        Self {
            schema: 1,
            bindings: Vec::new(),
            user_controls: Vec::new(),
        }
    }
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct UserControl {
    identity: Identity,
    user_enabled: bool,
}
struct Selected {
    identity: Identity,
    project: Project,
}
fn select(path: &Path) -> Result<Selected> {
    let selected = super::prompt::select(path, None)?;
    let project = Project::open(&selected.node_dir)?;
    let mut command = std::process::Command::new("git");
    command.current_dir(&selected.node_dir).args([
        "rev-parse",
        "--path-format=absolute",
        "--git-common-dir",
    ]);
    let git = command.output();
    let identity = match git {
        Ok(result) if result.status.success() => {
            let path = PathBuf::from(std::str::from_utf8(&result.stdout)?.trim());
            Identity {
                kind: "git".into(),
                root: portable(&path.canonicalize()?)?,
                node: selected.node_rel,
            }
        }
        _ if selected.git => {
            bail!("Git project identity unavailable; override binding was not changed")
        }
        _ => Identity {
            kind: "directory".into(),
            root: portable(&selected.node_dir)?,
            node: ".".into(),
        },
    };
    Ok(Selected { identity, project })
}
fn portable(path: &Path) -> Result<String> {
    path.to_str()
        .map(str::to_owned)
        .context("override binding requires a lossless UTF-8 local locator")
}
fn settings() -> Result<PathBuf> {
    let personal = vibe_core::user_project::admit_root(vibe_core::settings::settings_dir())
        .map_err(|error| anyhow::anyhow!("{error}"))?;
    Ok(personal
        .parent()
        .context("settings root absent")?
        .to_path_buf())
}
fn read(store: &ExternalStore) -> Result<Mapping> {
    let Some(bytes) = store.read_stable_bounded(FILE, CAP)? else {
        return Ok(Mapping::default());
    };
    let mapping: Mapping = toml::from_str(std::str::from_utf8(&bytes.bytes)?)
        .context("invalid override binding mapping; restore it explicitly")?;
    if !matches!(mapping.schema, 1 | 2)
        || (mapping.schema == 1 && !mapping.user_controls.is_empty())
    {
        bail!("unsupported override binding mapping schema");
    }
    let mut aliases = std::collections::BTreeSet::new();
    let mut ids = std::collections::BTreeSet::new();
    let mut identities = Vec::new();
    for row in &mapping.bindings {
        validate_alias(&row.alias)?;
        if row.id.len() != 32
            || !row.id.bytes().all(|c| c.is_ascii_alphanumeric())
            || !aliases.insert(&row.alias)
            || !ids.insert(&row.id)
            || row.identities.is_empty()
        {
            bail!("invalid or duplicate override binding identity");
        }
        for identity in &row.identities {
            if !valid_identity(identity) || identities.contains(identity) {
                bail!("invalid or duplicate override project locator");
            }
            identities.push(identity.clone());
        }
    }
    let mut controls = Vec::new();
    for row in &mapping.user_controls {
        if !valid_identity(&row.identity) || controls.contains(&row.identity) {
            bail!("invalid or duplicate personal layer project control");
        }
        controls.push(row.identity.clone());
    }
    Ok(mapping)
}
fn valid_identity(identity: &Identity) -> bool {
    matches!(identity.kind.as_str(), "git" | "directory")
        && Path::new(&identity.root).is_absolute()
        && !identity.node.is_empty()
        && !identity.node.split('/').any(|part| part == "..")
}
fn validate_alias(alias: &str) -> Result<()> {
    if alias.is_empty()
        || alias.len() > 80
        || !alias
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, b'-' | b'_'))
    {
        bail!("override alias must contain 1-80 ASCII letters, digits, '-' or '_'");
    }
    Ok(())
}
fn override_root(settings: &Path, binding: &Binding) -> PathBuf {
    settings.join("project-overrides").join(&binding.id)
}
fn initialized(root: &Path) -> Result<()> {
    if !matches!(
        vibe_workspace::user_project::status(root),
        vibe_workspace::user_project::Observation::Initialized { .. }
    ) {
        bail!(
            "bound override project is unavailable; restore it explicitly before enabling or binding"
        );
    }
    Ok(())
}
/// Read-only lookup for optional boot; absence creates no store or metadata.
pub(crate) fn selected_root(path: &Path) -> Result<Option<PathBuf>> {
    let Some((root, enabled)) = observed_root(path)? else {
        return Ok(None);
    };
    if !enabled {
        return Ok(None);
    }
    initialized(&root)?;
    Ok(Some(root))
}

/// View all bound states, including disabled/broken roots, without creating metadata.
pub(crate) fn observed_root(path: &Path) -> Result<Option<(PathBuf, bool)>> {
    let selected = select(path)?;
    let settings = settings()?;
    let Some(store) = ExternalStore::open_existing_disjoint(&settings, &selected.project)? else {
        return Ok(None);
    };
    let mapping = read(&store)?;
    let Some(binding) = mapping
        .bindings
        .iter()
        .find(|row| row.identities.contains(&selected.identity))
    else {
        return Ok(None);
    };
    Ok(Some((override_root(&settings, binding), binding.enabled)))
}
pub(crate) fn run(
    ctx: &output::Context,
    command: UserOverrideCommand,
    cancellation: &CancellationToken,
) -> Result<()> {
    use UserOverrideCommand::*;
    let (path, action, alias) = match &command {
        Init { path, alias } => (path, "init", Some(alias.as_str())),
        Bind { path, alias } => (path, "bind", Some(alias.as_str())),
        Status { path } => (path, "status", None),
        Enable { path } => (path, "enable", None),
        Disable { path } => (path, "disable", None),
    };
    if let Some(alias) = alias {
        validate_alias(alias)?;
    }
    let selected = select(path)?;
    let settings = settings()?;
    let mut store = ExternalStore::open_existing_disjoint(&settings, &selected.project)?;
    let mut mapping = if let Some(store) = &store {
        read(store)?
    } else {
        Mapping::default()
    };
    let mutation = action != "status";
    if mutation {
        cancellation.checkpoint()?;
        // Settings/bootstrap locks may persist even when later preparation fails.
        cancellation.publishing();
        if store.is_none() {
            store = Some(ExternalStore::open_or_create_disjoint(
                &settings,
                &selected.project,
            )?);
        }
    }
    let _lock = if mutation {
        Some(
            store
                .as_ref()
                .context("settings unavailable")?
                .open_and_lock_project("user-override-bindings")?,
        )
    } else {
        None
    };
    if mutation {
        mapping = read(store.as_ref().context("settings unavailable")?)?;
    }
    let owner = mapping
        .bindings
        .iter()
        .position(|row| row.identities.contains(&selected.identity));
    let mut changed = false;
    let index = match action {
        "init" => {
            let alias = alias.context("override alias required")?;
            if let Some(index) = mapping.bindings.iter().position(|row| row.alias == alias) {
                if owner != Some(index) {
                    bail!(
                        "override alias already belongs to another project; use explicit override bind"
                    );
                }
                initialized(&override_root(&settings, &mapping.bindings[index]))?;
                Some(index)
            } else {
                if owner.is_some() {
                    bail!("project already has an override alias; refusing implicit reassignment");
                }
                // tempfile supplies only a random spelling; the held SafeFS parent
                // exclusively reserves the final directory. No path-owned cleanup.
                let store = store.as_ref().context("settings unavailable")?;
                let parent = store.root()?.ensure_child("project-overrides")?;
                let allocation = tempfile::Builder::new()
                    .prefix("")
                    .rand_bytes(32)
                    .disable_cleanup(true)
                    .make_in(parent.join("."), |path| {
                        let name = path
                            .file_name()
                            .and_then(|name| name.to_str())
                            .ok_or_else(|| std::io::Error::other("opaque id unavailable"))?;
                        parent
                            .create_child_exclusive(name)
                            .map_err(std::io::Error::other)
                    })?;
                let id = allocation
                    .path()
                    .file_name()
                    .and_then(|name| name.to_str())
                    .context("opaque id unavailable")?
                    .to_owned();
                let root = settings.join("project-overrides").join(&id);
                let allocated_identity = allocation.as_file().identity()?;
                let outer = cancellation.clone();
                let root_token =
                    CancellationToken::new().with_poll(std::sync::Arc::new(move || {
                        if outer.is_cancelled() {
                            let _ = outer.checkpoint();
                            true
                        } else {
                            false
                        }
                    }));
                match vibe_workspace::user_project::init(&root, &root_token).with_context(|| {
                    format!(
                        "override initialization incomplete at {}; mapping not published",
                        root.display()
                    )
                })? {
                    vibe_workspace::user_project::Initialized::Created(_) => {}
                    vibe_workspace::user_project::Initialized::AlreadyInitialized(_) => {
                        bail!("reserved override directory was replaced; no project was adopted")
                    }
                }
                if Project::open(&root)?.root_identity()? != allocated_identity {
                    bail!(
                        "reserved override directory changed; mapping not published; inspect {}",
                        root.display()
                    );
                }
                mapping.bindings.push(Binding {
                    alias: alias.into(),
                    id,
                    enabled: true,
                    identities: vec![selected.identity.clone()],
                });
                changed = true;
                Some(mapping.bindings.len() - 1)
            }
        }
        "bind" => {
            let index = mapping
                .bindings
                .iter()
                .position(|row| Some(row.alias.as_str()) == alias)
                .context("override alias does not exist; initialize it explicitly")?;
            if owner.is_some_and(|owner| owner != index) {
                bail!("project is already bound to another override alias");
            }
            initialized(&override_root(&settings, &mapping.bindings[index]))?;
            if owner.is_none() {
                mapping.bindings[index].identities.push(selected.identity);
                changed = true;
            }
            Some(index)
        }
        "enable" | "disable" => {
            let index =
                owner.context("project has no override binding; initialize or bind explicitly")?;
            if action == "enable" {
                initialized(&override_root(&settings, &mapping.bindings[index]))?;
            }
            let enabled = action == "enable";
            changed = mapping.bindings[index].enabled != enabled;
            mapping.bindings[index].enabled = enabled;
            Some(index)
        }
        _ => owner,
    };
    if mutation && mapping.schema != 2 {
        mapping.schema = 2;
        changed = true;
    }
    if changed {
        cancellation.checkpoint().context("override operation stopped before mapping publication; inspect project-overrides for an unbound initialized root")?;
        let bytes = toml::to_string(&mapping)?;
        store
            .as_ref()
            .context("settings unavailable")?
            .write_durable(FILE, bytes.as_bytes())
            .map_err(|error| anyhow::anyhow!("override mapping publication incomplete: {error}"))?;
    }
    if mutation {
        cancellation.committed();
    }
    let binding = index.map(|index| &mapping.bindings[index]);
    let root = binding.map(|binding| override_root(&settings, binding));
    let state = match &root {
        None => "absent",
        Some(root) if initialized(root).is_ok() => "initialized",
        Some(_) => "unavailable",
    };
    let report = vibe_wire::generated::user_override_report::UserOverrideReport {
        command: format!("user:override:{action}"),
        ok: state != "unavailable" || action == "disable",
        state: state.into(),
        alias: binding.map(|binding| binding.alias.clone()),
        root: root.as_deref().map(vibe_core::machine_json_path),
        enabled: binding.is_some_and(|binding| binding.enabled),
    };
    if ctx.is_json() {
        ctx.emit_json(&report)?;
    } else {
        ctx.summary(&format!(
            "override {state}: {}{}",
            report.alias.as_deref().unwrap_or("unbound"),
            report
                .root
                .as_ref()
                .map(|root| format!(" — {root}"))
                .unwrap_or_default()
        ));
    }
    if !report.ok {
        bail!("bound override project is unavailable");
    }
    Ok(())
}

/// Per-project personal loading is independent from override bindings.
pub(crate) fn user_enabled(path: &Path) -> Result<bool> {
    let selected = select(path)?;
    let settings = settings()?;
    let Some(store) = ExternalStore::open_existing_disjoint(&settings, &selected.project)? else {
        return Ok(true);
    };
    let mapping = read(&store)?;
    Ok(mapping
        .user_controls
        .iter()
        .find(|row| row.identity == selected.identity)
        .is_none_or(|row| row.user_enabled))
}
pub(crate) fn set_user_enabled(
    ctx: &output::Context,
    path: &Path,
    enabled: bool,
    cancellation: &CancellationToken,
) -> Result<()> {
    let selected = select(path)?;
    let settings = settings()?;
    cancellation.checkpoint()?;
    cancellation.publishing();
    let store = ExternalStore::open_or_create_disjoint(&settings, &selected.project)?;
    let _lock = store.open_and_lock_project("user-override-bindings")?;
    let mut mapping = read(&store)?;
    let mut changed = mapping.schema != 2;
    mapping.schema = 2;
    if let Some(row) = mapping
        .user_controls
        .iter_mut()
        .find(|row| row.identity == selected.identity)
    {
        changed |= row.user_enabled != enabled;
        row.user_enabled = enabled;
    } else {
        mapping.user_controls.push(UserControl {
            identity: selected.identity,
            user_enabled: enabled,
        });
        changed = true;
    }
    if changed {
        cancellation.checkpoint()?;
        store.write_durable(FILE, toml::to_string(&mapping)?.as_bytes())?;
    }
    cancellation.committed();
    if ctx.is_json() {
        ctx.emit_json(&serde_json::json!({"ok":true,"command":if enabled {"user:enable"} else {"user:disable"},"enabled":enabled}))
    } else {
        ctx.summary(if enabled {
            "personal layer enabled for this project"
        } else {
            "personal layer disabled for this project"
        });
        Ok(())
    }
}
