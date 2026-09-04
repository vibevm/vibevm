//! Contract-driven, deterministic planning for terminal VibeVM project scrape.

#![forbid(unsafe_code)]

use std::path::{Component, Path, PathBuf};

pub mod contract;
pub mod glob;
pub mod health;
mod inventory;
pub mod model;
mod plan;
pub mod rewrite;

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
    let same_path_required = matches!(&request.mode, ScrapeMode::Export { .. });
    let capability_blockers =
        health::capability_blockers(&health, capabilities, same_path_required);
    health::add_blockers(&mut health, capability_blockers)
        .map_err(|error| ScrapeError::blocked(error.to_string()))?;
    let rewrite_preparation = rewrite::prepare_rewrites(&project, &contract.value, &inventory)?;
    let plan = plan::build(
        &project,
        &request,
        &contract,
        &inventory,
        &rewrite_preparation.rewrites,
        rewrite_preparation.blockers,
        &health,
        output_identity.as_deref(),
    )?;
    Ok(PreparedScrape {
        contract,
        inventory,
        rewrites: rewrite_preparation.rewrites,
        health,
        plan,
    })
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
            .ok_or_else(|| ScrapeError::contract(format!("contract `{portable}` is absent")))?;
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
mod tests {
    use super::*;
    use sha2::Digest as _;

    fn append_contract(root: &Path, extra: &str) {
        let path = root.join(DEFAULT_CONTRACT.replace('/', std::path::MAIN_SEPARATOR_STR));
        let mut text = std::fs::read_to_string(&path).unwrap();
        text.push_str(extra);
        std::fs::write(path, text).unwrap();
    }

    #[test]
    fn init_is_exclusive_and_emits_a_strict_valid_contract() {
        let root = tempfile::tempdir().unwrap();
        let path = init_contract(root.path()).unwrap();
        let bytes = std::fs::read(path).unwrap();
        contract::Contract::parse(&bytes).unwrap();
        let text = std::str::from_utf8(&bytes).unwrap();
        assert!(text.contains("max_result_bytes = 1048576"));
        assert!(text.contains("termination_grace_seconds = 5"));
        assert!(!text.contains("when ="));
        assert!(init_contract(root.path()).is_err());
    }

    #[test]
    fn health_is_prepared_once_and_wire_contains_no_placeholder_identity() {
        let source = tempfile::tempdir().unwrap();
        std::fs::create_dir(source.path().join("src")).unwrap();
        std::fs::write(
            source.path().join("Cargo.toml"),
            "[package]\nname='health-fixture'\nversion='0.1.0'\nedition='2024'\n",
        )
        .unwrap();
        std::fs::write(source.path().join("src/lib.rs"), "pub fn fixture() {}\n").unwrap();
        init_contract(source.path()).unwrap();
        let prepared = prepare(ScrapeRequest {
            root: source.path().to_path_buf(),
            contract: None,
            mode: ScrapeMode::InPlace,
        })
        .unwrap();
        assert_eq!(
            prepared.health.plan_id,
            prepared.plan.prepared_health.plan_id
        );
        assert!(prepared.health.checks.is_empty());
        assert!(
            prepared
                .health
                .blockers
                .iter()
                .any(|blocker| blocker.message.contains("version probe"))
        );
        assert!(
            !prepared
                .plan
                .blockers
                .iter()
                .any(|blocker| blocker.code == "health-preparation-required")
        );
        let json = serde_json::to_string(&prepared.plan.to_wire().unwrap()).unwrap();
        assert!(json.contains("health_plan_id"));
        assert!(!json.contains("health-preparation-required"));
    }

    #[test]
    fn health_resolver_failure_is_a_typed_blocker_without_fake_wire_row() {
        let source = tempfile::tempdir().unwrap();
        std::fs::create_dir(source.path().join("src")).unwrap();
        std::fs::write(
            source.path().join("Cargo.toml"),
            "[package]\nname='health-fixture'\nversion='0.1.0'\nedition='2024'\n",
        )
        .unwrap();
        std::fs::write(source.path().join("src/lib.rs"), "pub fn fixture() {}\n").unwrap();
        init_contract(source.path()).unwrap();
        std::fs::write(source.path().join("health.py"), "pass\n").unwrap();
        append_contract(
            source.path(),
            r#"
[[healthcheck]]
id = "missing-interpreter"
kind = "custom"
root = "."
source = "health.py"
snapshot = ["health.py"]
interpreter = "definitely-not-a-vibevm-health-tool"
argv = []
protocol = "exit-code"
reads = ["**"]
writes = []
spawn = true
network = "inherit"
timeout_seconds = 1
"#,
        );
        let prepared = prepare(ScrapeRequest {
            root: source.path().to_path_buf(),
            contract: None,
            mode: ScrapeMode::InPlace,
        })
        .unwrap();
        assert!(prepared.plan.blockers.iter().any(|blocker| {
            blocker.code == "health-preparation-failed"
                && blocker.rule_id.as_deref() == Some("missing-interpreter")
        }));
        let wire = prepared.plan.to_wire().unwrap();
        assert!(
            wire.healthchecks.is_empty(),
            "failed rows emit no placeholders"
        );
    }

    #[test]
    fn projected_final_blocks_deleted_health_manifest() {
        let source = tempfile::tempdir().unwrap();
        std::fs::create_dir(source.path().join("src")).unwrap();
        std::fs::write(
            source.path().join("Cargo.toml"),
            "[package]\nname='health-fixture'\nversion='0.1.0'\nedition='2024'\n",
        )
        .unwrap();
        std::fs::write(source.path().join("src/lib.rs"), "pub fn fixture() {}\n").unwrap();
        init_contract(source.path()).unwrap();
        append_contract(
            source.path(),
            r#"
[[classify]]
id = "delete-cargo-manifest"
kind = "delete"
patterns = ["Cargo.toml"]
owner = "vibe"
proof = "contract-assertion-v1"
modified = "delete"
require_match = true
"#,
        );
        let prepared = prepare(ScrapeRequest {
            root: source.path().to_path_buf(),
            contract: None,
            mode: ScrapeMode::InPlace,
        })
        .unwrap();
        assert!(prepared.plan.blockers.iter().any(|blocker| {
            blocker.code == "health-projected-operand-missing"
                && blocker.rule_id.as_deref() == Some("cargo")
        }));
    }

    #[test]
    fn projected_rewrite_cannot_remove_a_required_health_script() {
        let source = tempfile::tempdir().unwrap();
        std::fs::create_dir(source.path().join("src")).unwrap();
        std::fs::write(
            source.path().join("Cargo.toml"),
            "[package]\nname='health-fixture'\nversion='0.1.0'\nedition='2024'\n",
        )
        .unwrap();
        std::fs::write(source.path().join("src/lib.rs"), "pub fn fixture() {}\n").unwrap();
        std::fs::write(
            source.path().join("package.json"),
            r#"{"scripts":{"build":"echo build"}}"#,
        )
        .unwrap();
        std::fs::write(source.path().join("package-lock.json"), "{}").unwrap();
        init_contract(source.path()).unwrap();
        append_contract(
            source.path(),
            r#"
[[rewrite]]
id = "remove-build-script"
kind = "json-member-remove-v1"
path = "package.json"
object = ["scripts"]
members = ["build"]
matches = "exactly-one"

[[healthcheck]]
id = "web"
kind = "npm"
root = "."
manager = "npm"
lockfile = "package-lock.json"
install = "none"
build_script = "build"
tests = "skip"
timeout_seconds = 10
"#,
        );
        let prepared = prepare(ScrapeRequest {
            root: source.path().to_path_buf(),
            contract: None,
            mode: ScrapeMode::InPlace,
        })
        .unwrap();
        assert!(prepared.plan.blockers.iter().any(|blocker| {
            blocker.code == "health-projected-npm-script-missing"
                && blocker.rule_id.as_deref() == Some("web")
        }));
    }

    #[test]
    fn plan_identity_ignores_project_and_external_contract_display_paths() {
        let first = tempfile::tempdir().unwrap();
        let second = tempfile::tempdir().unwrap();
        init_contract(first.path()).unwrap();
        init_contract(second.path()).unwrap();
        let first_plan = prepare(ScrapeRequest {
            root: first.path().to_path_buf(),
            contract: None,
            mode: ScrapeMode::InPlace,
        })
        .unwrap()
        .plan;
        let second_plan = prepare(ScrapeRequest {
            root: second.path().to_path_buf(),
            contract: None,
            mode: ScrapeMode::InPlace,
        })
        .unwrap()
        .plan;
        assert_eq!(first_plan.plan_id, second_plan.plan_id);

        let external_one = tempfile::NamedTempFile::new().unwrap();
        let external_two = tempfile::NamedTempFile::new().unwrap();
        let external_contract =
            DEFAULT_CONTRACT_TEXT.replace("contract = \"delete-last\"", "contract = \"preserve\"");
        std::fs::write(external_one.path(), &external_contract).unwrap();
        std::fs::write(external_two.path(), &external_contract).unwrap();
        let empty_one = tempfile::tempdir().unwrap();
        let empty_two = tempfile::tempdir().unwrap();
        let one = prepare(ScrapeRequest {
            root: empty_one.path().to_path_buf(),
            contract: Some(external_one.path().to_path_buf()),
            mode: ScrapeMode::InPlace,
        })
        .unwrap();
        let two = prepare(ScrapeRequest {
            root: empty_two.path().to_path_buf(),
            contract: Some(external_two.path().to_path_buf()),
            mode: ScrapeMode::InPlace,
        })
        .unwrap();
        assert_ne!(one.contract.display_path, two.contract.display_path);
        assert_eq!(one.plan.plan_id, two.plan.plan_id);
    }

    #[test]
    fn export_output_identity_changes_the_plan_id() {
        let source = tempfile::tempdir().unwrap();
        let destinations = tempfile::tempdir().unwrap();
        init_contract(source.path()).unwrap();
        let prepare_for = |name: &str| {
            prepare(ScrapeRequest {
                root: source.path().to_path_buf(),
                contract: None,
                mode: ScrapeMode::Export {
                    output: destinations.path().join(name),
                },
            })
            .unwrap()
            .plan
            .plan_id
        };
        assert_ne!(prepare_for("one"), prepare_for("two"));
    }

    #[test]
    fn contract_boundary_prunes_only_ancestors_empty_in_the_projected_tree() {
        let source = tempfile::tempdir().unwrap();
        init_contract(source.path()).unwrap();
        std::fs::write(source.path().join("vibevm/keep.txt"), b"keep").unwrap();
        append_contract(
            source.path(),
            r#"

[[classify]]
id = "keep-one"
kind = "keep"
patterns = ["vibevm/keep.txt"]
owner = "project"
require_match = true
"#,
        );
        let prepared = prepare(ScrapeRequest {
            root: source.path().to_path_buf(),
            contract: None,
            mode: ScrapeMode::InPlace,
        })
        .unwrap();
        let crate::model::ContractBoundary::DeleteLast {
            empty_ancestors, ..
        } = prepared.plan.contract_boundary
        else {
            panic!("contained contract is delete-last")
        };
        assert_eq!(empty_ancestors, ["vibevm/scrape"]);
    }

    #[test]
    fn directory_relocation_records_exact_descendant_mapping_and_wire_tree_evidence() {
        let source = tempfile::tempdir().unwrap();
        init_contract(source.path()).unwrap();
        std::fs::create_dir(source.path().join("product")).unwrap();
        std::fs::write(source.path().join("product/a.txt"), b"a").unwrap();
        append_contract(
            source.path(),
            r#"

[[relocate]]
id = "move-product"
from = "product"
to = "moved"
conflict = "refuse"
required = true
"#,
        );
        let prepared = prepare(ScrapeRequest {
            root: source.path().to_path_buf(),
            contract: None,
            mode: ScrapeMode::InPlace,
        })
        .unwrap();
        let mapped = &prepared.plan.relocations[0].mapped_descendants;
        assert_eq!(
            mapped
                .iter()
                .map(|entry| (entry.from.as_str(), entry.to.as_str()))
                .collect::<Vec<_>>(),
            [("product", "moved"), ("product/a.txt", "moved/a.txt"),]
        );
        let wire = prepared.plan.to_wire().unwrap();
        assert_eq!(wire.relocations[0].bytes, "1");
        assert_ne!(
            wire.relocations[0].sha256,
            format!("sha256:{}", "0".repeat(64))
        );
    }

    #[test]
    fn relocation_destination_refuses_an_ancestor_file() {
        let source = tempfile::tempdir().unwrap();
        init_contract(source.path()).unwrap();
        std::fs::create_dir(source.path().join("product")).unwrap();
        std::fs::write(source.path().join("product/a.txt"), b"a").unwrap();
        std::fs::write(source.path().join("occupied"), b"file").unwrap();
        append_contract(
            source.path(),
            r#"

[[relocate]]
id = "move-product"
from = "product"
to = "occupied/child"
conflict = "refuse"
required = true
"#,
        );
        let error = prepare(ScrapeRequest {
            root: source.path().to_path_buf(),
            contract: None,
            mode: ScrapeMode::InPlace,
        })
        .unwrap_err();
        assert!(format!("{error:#}").contains("has file `occupied` as an ancestor"));
    }

    #[test]
    fn managed_block_whole_target_baseline_is_consumed_by_the_plan() {
        let source = tempfile::tempdir().unwrap();
        init_contract(source.path()).unwrap();
        let document = b"head\n<vibevm>\nmanaged\n</vibevm>\ntail\n";
        std::fs::write(source.path().join("AGENTS.md"), document).unwrap();
        let digest = format!("sha256:{:x}", sha2::Sha256::digest(document));
        append_contract(
            source.path(),
            &format!(
                r#"

[[baseline]]
path = "AGENTS.md"
sha256 = "{digest}"

[[rewrite]]
id = "managed"
kind = "managed-block-remove-v1"
paths = ["AGENTS.md"]
marker = "vibevm"
matches = "exactly-one-per-file"
"#
            ),
        );
        let prepared = prepare(ScrapeRequest {
            root: source.path().to_path_buf(),
            contract: None,
            mode: ScrapeMode::InPlace,
        })
        .unwrap();
        assert!(
            prepared
                .rewrites
                .iter()
                .flat_map(|rewrite| &rewrite.spans)
                .all(|span| !span.node.is_empty())
        );
        assert!(
            !prepared
                .plan
                .blockers
                .iter()
                .any(|blocker| blocker.code == "unused-baseline")
        );
    }

    #[test]
    fn modified_keep_residue_blocks_the_projected_final_plan() {
        let source = tempfile::tempdir().unwrap();
        init_contract(source.path()).unwrap();
        std::fs::write(source.path().join("vibevm/residue.txt"), b"residue").unwrap();
        let contract_path = source.path().join(DEFAULT_CONTRACT);
        let contract = std::fs::read_to_string(&contract_path)
            .unwrap()
            .replace("modified = \"refuse\"", "modified = \"keep\"");
        std::fs::write(contract_path, contract).unwrap();
        let prepared = prepare(ScrapeRequest {
            root: source.path().to_path_buf(),
            contract: None,
            mode: ScrapeMode::InPlace,
        })
        .unwrap();
        assert!(
            prepared
                .plan
                .blockers
                .iter()
                .any(|blocker| blocker.code == "projected-final-invalid")
        );
    }

    #[test]
    fn rewritten_source_relocates_with_its_last_prepared_after_image() {
        let source = tempfile::tempdir().unwrap();
        init_contract(source.path()).unwrap();
        let document = b"head\n<vibevm>\nmanaged\n</vibevm>\ntail\n";
        std::fs::write(source.path().join("guide.md"), document).unwrap();
        let digest = format!("sha256:{:x}", sha2::Sha256::digest(document));
        append_contract(
            source.path(),
            &format!(
                r#"

[[baseline]]
path = "guide.md"
sha256 = "{digest}"

[[rewrite]]
id = "strip-guide"
kind = "managed-block-remove-v1"
paths = ["guide.md"]
marker = "vibevm"
matches = "exactly-one-per-file"

[[relocate]]
id = "move-guide"
from = "guide.md"
to = "release/guide.md"
conflict = "refuse"
required = true

[[assert]]
id = "release-has-no-managed-marker"
kind = "text-literal-absent-v1"
patterns = ["release/**"]
needles = ["<vibevm>"]
"#
            ),
        );
        let prepared = prepare(ScrapeRequest {
            root: source.path().to_path_buf(),
            contract: None,
            mode: ScrapeMode::InPlace,
        })
        .unwrap();
        let item = prepared
            .plan
            .items
            .iter()
            .find(|item| item.path == "guide.md")
            .unwrap();
        assert_eq!(item.disposition, crate::model::Disposition::Relocate);
        assert_eq!(prepared.rewrites.len(), 1);
        assert_eq!(prepared.rewrites[0].after_bytes, b"head\ntail\n");
        let wire = prepared.plan.to_wire().unwrap();
        assert_eq!(
            wire.relocations[0].sha256,
            prepared.rewrites[0].after_sha256
        );
        assert_eq!(wire.relocations[0].bytes, "10");
        assert!(!prepared.plan.blockers.iter().any(|blocker| {
            blocker.code == "rewrite-classification-conflict"
                || blocker.code == "projected-final-invalid"
        }));
    }
}
