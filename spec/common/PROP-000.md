# PROP-000: Foundational technical decisions {#root}

This document pins the foundational technical decisions for the vibevm implementation. Every subsequent PROP/FEAT may assume these are true. Changing any of them requires an explicit amendment here first, then downstream updates.

Source authority: [`VIBEVM-SPEC.md`](../../VIBEVM-SPEC.md) §10 and the book in `refs/book/`. Where this PROP and the spec disagree, the spec wins.

---

## 1. Language: Rust {#language}

**Decision:** The vibevm CLI and all supporting crates are written in Rust.

**Why:** Single-binary distribution, no runtime dependency, cross-platform by default, strong type system aligned with the project's discipline philosophy, excellent CLI ecosystem (`clap`, `serde`, `toml`, `reqwest`, `git2`, `tokio`, `anyhow`, `thiserror`, `tracing`, `dialoguer`, `console`, `sha2`). See `VIBEVM-SPEC.md` §10.1.

**MSRV:** Latest stable at the time of each milestone. M0 pins MSRV to the latest stable at the tag.

**When to revisit:** Never, in the scope of v1. If Rust proves inadequate for a future milestone, open a new PROP superseding this one.

---

## 2. Build system: Cargo workspace {#build}

**Decision:** Single Cargo workspace at repo root. Crates live under `crates/` per `VIBEVM-SPEC.md` §10.2:

- `vibe-cli` — CLI entry point, argument parsing.
- `vibe-core` — types, manifest schemas, graph model.
- `vibe-graph` — graph builder and runner.
- `vibe-registry` — registry fetch/cache/resolve.
- `vibe-install` — install/uninstall/update logic.
- `vibe-llm` — LLM provider abstraction (stub in M0, real in M1.5).
- `vibe-check` — linter (M1).

**Why:** Standard Rust workspace layout, enables shared dependency versions via `[workspace.dependencies]`, supports independent testing of each crate.

**Binary name:** `vibe` (built from `vibe-cli`).

---

## 3. License {#license}

**Decision:** vibevm itself ships under a **proprietary EULA** in this phase (source-available, not open source). See [`LICENSE.md`](../../LICENSE.md) at the repo root for the placeholder terms. Crates in this workspace set `license-file = "LICENSE.md"` and `publish = false` so none of them can be accidentally pushed to crates.io.

**Why:** Owner's call — intent is to eventually relicense under the Universal Permissive License 1.0 (UPL), but that decision is not final. Until then, vibevm stays proprietary. `VIBEVM-SPEC.md` §1 explicitly defers the *produced* software's license to the owner; the owner's choice is this proprietary EULA.

**Third-party dependencies remain permissive-only.** Per `VIBEVM-SPEC.md` §10.3: every crate we depend on must be MIT / Apache-2.0 / BSD or equivalent. GPL / AGPL / LGPL are forbidden, period. The proprietary license of vibevm itself does not relax that constraint — it makes it more important, because anything we link becomes mingled with our proprietary code, and copyleft would force relicensing.

**When to revisit:** When the owner decides to relicense (most likely UPL 1.0). At that point, swap `LICENSE.md`, update the workspace `license-file` (or switch back to an SPDX `license` string like `UPL-1.0`), and remove `publish = false` if desired.

---

## 4. Manifest format: TOML {#manifests}

**Decision:** All vibevm manifests use TOML 1.0 (`toml` crate, `serde`-based).

Files:
- `vibe.toml` — project manifest. Schema: `VIBEVM-SPEC.md` §7.5.
- `vibe-package.toml` — package manifest. Schema: `VIBEVM-SPEC.md` §7.3.
- `vibe.lock` — lockfile. Schema: `VIBEVM-SPEC.md` §7.4.

**Why:** TOML is the Rust ecosystem default (cargo), readable, has clear escaping rules, and maps cleanly to `serde` structs. See `VIBEVM-SPEC.md` §10.1.

---

## 5. Directory layout {#layout}

**Decision:** Per `VIBEVM-SPEC.md` §4.2. The `spec/` directory is hardcoded — never configurable in v1. The `.vibe/` cache directory is gitignored and per-project. `refs/src/` is gitignored (external reference sources, cloned by the implementer for study, not part of the vibevm repo itself).

---

## 6. Package identity {#identity}

**Decision:** `<kind>:<name>@<version>` per `VIBEVM-SPEC.md` §7.1. `kind ∈ {flow, feat, stack, tool}`. `name` is kebab-case, unique within kind. `version` is semver.

Constraint forms in CLI:
- `flow:wal` → latest stable.
- `flow:wal@0.3.0` → exact.
- `flow:wal@^0.3` → semver range.

---

## 7. Registry model (M0 vs M1) {#registry}

**Decision:**
- **M0:** local-directory registry only. No git. Registry is a path on disk with the layout from `VIBEVM-SPEC.md` §8.2.
- **M1:** git registry added per `VIBEVM-SPEC.md` §8. Configured in `vibe.toml`'s `[registry]` section. Default public registry URL = `git@gitverse.ru:anarchic/vibespecs.git` (SSH, see `spec/boot/90-user.md`). `VIBEVM-SPEC.md` §7.5 now carries this URL directly (was a `github.com/anarchic-org/...` placeholder earlier). **Backend choice, trait design, cache layout, and Windows UX for M1** are pinned in [spec://vibevm/modules/vibe-registry/PROP-001](../modules/vibe-registry/PROP-001-git-backend.md) — in brief: shell-out to the system `git` (not `libgit2`), behind a `GitBackend` trait that leaves the door open for a future `libgit2` swap.

**Source repositories:**
- The vibevm tool itself: `git@gitverse.ru:anarchic/vibevm.git` (SSH) / `https://gitverse.ru/anarchic/vibevm` (web).
- The package registry: `git@gitverse.ru:anarchic/vibespecs.git` (SSH). Empty until M1 publish.

**Cache location:** `~/.vibe/registries/<hash>/` for cloned registries; `<project>/.vibe/cache/<kind>/<name>/<version>/` for per-package cache. See `VIBEVM-SPEC.md` §8.3.

---

## 8. Task graph model {#graph}

**Decision:** Built-in nodes only in v1 (content-only plugin contribution model per `VIBEVM-SPEC.md` §5.4). Runner is sequential (no parallelism) in v1 per §5.2. Typed value system per §5.3.

Workflows are graph queries (target node + transitive dependencies) per §5.5.

---

## 9. Conflict resolution {#conflicts}

**Decision:** Per book chapter 1 / `VIBEVM-SPEC.md` §2.2: **Human > Spec > Tests > Code**. AI never silently overrides spec; when it believes the spec is wrong it adds a `REVIEW` marker and surfaces the question.

---

## 10. Observability {#observability}

**Decision:** Use `tracing` for structured logs. CLI defaults to human-readable Markdown-flavored output; `--json` for machine-readable; `--quiet` for one-line summaries. Exit codes per `VIBEVM-SPEC.md` §9.4.

---

## 11. Cross-platform target {#platforms}

**Decision:** M0 builds and runs on macOS, Linux, and Windows. Path handling goes through `std::path::Path` — no manual separator manipulation. File operations respect platform casing rules where the OS enforces them.

**Test matrix:** M0 dev is primarily on Windows 11 (this machine). CI matrix for all three OSes lands in M2 per `VIBEVM-SPEC.md` §11.4.

---

## 12. Commit and push discipline {#commits}

**Decision:** Four non-negotiable rules govern commits, pushes, and attribution in this repository. These rules appear **verbatim** in [`CLAUDE.md`](../../CLAUDE.md) (and the identical `AGENTS.md` / `GEMINI.md`) so they are the very first thing read at every session boot. They are reproduced here as the authoritative reference.

### 12.1 Attribution — keep this repository human-authored

Never attribute authorship of any part of this repository to an artificial-intelligence (AI) or neural-network–based system of any kind — no commit messages, no Git trailers (`Co-Authored-By`, `Signed-off-by: <model-name>`, etc.), no branch names, no worktree branch names, no code comments, no README lines, no release notes. This applies to every such system regardless of provider, model family, or whether it is local or cloud-hosted.

**Why:** Some jurisdictions regulate or criminalize machine authorship of software. We are not currently subject to such regulation and are not violating any present law; the owner chooses this policy proactively so that any future regulation finds no hook on this project. The surface this repository presents is: a human wrote this code, full stop.

This paragraph (and its copy in `CLAUDE.md` / `AGENTS.md` / `GEMINI.md`) is the **single** place in the entire project where AI tooling is discussed in the attribution sense. Everywhere else — commits, code, docs, branches, CI, signing — assume human authorship only. `VIBEVM-SPEC.md` discusses AI integration as a **feature of the vibevm product** (what the tool does); that is product scope, not authorship attribution, and is not covered by this rule.

### 12.2 Conventional Commits {#conventional-commits}

Every commit follows the [Conventional Commits](https://www.conventionalcommits.org/) specification. Format:

```
type(scope): short imperative subject line

Longer body — anywhere from a sentence to a mini-article — explaining
WHY this change was made and what follows from it. The diff already
shows what changed; the commit message's job is to convey the reasoning
and downstream consequences that a future reader cannot reconstruct
from the diff alone.

Cite `spec://…` URIs where relevant.
```

- Subject line short enough to render cleanly in web Git UIs (target ≤ 60 characters, hard limit 72).
- Body free-form; prefer paragraphs over bullet lists when the reasoning is continuous.
- `type` is the standard set: `feat`, `fix`, `chore`, `docs`, `build`, `test`, `refactor`, `perf`, `style`, `ci`, `revert`.
- `scope` names the most affected crate, package, or subsystem (e.g. `core`, `install`, `wal`, `registry`, `spec`).

### 12.3 Group commits by meaning {#grouping}

When the working tree carries changes spanning multiple concerns, split them into separate commits grouped by topic — never by file name or time of edit. Each commit is one logical unit. A working set containing "fix typo in README" + "refactor the planner" + "update the manifest schema" is **three** commits.

### 12.4 Autonomy on routine changes {#autonomy}

Routine large changes — implementing a planned milestone, finishing a feature slice, touching many files for one coherent reason — may be committed and pushed without user approval, using rules 12.1–12.3. Ask the user first for anything non-routine: rewriting published history (rebase of pushed commits, `git commit --amend` on pushed work), `git push --force` / `--force-with-lease`, bringing in large binary blobs, changing CI or signing configuration, any operation whose reversal costs work. When uncertain, ask.

---

## 13. Package layout convention {#package-layout}

**Decision:** vibevm packages use a **mirror layout**. Every entry in a package's `writes.files` is simultaneously (a) the path of the file inside the package directory and (b) the path at which it will be installed in the consumer's project. There is no separate `target = "…"` field per entry; `writes.files` is the single source of truth for "where does this file go?"

Concretely, `packages/flow/wal/v0.1.0/` contains `spec/flows/wal/WAL-PROTOCOL.md` at exactly that relative path; after `vibe install flow:wal`, the file lives at `spec/flows/wal/WAL-PROTOCOL.md` inside the user's project. No mapping, no rewriting.

**Boot snippets are the one exception.** The `[boot_snippet]` table carries an explicit `source` field naming the path inside the package (conventionally under `boot/`), while the target is always the fixed `spec/boot/<filename>`.

**Why:** a single source of truth for source-and-target paths eliminates a whole class of authoring bug where the package layout drifts from the declared writes. It also makes a package directory instantly readable — a human looking at the tree knows exactly what will appear in a consumer's project without cross-referencing a separate mapping table.

**Where pinned:** `VIBEVM-SPEC.md` §13.1 shows the mirror-layout diagram and §13.2 the matching manifest. This PROP-000 entry is the decision record; the spec carries the operational definition. `vibe-install` relies on this convention — the source path of a planned write is computed by joining `cache_dir` with the manifest's declared target path.

---

## Invariants

(These restate the most load-bearing rules from the spec and the book. If anything below seems violated in practice, stop and reconcile before proceeding.)

1. **Vocabulary lock.** Never use Maven's "lifecycle/phase/goal" or Bazel's internal terminology in user-facing or internal code. The four kinds are `flow`, `feat`, `stack`, `tool`. The canonical process discipline vocabulary is the one in `VIBEVM-SPEC.md` §4 and the book.
2. **`spec/` is fixed.** The directory name and role cannot be configured away in v1.
3. **User-owned files are never written by `vibe`.** `spec/boot/00-core.md` and `spec/boot/90-user.md` are off-limits to install/uninstall/update.
4. **One commit = one logical unit.** Commit messages follow Conventional Commits (see §12) and reference `spec://…` URIs where relevant.
5. **Dogfood.** vibevm is being built using the same discipline it enforces. The `spec/` tree in this repo IS `vibe init`'s reference output.
6. **Human authorship is the only attribution.** See §12.1. This is the only place in the project where AI tooling is discussed in the attribution sense.
