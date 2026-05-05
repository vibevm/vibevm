//! `vibe init` — scaffold a new vibevm project.
//!
//! Spec: `VIBEVM-SPEC.md` §9.1, §11.1.
//! Acceptance: the produced tree matches §4.2; running twice does not destroy
//! user-modified files (idempotent).

use std::fs;
use std::path::Path;

use anyhow::{Context, Result, bail};
use serde::Serialize;
use vibe_core::manifest::{
    ActiveSection, DEFAULT_REGISTRY_GITVERSE_NAME, DEFAULT_REGISTRY_GITVERSE_URL,
    DEFAULT_REGISTRY_NAME, DEFAULT_REGISTRY_REF, DEFAULT_REGISTRY_URL, Lockfile, NamingConvention,
    ProjectManifest, ProjectSection, RegistrySection,
};

use crate::cli::InitArgs;
use crate::output;

const REDIRECT_LINE: &str = "Read every file in spec/boot/ in filename order, then await the user's instructions.\n";

pub fn run(ctx: &output::Context, args: InitArgs) -> Result<()> {
    fs::create_dir_all(&args.path)
        .with_context(|| format!("creating project directory `{}`", args.path.display()))?;

    let path = canonical_no_unc(&args.path)?;
    let display_root = normalize_display(&args.path, &path);

    if !path.is_dir() {
        bail!("target `{}` is not a directory", display_root);
    }

    let project_name = resolve_name(&args, &path)?;

    ctx.heading(&format!(
        "Initializing project `{project_name}` in `{display_root}`"
    ));

    let mut outcomes = Vec::<Outcome>::new();

    // 1. Redirect files (CLAUDE.md, AGENTS.md, GEMINI.md).
    for filename in ["CLAUDE.md", "AGENTS.md", "GEMINI.md"] {
        outcomes.push(ensure_file(
            ctx,
            &path,
            &path.join(filename),
            REDIRECT_LINE,
            "agent redirect",
        )?);
    }

    // 2. spec/ directory tree.
    for sub in ["boot", "flows", "feats", "stacks", "common", "modules"] {
        ensure_dir(&path.join("spec").join(sub))?;
    }

    // 3. User-owned boot snippets.
    outcomes.push(ensure_file(
        ctx,
        &path,
        &path.join("spec/boot/00-core.md"),
        &boot_00_core_template(&project_name),
        "boot: project foundation",
    )?);
    outcomes.push(ensure_file(
        ctx,
        &path,
        &path.join("spec/boot/90-user.md"),
        BOOT_90_USER_TEMPLATE,
        "boot: user overrides",
    )?);

    // 4. WAL.
    outcomes.push(ensure_file(
        ctx,
        &path,
        &path.join("spec/WAL.md"),
        &wal_template(&project_name),
        "WAL checkpoint",
    )?);

    // 5. Project manifest and empty lockfile.
    let registries = resolve_registry_sections(&args);
    outcomes.push(ensure_project_manifest(
        ctx,
        &path,
        &project_name,
        args.stack.as_deref(),
        registries,
    )?);
    outcomes.push(ensure_empty_lockfile(ctx, &path)?);

    // 6. `.vibe/` cache (gitignored per §4.2).
    ensure_dir(&path.join(".vibe/cache"))?;
    outcomes.push(ensure_file(
        ctx,
        &path,
        &path.join(".vibe/.gitignore"),
        "*\n",
        "gitignore: cache",
    )?);

    // 7. .gitignore at project root (only if absent — don't overwrite).
    outcomes.push(ensure_file(
        ctx,
        &path,
        &path.join(".gitignore"),
        ROOT_GITIGNORE_TEMPLATE,
        "gitignore: root",
    )?);

    report(ctx, &project_name, &display_root, &outcomes)?;
    Ok(())
}

#[derive(Debug, Clone, Serialize)]
struct Outcome {
    path: String,
    action: Action,
    reason: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
enum Action {
    Created,
    Kept,
}

fn ensure_file(
    ctx: &output::Context,
    root: &Path,
    path: &Path,
    content: &str,
    reason: &'static str,
) -> Result<Outcome> {
    if let Some(parent) = path.parent() {
        ensure_dir(parent)?;
    }
    let rel = relative_to_root(root, path);
    if path.exists() {
        ctx.skipped(&rel, "already exists");
        return Ok(Outcome {
            path: rel,
            action: Action::Kept,
            reason,
        });
    }
    fs::write(path, content).with_context(|| format!("writing `{}`", path.display()))?;
    ctx.created(&rel);
    Ok(Outcome {
        path: rel,
        action: Action::Created,
        reason,
    })
}

fn ensure_dir(path: &Path) -> Result<()> {
    fs::create_dir_all(path).with_context(|| format!("creating `{}`", path.display()))
}

fn ensure_project_manifest(
    ctx: &output::Context,
    root: &Path,
    name: &str,
    stack: Option<&str>,
    registries: Vec<RegistrySection>,
) -> Result<Outcome> {
    let path = root.join(ProjectManifest::FILENAME);
    let rel = relative_to_root(root, &path);
    if path.exists() {
        ctx.skipped(&rel, "already exists");
        return Ok(Outcome {
            path: rel,
            action: Action::Kept,
            reason: "project manifest",
        });
    }

    let manifest = ProjectManifest {
        project: ProjectSection {
            name: name.to_string(),
            version: "0.0.1".to_string(),
            authors: vec![],
        },
        active: stack.map(|s| ActiveSection {
            stack: Some(s.to_string()),
        }),
        llm: None,
        registries,
        mirrors: Vec::new(),
        overrides: Vec::new(),
        i18n: vibe_core::manifest::i18n::I18nDecl::default(),
    };

    manifest.write(&path)?;
    ctx.created(&rel);
    Ok(Outcome {
        path: rel,
        action: Action::Created,
        reason: "project manifest",
    })
}

/// Build the `[[registry]]` entries to write into a fresh `vibe.toml`.
///
/// - `--no-registry` → empty (vibe.toml has no `[[registry]]`).
/// - `--registry-url <URL>` → one entry — the operator's custom registry,
///   replacing the defaults entirely. `--registry-ref` further overrides
///   the ref.
/// - default → two entries:
///   1. `vibespecs` on GitHub (primary — drives `vibe registry publish`
///      and the first stop on resolve fallback).
///   2. `vibespecs-gitverse` on GitVerse (secondary — different package
///      set; consulted on `UnknownPackage` fall-through). Publishing here
///      is currently a stub; resolve-time read works.
fn resolve_registry_sections(args: &InitArgs) -> Vec<RegistrySection> {
    if args.no_registry {
        return Vec::new();
    }
    if let Some(url) = &args.registry_url {
        return vec![RegistrySection {
            name: DEFAULT_REGISTRY_NAME.to_string(),
            url: url.clone(),
            r#ref: args
                .registry_ref
                .clone()
                .unwrap_or_else(|| DEFAULT_REGISTRY_REF.to_string()),
            naming: NamingConvention::KindName,
        }];
    }
    let github = RegistrySection {
        name: DEFAULT_REGISTRY_NAME.to_string(),
        url: DEFAULT_REGISTRY_URL.to_string(),
        r#ref: args
            .registry_ref
            .clone()
            .unwrap_or_else(|| DEFAULT_REGISTRY_REF.to_string()),
        naming: NamingConvention::KindName,
    };
    // GitVerse default uses `naming = "name"` (no kind prefix). The
    // public `vibespecs` org on GitVerse provisions repos under their
    // package name only — `vibespecs/vibevm-direct-push-smoke` rather
    // than `vibespecs/flow-vibevm-direct-push-smoke`. Keeping the
    // default consistent with what the org actually carries means a
    // fresh `vibe init` resolves GitVerse-only packages correctly out
    // of the box. The convention is recorded per-registry in
    // `vibe.toml`, so a project mirroring a different org (where
    // kind-name is the convention) overrides it freely.
    let gitverse = RegistrySection {
        name: DEFAULT_REGISTRY_GITVERSE_NAME.to_string(),
        url: DEFAULT_REGISTRY_GITVERSE_URL.to_string(),
        r#ref: DEFAULT_REGISTRY_REF.to_string(),
        naming: NamingConvention::Name,
    };
    vec![github, gitverse]
}

fn ensure_empty_lockfile(ctx: &output::Context, root: &Path) -> Result<Outcome> {
    let path = root.join(Lockfile::FILENAME);
    let rel = relative_to_root(root, &path);
    if path.exists() {
        ctx.skipped(&rel, "already exists");
        return Ok(Outcome {
            path: rel,
            action: Action::Kept,
            reason: "lockfile",
        });
    }
    let lockfile = Lockfile::empty(
        format!("vibe {}", env!("CARGO_PKG_VERSION")),
        current_timestamp_utc(),
    );
    lockfile.write(&path)?;
    ctx.created(&rel);
    Ok(Outcome {
        path: rel,
        action: Action::Created,
        reason: "lockfile",
    })
}

fn resolve_name(args: &InitArgs, path: &Path) -> Result<String> {
    if let Some(n) = &args.name {
        return Ok(n.clone());
    }
    let basename = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("project");
    Ok(basename.to_string())
}

fn relative_to_root(root: &Path, full: &Path) -> String {
    let stripped = full.strip_prefix(root).unwrap_or(full);
    display_pathbuf(stripped)
}

fn display_pathbuf(p: &Path) -> String {
    // Display with forward slashes — consistent across macOS/Linux/Windows.
    let s = p.display().to_string();
    s.replace('\\', "/")
}

/// Canonicalize and strip Windows UNC (`\\?\`) prefix where present.
fn canonical_no_unc(path: &Path) -> Result<std::path::PathBuf> {
    let canon = path
        .canonicalize()
        .with_context(|| format!("canonicalizing `{}`", path.display()))?;
    Ok(strip_unc(canon))
}

#[cfg(windows)]
fn strip_unc(p: std::path::PathBuf) -> std::path::PathBuf {
    let s = p.as_os_str().to_string_lossy();
    if let Some(rest) = s.strip_prefix(r"\\?\") {
        std::path::PathBuf::from(rest)
    } else {
        p
    }
}

#[cfg(not(windows))]
fn strip_unc(p: std::path::PathBuf) -> std::path::PathBuf {
    p
}

/// Re-export for sibling command modules.
pub(crate) fn strip_unc_public(p: std::path::PathBuf) -> std::path::PathBuf {
    strip_unc(p)
}

/// Prefer the user-supplied display (e.g. `.`) if it still points at the
/// canonical path; otherwise fall back to the canonical (UNC-stripped) form.
fn normalize_display(requested: &Path, canonical: &Path) -> String {
    let requested_matches = requested
        .canonicalize()
        .map(|c| strip_unc(c) == *canonical)
        .unwrap_or(false);
    if requested_matches {
        display_pathbuf(requested)
    } else {
        display_pathbuf(canonical)
    }
}

pub(crate) fn current_timestamp_utc() -> String {
    vibe_core::timestamp::now_utc()
}

fn report(
    ctx: &output::Context,
    name: &str,
    display_root: &str,
    outcomes: &[Outcome],
) -> Result<()> {
    let created = outcomes.iter().filter(|o| o.action == Action::Created).count();
    let kept = outcomes.iter().filter(|o| o.action == Action::Kept).count();

    if ctx.is_json() {
        use vibe_wire::generated::init_report::{
            InitReport, Outcome as WireOutcome, OutcomeAction,
        };
        let payload = InitReport {
            ok: true,
            command: "init".to_string(),
            project: name.to_string(),
            path: display_root.to_string(),
            created: u32::try_from(created).unwrap_or(u32::MAX),
            kept: u32::try_from(kept).unwrap_or(u32::MAX),
            outcomes: outcomes
                .iter()
                .map(|o| WireOutcome {
                    path: o.path.clone(),
                    action: match o.action {
                        Action::Created => OutcomeAction::Created,
                        Action::Kept => OutcomeAction::Kept,
                    },
                    reason: o.reason.to_string(),
                })
                .collect(),
        };
        ctx.emit_json(&payload)?;
        return Ok(());
    }

    if ctx.is_quiet() {
        ctx.summary(&format!(
            "vibe init: {created} created, {kept} kept in `{display_root}`"
        ));
        return Ok(());
    }

    println!();
    ctx.summary(&format!(
        "Done. Project `{name}`: {created} file{} created, {kept} kept.",
        if created == 1 { "" } else { "s" }
    ));
    println!();
    println!("Next:");
    println!("  • edit spec/boot/00-core.md and spec/common/ as your project takes shape");
    println!("  • install packages with `vibe install <kind>:<name>` (e.g. flow:wal)");
    Ok(())
}

// ==== Templates ============================================================

const BOOT_90_USER_TEMPLATE: &str = r#"# User overrides

User-owned. `vibe install` / `vibe uninstall` never touch this file. Add any
project-specific conventions that should be read at session boot — coding
style, naming rules, deploy commands, anything the AI agent should know up
front and should not have to rediscover each session.
"#;

fn boot_00_core_template(project_name: &str) -> String {
    format!(
        r#"# Project boot snippet — `{project_name}`

User-owned. `vibe install` / `vibe uninstall` never touch this file.

## About this project

_TODO: one paragraph describing what `{project_name}` is and who it is for._

## Session boot sequence

Every AI session starts here. In order:
1. Read every file in `spec/boot/` in filename order.
2. Read `spec/WAL.md` — current project state (checkpoint, not history log).
3. Read the relevant PROP/FEAT documents under `spec/common/` and
   `spec/modules/` for the task at hand.
4. Only then begin work.

If `spec/WAL.md` is older than 24 hours, verify the state with the user before
doing destructive work.

## Memory layers

- **Head** (human): persistent but private.
- **WAL** (`spec/WAL.md`): volatile, rewritten each session, current state only.
- **Spec** (other files under `spec/`): stable decisions, addressable via
  `spec://<module>/<doc>#<section>` URIs.
- **Code** (`src/`, `tests/`): artefacts, regenerable.

Information flows top-down. When code changes first, reconcile up with a
Sync-from-Code proposal before rewriting code back to spec.

## Conflict resolution

Priority: **Human > Spec > Tests > Code.** When the AI believes the spec is
wrong, add a `<!-- REVIEW: … -->` marker, implement what the spec says, and
surface the disagreement in the end-of-session report.
"#
    )
}

fn wal_template(project_name: &str) -> String {
    let today = current_date_utc();
    format!(
        r#"# WAL — Project Continuation State
_Updated: {today}_

## Current phase

Project `{project_name}` — just initialized. No work in flight.

## Constraints (do not violate without discussion)

- (none yet — add as decisions are made)

## Done

- [x] Project initialized with `vibe init`.

## In progress

- (nothing)

## Next

- (fill in before starting the first real session)

## Known issues

- (none)

## Session context

- Start of next session: read this WAL, then `spec/boot/`, then the relevant
  PROP/FEAT under `spec/common/` or `spec/modules/`.
"#
    )
}

fn current_date_utc() -> String {
    let ts = current_timestamp_utc();
    ts.split('T').next().unwrap_or(&ts).to_string()
}

const ROOT_GITIGNORE_TEMPLATE: &str = r#"# vibevm cache (per-project, should never be committed)
/.vibe/

# OS / editor noise
.DS_Store
Thumbs.db
desktop.ini
.idea/
.vscode/
"#;

