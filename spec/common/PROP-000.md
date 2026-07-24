# PROP-000: Foundational technical decisions {#root}

<status stage="spec" state="done" comment="B1 2026-07-24: foundational decisions in force; had no legacy status line; fact grain 2026-07-24"/>

- ##DOC-PINS This document pins the foundational technical decisions for the vibevm implementation. @spec/done
- ##ASSUME-TRUE Every subsequent PROP/FEAT may assume these are true. @spec/done
- ##AMEND-FIRST Changing any of them requires an explicit amendment here first, then downstream updates. @spec/done

- ##SOURCE-AUTHORITY Source authority: [`VIBEVM-SPEC.md`](../../VIBEVM-SPEC.md) §10 and the book in `refs/book/`. @spec/done
- ##SPEC-WINS Where this PROP and the spec disagree, the spec wins. @spec/done

---

## 1. Language: Rust {#language}

##LANG-RUST **Decision:** The vibevm CLI and all supporting crates are written in Rust. @spec/done

##LANG-WHY **Why:** Single-binary distribution, no runtime dependency, cross-platform by default, strong type system aligned with the project's discipline philosophy, excellent CLI ecosystem (`clap`, `serde`, `toml`, `reqwest`, `git2`, `tokio`, `anyhow`, `thiserror`, `tracing`, `dialoguer`, `console`, `sha2`). See `VIBEVM-SPEC.md` §10.1. @spec/done

- ##MSRV-POLICY **MSRV:** Latest stable at the time of each milestone. @spec/done
- ##MSRV-M0 M0 pins MSRV to the latest stable at the tag. @spec/done

##LANG-REVISIT **When to revisit:** Never, in the scope of v1. If Rust proves inadequate for a future milestone, open a new PROP superseding this one. @spec/done

---

## 2. Build system: Cargo workspace {#build}

##WORKSPACE-LAYOUT **Decision:** Single Cargo workspace at repo root. Crates live under `crates/` per `VIBEVM-SPEC.md` §10.2: @spec/done

- ##CRATE-CLI `vibe-cli` — CLI entry point, argument parsing. @spec/done
- ##CRATE-CORE `vibe-core` — types, manifest schemas, graph model. @spec/done
- ##CRATE-GRAPH `vibe-graph` — graph builder and runner. @spec/done
- ##CRATE-REGISTRY `vibe-registry` — registry fetch/cache/resolve. @spec/done
- ##CRATE-INSTALL `vibe-install` — install/uninstall/update logic. @spec/done
- ##CRATE-LLM `vibe-llm` — LLM provider abstraction (stub in M0, real in M1.5). @spec/done
- ##CRATE-CHECK `vibe-check` — linter (M1). @spec/done

##BUILD-WHY **Why:** Standard Rust workspace layout, enables shared dependency versions via `[workspace.dependencies]`, supports independent testing of each crate. @spec/done

##BINARY-NAME **Binary name:** `vibe` (built from `vibe-cli`). @spec/done

---

## 3. License {#license}

- ##LICENSE-EULA **Decision:** vibevm itself ships under a **proprietary EULA** in this phase (source-available, not open source). See [`LICENSE.md`](../../LICENSE.md) at the repo root for the placeholder terms. @spec/done
- ##NO-CRATES-IO Crates in this workspace set `license-file = "LICENSE.md"` and `publish = false` so none of them can be accidentally pushed to crates.io. @spec/done

- ##LICENSE-OWNER-CALL **Why:** Owner's call — intent is to eventually relicense under the Universal Permissive License 1.0 (UPL), but that decision is not final. Until then, vibevm stays proprietary. @spec/done
- ##LICENSE-SPEC-DEFERS `VIBEVM-SPEC.md` §1 explicitly defers the *produced* software's license to the owner; the owner's choice is this proprietary EULA. @spec/done

- ##DEPS-PERMISSIVE-ONLY **Third-party dependencies remain permissive-only.** Per `VIBEVM-SPEC.md` §10.3: every crate we depend on must be MIT / Apache-2.0 / BSD or equivalent. @spec/done
- ##COPYLEFT-FORBIDDEN GPL / AGPL / LGPL are forbidden, period. @spec/done
- ##PROPRIETARY-TIGHTENS The proprietary license of vibevm itself does not relax that constraint — it makes it more important, because anything we link becomes mingled with our proprietary code, and copyleft would force relicensing. @spec/done

##LICENSE-REVISIT **When to revisit:** When the owner decides to relicense (most likely UPL 1.0). At that point, swap `LICENSE.md`, update the workspace `license-file` (or switch back to an SPDX `license` string like `UPL-1.0`), and remove `publish = false` if desired. @spec/done

---

## 4. Manifest format: TOML {#manifests}

##MANIFEST-TOML **Decision:** All vibevm manifests use TOML 1.0 (`toml` crate, `serde`-based). @spec/done

##files-lead Files: @spec/done

- ##FILE-VIBE-TOML `vibe.toml` — project manifest. Schema: `VIBEVM-SPEC.md` §7.5. @spec/done
- ##FILE-PACKAGE-TOML `vibe-package.toml` — package manifest. Schema: `VIBEVM-SPEC.md` §7.3. @spec/done
- ##FILE-LOCK `vibe.lock` — lockfile. Schema: `VIBEVM-SPEC.md` §7.4. @spec/done

##TOML-WHY **Why:** TOML is the Rust ecosystem default (cargo), readable, has clear escaping rules, and maps cleanly to `serde` structs. See `VIBEVM-SPEC.md` §10.1. @spec/done

---

## 5. Directory layout {#layout}

- ##LAYOUT-PER-SPEC **Decision:** Per `VIBEVM-SPEC.md` §4.2. @spec/done
- ##SPEC-DIR-FIXED The `spec/` directory is hardcoded — never configurable in v1. @spec/done
- ##VIBE-DIR-IGNORED The `.vibe/` cache directory is gitignored and per-project. @spec/done
- ##REFS-SRC-IGNORED `refs/src/` is gitignored (external reference sources, cloned by the implementer for study, not part of the vibevm repo itself). @spec/done

---

## 6. Package identity {#identity}

- ##IDENTITY-FORM **Decision:** `<kind>:<name>@<version>` per `VIBEVM-SPEC.md` §7.1. @spec/done
- ##KIND-SET `kind ∈ {flow, feat, stack, tool}`. @spec/done
- ##NAME-VERSION-RULES `name` is kebab-case, unique within kind. `version` is semver. @spec/done

##constraint-forms-lead Constraint forms in CLI: @spec/done

- ##CF-LATEST `flow:wal` → latest stable. @spec/done
- ##CF-EXACT `flow:wal@0.3.0` → exact. @spec/done
- ##CF-RANGE `flow:wal@^0.3` → semver range. @spec/done

---

## 7. Registry model (M0 vs M1) {#registry}

- ##REG-M0 **Decision:** **M0:** local-directory registry only. No git. Registry is a path on disk with the layout from `VIBEVM-SPEC.md` §8.2. @spec/done
- ##REG-M1 **M1:** git registry added per `VIBEVM-SPEC.md` §8. Configured in `vibe.toml`'s `[[registry]]` array. Default public registry URL = `https://github.com/vibespecs` (HTTPS org root; per-package URLs are derived at fetch time via [`NamingConvention`](../../crates/vibe-core/src/manifest/project.rs)). @spec/done
- ##REG-BACKEND-PIN **Backend choice, trait design, cache layout, and Windows UX for M1** are pinned in [spec://vibevm/modules/vibe-registry/PROP-001](../modules/vibe-registry/PROP-001-git-backend.md) — in brief: shell-out to the system `git` (not `libgit2`), behind a `GitBackend` trait that leaves the door open for a future `libgit2` swap. @spec/done

- ##INIT-DEFAULT-REGISTRY **Default in new projects.** `vibe init` writes the default registry URL (`DEFAULT_REGISTRY_URL` in [`vibe_core::manifest`](../../crates/vibe-core/src/manifest/project.rs)) into every new `vibe.toml`'s `[[registry]]` entry unless the operator passes `--no-registry` or overrides with `--registry-url <URL>` / `--registry-ref <REF>`. @spec/done
- ##DEFAULT-RATIONALE The default exists so that a plain `vibe init` → `vibe install flow:wal` flow works out of the box against the public registry; overrides are there for forks, staging registries, and offline / air-gapped setups. @spec/done
- ##URL-SSOT The single source of truth for the URL is the constant in `vibe-core` — manual-tests, smoke scripts, and docs all reference it from there. @spec/done

##SPLIT-HOST-POSTURE **Source repositories — split-host posture.** The vibevm project and the package registry live on **separate hosts** by deliberate decision (2026-04-29). Each host is chosen on its own merits: @spec/done

- ##HOST-SOURCE-GITVERSE **vibevm tool source: GitVerse.** `git@gitverse.ru:vibevm/vibevm.git` (SSH) / `https://gitverse.ru/vibevm/vibevm` (web). Stays on GitVerse — the source-of-truth repository, contributor SSH keys, mirroring posture, and Russian-jurisdiction hosting are all already wired up here. @spec/done
- ##HOST-REGISTRY-GITHUB **Package registry: GitHub, organization `vibespecs`.** `https://github.com/vibespecs` (org root) — per-package repos are `https://github.com/vibespecs/<kind>-<name>` per [PROP-002](../modules/vibe-registry/PROP-002-decentralized-registry.md#registry-model) `NamingConvention::KindName`. @spec/done
  - ##REG-MIGRATION-WHY The migration from `git@gitverse.ru:vibespecs/*` happened on 2026-04-29 because GitVerse's public REST API does not expose org-scoped repo creation (`POST /orgs/{org}/repos` returns 404 / WAF 403; documented exhaustively in [PROP-002 §2.10](../modules/vibe-registry/PROP-002-decentralized-registry.md#publish) and `crates/vibe-publish/src/gitverse.rs`). Without that endpoint `vibe registry publish` cannot fully drive the publish loop end to end. @spec/done
  - ##REG-GITHUB-WORKS GitHub's equivalent endpoint works natively, so the registry organization moved while the vibevm project repository stays put. @spec/done
  - ##REG-HASH-STABLE Identity is content-hashed (PROP-002 §2.1) — the lockfile's `source_url` rotates but no `content_hash` value is invalidated by the host change. @spec/done
- ##HOST-LEGACY-READONLY **Legacy registry, read-only.** `git@gitverse.ru:anarchic/vibespecs.git` (HEAD `2203239`, 2026-04-23, three v0.1.0 flows in monorepo form). Kept readable for any project still on schema-v1 lockfiles until they migrate; no new publishes happen there. @spec/done

- ##CACHE-REGISTRIES **Cache location:** `~/.vibe/registries/<hash>/` for cloned registries. @spec/done
- ##CACHE-PACKAGES `<project>/.vibe/cache/<kind>/<name>/<version>/` for per-package cache. See `VIBEVM-SPEC.md` §8.3. @spec/done

---

## 8. Task graph model {#graph}

- ##GRAPH-BUILTIN-NODES **Decision:** Built-in nodes only in v1 (content-only plugin contribution model per `VIBEVM-SPEC.md` §5.4). @spec/done
- ##RUNNER-SEQUENTIAL Runner is sequential (no parallelism) in v1 per §5.2. @spec/done
- ##TYPED-VALUES Typed value system per §5.3. @spec/done

##WORKFLOWS-QUERIES Workflows are graph queries (target node + transitive dependencies) per §5.5. @spec/done

---

## 9. Conflict resolution {#conflicts}

##CONFLICT-ORDER vibevm's writer-conflict resolution — the **Human > Spec > Tests > Code** order (also pinned in [`VIBEVM-SPEC.md`](../../VIBEVM-SPEC.md) §2.2 and book chapter 1) — is the `conflict-protocol` flow: `spec://org.vibevm.world/conflict-protocol/flows/conflict-protocol/CONFLICT-PROTOCOL#root`. @spec/done

---

## 10. Observability {#observability}

- ##OBS-TRACING **Decision:** Use `tracing` for structured logs. @spec/done
- ##OBS-OUTPUT-MODES CLI defaults to human-readable Markdown-flavored output; `--json` for machine-readable; `--quiet` for one-line summaries. @spec/done
- ##OBS-EXIT-CODES Exit codes per `VIBEVM-SPEC.md` §9.4. @spec/done

---

## 11. Cross-platform target {#platforms}

- ##PLATFORMS-TRIO **Decision:** M0 builds and runs on macOS, Linux, and Windows. @spec/done
- ##PATH-HANDLING Path handling goes through `std::path::Path` — no manual separator manipulation. @spec/done
- ##CASING-RULES File operations respect platform casing rules where the OS enforces them. @spec/done

- ##DEV-ON-WINDOWS **Test matrix:** M0 dev is primarily on Windows 11 (this machine). @spec/done
- ##CI-MATRIX-M2 CI matrix for all three OSes lands in M2 per `VIBEVM-SPEC.md` §11.4. @spec/done

---

## 12. Commit and push discipline {#commits}

##GIT-PRACTICES-FAMILY The repository's commit-and-push discipline is the **git-practices** family (a host dependency), whose members carry the full text: @spec/done

- ##GP-ATTRIBUTION human-authored **attribution** — `spec://org.vibevm.world/git-attribution-policy/flows/attribution-policy/ATTRIBUTION-POLICY#root`; @spec/done
- ##GP-CONVENTIONAL the **Conventional Commits** message format — `spec://org.vibevm.world/git-conventional-commits/flows/conventional-commits/conventional-commits#root`; @spec/done
- ##GP-ATOMICITY **atomicity**, one commit = one logical idea — `spec://org.vibevm.world/git-atomic-commits/flows/atomic-commits/ATOMIC-COMMITS-PROTOCOL#root`; @spec/done
- ##GP-AUTONOMY commit **autonomy** — routine proceeds, and the red lines (history rewrites, force-push, large blobs, CI / signing / secrets) stop and ask — `spec://org.vibevm.world/git-autonomy/flows/autonomy/AUTONOMY-PROTOCOL#root`. @spec/done

---

## 13. Package layout convention {#package-layout}

- ##MIRROR-LAYOUT **Decision:** vibevm packages use a **mirror layout**. Every entry in a package's `writes.files` is simultaneously (a) the path of the file inside the package directory and (b) the path at which it will be installed in the consumer's project. @spec/done
- ##NO-TARGET-FIELD There is no separate `target = "…"` field per entry; `writes.files` is the single source of truth for "where does this file go?" @spec/done

##mirror-example Concretely, the canonical `flow:wal@0.1.0` payload (vendored as a hermetic e2e test fixture under `fixtures/registry/flow/wal/v0.1.0/`) contains `spec/flows/wal/WAL-PROTOCOL.md` at exactly that relative path; after `vibe install flow:wal`, the file lives at `spec/flows/wal/WAL-PROTOCOL.md` inside the user's project. No mapping, no rewriting. @spec/done

##BOOT-SNIPPET-EXCEPTION **Boot snippets are the one exception.** The `[boot_snippet]` table carries an explicit `source` field naming the path inside the package (conventionally under `boot/`), while the target is always the fixed `spec/boot/<filename>`. @spec/done

- ##MIRROR-WHY-DRIFT **Why:** a single source of truth for source-and-target paths eliminates a whole class of authoring bug where the package layout drifts from the declared writes. @spec/done
- ##MIRROR-WHY-READABLE It also makes a package directory instantly readable — a human looking at the tree knows exactly what will appear in a consumer's project without cross-referencing a separate mapping table. @spec/done

- ##MIRROR-PINNED **Where pinned:** `VIBEVM-SPEC.md` §13.1 shows the mirror-layout diagram and §13.2 the matching manifest. This PROP-000 entry is the decision record; the spec carries the operational definition. @spec/done
- ##INSTALL-RELIES `vibe-install` relies on this convention — the source path of a planned write is computed by joining `cache_dir` with the manifest's declared target path. @spec/done

---

## 14. Manual-test protocol {#manual-tests}

- ##MT-LOCATION **Decision:** human-runnable smoke-tests live in [`manual-tests/`](../../manual-tests/) at the repo root, one Markdown file per scenario, named `<milestone>-<slug>.md` (e.g. `M1.1-git-registry-smoke.md`); the directory's own [`README.md`](../../manual-tests/README.md) carries the index. @spec/done
- ##MT-FLOW-POINTER The tier itself — why a project keeps a second, human-run test layer alongside the automated suite, what a manual test is and is not, when to run one, and who signs it off (a human, over an agent's pre-run) — is the `manual-tests` flow: `spec://org.vibevm.world/manual-tests/flows/manual-tests/MANUAL-TESTS-PROTOCOL#root`, with the four authoring rules in its `authoring-rules` document and the copy-ready skeleton in `test-template`. @spec/done

- ##MT-HERMETIC-SUITE **vibevm's real-world surfaces.** `cargo test --workspace` uses fakes, tempdirs, and local bare repositories for speed and hermeticity (the flow's `spec://org.vibevm.world/manual-tests/flows/manual-tests/MANUAL-TESTS-PROTOCOL#why-second-tier`). @spec/done
- ##MT-LAST-MILE The manual tier is the last mile for what only the real world has here: SSH auth against GitVerse, the lockfile `source_uri` exactly as a downstream consumer receives it, the `~/.vibe/` layout on a user's actual filesystem, and a human confirming the CLI output says what they meant. @spec/done

- ##MT-ISOLATION **vibevm's bindings.** Every test isolates state with `mktemp -d` for the project and `VIBE_REGISTRY_CACHE` pointing inside the scratch dir for the registry cache — the user's real `~/.vibe/` is never touched by a run. @spec/done
- ##MT-GIT-BASH Git Bash on Windows is the primary smoke-test environment (macOS and Linux must work too); where platform output differs (path separators, `.exe` suffix), the Windows form comes first with a portable note. @spec/done

##mt-when-lead **When to run** (the flow's `spec://org.vibevm.world/manual-tests/flows/manual-tests/MANUAL-TESTS-PROTOCOL#when`, against vibevm's surfaces): @spec/done

- ##MT-WHEN-TAGGING before tagging any milestone; @spec/done
- ##MT-WHEN-CHANGES after changes to the git backend, CLI arg parsing, or lockfile format even when `cargo test` stays green; @spec/done
- ##MT-WHEN-REPRO and as reproducers whenever a user files an integration bug. @spec/done

##MT-WAL-NAMES [`spec/WAL.md`](../WAL.md) names the outstanding manual runs for the current milestone. @spec/done

---

## 15. Dependency weight is not a decision factor {#dep-weight}

- ##DEP-WEIGHT-NOT-FACTOR **Decision:** Binary size, crate count, transitive dep weight are NOT decision factors when selecting third-party libraries. @spec/done
- ##PICK-STRONGEST Pick the strongest available library for the job — for both the Rust CLI and any future Java / frontend side. @spec/done

- ##WHY-PRECEDENT **Why:** Software of comparable surface area (Chrome, modern IDEs, production package managers) routinely ships tens to hundreds of dependencies and remains fast and capable. @spec/done
- ##WHY-DEBT Under-specifying a load-bearing component to save megabytes creates ongoing architectural debt that is much more expensive to repay than the weight it saves. vibevm intends to be best-in-class, and best-in-class means using best-in-class primitives. @spec/done

##reject-reasons-lead **Legitimate reasons to reject a dep:** @spec/done

- ##REJECT-LICENSE non-permissive license (see §3 — MIT / Apache-2.0 / BSD / Unlicense only; GPL / AGPL / LGPL forbidden; MPL-2.0 allowed case by case, since its weak copyleft does not taint consumers), @spec/done
- ##REJECT-ABANDONED abandoned upstream, @spec/done
- ##REJECT-SECURITY demonstrated security issues (CVE history, unpatched known exploit), @spec/done
- ##REJECT-ERGONOMICS fundamentally bad API ergonomics that would propagate into our own interfaces. @spec/done

##TOO-HEAVY-NOT-REASON "Too heavy" alone is **not** a reason. @spec/done

- ##READMISSIBLE **Concrete consequences:** libraries previously rejected on footprint grounds are re-admissible. Notable: `libsolv` (C, with Rust bindings), `git2` (wrapping `libgit2`), bundled native C deps, embedded interpreters when justified. @spec/done
- ##PROP-001-PRUNE The size-based argument in [PROP-001 §2.1](../modules/vibe-registry/PROP-001-git-backend.md#backend) against `git2` is to be pruned — the remaining arguments (Windows SSH auth, shell-out diagnostic clarity) may still carry that decision, but not the size one. @spec/done

---

## 16. JTD + codegen for wire contracts {#jtd}

- ##JTD-SSOT **Decision:** JSON Type Definition (RFC 8927) schemas are the single source of truth for every client/server and machine-to-machine contract in this project. @spec/done
- ##JTD-CODEGEN Rust types — and types in any future non-Rust clients — are **generated** from JTD schemas via `jtd-codegen`, not hand-maintained. @spec/done
- ##NO-DUPLICATION No client/server duplication is permitted on contracts. @spec/done

- ##JTD-WHY-SKEW **Why:** duplication between a server contract and a hand-written client is a classic source of version-skew bugs; schema-first codegen eliminates that class of bug categorically. @spec/done
- ##JTD-OVER-JSONSCHEMA JTD specifically (over JSON Schema / OpenAPI alone) because JTD is deliberately narrower: its schema grammar is constructed so every JTD schema maps to a clean static type in every target language, with no language-specific escape hatches. @spec/done

##JTD-IN-SCOPE **In scope:** LLM provider API wrappers (Anthropic, OpenAI, OpenRouter, Ollama), GitVerse public-API client, `vibe --json` CLI output, telemetry / event log formats, future hosted-registry HTTP surface. @spec/done

##JTD-OUT-OF-SCOPE **Out of scope:** human-authored manifests — `vibe.toml`, `vibe.lock`, `vibe-package.toml` — stay TOML via `serde`. JTD is for wire, not for configs humans hand-edit. @spec/done

##toolchain-lead **Toolchain placement:** @spec/done

- ##TC-BINARY `jtd-codegen` binary in project-local `tools/jtd-codegen/` (gitignored; version pinned). @spec/done
- ##TC-SCHEMAS Schemas in `schemas/` at repo root, one `.jtd.json` file per contract, committed. @spec/done
- ##TC-GENERATED Generated Rust code in `crates/vibe-wire/src/generated/`, committed, with a `// DO NOT EDIT — regenerate via cargo xtask codegen` header on every file. @spec/done
- ##TC-REGEN Regeneration via `cargo xtask codegen`. CI enforces zero drift (`cargo xtask codegen && git diff --exit-code`). @spec/done

- ##TC-AGENT-SETS-UP **Toolchain install ownership:** the coding agent sets up the codegen toolchain itself. @spec/done
- ##TC-RUNAS Machine-global changes (PATH mutation, admin-level installs, env-var additions) go through `runas` with an operator confirmation at the moment of the change. @spec/done

---

## 17. Production architecture in the prototype phase {#prod-arch}

- ##PROD-QUALITY-DAY-ONE **Decision:** Load-bearing surfaces — lockfile schema, registry protocol, dep-resolver semantics, wire formats, identity model — are designed to production quality from day one. @spec/done
- ##FORMATS-BIND The project is a prototype today; the formats and protocols it chooses today are the ones its future users will be bound to. Changing them later is orders of magnitude more expensive than designing them correctly now. @spec/done

- ##PRINCIPAL-LENS **Lens:** "a principal engineer at a top-tier infrastructure company, designing a format or protocol that will be used by millions" is one of the reflection lenses to reach for when a design decision lands. @spec/done
- ##LENS-NOT-ONLY It is **not** the only lens — "the simplest thing that works" remains valid for leaf features — but architecture-heavy surfaces prefer the principal-engineer lens. @spec/done

##prod-consequences-lead **Consequences:** @spec/done

- ##PC-PREFER-DESIGNED Prefer a recent-but-well-designed library over a tactical shortcut, even when the shortcut is cheaper in the short term. @spec/done
- ##PC-EXTENSION-POINTS Extension points, versioning markers, and forward-compatibility hooks land with the initial cut, not in a later "hardening" pass. @spec/done
- ##PC-REVERSIBILITY Reversibility matters: if a format or protocol decision is hard to reverse (lockfile schema, registry URL scheme, identity hash), lean heavier into design rigour before first commit. @spec/done
- ##PC-FIX-LATER-SCOPE "We'll fix it later" is a valid stance only for implementation quality inside a well-chosen architectural surface — not for the surface itself. @spec/done

---

## 18. Complexity expectation: higher than RPM {#complexity}

- ##RPM-CLASS-TARGET **Decision:** The dependency / package model is designed to handle complexity **at least** matching RPM-class systems (zypper, DNF), and in several dimensions greater. @spec/done
- ##RICH-DEPS-DAY-ONE Manifest grammar and lockfile schema reserve fields for — and the resolver actually implements — capabilities, provides / requires / obsoletes / conflicts / supplements / recommends, disjunctions (`A or B`), boolean rich-dep syntax, capability-based resolve, multi-kind cross-deps, and semantic (LLM-reviewed) conflicts. These are designed in from day one, not deferred. @spec/done

- ##WIDER-THAN-RPM **Why:** vibevm's dependency surface is not simpler than RPM — it is wider. A `feat` package may require a `stack` providing a specific capability, `flow`s may declare semantic compatibility with other `flow`s, LLM-backed review adds a non-mechanical conflict dimension RPM never had. @spec/done
- ##UNDERSHOOT-COST Undershoot — picking a resolver that lacks virtual packages or disjunctions, or a manifest that cannot express capability-based requires — would force an incompatible schema migration after users exist. @spec/done

- ##RESOLVER-RESOLVO **Resolver choice** (pinned in the module PROP): `resolvo` crate as the primary depsolver. @spec/done
- ##RESOLVER-LIBSOLV-FALLBACK `libsolv` as an explicit FFI-backed fallback behind a `DepSolver` trait (analogous to [PROP-001 §2.2](../modules/vibe-registry/PROP-001-git-backend.md#backend-trait)'s `GitBackend` pattern). @spec/done
- ##RESOLVER-PUBGRUB-REJECTED PubGrub is rejected for the *primary* role — its algorithm does not handle virtual packages or disjunctions — but is acceptable for explanatory rendering of conflicts in CLI output if it proves superior there. @spec/done

---

## 19. Load-bearing setup documentation {#setup-docs}

##SETUP-DOCS **Decision:** Two files at the repo root are load-bearing for the project: @spec/done

- ##DOC-DEV-GUIDE [`DEV-GUIDE.md`](../../DEV-GUIDE.md) — contributor-facing: everything to install on a fresh machine to clone, build, test, contribute to, and (if authorized) publish from this repository. @spec/done
- ##DOC-RUNTIME-GUIDE [`RUNTIME-GUIDE.md`](../../RUNTIME-GUIDE.md) — user-facing: everything to install and env-configure to run the shipped `vibe` CLI. @spec/done

##SETUP-DOCS-FLOW vibevm's setup docs are [`DEV-GUIDE.md`](../../DEV-GUIDE.md) (contributor / build) and [`RUNTIME-GUIDE.md`](../../RUNTIME-GUIDE.md) (runtime / user). The same-commit obligation that binds them is the `dev-runtime-docs` flow: `spec://org.vibevm.world/dev-runtime-docs/flows/dev-runtime-docs/DEV-RUNTIME-DOCS-PROTOCOL#obligation`. @spec/done

---

## 20. Token secrecy and adapter scope {#token-secrecy}

##req-token-secrecy `req r1` @spec/done

- ##TOKEN-SURFACE-SECRET **Decision.** Publish tokens, registry-API tokens, and any LLM-provider keys handled by vibevm are **surface secrets** in the sense of the `secrets-hygiene` flow (`spec://org.vibevm.world/secrets-hygiene/flows/secrets-hygiene/SECRETS-HYGIENE-PROTOCOL#surface-secret`): their **value** MUST NOT appear on any surface vibevm produces, though their **source** (env-var name, file path) may be printed. @spec/done
- ##token-bindings-lead vibevm's bindings of the flow's four laws (`spec://org.vibevm.world/secrets-hygiene/flows/secrets-hygiene/SECRETS-HYGIENE-PROTOCOL#laws`): @spec/done

- ##TS-NEVER-PRINTED **Never printed.** Not to stdout, stderr, the CLI log, the `--json` event stream, error messages, panic traces, telemetry, or the lockfile. The CLI prints the *source* of a token (explicit / env-var name / file path) but never the value. The in-process wrapper types (`vibe_publish::Token`, future `vibe_llm::ApiKey`) MUST redact on `Display` and `Debug` — verified by unit tests (the flow's `spec://org.vibevm.world/secrets-hygiene/flows/secrets-hygiene/SECRETS-HYGIENE-PROTOCOL#law-tested`). @spec/done
- ##TS-NEVER-PERSISTED **Never persisted.** Not committed to the repository, not written into the lockfile, not embedded in cache files, not landed in the `.vibe/` tree. The single sanctioned at-rest location is the operator's `~/.vibe/<host>.publish.token` file (per-user, chmod-protected). @spec/done
- ##TS-BOUNDARIES **Sanctioned process boundaries.** The token may cross a process boundary only via: (a) the host API's `Authorization: Bearer …` header, sent over TLS; (b) a single `git remote add` / `git push` invocation where the token is embedded in the URL as `https://x-access-token:<TOKEN>@host/…` (modern git ≥ 2.31 redacts URL passwords in its own log output to `***`). No other path is allowed — in particular, never into a spawned third-party hook's environment (the flow's `#law-boundaries`). @spec/done
- ##TS-ADAPTER-SCOPE **Adapter scope.** A `RepoCreator` impl MUST refuse to operate outside the organization specified in the project's `[[registry]].url`. A publish run targeting `github.com/vibespecs` may not create, modify, or even probe a repository under a different `github.com` org or under any user namespace. Adapter implementations carry an explicit org-prefix check and surface a `PublishError` on attempted scope escalation. (The general integration-scope discipline is the flow's `scope-discipline` document.) @spec/done

- ##BLAST-RADIUS **Why global, not module-local.** The blast radius of a leaked publish token is the entire organization it can reach (cross-repo writes, branch deletes, CI-secret reads); of an escalated adapter, the entire host account (the flow's `spec://org.vibevm.world/secrets-hygiene/flows/secrets-hygiene/SECRETS-HYGIENE-PROTOCOL#blast-radius`). Both are catastrophic beyond what module-local discipline can bound, so the rules are global and every code path touching a `Token` or a `RepoCreator` is audited. @spec/done
- ##ROTATE-FIRST On any suspected leak, rotate first (the flow's `#leak-drill`). @spec/done

##TOKEN-PINNED-WHERE **Where pinned (operationally):** [PROP-002 §2.10](../modules/vibe-registry/PROP-002-decentralized-registry.md#publish) carries the publish-side mechanics; [`spec/boot/90-user.md`](../boot/90-user.md) carries the operator-facing rule for this machine. Both are subordinate to this PROP-000 entry. @spec/done

---

## Invariants {#invariants}

##INVARIANTS-STOP-RULE (These restate the most load-bearing rules from the spec and the book. If anything below seems violated in practice, stop and reconcile before proceeding.) @spec/done

1. ##INV-VOCABULARY **Vocabulary lock.** Never use Maven's "lifecycle/phase/goal" or Bazel's internal terminology in user-facing or internal code. The installable kinds are `flow`, `feat`, `stack`, `tool`, `mcp` — the register grows only by owner amendment to `VIBEVM-SPEC.md` §4.1 (`app` is anticipated). The canonical process discipline vocabulary is the one in `VIBEVM-SPEC.md` §4 and the book. @spec/done
2. ##INV-SPEC-DIR **`spec/` is fixed.** The directory name and role cannot be configured away in v1. @spec/done
3. ##INV-USER-FILES **User-owned files are never written by `vibe`.** `spec/boot/00-core.md` and `spec/boot/90-user.md` are off-limits to install/uninstall/update. @spec/done
4. ##INV-ATOMIC-COMMITS **One commit = one logical unit.** Commit messages follow the git-practices family (§12) and reference `spec://…` URIs where relevant. @spec/done
5. ##INV-DOGFOOD **Dogfood.** vibevm is being built using the same discipline it enforces. The `spec/` tree in this repo IS `vibe init`'s reference output. @spec/done
6. ##INV-HUMAN-AUTHORSHIP **Human authorship is the only attribution.** The posture is the `attribution-policy` flow (a git-practices member, §12); everywhere else assume human authorship only. @spec/done
7. ##INV-TOKEN-SECRECY **Tokens never appear in vibevm output.** See §20. Audited in unit tests; any new code path touching a `Token` or `RepoCreator` is reviewed for redaction and scope-escalation safety. @spec/done
