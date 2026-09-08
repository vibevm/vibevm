//! Contract-driven, deterministic planning for terminal VibeVM project scrape.

#![forbid(unsafe_code)]

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-056#IMPL-A");

use std::path::{Component, Path, PathBuf};

pub mod contract;
pub mod glob;
pub mod health;
mod inventory;
pub mod model;
mod plan;
pub mod rewrite;
pub mod transaction;

pub use model::{PreparedScrape, ScrapeError, ScrapeMode, ScrapeRequest};

const DEFAULT_CONTRACT: &str = "vibevm/scrape/contract.toml";
const CONTRACT_CAP: usize = 1024 * 1024;

/// Parse, observe and fully prepare a scrape without mutating the project.
pub fn prepare(request: ScrapeRequest) -> Result<PreparedScrape, ScrapeError> {
    if !request.root.is_absolute() {
        return Err(ScrapeError::request("project root must be absolute"));
    }
    let project = vibe_safefs::Project::open(&request.root)
        .map_err(|error| ScrapeError::request(format!("opening project root: {error:#}")))?;
    let output_identity = validate_mode(&request, &project)?;
    let contract = load_contract(&request, &project)?;
    let inventory = inventory::collect(&project)?;
    let mut health_resolver = health::SystemHealthResolver::new(&project);
    let mut health = health::prepare(&project, &contract.value, &inventory, &mut health_resolver)
        .map_err(|error| ScrapeError::blocked(error.to_string()))?;
    let platform = health::LocalProcessBackend::new();
    let capabilities = health::HealthBackend::capabilities(&platform);
    // Epoch-1 execution uses an isolated before copy and transaction-owned
    // final-path execution guarded by complete pre/post tree reproof.
    let same_path_required = false;
    let capability_blockers =
        health::capability_blockers(&health, capabilities, same_path_required);
    health::add_blockers(&mut health, capability_blockers)
        .map_err(|error| ScrapeError::blocked(error.to_string()))?;
    let rewrite_preparation = rewrite::prepare_rewrites(&project, &contract.value, &inventory)?;
    let preparation_blockers = platform_blockers(rewrite_preparation.blockers);
    let plan = plan::build(
        &project,
        &request,
        &contract,
        &inventory,
        &rewrite_preparation.rewrites,
        preparation_blockers,
        &health,
        output_identity.as_deref(),
    )?;
    Ok(PreparedScrape {
        contract,
        inventory,
        rewrites: rewrite_preparation.rewrites,
        health,
        plan,
        mode: request.mode,
    })
}

#[cfg(windows)]
fn platform_blockers(blockers: Vec<model::Blocker>) -> Vec<model::Blocker> {
    blockers
}

#[cfg(not(windows))]
fn platform_blockers(mut blockers: Vec<model::Blocker>) -> Vec<model::Blocker> {
    blockers.push(model::Blocker::new(
        "scrape-platform-unsupported",
        "scrape mutation and recovery are unsupported outside Windows in epoch 1",
    ));
    blockers
}

/// Parse and plan the selected contract read-only.
pub fn check_contract(request: ScrapeRequest) -> Result<PreparedScrape, ScrapeError> {
    prepare(request)
}

/// Create the conservative schema-1 default contract, refusing an existing
/// file. The parent walk rejects links before the exclusive create.
pub fn init_contract(root: &Path) -> Result<PathBuf, ScrapeError> {
    if !root.is_absolute() {
        return Err(ScrapeError::request("project root must be absolute"));
    }
    let project = vibe_safefs::Project::open(root)
        .map_err(|error| ScrapeError::request(format!("opening project root: {error:#}")))?;
    let parent = project
        .dir(&["vibevm", "scrape"], true)
        .map_err(|error| ScrapeError::io(format!("creating contract parent: {error:#}")))?;
    let path = root.join(DEFAULT_CONTRACT.replace('/', std::path::MAIN_SEPARATOR_STR));
    project
        .publish_new_in(&parent, "contract.toml", DEFAULT_CONTRACT_TEXT.as_bytes())
        .map_err(|error| {
            ScrapeError::io(format!(
                "creating `{}` without replacement: {error}",
                path.display()
            ))
        })?;
    Ok(path)
}

fn validate_mode(
    request: &ScrapeRequest,
    project: &vibe_safefs::Project,
) -> Result<Option<String>, ScrapeError> {
    let ScrapeMode::Export { output } = &request.mode else {
        return Ok(None);
    };
    if !output.is_absolute() {
        return Err(ScrapeError::request(
            "scrape export output must be absolute",
        ));
    }
    let pinned = vibe_safefs::Project::pin_absent_path(output).map_err(|error| {
        ScrapeError::request(format!(
            "pinning absent scrape export output `{}`: {error:#}",
            output.display()
        ))
    })?;
    if pinned.descends_from(project).map_err(|error| {
        ScrapeError::request(format!("comparing export/source ancestry: {error:#}"))
    })? {
        return Err(ScrapeError::request(
            "scrape export output must be disjoint from the source root",
        ));
    }
    Ok(Some(pinned.identity_token()))
}

fn load_contract(
    request: &ScrapeRequest,
    project: &vibe_safefs::Project,
) -> Result<model::ContractSnapshot, ScrapeError> {
    let selected_default = request.contract.is_none();
    let selected = request
        .contract
        .clone()
        .unwrap_or_else(|| PathBuf::from(DEFAULT_CONTRACT));
    let (absolute, display, contained, snapshot) = if selected.is_absolute() {
        let pinned = vibe_safefs::Project::pin_absolute_file(&selected).map_err(|error| {
            ScrapeError::contract(format!(
                "pinning contract `{}` no-follow: {error:#}",
                selected.display()
            ))
        })?;
        let relative = pinned.relative_to(project).map_err(|error| {
            ScrapeError::contract(format!("comparing contract/project ancestry: {error:#}"))
        })?;
        let snapshot = pinned
            .read_snapshot_bounded(project, CONTRACT_CAP)
            .map_err(|error| {
                ScrapeError::contract(format!("reading contract stably: {error:#}"))
            })?;
        let contained = relative.is_some();
        let display = relative.unwrap_or_else(|| selected.display().to_string());
        (selected, display, contained, snapshot)
    } else {
        let portable = portable_display(&selected)?;
        crate::glob::PortablePath::parse(&portable)?;
        let snapshot = project
            .read_file_snapshot_bounded(&portable, CONTRACT_CAP)
            .map_err(|error| {
                ScrapeError::contract(format!("reading contract `{portable}` stably: {error:#}"))
            })?
            .ok_or_else(|| {
                if selected_default {
                    ScrapeError::contract(format!(
                        "not a Vibe project: default scrape contract `{portable}` is absent"
                    ))
                } else {
                    ScrapeError::contract(format!("contract `{portable}` is absent"))
                }
            })?;
        let absolute = request
            .root
            .join(portable.replace('/', std::path::MAIN_SEPARATOR_STR));
        (absolute, portable, true, snapshot)
    };
    let sha256 = format!("sha256:{}", snapshot.sha256);
    let identity = snapshot.identity;
    let bytes = snapshot.bytes;
    let value = contract::Contract::parse(&bytes)?;
    match (contained, value.commit.contract) {
        (true, contract::ContractAction::DeleteLast)
        | (false, contract::ContractAction::Preserve) => {}
        (true, _) => {
            return Err(ScrapeError::contract(
                "a contained contract requires commit.contract = delete-last",
            ));
        }
        (false, _) => {
            return Err(ScrapeError::contract(
                "an external contract requires commit.contract = preserve",
            ));
        }
    }
    Ok(model::ContractSnapshot {
        source_path: absolute,
        display_path: display,
        contained,
        bytes,
        sha256,
        identity,
        value,
    })
}

fn portable_display(path: &Path) -> Result<String, ScrapeError> {
    let mut parts = Vec::new();
    for component in path.components() {
        match component {
            Component::Normal(value) => parts.push(
                value
                    .to_str()
                    .ok_or_else(|| ScrapeError::request("contract path is not UTF-8"))?,
            ),
            _ => {
                return Err(ScrapeError::request(format!(
                    "contract path `{}` is not a portable relative literal",
                    path.display()
                )));
            }
        }
    }
    Ok(parts.join("/"))
}

const DEFAULT_CONTRACT_TEXT: &str = r#"schema = 1
id = "org.example.scrape"

[policy]
unclassified = "refuse"
links = "refuse"
concurrent_change = "refuse"

[scope]
closed_roots = ["vibevm", ".vibe"]
outside = "implicit-keep"

[commit]
contract = "delete-last"

[[classify]]
id = "remove-vibevm"
kind = "delete"
patterns = ["vibevm", "vibevm/**"]
owner = "vibe"
proof = "contract-assertion-v1"
modified = "refuse"
require_match = true

[[classify]]
id = "remove-vibe-state"
kind = "delete"
patterns = [".vibe", ".vibe/**", "vibe.toml", "vibe.lock"]
owner = "vibe"
proof = "contract-assertion-v1"
modified = "refuse"
require_match = false

[[assert]]
id = "vibe-paths-absent"
kind = "paths-absent-v1"
patterns = ["vibevm", "vibevm/**", ".vibe", ".vibe/**", "vibe.toml", "vibe.lock"]

[health]
baseline = "strict"
before_failure = "refuse"
after_failure = "rollback"
parallel = false
network = "tool-offline"
max_stdout_bytes = 1048576
max_stderr_bytes = 1048576
max_result_bytes = 1048576
termination_grace_seconds = 5

[[healthcheck]]
id = "cargo"
kind = "cargo"
root = "."
build = "check"
workspace = true
locked = true
all_targets = true
tests = "skip"
profile = "dev"
features = []
timeout_seconds = 900
"#;

#[cfg(test)]
mod tests;
