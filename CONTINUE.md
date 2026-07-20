# CONTINUE — cold-resume checkpoint

_Written 2026-07-20 (end of the settings-home + global-registry session). Overwrites the prior
vibeterm-M1 CONTINUE. `spec/WAL.md` is the canonical living state and supersedes this snapshot if they
diverge._

## TL;DR

Two owner-requested features shipped this session, plus two follow-up refinements — all on `main`,
**floor-green**, **not yet pushed** (`main` is **11 ahead of `origin/main`, 0 behind** → fast-forward):

1. **Settings-home consolidation.** One chokepoint `vibe_core::settings` is now the single authority for
   the per-user settings dir: `$VIBE_SETTINGS` overrides, else `<home>/.vibe`. Every previously-dispersed
   resolver (registry/search caches, PROP-040 L1 `settings.toml` + its path-classifier, publish tokens,
   aiui discovery, user `config.toml`) routes through it. The legacy `~/.vibevm` and the XDG
   `~/.config/vibe/config.toml` are folded into `~/.vibe`, with **read-only fallback** to the old paths so
   nothing breaks mid-migration. **The owner's live files were migrated on disk** (see §Live migration).
2. **Machine-global registry config** `~/.vibe/registry.toml` — same `[[registry]]`/`[[mirror]]`/
   `[[override]]` shape as a project `vibe.toml`, **any** registry (remote or local), merged **project-first**
   (dedupe by `name`). `--offline` refined to resolve **local repos only** (drops remote).
3. **Refinement:** clarified (spec + test) that the global file accepts **any** registry, not just local.
4. **Refinement:** optional **`enabled`** flag on `[[registry]]` (default `true`, skipped-on-serialize) —
   switch a registry off without deleting it; skipped by every resolution path.

**No blocker.** The one open item is the **push** (11 commits, via `cargo xtask mirror` — see §Push).

## Where work stands

- Branch `main`, **11 ahead / 0 behind** `origin/main`. Working tree clean except one untracked file that
  is **NOT ours** — `research/vibeterm/projectx-function-map.md` (never commit it; likewise never commit
  `packages/org.vibevm.vibeapp/` if it reappears).
- Floor **all green**: `cargo fmt --all --check`, `cargo test --workspace`, `cargo clippy --workspace
  --all-targets -D warnings`, `cargo xtask conform check` (**0 findings**), `cargo xtask specmap` (**0 new**
  orphans), `cargo run -p vibe-cli -- check` (**0 errors**; 1 pre-existing warning: stale boot artifact for
  `org.vibevm.world/git-practices`, unrelated).
- `cargo xtask specmap --check` reports **34 gated orphans** — these are **pre-existing baseline** in
  `vibe-spec` (PROP-035 spec-compiler, provisional/exempt) + `vibe-resolver::EmbeddedPrecedence`; **none are
  this session's code**. The operative gate is `cargo xtask specmap` (write, exit 0), which is green.

## Push (the one open item)

`main` is 11 fast-forward commits ahead of `origin`. **Rollout is `cargo xtask mirror`** (GitVerse +
GitHub), **not** a bare `git push origin` (that hits GitVerse only). Held for the owner's explicit go this
session (the owner asked to be the one to authorise the push). To push:

```sh
cargo xtask mirror          # fast-forward-only to every target in mirrors.toml
cargo xtask mirror --check  # verify both mirrors are in sync
```

## Live migration (owner's home dir — DONE this session)

`~/.vibevm` **removed**; everything moved into `~/.vibe`:
- `git.publish.token`, `github.publish.token`, `zai.api.token`, `zai.api.token.2`, `aiui/` — copied
  byte-identical (`cmp`-verified), old dir deleted.
- `~/.config/vibe/config.toml` did **not** exist on this box (nothing to fold).
- `~/.fractality/profiles.toml` `token_file` edited `~/.vibevm/zai.api.token` → **`~/.vibe/zai.api.token`**
  (verified: profiles points at the existing, byte-identical file). Token bytes unchanged → **fractality /
  z.ai delegation keeps working** (no re-auth). No live spawn was run to confirm — the owner asked not to
  delegate to GLM workers this session, and the token-read chain is logically verified.

## Next steps (pick up cold)

1. **Push** (§Push above) once authorised.
2. Optional polish on the new features:
   - `vibe registry enable/disable <name>` CLI toggle (today `enabled` is a manifest field only; edit
     `vibe.toml` / `~/.vibe/registry.toml` by hand). Would live near
     `crates/vibe-cli/src/commands/registry/config/`.
   - Extend `enabled` to `[[mirror]]` / `[[override]]` for uniformity (same 3-line pattern: field +
     filter). Scoped out this session (the owner asked about "repository").
   - A `vibe registry list` marker showing disabled entries.
3. The prior **vibeterm M1** track is also on `main` (see the commit chain / the WAL's 2026-07-20 vibeterm
   checkpoint) — its next steps (pixel-polish, placeholder controls, M1 close) are independent of this work.

## Non-obvious findings this session

- **conform's extracted-test detection.** Moving an inline `#[cfg(test)] mod tests { … }` to a sibling
  `foo/tests.rs` is safe **only if every fn in it is `#[test]`**. A *non-`#[test]` helper fn* with
  `.unwrap()`/`.expect()` in an extracted file is flagged `no-unwrap-in-domain` (conform parses the file
  standalone and doesn't see the parent's `#[cfg(test)]`). Fix: keep tests **inline** and split *domain*
  code instead, **or** ensure the extracted file has only `#[test]` fns. (`resolver.rs` split its
  `apply_git_source_flag` into `resolver/git_source_flag.rs` and kept tests inline; `project.rs` extracted
  its tests to `project/tests.rs` cleanly because they are all `#[test]`.)
- **specmap editorial drift.** Editing the *prose* under a spec anchor (not the REQ) makes
  `specmap --check` report `unbumped-hash`. Resolution: `cargo xtask specmap` (regenerate) **and** mark the
  commit body `spec-editorial: <anchor-slug>` (don't bump `r`). Used for `#publish` / `#trust-gate` /
  `#global-config`.
- **env-read composition roots** are an allow-list in `conform.toml` (`env_roots = [...]`). A new file that
  legitimately reads `$VIBE_SETTINGS`/`HOME` (like `settings.rs`) must be added there, or conform flags
  `ambient-env`.
- **Push is multi-homed.** `git push origin` hits **only GitVerse**; the repo is mirrored to GitHub too.
  The correct rollout is **`cargo xtask mirror`** (fast-forward-only to every target in `mirrors.toml`).
- **fractality reads `token_file` with no fallback** — it expands exactly the path in `profiles.toml`. The
  `~/.vibevm`→`~/.vibe` fallback lives in the *vibe* publish/aiui readers, not in fractality; that is why
  the live `profiles.toml` had to be edited.

## The settings + registry architecture (in force)

- `crates/vibe-core/src/settings.rs` — **THE** settings-dir chokepoint. `settings_dir()` = `$VIBE_SETTINGS`
  else `<home>/.vibe`; `legacy_settings_dir()` = `<home>/.vibevm` (read fallback only). Typed accessors:
  `registry_config_path` / `user_config_path` / `settings_toml_path` / `registries_cache_dir` /
  `search_cache_dir` / `aiui_dir`. Pure core `settings_dir_from(override, home)` is the tested seam.
- `crates/vibe-core/src/global_registry.rs` — `GlobalRegistryConfig` (loads `~/.vibe/registry.toml`),
  `merge_effective(project, global)` (project-first, dedupe by name / pkgref; mirrors concatenated),
  `EffectiveRegistryConfig::local_only()` (the `--offline` filter), `url_is_local()` (scheme classifier).
- `crates/vibe-cli/src/commands/install/resolver.rs` — the composition root: `effective_registry_config`
  loads+merges the global config once and threads it into `build_install_resolver` (kept test-hermetic by
  DI, not a hidden filesystem read).
- **`enabled` filter** lives in `crates/vibe-registry/src/multi_registry_resolver/mod.rs`
  (`from_manifest`, the single construction point) — `if !reg.enabled { continue; }` — so **every**
  resolution path honours it. The field is on `RegistrySection` (`crates/vibe-core/src/manifest/project.rs`,
  `#[serde(default = "default_true", skip_serializing_if = "is_true")]`).
- Normative spec: **PROP-002 §2.2.2** (`#global-config`, `#offline-local`) and **§2.2.3** (`#enabled`);
  design record: `spec/research/SETTINGS-HOME-AND-GLOBAL-REGISTRY-PLAN-v0.1.md`.

## Repository map (top level)

- `crates/` — the Rust workspace. Key crates this session: `vibe-core` (manifest schemas +
  `settings`/`global_registry`), `vibe-registry` (`MultiRegistryResolver`), `vibe-cli` (commands, the
  install composition root), `vibe-publish` (tokens), `vibe-settings` (PROP-040 layered prefs +
  path-classifier), `vibe-workspace` (boot artifacts), `vibe-spec` (PROP-035 spec-compiler, provisional).
- `apps/vibeterm/`, `apps/vibeframe/` — the Electron terminal shells (JS/TS); `vibeterm/main.cjs` writes
  the aiui discovery files (now under `~/.vibe/aiui`, `$VIBE_SETTINGS`-aware).
- `spec/` — the normative tree: `spec/common/PROP-000` (secrets), `spec/modules/vibe-registry/PROP-002`
  (registry), `spec/modules/vibe-workspace/PROP-009/011/035/040` etc., `spec/research/` (design docs),
  `spec/boot/` (session boot lane incl. `90-user.md`), `spec/WAL.md` (living state).
- `packages/org.vibevm.fractality/` — the fractality specspace (own boot/WAL/CONTINUE; delegation engine).
- `xtask/` — `cargo xtask {conform check, specmap, mirror, codegen}`.
- Root docs: `RUNTIME-GUIDE.md` (paths/env table — updated), `DEV-GUIDE.md`, `VIBEVM-SPEC.md`, `ROADMAP.md`,
  `CLAUDE.md`/`AGENTS.md`/`GEMINI.md` (kept identical), `conform.toml`, `specmap.json`.

## Discipline (unchanged, binds every commit)

- **Never** write the reference app's real name anywhere (repo/history/chat) — it is "ProjectX" / "the
  reference"; our feature is the **VibeTerm** shell.
- **No AI attribution** on any commit (no `Co-Authored-By` / `Claude` trailers).
- Commits via **heredoc** `git commit -F - <<'MSG'` only (backtick `-m` corrupted messages before).
- File edits via **Edit/Write** only — PS 5.1 corrupts UTF-8-no-BOM round-trips (`git restore` to recover).
- **Never** `Stop-Process` by bare name (killed the Claude Code terminal once) — filter to your own PID.
- **Push via `cargo xtask mirror`** (GitVerse + GitHub), never a bare `git push origin` (GitVerse-only).
- Secrets: never `cat`/`Read`/`echo` a token file; `cp`/`cmp`/size only.

## Recent commit chain (last 25, newest first)

```
8b9b630 feat(registry): optional `enabled` flag to switch a registry off without deleting it
e19efec docs(registry): clarify ~/.vibe/registry.toml accepts any registry, not just local
14e1174 docs: point tokens, config, and aiui discovery at the canonical ~/.vibe
74f08cc chore(discipline): satisfy the conform + specmap floor for the settings work
8aec7cc feat(registry): machine-global registry config merged project-first
f0e89db feat(settings): consolidate the settings home behind one chokepoint
df16b5a docs(research): plan the settings-home consolidation + global registry
65fcc00 feat: compile normal-format packages in the static boot lane
dbc8eea test(traceability): trace the new auth / offline tests to spec
a8b508d feat(cli): add --offline and --embedded-short-circuit install knobs
5fc2f0c fix(registry): silence credential prompts for public (auth=none) registries
7106c1e docs(continue): cold-resume checkpoint -- vibeterm M1 + ProjectX redesign
03848c8 docs(wal): session checkpoint -- vibeterm M1 build-out + ProjectX redesign
0976b03 feat(vibeterm): chrome visual fidelity to the ProjectX reference
a0cea60 feat(vibeterm): split view + tear-off (M1 P2/P3)
0721ad2 fix(vibeterm): initial openTab + IPC-handler race in the offscreen shell
7c297e1 feat(vibeterm): offscreen shell + CDP for headless screenshot/drive
4c7fb4c docs(continue): cold-resume checkpoint -- vibeterm UI-arch campaign
cb32901 docs(wal): session checkpoint -- vibeterm UI-arch campaign done
43f6716 feat(vibeterm): D4 pre-MVP shell -- render-free engine + Solid chrome + per-tab terminals
cb15828 docs(vibeterm): D3 contracts -- the vibeterm PROP family
2932349 docs(vibeterm): D2 design-doc -- architecture + design system lore
a6e22fc docs(research): vibeterm research close -- Phase 2/3/4 (deltas)
3bd277e docs(research): vibeterm Phase 1 -- internal methodology extraction
6ff6a2a docs(research): sharpen the vibeterm UI-architecture plan before Phase 1
```

## Quick-start

```sh
# floor (from repo root)
cargo fmt --all -- --check
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo xtask conform check
cargo xtask specmap            # regenerate; --check for strict CI drift
cargo run -q -p vibe-cli -- check

# push both mirrors (the correct rollout — NOT `git push origin`)
cargo xtask mirror
cargo xtask mirror --check
```
