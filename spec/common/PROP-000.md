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
- **M1:** git registry added per `VIBEVM-SPEC.md` §8. Configured in `vibe.toml`'s `[[registry]]` array. Default public registry URL = `https://github.com/vibespecs` (HTTPS org root; per-package URLs are derived at fetch time via [`NamingConvention`](../../crates/vibe-core/src/manifest/project.rs)). **Backend choice, trait design, cache layout, and Windows UX for M1** are pinned in [spec://vibevm/modules/vibe-registry/PROP-001](../modules/vibe-registry/PROP-001-git-backend.md) — in brief: shell-out to the system `git` (not `libgit2`), behind a `GitBackend` trait that leaves the door open for a future `libgit2` swap.

**Default in new projects.** `vibe init` writes the default registry URL (`DEFAULT_REGISTRY_URL` in [`vibe_core::manifest`](../../crates/vibe-core/src/manifest/project.rs)) into every new `vibe.toml`'s `[[registry]]` entry unless the operator passes `--no-registry` or overrides with `--registry-url <URL>` / `--registry-ref <REF>`. The default exists so that a plain `vibe init` → `vibe install flow:wal` flow works out of the box against the public registry; overrides are there for forks, staging registries, and offline / air-gapped setups. The single source of truth for the URL is the constant in `vibe-core` — manual-tests, smoke scripts, and docs all reference it from there.

**Source repositories — split-host posture.** The vibevm project and the package registry live on **separate hosts** by deliberate decision (2026-04-29). Each host is chosen on its own merits:
- **vibevm tool source: GitVerse.** `git@gitverse.ru:vibevm/vibevm.git` (SSH) / `https://gitverse.ru/vibevm/vibevm` (web). Stays on GitVerse — the source-of-truth repository, contributor SSH keys, mirroring posture, and Russian-jurisdiction hosting are all already wired up here.
- **Package registry: GitHub, organization `vibespecs`.** `https://github.com/vibespecs` (org root) — per-package repos are `https://github.com/vibespecs/<kind>-<name>` per [PROP-002](../modules/vibe-registry/PROP-002-decentralized-registry.md#registry-model) `NamingConvention::KindName`. The migration from `git@gitverse.ru:vibespecs/*` happened on 2026-04-29 because GitVerse's public REST API does not expose org-scoped repo creation (`POST /orgs/{org}/repos` returns 404 / WAF 403; documented exhaustively in [PROP-002 §2.10](../modules/vibe-registry/PROP-002-decentralized-registry.md#publish) and `crates/vibe-publish/src/gitverse.rs`). Without that endpoint `vibe registry publish` cannot fully drive the publish loop end to end. GitHub's equivalent endpoint works natively, so the registry organization moved while the vibevm project repository stays put. Identity is content-hashed (PROP-002 §2.1) — the lockfile's `source_url` rotates but no `content_hash` value is invalidated by the host change.
- **Legacy registry, read-only.** `git@gitverse.ru:anarchic/vibespecs.git` (HEAD `2203239`, 2026-04-23, three v0.1.0 flows in monorepo form). Kept readable for any project still on schema-v1 lockfiles until they migrate; no new publishes happen there.

**Cache location:** `~/.vibe/registries/<hash>/` for cloned registries; `<project>/.vibe/cache/<kind>/<name>/<version>/` for per-package cache. See `VIBEVM-SPEC.md` §8.3.

---

## 8. Task graph model {#graph}

**Decision:** Built-in nodes only in v1 (content-only plugin contribution model per `VIBEVM-SPEC.md` §5.4). Runner is sequential (no parallelism) in v1 per §5.2. Typed value system per §5.3.

Workflows are graph queries (target node + transitive dependencies) per §5.5.

---

## 9. Conflict resolution {#conflicts}

vibevm's writer-conflict resolution — the **Human > Spec > Tests > Code** order (also pinned in [`VIBEVM-SPEC.md`](../../VIBEVM-SPEC.md) §2.2 and book chapter 1) — is the `conflict-protocol` flow: `spec://org.vibevm.world/conflict-protocol/flows/conflict-protocol/CONFLICT-PROTOCOL#root`.

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

Concretely, the canonical `flow:wal@0.1.0` payload (vendored as a hermetic e2e test fixture under `fixtures/registry/flow/wal/v0.1.0/`) contains `spec/flows/wal/WAL-PROTOCOL.md` at exactly that relative path; after `vibe install flow:wal`, the file lives at `spec/flows/wal/WAL-PROTOCOL.md` inside the user's project. No mapping, no rewriting.

**Boot snippets are the one exception.** The `[boot_snippet]` table carries an explicit `source` field naming the path inside the package (conventionally under `boot/`), while the target is always the fixed `spec/boot/<filename>`.

**Why:** a single source of truth for source-and-target paths eliminates a whole class of authoring bug where the package layout drifts from the declared writes. It also makes a package directory instantly readable — a human looking at the tree knows exactly what will appear in a consumer's project without cross-referencing a separate mapping table.

**Where pinned:** `VIBEVM-SPEC.md` §13.1 shows the mirror-layout diagram and §13.2 the matching manifest. This PROP-000 entry is the decision record; the spec carries the operational definition. `vibe-install` relies on this convention — the source path of a planned write is computed by joining `cache_dir` with the manifest's declared target path.

---

## 14. Manual-test protocol {#manual-tests}

**Decision:** human-runnable smoke-tests live in [`manual-tests/`](../../manual-tests/) at the repo root, one Markdown file per scenario, named `<milestone>-<slug>.md` (e.g. `M1.1-git-registry-smoke.md`). The directory's own [`README.md`](../../manual-tests/README.md) carries the authoring conventions, the clean-slate protocol, and the index.

**Why a second test tier.** `cargo test --workspace` uses fakes, tempdirs, and local bare repositories for speed and hermeticity. That tier cannot prove the integration surfaces that only matter in the real world — SSH auth against GitVerse, the lockfile `source_uri` exactly as it appears to downstream consumers, the `~/.vibe/` layout on a user's actual filesystem, a human looking at CLI output and saying "yes, that's what I meant". These scripts are that last mile. They complement `cargo test` — they do not replace it.

**Authoring rules** (full versions in `manual-tests/README.md`):

1. **Clean slate is mandatory.** Every test isolates its state with `mktemp -d` for the project and with `VIBE_REGISTRY_CACHE` pointing inside the scratch dir for the registry cache. The user's real `~/.vibe/` must never be touched by a test run.
2. **Self-contained walkthrough.** A reader opens the file, executes top-to-bottom, and finishes with no ambient state. Each step has a command in a fenced block and an "Expected" paragraph.
3. **Platform coverage.** Commands are POSIX-shell compatible. Git Bash on Windows is the primary smoke-test environment; macOS and Linux must work too. When platform output differs (path separators, `stat` flags, `.exe` suffix), the Windows form comes first with a portable note.
4. **Cleanup + what-to-file-if-it-fails.** Every test ends with a copy-pasteable teardown block and a section listing the artefacts to collect if something broke.

**When to run.** Before tagging any milestone; after changes to integration surfaces (git backend, CLI arg parsing, lockfile format) even when `cargo test` stays green; and as reproducers whenever a user files an integration bug.

**Where pinned:** this PROP entry is the policy; [`manual-tests/README.md`](../../manual-tests/README.md) is the operational guide. [`spec/WAL.md`](../WAL.md) names the outstanding manual runs for the current milestone.

---

## 15. Dependency weight is not a decision factor {#dep-weight}

**Decision:** Binary size, crate count, transitive dep weight are NOT decision factors when selecting third-party libraries. Pick the strongest available library for the job — for both the Rust CLI and any future Java / frontend side.

**Why:** Software of comparable surface area (Chrome, modern IDEs, production package managers) routinely ships tens to hundreds of dependencies and remains fast and capable. Under-specifying a load-bearing component to save megabytes creates ongoing architectural debt that is much more expensive to repay than the weight it saves. vibevm intends to be best-in-class, and best-in-class means using best-in-class primitives.

**Legitimate reasons to reject a dep:**
- non-permissive license (see §3 — MIT / Apache-2.0 / BSD / Unlicense only; GPL / AGPL / LGPL forbidden; MPL-2.0 allowed case by case, since its weak copyleft does not taint consumers),
- abandoned upstream,
- demonstrated security issues (CVE history, unpatched known exploit),
- fundamentally bad API ergonomics that would propagate into our own interfaces.

"Too heavy" alone is **not** a reason.

**Concrete consequences:** libraries previously rejected on footprint grounds are re-admissible. Notable: `libsolv` (C, with Rust bindings), `git2` (wrapping `libgit2`), bundled native C deps, embedded interpreters when justified. The size-based argument in [PROP-001 §2.1](../modules/vibe-registry/PROP-001-git-backend.md#backend) against `git2` is to be pruned — the remaining arguments (Windows SSH auth, shell-out diagnostic clarity) may still carry that decision, but not the size one.

---

## 16. JTD + codegen for wire contracts {#jtd}

**Decision:** JSON Type Definition (RFC 8927) schemas are the single source of truth for every client/server and machine-to-machine contract in this project. Rust types — and types in any future non-Rust clients — are **generated** from JTD schemas via `jtd-codegen`, not hand-maintained. No client/server duplication is permitted on contracts.

**Why:** duplication between a server contract and a hand-written client is a classic source of version-skew bugs; schema-first codegen eliminates that class of bug categorically. JTD specifically (over JSON Schema / OpenAPI alone) because JTD is deliberately narrower: its schema grammar is constructed so every JTD schema maps to a clean static type in every target language, with no language-specific escape hatches.

**In scope:** LLM provider API wrappers (Anthropic, OpenAI, OpenRouter, Ollama), GitVerse public-API client, `vibe --json` CLI output, telemetry / event log formats, future hosted-registry HTTP surface.

**Out of scope:** human-authored manifests — `vibe.toml`, `vibe.lock`, `vibe-package.toml` — stay TOML via `serde`. JTD is for wire, not for configs humans hand-edit.

**Toolchain placement:**
- `jtd-codegen` binary in project-local `tools/jtd-codegen/` (gitignored; version pinned).
- Schemas in `schemas/` at repo root, one `.jtd.json` file per contract, committed.
- Generated Rust code in `crates/vibe-wire/src/generated/`, committed, with a `// DO NOT EDIT — regenerate via cargo xtask codegen` header on every file.
- Regeneration via `cargo xtask codegen`. CI enforces zero drift (`cargo xtask codegen && git diff --exit-code`).

**Toolchain install ownership:** the coding agent sets up the codegen toolchain itself. Machine-global changes (PATH mutation, admin-level installs, env-var additions) go through `runas` with an operator confirmation at the moment of the change.

---

## 17. Production architecture in the prototype phase {#prod-arch}

**Decision:** Load-bearing surfaces — lockfile schema, registry protocol, dep-resolver semantics, wire formats, identity model — are designed to production quality from day one. The project is a prototype today; the formats and protocols it chooses today are the ones its future users will be bound to. Changing them later is orders of magnitude more expensive than designing them correctly now.

**Lens:** "a principal engineer at a top-tier infrastructure company, designing a format or protocol that will be used by millions" is one of the reflection lenses to reach for when a design decision lands. It is **not** the only lens — "the simplest thing that works" remains valid for leaf features — but architecture-heavy surfaces prefer the principal-engineer lens.

**Consequences:**
- Prefer a recent-but-well-designed library over a tactical shortcut, even when the shortcut is cheaper in the short term.
- Extension points, versioning markers, and forward-compatibility hooks land with the initial cut, not in a later "hardening" pass.
- Reversibility matters: if a format or protocol decision is hard to reverse (lockfile schema, registry URL scheme, identity hash), lean heavier into design rigour before first commit.
- "We'll fix it later" is a valid stance only for implementation quality inside a well-chosen architectural surface — not for the surface itself.

---

## 18. Complexity expectation: higher than RPM {#complexity}

**Decision:** The dependency / package model is designed to handle complexity **at least** matching RPM-class systems (zypper, DNF), and in several dimensions greater. Manifest grammar and lockfile schema reserve fields for — and the resolver actually implements — capabilities, provides / requires / obsoletes / conflicts / supplements / recommends, disjunctions (`A or B`), boolean rich-dep syntax, capability-based resolve, multi-kind cross-deps, and semantic (LLM-reviewed) conflicts. These are designed in from day one, not deferred.

**Why:** vibevm's dependency surface is not simpler than RPM — it is wider. A `feat` package may require a `stack` providing a specific capability, `flow`s may declare semantic compatibility with other `flow`s, LLM-backed review adds a non-mechanical conflict dimension RPM never had. Undershoot — picking a resolver that lacks virtual packages or disjunctions, or a manifest that cannot express capability-based requires — would force an incompatible schema migration after users exist.

**Resolver choice** (pinned in the module PROP): `resolvo` crate as the primary depsolver, with `libsolv` as an explicit FFI-backed fallback behind a `DepSolver` trait (analogous to [PROP-001 §2.2](../modules/vibe-registry/PROP-001-git-backend.md#backend-trait)'s `GitBackend` pattern). PubGrub is rejected for the *primary* role — its algorithm does not handle virtual packages or disjunctions — but is acceptable for explanatory rendering of conflicts in CLI output if it proves superior there.

---

## 19. Load-bearing setup documentation {#setup-docs}

**Decision:** Two files at the repo root are load-bearing for the project:

- [`DEV-GUIDE.md`](../../DEV-GUIDE.md) — contributor-facing: everything to install on a fresh machine to clone, build, test, contribute to, and (if authorized) publish from this repository.
- [`RUNTIME-GUIDE.md`](../../RUNTIME-GUIDE.md) — user-facing: everything to install and env-configure to run the shipped `vibe` CLI.

vibevm's setup docs are [`DEV-GUIDE.md`](../../DEV-GUIDE.md) (contributor / build) and [`RUNTIME-GUIDE.md`](../../RUNTIME-GUIDE.md) (runtime / user). The same-commit obligation that binds them is the `dev-runtime-docs` flow: `spec://org.vibevm.world/dev-runtime-docs/flows/dev-runtime-docs/DEV-RUNTIME-DOCS-PROTOCOL#obligation`.

---

## 20. Token secrecy and adapter scope {#token-secrecy}

`req r1`

**Decision.** Publish tokens, registry-API tokens, and any LLM-provider keys handled by vibevm are surface secrets. They MUST NOT appear in any human- or machine-readable surface that vibevm produces. Concretely:

- **Never printed.** Not to stdout, stderr, the CLI log, the `--json` event stream, error messages, panic traces, telemetry, or the lockfile. The CLI prints the *source* of a token (explicit / env-var name / file path) but never the value. The in-process wrapper type (`vibe_publish::Token`, future `vibe_llm::ApiKey`) MUST redact on `Display` and `Debug` — verified by unit tests.
- **Never persisted.** Not committed to the repository, not written into the lockfile, not embedded in cache files, not landed in the `.vibe/` tree. The single sanctioned at-rest location is the operator's `~/.vibevm/<host>.publish.token` file (per-user, chmod-protected).
- **Sanctioned process boundaries.** The token may cross a process boundary only via: (a) the host API's `Authorization: Bearer …` header, sent over TLS; (b) a single `git remote add` / `git push` invocation where the token is embedded in the URL as `https://x-access-token:<TOKEN>@host/…` (modern git ≥ 2.31 redacts URL passwords in its own log output to `***`). No other path is allowed.
- **Adapter scope.** A `RepoCreator` impl MUST refuse to operate outside the organization specified in the project's `[[registry]].url`. A publish run targeting `github.com/vibespecs` may not create, modify, or even probe a repository under a different `github.com` org or under any user namespace. Adapter implementations carry an explicit org-prefix check and surface a `PublishError` on attempted scope escalation.

**Why this is a §20-level invariant rather than module-local.** The blast radius of a leaked publish token is the entire organization the token has access to (cross-repo writes, branch deletes, CI secret read). The blast radius of an escalated adapter is the entire host account. Both failure modes are catastrophic in a way that hand-rolled module discipline cannot bound at the language level — the only safe posture is to make the rules global, audit every code path that touches a `Token` or a `RepoCreator`, and reject changes that introduce a new escape hatch.

**Where pinned (operationally):** [PROP-002 §2.10](../modules/vibe-registry/PROP-002-decentralized-registry.md#publish) carries the publish-side mechanics; [`spec/boot/90-user.md`](../boot/90-user.md) carries the operator-facing rule for this machine. Both are subordinate to this PROP-000 entry.

---

## Invariants

(These restate the most load-bearing rules from the spec and the book. If anything below seems violated in practice, stop and reconcile before proceeding.)

1. **Vocabulary lock.** Never use Maven's "lifecycle/phase/goal" or Bazel's internal terminology in user-facing or internal code. The installable kinds are `flow`, `feat`, `stack`, `tool`, `mcp` — the register grows only by owner amendment to `VIBEVM-SPEC.md` §4.1 (`app` is anticipated). The canonical process discipline vocabulary is the one in `VIBEVM-SPEC.md` §4 and the book.
2. **`spec/` is fixed.** The directory name and role cannot be configured away in v1.
3. **User-owned files are never written by `vibe`.** `spec/boot/00-core.md` and `spec/boot/90-user.md` are off-limits to install/uninstall/update.
4. **One commit = one logical unit.** Commit messages follow Conventional Commits (see §12) and reference `spec://…` URIs where relevant.
5. **Dogfood.** vibevm is being built using the same discipline it enforces. The `spec/` tree in this repo IS `vibe init`'s reference output.
6. **Human authorship is the only attribution.** See §12.1. This is the only place in the project where AI tooling is discussed in the attribution sense.
7. **Tokens never appear in vibevm output.** See §20. Audited in unit tests; any new code path touching a `Token` or `RepoCreator` is reviewed for redaction and scope-escalation safety.
