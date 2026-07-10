//! `rust-ai-native init` — bootstrap a project's discipline surface: the
//! two engine policies (`conform.toml`, `specmap.toml`), the BROWNFIELD §3
//! registries under `discipline/registry/`, and the `[[external_specs]]`
//! resolution entries discovered from the materialised `vibedeps/` slots —
//! so a fresh consumer reaches a runnable floor without reading the
//! engines' source. Everything is generated from the tree's actual
//! topology; nothing existing is ever overwritten without `--force`.

use std::path::Path;

use anyhow::{Context, Result};

/// What `init` should generate and how.
pub struct InitOptions {
    /// The `spec://<namespace>/…` segment for the project's own units.
    /// `None` → the root directory's name.
    pub namespace: Option<String>,
    /// Overwrite files init owns even when they already exist.
    pub force: bool,
}

/// One `[[external_specs]]` discovery: an installed package whose spec tree
/// can resolve this project's cross-package citations.
struct ExternalSpec {
    namespace: String,
    root: String,
}

/// Scan `vibedeps/<slot>/<version>/` for installed packages that carry a
/// `spec/` tree, reading each slot's `vibe.toml` `[package] name` as its
/// namespace. Deterministic order (sorted by slot path).
fn discover_external_specs(root: &Path) -> Vec<ExternalSpec> {
    let mut out = Vec::new();
    let vibedeps = root.join("vibedeps");
    let Ok(slots) = std::fs::read_dir(&vibedeps) else {
        return out;
    };
    let mut slot_dirs: Vec<_> = slots
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.is_dir())
        .collect();
    slot_dirs.sort();
    for slot in slot_dirs {
        let Ok(versions) = std::fs::read_dir(&slot) else {
            continue;
        };
        let mut version_dirs: Vec<_> = versions
            .filter_map(Result::ok)
            .map(|e| e.path())
            .filter(|p| p.is_dir())
            .collect();
        version_dirs.sort();
        for vdir in version_dirs {
            if !vdir.join("spec").is_dir() {
                continue;
            }
            let Ok(manifest) = std::fs::read_to_string(vdir.join("vibe.toml")) else {
                continue;
            };
            let Ok(table) = manifest.parse::<toml::Table>() else {
                continue;
            };
            let Some(name) = table
                .get("package")
                .and_then(|p| p.get("name"))
                .and_then(|n| n.as_str())
            else {
                continue;
            };
            let rel = vdir.join("spec");
            let rel = rel.strip_prefix(root).unwrap_or(&rel);
            out.push(ExternalSpec {
                namespace: name.to_string(),
                root: rel.to_string_lossy().replace('\\', "/"),
            });
        }
    }
    out
}

/// Crate-shaped subdirectories of `crates/` (a dir carrying a Cargo.toml),
/// sorted — the pre-adoption exemption list for a workspace layout.
fn workspace_crates(root: &Path) -> Vec<String> {
    let mut names = Vec::new();
    if let Ok(rd) = std::fs::read_dir(root.join("crates")) {
        for e in rd.filter_map(Result::ok) {
            if e.path().is_dir() && e.path().join("Cargo.toml").exists() {
                names.push(e.file_name().to_string_lossy().into_owned());
            }
        }
    }
    names.sort();
    names
}

/// The single-crate label: the project directory's basename — the name
/// the scanner attributes every scanned file to, and the name the tree
/// invariant derives for a literal `.` root. The `[package] name` is
/// deliberately NOT used: the engine never reads Cargo.toml for
/// attribution, so a policy keyed by the manifest name stops matching
/// the scan the moment the two differ — the gates then green by
/// vacuity and the invariant refuses the entry as a phantom. A missing
/// Cargo.toml still means "no root crate to classify".
fn root_crate_name(root: &Path) -> Option<String> {
    if !root.join("Cargo.toml").exists() {
        return None;
    }
    let resolved = std::path::absolute(root).unwrap_or_else(|_| root.to_path_buf());
    resolved
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
}

/// Write `path` unless it exists (or `force`); returns whether it wrote.
fn write_once(path: &Path, body: &str, force: bool) -> Result<bool> {
    if path.exists() && !force {
        return Ok(false);
    }
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("creating {}", parent.display()))?;
    }
    std::fs::write(path, body).with_context(|| format!("writing {}", path.display()))?;
    Ok(true)
}

fn report(wrote: bool, rel: &str) {
    if wrote {
        eprintln!("  init: wrote {rel}");
    } else {
        eprintln!("  init: {rel} exists — left untouched (pass --force to overwrite)");
    }
}

/// Generate the discipline surface for the project at `root`.
pub fn run_init(root: &Path, opts: &InitOptions) -> Result<()> {
    let namespace = match &opts.namespace {
        Some(ns) => ns.clone(),
        None => root
            .canonicalize()
            .ok()
            .and_then(|p| p.file_name().map(|n| n.to_string_lossy().into_owned()))
            .unwrap_or_else(|| "project".to_string()),
    };

    let is_workspace = root.join("crates").is_dir();
    let crates = if is_workspace {
        workspace_crates(root)
    } else {
        root_crate_name(root).into_iter().collect()
    };
    let scan_roots = if is_workspace {
        "[\"crates/*\"]"
    } else {
        "[\".\"]"
    };

    // conform.toml — everything discovered starts exempt-with-a-reason: the
    // expand-as-you-conform rhythm flips a crate into gated_crates only
    // after it drains to zero (Sweep Playbook, Tier 1).
    let mut conform = String::new();
    conform.push_str(&format!(
        "# conform.toml — this project's AI-Native discipline policy\n\
         # (ENGINE-CONFORM; generated by `rust-ai-native init`, then owned by you).\n\
         #\n\
         # The gate enforces only what this file gates. Crates start EXEMPT and\n\
         # flip into `gated_crates` one by one, each after its violations drain\n\
         # to zero (expand-as-you-conform — a flip must never widen a baseline).\n\
         \n\
         roots = {scan_roots}\n\
         exclude_substrings = [\"/generated/\"]\n\
         max_file_lines = 600\n\
         gated_crates = []\n\
         gated_pub_doctest = []\n\
         audit_crates = []\n\
         env_roots = []\n"
    ));
    for c in &crates {
        conform.push_str(&format!(
            "\n[[exempt]]\ncrate = \"{c}\"\nreason = \"pre-adoption — flip into gated_crates after draining (expand-as-you-conform)\"\n"
        ));
    }

    // specmap.toml — namespace + topology + the external resolution set.
    let externals = discover_external_specs(root);
    let mut specmap = format!(
        "# specmap.toml — this project's traceability policy\n\
         # (PROP-014; generated by `rust-ai-native init`, then owned by you).\n\
         \n\
         # The spec:// namespace this project's units are minted under.\n\
         namespace = \"{namespace}\"\n\
         \n\
         scan_roots = {scan_roots}\n\
         spec_roots = [\"spec\"]\n\
         root_spec_docs = []\n\
         exempt = []\n\
         dispositioned = []\n"
    );
    if externals.is_empty() {
        specmap.push_str(
            "\n# No installed packages with spec trees were found under vibedeps/.\n\
             # After `vibe install`, re-run `rust-ai-native init --force` (or add\n\
             # [[external_specs]] entries by hand) so cross-package citations resolve.\n",
        );
    } else {
        specmap.push_str(
            "\n# Installed packages' spec trees, read for URI RESOLUTION only\n\
             # (PROP-014 §7.1) — discovered from vibedeps/ at init time.\n",
        );
        for e in &externals {
            specmap.push_str(&format!(
                "[[external_specs]]\nnamespace = \"{}\"\nroot = \"{}\"\n\n",
                e.namespace, e.root
            ));
        }
    }

    // The BROWNFIELD §3 registries — empty valid forms (top-level key is
    // `entries` in all three, the shape testgate/tripwire parse); the
    // terraform procedure fills them with reality.
    let tests_baseline = "{ \"schema\": 1, \"entries\": [] }\n";
    let debt = "{ \"schema\": 1, \"entries\": [] }\n";
    let intent = "{ \"schema\": 1, \"entries\": [] }\n";
    let conform_baseline = "{\"schema\":1,\"findings\":[]}\n";

    eprintln!(
        "rust-ai-native init: namespace `{namespace}`, {} layout",
        if is_workspace {
            "workspace"
        } else {
            "single-crate"
        }
    );
    report(
        write_once(&root.join("conform.toml"), &conform, opts.force)?,
        "conform.toml",
    );
    report(
        write_once(&root.join("specmap.toml"), &specmap, opts.force)?,
        "specmap.toml",
    );
    report(
        write_once(
            &root.join(crate::DEFAULT_CONFORM_BASELINE),
            conform_baseline,
            opts.force,
        )?,
        crate::DEFAULT_CONFORM_BASELINE,
    );
    report(
        write_once(
            &root.join(crate::DEFAULT_TESTS_BASELINE),
            tests_baseline,
            opts.force,
        )?,
        crate::DEFAULT_TESTS_BASELINE,
    );
    report(
        write_once(&root.join(crate::DEFAULT_DEBT_REGISTRY), debt, opts.force)?,
        crate::DEFAULT_DEBT_REGISTRY,
    );
    report(
        write_once(
            &root.join(crate::DEFAULT_INTENT_REGISTRY),
            intent,
            opts.force,
        )?,
        crate::DEFAULT_INTENT_REGISTRY,
    );

    eprintln!(
        "rust-ai-native init: done. Next steps:\n\
         \x20 1. wire your workspace Cargo.toml (GUIDE §13) — dep the tags AND exclude the slots:\n\
         \x20    [workspace] exclude = [\"vibedeps\"]   # the packages are their own workspaces\n\
         \x20    [workspace.dependencies] specmark = {{ path = \"vibedeps/<stack-slot>/crates/vendor/specmark\" }}\n\
         \x20 2. write your first spec unit (spec/PROP-001.md with a {{#req-…}} anchor)\n\
         \x20    and tag the implementing module: specmark::scope!(\"spec://{namespace}/PROP-001#req-…\")\n\
         \x20 3. `rust-ai-native specmap` to mint the index, then `rust-ai-native floor`\n\
         \x20 4. adopt crate-by-crate: drain a crate, flip it into conform.toml's gated_crates\n\
         \x20    (the sweep skill walks this: /rust-ai-native-sweep; brownfield: /rust-ai-native-terraform)\n\
         (layout changed since init? re-run with --force to regenerate the topology-derived files)"
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn opts() -> InitOptions {
        InitOptions {
            namespace: Some("demo".into()),
            force: false,
        }
    }

    #[test]
    fn generates_the_six_artifacts_and_is_idempotent() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        std::fs::write(root.join("Cargo.toml"), "[package]\nname = \"demo-app\"\n").unwrap();
        run_init(root, &opts()).unwrap();
        for rel in [
            "conform.toml",
            "specmap.toml",
            crate::DEFAULT_CONFORM_BASELINE,
            crate::DEFAULT_TESTS_BASELINE,
            crate::DEFAULT_DEBT_REGISTRY,
            crate::DEFAULT_INTENT_REGISTRY,
        ] {
            assert!(root.join(rel).exists(), "{rel} missing");
        }
        let specmap = std::fs::read_to_string(root.join("specmap.toml")).unwrap();
        assert!(specmap.contains("namespace = \"demo\""));
        assert!(specmap.contains("scan_roots = [\".\"]"), "{specmap}");
        // The single-crate exempt entry carries the DIRECTORY basename —
        // the name the scanner attributes files to — never the manifest's
        // `[package] name` (here deliberately different: "demo-app").
        let conform = std::fs::read_to_string(root.join("conform.toml")).unwrap();
        let label = root.file_name().unwrap().to_string_lossy().into_owned();
        assert!(
            conform.contains(&format!("crate = \"{label}\"")),
            "{conform}"
        );
        assert!(!conform.contains("crate = \"demo-app\""), "{conform}");
        // The generated policy satisfies the engine's tree invariant as-is
        // (the acceptance the workspace-layout twin below always had).
        let cfg = conform_core::Config::load(&root.join("conform.toml")).unwrap();
        cfg.validate_against_tree(root).unwrap();

        // The generated registries parse with the engines that read them —
        // the format contract that once drifted (`tests` vs `entries`).
        let baseline = std::fs::read_to_string(root.join(crate::DEFAULT_TESTS_BASELINE)).unwrap();
        assert!(
            specmap_core::testgate::parse_baseline(&baseline)
                .unwrap()
                .is_empty()
        );
        let debt = std::fs::read_to_string(root.join(crate::DEFAULT_DEBT_REGISTRY)).unwrap();
        assert!(
            specmap_core::tripwire::evaluate(&debt, &["x".into()])
                .unwrap()
                .is_empty()
        );

        // Idempotence: a second run leaves user-owned files untouched.
        std::fs::write(root.join("conform.toml"), "# my edited policy\n").unwrap();
        run_init(root, &opts()).unwrap();
        let kept = std::fs::read_to_string(root.join("conform.toml")).unwrap();
        assert_eq!(kept, "# my edited policy\n");
    }

    #[test]
    fn workspace_layout_scans_crates_and_exempts_each() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        for c in ["app", "helper"] {
            let d = root.join("crates").join(c);
            std::fs::create_dir_all(&d).unwrap();
            std::fs::write(d.join("Cargo.toml"), "[package]\n").unwrap();
        }
        run_init(root, &opts()).unwrap();
        let conform = std::fs::read_to_string(root.join("conform.toml")).unwrap();
        assert!(conform.contains("roots = [\"crates/*\"]"));
        assert!(conform.contains("crate = \"app\""));
        assert!(conform.contains("crate = \"helper\""));
        // The generated policy satisfies the engine's tree invariant as-is.
        let cfg = conform_core::Config::load(&root.join("conform.toml")).unwrap();
        cfg.validate_against_tree(root).unwrap();
    }

    #[test]
    fn external_specs_are_discovered_from_vibedeps() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        let slot = root.join("vibedeps/flow-some-core/0.3.0");
        std::fs::create_dir_all(slot.join("spec")).unwrap();
        std::fs::write(
            slot.join("vibe.toml"),
            "[package]\nname = \"some-core\"\ngroup = \"org.x\"\nkind = \"flow\"\nversion = \"0.3.0\"\n",
        )
        .unwrap();
        run_init(root, &opts()).unwrap();
        let specmap = std::fs::read_to_string(root.join("specmap.toml")).unwrap();
        assert!(specmap.contains("namespace = \"some-core\""), "{specmap}");
        assert!(
            specmap.contains("root = \"vibedeps/flow-some-core/0.3.0/spec\""),
            "{specmap}"
        );
        // And the engine parses what init wrote.
        let cfg = specmap_core::config::Config::load(root).unwrap().unwrap();
        assert_eq!(cfg.external_specs.len(), 1);
    }
}
