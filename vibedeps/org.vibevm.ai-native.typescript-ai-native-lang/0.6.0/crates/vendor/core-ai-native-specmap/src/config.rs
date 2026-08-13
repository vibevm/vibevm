//! `specmap.toml` — the project's traceability policy, lifted out of
//! hardcoded paths so the specmap engine runs on *any* project, not only
//! the one it was built in (PROP-014; the same productisation conform made
//! in its Ф3, mirrored here in the Traceability Relocation Plan Phase 2).
//!
//! The driver (or the `rust-ai-native-specmap` binary) loads this once at startup and
//! constructs the scan + the orphan ratchet from it; nothing about which
//! roots to walk or which crates are exempt is hardcoded in the engine.
//! An absent `specmap.toml` yields the default policy and turns the orphan
//! ratchet off — the pre-config behaviour.

specmark::scope!("spec://org.vibevm.ai-native/core-ai-native/mechanisms/PROP-014#index");

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::Deserialize;

/// The specmap policy for one project: the project's `spec://` namespace,
/// which code roots carry taggable source, which markdown trees hold spec
/// units, and which crates are exempt from the orphan ratchet. Loaded from a
/// `specmap.toml` at the project root. A present file MUST set `namespace`
/// (the `<package>` segment every minted `spec://<namespace>/…` URI carries);
/// every other field defaults. An absent file yields the placeholder default
/// (`namespace = "project"`).
///
/// ```
/// let cfg: core_ai_native_specmap::config::Config = toml::from_str(
///     "namespace = \"demo\"\nscan_roots = [\"crates/*\"]\nexempt = [\"vibe-wire\"]\n",
/// )
/// .unwrap();
/// assert_eq!(cfg.namespace, "demo");
/// assert_eq!(cfg.scan_roots, vec!["crates/*".to_string()]);
/// // Unset fields fall back to the defaults.
/// assert_eq!(cfg.spec_roots, vec!["spec".to_string()]);
/// assert!(cfg.root_spec_docs.is_empty());
/// assert!(cfg.spec_exclude.is_empty());
/// assert!(cfg.schema_roots.is_empty());
/// // Quality thresholds default to the start placeholders; both gate off at 0.
/// assert_eq!(cfg.max_connections_per_item, 3);
/// assert_eq!(cfg.max_section_lines, 120);
/// assert_eq!(cfg.section_grain, core_ai_native_specmap::config::SectionGrain::Leaf);
/// ```
#[derive(Debug, Clone, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Config {
    /// The `<package>` segment of minted `spec://` URIs (PROP-014 §2.1) —
    /// the project's spec namespace. Required in a present `specmap.toml`
    /// (field-level default is empty so [`Config::load`] can tell "unset"
    /// from any real value); the struct default is the `"project"`
    /// placeholder used when no policy file exists.
    #[serde(default)]
    pub namespace: String,
    /// Code roots to scan for `#[spec]`/`#[verifies]`/`scope!` tags. A
    /// `<dir>/*` entry scans each subdirectory of `<dir>` as one crate; any
    /// other entry is a literal crate dir.
    pub scan_roots: Vec<String>,
    /// Markdown trees walked for anchored spec units (`<root>/**/*.md`).
    pub spec_roots: Vec<String>,
    /// Individual root-level spec documents scanned in addition to
    /// [`spec_roots`](Config::spec_roots) — for a project whose frozen
    /// top-level spec lives at the repo root, outside any `spec/` tree.
    pub root_spec_docs: Vec<String>,
    /// Spec-markdown files to exclude from the inventory — globs matched
    /// against the `/`-separated repo-relative path (the exact string each
    /// [`SpecUnit`](crate::generated::specmap::SpecUnit) carries as `file`),
    /// applied to what [`spec_roots`](Config::spec_roots) and
    /// [`root_spec_docs`](Config::root_spec_docs) would otherwise surface.
    /// The match leaves the inventory before it is parsed into units — the
    /// exclude is layered **after** the include half, by the same law as the
    /// progress gate's `exclude`: the include names the forest, the exclude
    /// names the trees to cut.
    ///
    /// This config has two root families — [`scan_roots`](Config::scan_roots)
    /// (code) and [`spec_roots`](Config::spec_roots) (markdown) — so a bare
    /// `exclude` would not say which half it prunes. `spec_exclude` names the
    /// spec-markdown half (the only half with the finding today) and leaves
    /// the name `code_exclude` free should the code half ever need one.
    ///
    /// PROP-014's include half is enumerated by design so nothing is observed
    /// by accident, and an *enumerated* exclude list serves that purpose
    /// exactly as well as an enumerated include — both are explicit and both
    /// are reviewable. What it must not become is a wildcard escape hatch:
    /// a pattern that matches no file is reported (the `stale-exclude`
    /// warning), never tolerated, and a pattern that is not a valid glob is
    /// reported (the `bad-exclude-glob` warning), never silently skipped —
    /// a skip would leave the corpus wider than the config says.
    ///
    /// Absent ⇒ empty ⇒ the behaviour of a config that never had the key.
    pub spec_exclude: Vec<String>,
    /// JTD schema trees walked for generator-input units
    /// (`<root>/**/*.jtd.json`) — the taggable surface
    /// `##RULE-GENERATED-CODE-IS-EXCLUDED` (PROP-014) points at: a `.jtd.json`
    /// is what a code generator *reads*, so the traceability tag belongs on
    /// the schema, not on the `/generated/` code it produces. Each schema
    /// file's root object is one `schema` unit and every `definitions` entry
    /// its own `schema-def` unit; the `metadata.spec` map ("verb → URI")
    /// mirrors `#[spec(verb = "…")]` and mints the unit's edges. Empty (the
    /// default) ⇒ the schema scanner contributes nothing ⇒ the index of a
    /// project with no schema roots is byte-stable against the Rust-only scan.
    pub schema_roots: Vec<String>,
    /// Installed packages' spec trees that participate in **resolution
    /// only** (PROP-014 §7.1): their units suppress dangling-edge warnings
    /// and feed queries, but are never serialised into this project's
    /// `specmap.json`. Typically generated by `rust-ai-native init` from
    /// the materialised `vibedeps/` slots.
    pub external_specs: Vec<ExternalSpec>,
    /// Crates exempt from the orphan ratchet (PLAYBOOK `#phase2`). A crate
    /// **not** listed is gated: its `pub` items must carry an own edge or a
    /// `scope!`-inherited module edge. Empty = every crate gated.
    pub exempt: Vec<String>,
    /// Orphans allowed to stand, each carrying its debt id (the "dispositioned
    /// into debt.json" arm of the Phase 2 acceptance).
    pub dispositioned: Vec<Disposition>,
    /// Max distinct spec points one code element may realise before the
    /// `overloaded-item` warning fires — **inclusive** (an element reaching
    /// the threshold is flagged). Language-neutral — the map is one for all
    /// languages, and this config models none — so it sits at the root, not
    /// under any language section. `0` disables the check. Start value `3`:
    /// a placeholder until the live corpus calibrates it, which the warning
    /// itself gathers.
    #[serde(default = "default_max_connections_per_item")]
    pub max_connections_per_item: usize,
    /// Max lines a **leaf** spec section (one with no nested subsection) may
    /// span before the `long-section` warning fires — **inclusive**. Leaves
    /// only by default ([`section_grain`](Config::section_grain)): a container
    /// section is long because the document is, which measures genre, not
    /// discipline. `0` disables. Start value `120`, a placeholder for
    /// calibration.
    #[serde(default = "default_max_section_lines")]
    pub max_section_lines: usize,
    /// Grain at which [`max_section_lines`](Config::max_section_lines) is
    /// measured: `leaf` (default) — only sections with no nested subsection;
    /// `all` — every section, containers included (measures document size,
    /// not discipline; opt in deliberately).
    #[serde(default)]
    pub section_grain: SectionGrain,
}

/// The grain at which the `long-section` threshold is measured
/// ([`Config::max_section_lines`]). `Leaf` — only sections with no nested
/// subsection (the default; measures section discipline, not document
/// size). `All` — every section, containers included.
///
/// ```
/// use core_ai_native_specmap::config::SectionGrain;
/// assert_eq!(SectionGrain::default(), SectionGrain::Leaf);
/// ```
#[derive(Debug, Clone, Copy, Default, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum SectionGrain {
    #[default]
    Leaf,
    All,
}

/// Default for [`Config::max_connections_per_item`] — the start placeholder.
fn default_max_connections_per_item() -> usize {
    3
}

/// Default for [`Config::max_section_lines`] — the start placeholder.
fn default_max_section_lines() -> usize {
    120
}

impl Default for Config {
    fn default() -> Self {
        Config {
            namespace: "project".into(),
            scan_roots: vec!["crates/*".into()],
            spec_roots: vec!["spec".into()],
            root_spec_docs: Vec::new(),
            spec_exclude: Vec::new(),
            schema_roots: Vec::new(),
            external_specs: Vec::new(),
            exempt: Vec::new(),
            dispositioned: Vec::new(),
            max_connections_per_item: default_max_connections_per_item(),
            max_section_lines: default_max_section_lines(),
            section_grain: SectionGrain::default(),
        }
    }
}

/// One installed package's spec tree, read for URI resolution only: units
/// found under `root` are minted as `spec://<namespace>/…` and used to
/// resolve edges, never inventoried into the project's own index.
///
/// ```
/// let e: core_ai_native_specmap::config::ExternalSpec = toml::from_str(
///     "namespace = \"core-ai-native\"\nroot = \"vibedeps/flow-core-ai-native/0.3.0/spec\"\n",
/// )
/// .unwrap();
/// assert_eq!(e.namespace, "core-ai-native");
/// ```
#[derive(Debug, Clone, Deserialize)]
pub struct ExternalSpec {
    /// The package's spec namespace (its `spec://<namespace>/…` segment).
    pub namespace: String,
    /// Project-root-relative path to the package's spec tree (the directory
    /// whose contents map to `spec://<namespace>/<docpath>`).
    pub root: String,
}

/// One `[[external_specs]]` entry whose declared [`ExternalSpec::root`] is not
/// a directory on disk — the "not yet installed" state, surfaced (not
/// silenced) by [`Config::missing_external_spec_roots`]. Carries everything a
/// caller needs to print a useful warning: the namespace quotes will fail to
/// resolve into, the path exactly as declared in `specmap.toml`, and that path
/// resolved against the project root.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MissingExternalSpecRoot {
    /// The spec namespace quotes cannot resolve into while the root is absent
    /// (`spec://<namespace>/…`).
    pub namespace: String,
    /// The project-root-relative path exactly as declared under
    /// `[[external_specs]]` `root`.
    pub declared: String,
    /// That path resolved against the project root — the directory that was
    /// not found.
    pub resolved: PathBuf,
}

/// One orphan held outside the gate, with the debt id that records why it is
/// allowed to stand.
///
/// ```
/// let d = core_ai_native_specmap::config::Disposition {
///     symbol: "vibe_cli::commands::mcp::serve".into(),
///     debt: "DBT-0020".into(),
/// };
/// assert_eq!(d.debt, "DBT-0020");
/// ```
#[derive(Debug, Clone, Deserialize)]
pub struct Disposition {
    /// The module-qualified symbol the disposition covers.
    pub symbol: String,
    /// The debt id it is filed under.
    pub debt: String,
}

impl Config {
    /// Repo-relative location of the policy file.
    pub const REL_PATH: &'static str = "specmap.toml";

    /// Load `specmap.toml` from `root`. `Ok(None)` when the file is absent —
    /// the caller defaults the scan and turns the ratchet off. A present
    /// file must set `namespace`: minted URIs are identity, so a policy
    /// that scans without saying whose `spec://` segment to mint is a
    /// config error, not a defaultable gap.
    ///
    /// A `[[external_specs]]` root whose directory is absent is a *legitimate*
    /// state — "that package is not installed yet" — but one the project MUST
    /// hear about out loud: every quote into that namespace resolves to
    /// nothing while it stands, and the only failure that ever bit us was the
    /// silence around it, never the absence itself. So
    /// [`missing_external_spec_roots`](Self::missing_external_spec_roots) is
    /// consulted here and, when non-empty, a warning is printed to stderr
    /// (each namespace, its declared path, the resolved path, the consequence,
    /// and the fix); the config is then returned all the same, never refused.
    /// The built-in degradation — the resolution layer skips a missing root —
    /// stays in force; this only ends the silence around it.
    pub fn load(root: &Path) -> Result<Option<Config>> {
        let path = root.join(Self::REL_PATH);
        if !path.exists() {
            return Ok(None);
        }
        let text = std::fs::read_to_string(&path)
            .with_context(|| format!("reading specmap config {}", path.display()))?;
        let cfg: Config = toml::from_str(&text)
            .with_context(|| format!("parsing specmap config {}", path.display()))?;
        if cfg.namespace.is_empty() {
            anyhow::bail!(
                "{}: `namespace` is required — the spec://<namespace>/… segment minted \
                 for this project's units (run `rust-ai-native init`, or add e.g. \
                 `namespace = \"myproject\"`)",
                path.display()
            );
        }
        // An absent `[[external_specs]]` root is the legitimate "not yet
        // installed" state, not a refusal — but it must be said out loud,
        // once and for every broken namespace, so version drift cannot pass
        // silently. Print the warning here (the engine's own idiom for a
        // missing external root, and for a vacuously-green gate, is to warn on
        // stderr rather than fail); then hand the config back regardless.
        let missing = cfg.missing_external_spec_roots(root);
        if !missing.is_empty() {
            eprintln!(
                "specmap: WARNING — {} `external_specs` root(s) not found on disk; quotes \
                 into their namespaces will not resolve. Reinstall dependencies or fix \
                 specmap.toml:",
                missing.len()
            );
            for m in &missing {
                eprintln!(
                    "  external spec root `{}` (namespace `{}`) -> {} (not found)",
                    m.declared,
                    m.namespace,
                    m.resolved.display(),
                );
            }
        }
        Ok(Some(cfg))
    }

    /// Resolve [`scan_roots`](Config::scan_roots) to concrete crate
    /// directories under `root`, deterministically (sorted). A `<dir>/*`
    /// entry expands to each existing subdirectory of `<dir>`; any other
    /// entry is taken literally. The sort makes the downstream index
    /// order — and therefore `specmap.json` — stable across platforms.
    pub fn scan_dirs(&self, root: &Path) -> Vec<PathBuf> {
        let mut dirs = Vec::new();
        for entry in &self.scan_roots {
            if let Some(parent) = entry.strip_suffix("/*") {
                if let Ok(rd) = std::fs::read_dir(root.join(parent)) {
                    for e in rd.filter_map(std::result::Result::ok) {
                        if e.path().is_dir() {
                            dirs.push(e.path());
                        }
                    }
                }
            } else {
                dirs.push(root.join(entry));
            }
        }
        dirs.sort();
        dirs
    }

    /// The `[[external_specs]]` entries whose declared
    /// [`ExternalSpec::root`] is not a directory under `root` — the "not yet
    /// installed" set, in declaration order. Each carries its namespace, the
    /// path as declared in `specmap.toml`, and that path resolved against the
    /// project root (the directory that was not found). A missing root is a
    /// *legitimate* state (the package may simply not be installed yet), so
    /// this returns the set for a caller to warn about; it is not itself a
    /// failure. Mirrors the conform engine's `rust_scope_warnings` /
    /// `vacuously_gated` helpers, which hand a caller a list to print rather
    /// than printing it themselves.
    pub fn missing_external_spec_roots(&self, root: &Path) -> Vec<MissingExternalSpecRoot> {
        self.external_specs
            .iter()
            .filter_map(|ext| {
                let resolved = root.join(&ext.root);
                if resolved.is_dir() {
                    None
                } else {
                    Some(MissingExternalSpecRoot {
                        namespace: ext.namespace.clone(),
                        declared: ext.root.clone(),
                        resolved,
                    })
                }
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn absent_file_is_none() {
        let tmp = tempfile::tempdir().unwrap();
        assert!(Config::load(tmp.path()).unwrap().is_none());
    }

    #[test]
    fn defaults_are_project_neutral() {
        let cfg = Config::default();
        assert_eq!(cfg.namespace, "project");
        assert_eq!(cfg.scan_roots, ["crates/*"]);
        assert_eq!(cfg.spec_roots, ["spec"]);
        assert!(cfg.root_spec_docs.is_empty());
        assert!(cfg.external_specs.is_empty());
        // Quality thresholds: the start placeholders, leaf grain.
        assert_eq!(cfg.max_connections_per_item, 3);
        assert_eq!(cfg.max_section_lines, 120);
        assert_eq!(cfg.section_grain, SectionGrain::Leaf);
    }

    #[test]
    fn quality_thresholds_parse_and_disable() {
        // Overridable, the disable sentinel `0` carries through, and the
        // grain enum reads its lowercase form.
        let cfg: Config = toml::from_str(
            "namespace = \"demo\"\n\
             max_connections_per_item = 5\n\
             max_section_lines = 0\n\
             section_grain = \"all\"\n",
        )
        .unwrap();
        assert_eq!(cfg.max_connections_per_item, 5);
        assert_eq!(cfg.max_section_lines, 0);
        assert_eq!(cfg.section_grain, SectionGrain::All);
        // A bogus grain value is rejected (deny by the enum, not unknown_fields).
        assert!(
            toml::from_str::<Config>("namespace = \"x\"\nsection_grain = \"weird\"\n").is_err()
        );
    }

    #[test]
    fn spec_exclude_defaults_empty_and_reads_from_toml() {
        // Absent key ⇒ empty list ⇒ no exclusion (the pre-field behaviour).
        assert!(Config::default().spec_exclude.is_empty());
        // The key reads. Under `deny_unknown_fields` this same toml was a
        // hard error before the field existed, so a successful parse is
        // itself the proof the field is wired into the schema.
        let cfg: Config = toml::from_str(
            "namespace = \"demo\"\nspec_exclude = [\"spec/WAL.md\", \"**/INDEX.md\"]\n",
        )
        .unwrap();
        assert_eq!(cfg.spec_exclude, vec!["spec/WAL.md", "**/INDEX.md"]);
    }

    #[test]
    fn schema_roots_defaults_empty_and_reads_from_toml() {
        // Absent key ⇒ empty list ⇒ the schema scanner is a no-op (the
        // pre-field behaviour, byte-stable).
        assert!(Config::default().schema_roots.is_empty());
        // The key reads. Under `deny_unknown_fields` this same toml was a
        // hard error before the field existed, so a successful parse is
        // itself the proof the field is wired into the schema.
        let cfg: Config = toml::from_str(
            "namespace = \"demo\"\nschema_roots = [\"schemas\", \"packages/*/schemas\"]\n",
        )
        .unwrap();
        assert_eq!(cfg.schema_roots, vec!["schemas", "packages/*/schemas"]);
    }

    #[test]
    fn present_file_requires_namespace() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(
            tmp.path().join("specmap.toml"),
            "scan_roots = [\"crates/*\"]\n",
        )
        .unwrap();
        let err = Config::load(tmp.path()).unwrap_err();
        assert!(err.to_string().contains("`namespace` is required"), "{err}");
        std::fs::write(
            tmp.path().join("specmap.toml"),
            "namespace = \"demo\"\nscan_roots = [\"crates/*\"]\n",
        )
        .unwrap();
        assert_eq!(Config::load(tmp.path()).unwrap().unwrap().namespace, "demo");
    }

    #[test]
    fn external_specs_parse() {
        let cfg: Config = toml::from_str(
            "namespace = \"demo\"\n\
             [[external_specs]]\n\
             namespace = \"core-ai-native\"\n\
             root = \"vibedeps/flow-core-ai-native/0.3.0/spec\"\n",
        )
        .unwrap();
        assert_eq!(cfg.external_specs.len(), 1);
        assert_eq!(cfg.external_specs[0].namespace, "core-ai-native");
    }

    #[test]
    fn external_specs_root_present_loads() {
        // A materialised external spec tree: the entry's directory exists, so
        // the load succeeds and names no missing root.
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        std::fs::create_dir_all(root.join("vibedeps/flow-core-ai-native/0.3.0/spec")).unwrap();
        std::fs::write(
            root.join("specmap.toml"),
            "namespace = \"demo\"\n\
             [[external_specs]]\n\
             namespace = \"core-ai-native\"\n\
             root = \"vibedeps/flow-core-ai-native/0.3.0/spec\"\n",
        )
        .unwrap();
        let cfg = Config::load(root).unwrap().unwrap();
        assert_eq!(cfg.external_specs.len(), 1);
        assert_eq!(cfg.external_specs[0].namespace, "core-ai-native");
        // A present root is no one's warning.
        assert!(cfg.missing_external_spec_roots(root).is_empty());
    }

    #[test]
    fn external_specs_root_missing_warns_not_fails() {
        // The version-drift / not-yet-installed state: the declared root is
        // absent. `load` still returns the config (legitimate degradation),
        // but the missing root is surfaced — exactly one, with its namespace
        // and declared path visible — so the silence cannot stand.
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        std::fs::write(
            root.join("specmap.toml"),
            "namespace = \"demo\"\n\
             [[external_specs]]\n\
             namespace = \"core-ai-native\"\n\
             root = \"vibedeps/flow-core-ai-native/0.9.0/spec\"\n",
        )
        .unwrap();
        let cfg = Config::load(root).unwrap().unwrap();
        let missing = cfg.missing_external_spec_roots(root);
        assert_eq!(missing.len(), 1, "exactly the one absent root");
        assert_eq!(missing[0].namespace, "core-ai-native");
        assert_eq!(
            missing[0].declared,
            "vibedeps/flow-core-ai-native/0.9.0/spec"
        );
        assert_eq!(
            missing[0].resolved,
            root.join("vibedeps/flow-core-ai-native/0.9.0/spec")
        );
    }

    #[test]
    fn glob_expands_subdirs_and_literals_pass_through() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        std::fs::create_dir_all(root.join("crates/a")).unwrap();
        std::fs::create_dir_all(root.join("crates/b")).unwrap();
        std::fs::create_dir_all(root.join("tooling")).unwrap();
        let cfg = Config {
            scan_roots: vec!["crates/*".into(), "tooling".into()],
            ..Config::default()
        };
        let dirs = cfg.scan_dirs(root);
        // crates/a, crates/b (glob) + tooling (literal), sorted.
        assert_eq!(
            dirs,
            vec![
                root.join("crates/a"),
                root.join("crates/b"),
                root.join("tooling"),
            ]
        );
    }

    #[test]
    fn unknown_field_is_rejected() {
        let err = toml::from_str::<Config>("bogus = 1\n");
        assert!(err.is_err(), "deny_unknown_fields must reject typos");
    }
}
