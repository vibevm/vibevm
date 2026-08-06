# PROP-000: Foundational technical decisions {#root}

<status stage="spec" state="done" comment="B1 2026-07-24: foundational decisions in force; had no legacy status line; fact grain 2026-07-24"/>

- @fact:DOC-PINS This document pins the foundational technical decisions for the vibevm implementation. @status:spec/done
- @fact:ASSUME-TRUE Every subsequent PROP/FEAT may assume these are true. @status:spec/done
- @fact:AMEND-FIRST Changing any of them requires an explicit amendment here first, then downstream updates. @status:spec/done

- @fact:SOURCE-AUTHORITY Source authority: [`VIBEVM-SPEC.md`](../../VIBEVM-SPEC.md) §10 and the book in `refs/book/`. @status:spec/done
- @fact:SPEC-WINS Where this PROP and the spec disagree, the spec wins. @status:spec/done

---

## 1. Language: Rust {#language}

@fact:LANG-RUST **Decision:** The vibevm CLI and all supporting crates are written in Rust. @status:spec/done

@fact:LANG-WHY **Why:** Single-binary distribution, no runtime dependency, cross-platform by default, strong type system aligned with the project's discipline philosophy, excellent CLI ecosystem (`clap`, `serde`, `toml`, `reqwest`, `git2`, `tokio`, `anyhow`, `thiserror`, `tracing`, `dialoguer`, `console`, `sha2`). See `VIBEVM-SPEC.md` §10.1. @status:spec/done

- @fact:MSRV-POLICY **MSRV:** Latest stable at the time of each milestone. @status:spec/done
- @fact:MSRV-M0 M0 pins MSRV to the latest stable at the tag. @status:spec/done

@fact:LANG-REVISIT **When to revisit:** Never, in the scope of v1. If Rust proves inadequate for a future milestone, open a new PROP superseding this one. @status:spec/done

---

## 2. Build system: Cargo workspace {#build}

@fact:WORKSPACE-LAYOUT **Decision:** Single Cargo workspace at repo root. Crates live under `crates/` per `VIBEVM-SPEC.md` §10.2: @status:spec/done

- @fact:CRATE-CLI `vibe-cli` — CLI entry point, argument parsing. @status:spec/done
- @fact:CRATE-CORE `vibe-core` — types, manifest schemas, graph model. @status:spec/done
- @fact:CRATE-GRAPH `vibe-graph` — graph builder and runner. @status:spec/done
- @fact:CRATE-REGISTRY `vibe-registry` — registry fetch/cache/resolve. @status:spec/done
- @fact:CRATE-INSTALL `vibe-install` — install/uninstall/update logic. @status:spec/done
- @fact:CRATE-LLM `vibe-llm` — LLM provider abstraction (stub in M0, real in M1.5). @status:spec/done
- @fact:CRATE-CHECK `vibe-check` — linter (M1). @status:spec/done

@fact:BUILD-WHY **Why:** Standard Rust workspace layout, enables shared dependency versions via `[workspace.dependencies]`, supports independent testing of each crate. @status:spec/done

@fact:BINARY-NAME **Binary name:** `vibe` (built from `vibe-cli`). @status:spec/done

---

## 3. License {#license}

- @fact:LICENSE-EULA **Decision:** vibevm ships under the **Universal Permissive License 1.0** (UPL-1.0) — open source, relicensed 2026-07-12. See [`LICENSE.md`](../../LICENSE.md) at the repo root for the terms. The scope is *this* repository's shipped surface — the host tree and every `packages/org.vibevm.*` package; separately-developed products carry their own licences and are not governed here. The project's first phase shipped under a placeholder proprietary EULA; that phase is over. @status:impl/done
- @fact:NO-CRATES-IO Crates in this workspace set `license-file = "LICENSE.md"` and `publish = false` so none of them can be accidentally pushed to crates.io. @status:spec/done

- @fact:LICENSE-OWNER-CALL **Why:** Owner's call, taken 2026-07-12 and executed the same day: the whole shipped surface — the host tree and every `packages/org.vibevm.*` package — carries UPL-1.0, so a consumer of any part of vibevm gets one permissive licence and no per-package archaeology. @status:spec/done
- @fact:LICENSE-SPEC-DEFERS `VIBEVM-SPEC.md` §1 explicitly defers the *produced* software's license to the owner; the owner's choice is UPL-1.0. @status:spec/done
- @fact:license-rejected **Considered and rejected:** the placeholder **proprietary EULA** of the project's first phase — superseded 2026-07-12, *"that phase is over"* (`##LICENSE-EULA`); **per-package licensing** across the shipped surface — rejected because a consumer of any part of vibevm would then face per-package archaeology instead of one permissive licence (`##LICENSE-OWNER-CALL`); **any copyleft licence** (GPL / AGPL / LGPL) — never in contention, for the same reason it is forbidden in dependencies: it would force the whole product to relicense, *"which is exactly what UPL-1.0 exists to prevent"* (`##COPYLEFT-FORBIDDEN`, `##PROPRIETARY-TIGHTENS`). @status:spec/done

- @fact:DEPS-PERMISSIVE-ONLY **Third-party dependencies remain permissive-only.** Per `VIBEVM-SPEC.md` §10.3: every crate we depend on must be MIT / Apache-2.0 / BSD or equivalent. @status:spec/done
- @fact:COPYLEFT-FORBIDDEN GPL / AGPL / LGPL are forbidden, period. @status:spec/done
- @fact:PROPRIETARY-TIGHTENS The permissive license of vibevm itself does not relax that constraint — a copyleft dependency mingles with our code and would force the whole product to relicense, which is exactly what UPL-1.0 exists to prevent. @status:spec/done

@fact:LICENSE-REVISIT **When to revisit:** the previous trigger — "when the owner decides to relicense (most likely UPL 1.0)" — **fired on 2026-07-12** and is spent. Re-open when either (a) a crate is to be published to crates.io: swap `license-file` for the SPDX string `license = "UPL-1.0"` and drop `publish = false`; or (b) a dependency or contribution arrives under terms UPL-1.0 cannot absorb. @status:spec/done

---

## 4. Manifest format: TOML {#manifests}

@fact:MANIFEST-TOML **Decision:** All vibevm manifests use TOML 1.0 (`toml` crate, `serde`-based). @status:spec/done

@fact:files-lead Files: @status:spec/done

- @fact:FILE-VIBE-TOML `vibe.toml` — project manifest. Schema: `VIBEVM-SPEC.md` §7.5. @status:spec/done
- @fact:FILE-PACKAGE-TOML `vibe.toml` is the *only* manifest: one file per node, the role set by section (`[project]` ⊕ `[package]`, optionally `[workspace]`). The separate `vibe-package.toml` of the early schema is **retired** (workspace fork 7e); the package schema of `VIBEVM-SPEC.md` §7.3 now lives in `vibe.toml`'s `[package]` section. @status:impl/done
- @fact:FILE-LOCK `vibe.lock` — lockfile. Schema: `VIBEVM-SPEC.md` §7.4. @status:spec/done

@fact:TOML-WHY **Why:** TOML is the Rust ecosystem default (cargo), readable, has clear escaping rules, and maps cleanly to `serde` structs. See `VIBEVM-SPEC.md` §10.1. @status:spec/done

---

## 5. Directory layout {#layout}

- @fact:LAYOUT-PER-SPEC **Decision:** Per `VIBEVM-SPEC.md` §4.2. @status:spec/done
- @fact:SPEC-DIR-FIXED The `spec/` directory is hardcoded — never configurable in v1. @status:spec/done
- @fact:VIBE-DIR-IGNORED The `.vibe/` cache directory is gitignored and per-project. @status:spec/done
- @fact:REFS-SRC-IGNORED `refs/src/` is gitignored (external reference sources, cloned by the implementer for study, not part of the vibevm repo itself). @status:spec/done

---

## 6. Package identity {#identity}

- @fact:IDENTITY-FORM **Decision:** `[<kind>:]<group>/<name>@<version>` — identity is **qualified** since M1.19 ([PROP-008 §2.2](../modules/vibe-registry/PROP-008-qualified-naming.md#identity)); the unqualified `<kind>:<name>@<version>` of `VIBEVM-SPEC.md` §7.1 is CLI sugar that resolves once, at the human boundary. @status:impl/done
- @fact:KIND-SET `kind ∈ {flow, feat, stack, tool, mcp, lang}` — six kinds; `mcp` shipped with [PROP-027](../modules/vibe-mcp/PROP-027-mcp-packages.md). (§Invariants `INV-VOCABULARY` in this file carries the same list.) @status:impl/done
- @fact:NAME-VERSION-RULES `name` is kebab-case and `(group, name)` is globally unique ([PROP-008](../modules/vibe-registry/PROP-008-qualified-naming.md#identity) — uniqueness moved from *within kind* to *within group*). `version` is semver. @status:impl/done

@fact:constraint-forms-lead Constraint forms in CLI: @status:spec/done

- @fact:CF-LATEST `flow:wal` → latest stable. @status:spec/done
- @fact:CF-EXACT `flow:wal@0.3.0` → exact. @status:spec/done
- @fact:CF-RANGE `flow:wal@^0.3` → semver range. @status:spec/done

---

## 7. Registry model (M0 vs M1) {#registry}

- @fact:REG-M0 **Decision:** **M0:** local-directory registry only. No git. Registry is a path on disk with the layout from `VIBEVM-SPEC.md` §8.2. @status:spec/done
- @fact:REG-M1 **M1:** git registry added per `VIBEVM-SPEC.md` §8. Configured in `vibe.toml`'s `[[registry]]` array. Default public registry URL = `https://github.com/vibespecs` (HTTPS org root; per-package URLs are derived at fetch time via [`NamingConvention`](../../crates/vibe-core/src/manifest/project.rs)). @status:spec/done
- @fact:REG-BACKEND-PIN **Backend choice, trait design, cache layout, and Windows UX for M1** are pinned in [spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-001](../modules/vibe-registry/PROP-001-git-backend.md) — in brief: shell-out to the system `git` (not `libgit2`), behind a `GitBackend` trait that leaves the door open for a future `libgit2` swap. @status:spec/done

- @fact:INIT-DEFAULT-REGISTRY **Default in new projects.** `vibe init` writes the default registry URL (`DEFAULT_REGISTRY_URL` in [`vibe_core::manifest`](../../crates/vibe-core/src/manifest/project.rs)) into every new `vibe.toml`'s `[[registry]]` entry unless the operator passes `--no-registry` or overrides with `--registry-url <URL>` / `--registry-ref <REF>`. @status:spec/done
- @fact:DEFAULT-RATIONALE The default exists so that a plain `vibe init` → `vibe install flow:wal` flow works out of the box against the public registry; overrides are there for forks, staging registries, and offline / air-gapped setups. @status:spec/done
- @fact:URL-SSOT The single source of truth for the URL is the constant in `vibe-core` — manual-tests, smoke scripts, and docs all reference it from there. @status:spec/done

@fact:SPLIT-HOST-POSTURE **Source repositories — split-host posture.** The vibevm project and the package registry live on **separate hosts** by deliberate decision (2026-04-29). Each host is chosen on its own merits: @status:spec/done

- @fact:HOST-SOURCE-GITVERSE **vibevm tool source: multi-homed.** GitVerse `git@gitverse.ru:vibevm/vibevm.git` (web `https://gitverse.ru/vibevm/vibevm`) and GitHub `git@github.com:vibevm/vibevm.git` (web `https://github.com/vibevm/vibevm`), both public and canonical for reading. **No host is primary** — mainline is the maintainer's local `main`, every host is a downstream read-replica, and rollout is the fast-forward-only fan-out `cargo xtask mirror` ([PROP-016](PROP-016-source-mirrors.md), 2026-06-14, which supersedes the single-source-of-truth reading recorded here before it). @status:impl/done
- @fact:HOST-REGISTRY-GITHUB **Package registry: GitHub, organization `vibespecs`.** `https://github.com/vibespecs` (org root) — per-package repos are `https://github.com/vibespecs/<group>_<name>` per [PROP-008 §2.5](../modules/vibe-registry/PROP-008-qualified-naming.md#repo-naming) `NamingConvention::Fqdn`, the default since M1.19 (e.g. `org.vibevm_wal`). The earlier `<kind>-<name>` repos (`NamingConvention::KindName`, [PROP-002](../modules/vibe-registry/PROP-002-decentralized-registry.md#registry-model)) are archived read-only. @status:impl/done
  - @fact:REG-MIGRATION-WHY The migration from `git@gitverse.ru:vibespecs/*` happened on 2026-04-29 because GitVerse's public REST API does not expose org-scoped repo creation (`POST /orgs/{org}/repos` returns 404 / WAF 403; documented exhaustively in [PROP-002 §2.10](../modules/vibe-registry/PROP-002-decentralized-registry.md#publish) and `crates/vibe-publish/src/gitverse.rs`). Without that endpoint `vibe registry publish` cannot fully drive the publish loop end to end. @status:spec/done
  - @fact:REG-GITHUB-WORKS GitHub's equivalent endpoint works natively, so the registry organization moved while the vibevm project repository stays put. @status:spec/done
  - @fact:REG-HASH-STABLE Identity is content-hashed (PROP-002 §2.1) — the lockfile's `source_url` rotates but no `content_hash` value is invalidated by the host change. @status:spec/done
- @fact:HOST-LEGACY-READONLY **Legacy registry, read-only.** `git@gitverse.ru:anarchic/vibespecs.git` (HEAD `2203239`, 2026-04-23, three v0.1.0 flows in monorepo form). Kept readable for any project still on schema-v1 lockfiles until they migrate; no new publishes happen there. @status:spec/done
- @fact:split-host-decision **Decision (owner, 2026-08-01 — the current posture, refining the equal-canonical reading above):** **GitHub carries the leading role** for both surfaces — the canonical public home of the vibevm source and of the `vibespecs` registry. **GitVerse is supplementary**: (1) a full mirror of the vibevm source — the fan-out mechanics of PROP-016 are unchanged, mainline stays the maintainer's local `main`, every host a fast-forward-only read-replica; (2) additional storage for the registry, **used in full but published to deliberately, package by package — never blanket-mirrored**. @status:spec/done
- @fact:split-host-why **Why:** two grounds, one technical and one observational. The technical one is recorded above: GitVerse's public REST API exposes no org-scoped repo creation (`##REG-MIGRATION-WHY`), so the publish loop cannot run end-to-end there while that stands. The observational one is the owner's, 2026-08-01: which host the audience actually reads is visible from the owner's seat, not from this tree, and the observed answer is GitHub. @status:spec/done
- @fact:split-host-rejected **Considered and rejected:** **the equal-canonical dual-host posture** (the reading this section carried until 2026-08-01) — superseded: kept as mirror mechanics, dropped as a role statement; **GitVerse-led** — impossible while `##REG-MIGRATION-WHY`'s endpoint gap stands; **blanket-mirroring the registry to GitVerse** — rejected in the ruling's own words («пакеты туда должны выкладываться специально, а не зеркалироваться всё подряд»): the supplementary store is curated by deliberate publication, never synchronised wholesale. @status:spec/done
- @fact:split-host-revisit **Revisit when:** the halves re-open separately. The publish-loop half: GitVerse's `POST /orgs/{org}/repos` starts returning 2xx — observation point: one request against gitverse.ru; the probe and its failure codes are documented in [PROP-002 §2.10](../modules/vibe-registry/PROP-002-decentralized-registry.md#publish). The leading-role half: by the owner's notice from external observation — deliberately no code-observable trigger, per the same-day B-015 precedent (`BACKLOG.md`). @status:spec/done

- @fact:CACHE-REGISTRIES **Cache location:** `~/.vibe/registries/<hash>/` for cloned registries. @status:spec/done
- @fact:CACHE-PACKAGES `<project>/.vibe/cache/<kind>/<name>/<version>/` for per-package cache. See `VIBEVM-SPEC.md` §8.3. @status:spec/done

---

## 8. Task graph model {#graph}

- @fact:GRAPH-BUILTIN-NODES **Decision:** Built-in nodes only in v1 (content-only plugin contribution model per `VIBEVM-SPEC.md` §5.4). @status:spec/done
- @fact:RUNNER-SEQUENTIAL Runner is sequential (no parallelism) in v1 per §5.2. @status:spec/done
- @fact:TYPED-VALUES Typed value system per §5.3. @status:spec/done
- @fact:graph-nodes-why **Why:** the frozen `VIBEVM-SPEC.md` §5.4 states the constraint and its reason: v1's contribution model is *content-only* — *"a package materialises as a verbatim `vibedeps/` subtree and contributes a boot snippet, but does not contribute executable nodes. This keeps v1 small."* @status:spec/done
- @fact:graph-nodes-rejected **Considered and rejected:** **packages contributing executable / LLM nodes** (e.g. a flow adding a `wal:checkpoint` node bound after `build:compile`) — **deferred, not rejected**: `VIBEVM-SPEC.md` §5.4 targets v1.5 and directs *"document the extension point but do not implement it in v1."* Plugins influence the graph in v1 only by changing what content the built-in nodes operate on. @status:spec/done
- @fact:graph-nodes-revisit **Revisit when:** the v1.5 milestone opens — `VIBEVM-SPEC.md` §5.4 names it as the target for the extension point (observation point: the milestone list in `spec/WAL.md`) — or earlier, when a published package needs a graph node the built-in set does not provide, observed as a `[hooks].post-install` doing work a node should do or a `requires` no built-in node can satisfy (adopted with both clauses, owner 2026-08-01). @status:spec/done

@fact:WORKFLOWS-QUERIES Workflows are graph queries (target node + transitive dependencies) per §5.5. @status:spec/done

---

## 9. Conflict resolution {#conflicts}

@fact:CONFLICT-ORDER vibevm's writer-conflict resolution — the **Human > Spec > Tests > Code** order (also pinned in [`VIBEVM-SPEC.md`](../../VIBEVM-SPEC.md) §2.2 and book chapter 1) — is the `conflict-protocol` flow: `spec://org.vibevm.world/conflict-protocol/flows/conflict-protocol/CONFLICT-PROTOCOL#root`. @status:spec/done

---

## 10. Observability {#observability}

- @fact:OBS-TRACING **Decision:** Use `tracing` for structured logs. @status:spec/done
- @fact:OBS-OUTPUT-MODES CLI defaults to human-readable Markdown-flavored output; `--json` for machine-readable; `--quiet` for one-line summaries. @status:spec/done
- @fact:OBS-EXIT-CODES Exit codes per `VIBEVM-SPEC.md` §9.4. @status:spec/done

---

## 11. Cross-platform target {#platforms}

- @fact:PLATFORMS-TRIO **Decision:** M0 builds and runs on macOS, Linux, and Windows. @status:spec/done
- @fact:PATH-HANDLING Path handling goes through `std::path::Path` — no manual separator manipulation. @status:spec/done
- @fact:CASING-RULES File operations respect platform casing rules where the OS enforces them. @status:spec/done

- @fact:DEV-ON-WINDOWS **Test matrix:** M0 dev is primarily on Windows 11 (this machine). @status:spec/done
- @fact:CI-MATRIX-M2 CI matrix for all three OSes lands in M2 per `VIBEVM-SPEC.md` §11.4. @status:spec/done

---

## 12. Commit and push discipline {#commits}

@fact:GIT-PRACTICES-FAMILY The repository's commit-and-push discipline is the **git-practices** family (a host dependency), whose members carry the full text: @status:spec/done

- @fact:GP-ATTRIBUTION human-authored **attribution** — `spec://org.vibevm.world/git-attribution-policy/flows/attribution-policy/ATTRIBUTION-POLICY#root`; @status:spec/done
- @fact:GP-CONVENTIONAL the **Conventional Commits** message format — `spec://org.vibevm.world/git-conventional-commits/flows/conventional-commits/conventional-commits#root`; @status:spec/done
- @fact:GP-ATOMICITY **atomicity**, one commit = one logical idea — `spec://org.vibevm.world/git-atomic-commits/flows/atomic-commits/ATOMIC-COMMITS-PROTOCOL#root`; @status:spec/done
- @fact:GP-AUTONOMY commit **autonomy** — routine proceeds, and the red lines (history rewrites, force-push, large blobs, CI / signing / secrets) stop and ask — `spec://org.vibevm.world/git-autonomy/flows/autonomy/AUTONOMY-PROTOCOL#root`. @status:spec/done

@fact:ATTRIBUTION-ENFORCEMENT-EXCEPTION **Marked exception (owner ruling, 2026-08-01):** the attribution posture is enforced **procedurally, not mechanically** — the rules live in the boot contract and every session holds them; no commit hook, trailer scanner, or CI check exists or is planned, consistent with the standing no-CI decision. Recorded so the absence of machinery reads as a deliberate choice, not an oversight. @status:spec/done

@fact:ATTRIBUTION-BOOT-SURFACE-EXCEPTION **Marked exception (owner ruling, 2026-08-02):** the installed flow's single-place law (`spec://org.vibevm.world/git-attribution-policy/flows/attribution-policy/ATTRIBUTION-POLICY#root`, its boot snippet's `#SCOPE-THE-ONLY-PLACES-THE-TOPIC-IS-DISCUSSED`) is deliberately not kept literally by this host's **boot surfaces**: `spec/boot/00-core.md` (Rule 1's summary), the `CLAUDE.md` / `AGENTS.md` / `GEMINI.md` triple (byte-identical by contract, gated by self-check step 0c), and the agent instruction files under `.claude/agents/` each carry a short digest of the four rules **by design** — «правила обязаны доезжать до каждого агента на старте»: a session reads its boot files and does not resolve links at boot, so boot reliability wins over single-statement purity. The legal restatement set is exactly: the boot surfaces above; this §12, the host's authoritative record; and the invariant roster's one-line echo (`##INV-HUMAN-AUTHORSHIP`), which names its source in the same sentence. Everything else cites the flow or this section; a new restatement outside a boot surface is a defect. @status:spec/done

---

## 13. Package layout convention {#package-layout}

- @fact:MIRROR-LAYOUT **Decision:** vibevm packages use a **mirror layout**. Every entry in a package's `writes.files` is simultaneously (a) the path of the file inside the package directory and (b) the path at which it will be installed in the consumer's project. @status:spec/done
- @fact:NO-TARGET-FIELD There is no separate `target = "…"` field per entry; `writes.files` is the single source of truth for "where does this file go?" @status:spec/done

@fact:mirror-example Concretely, the canonical `flow:wal@0.1.0` payload (vendored as a hermetic e2e test fixture under `fixtures/registry/flow/wal/v0.1.0/`) contains `spec/flows/wal/WAL-PROTOCOL.md` at exactly that relative path; after `vibe install flow:wal`, the file lives at `spec/flows/wal/WAL-PROTOCOL.md` inside the user's project. No mapping, no rewriting. @status:spec/done

@fact:BOOT-SNIPPET-EXCEPTION **Boot snippets are the one exception.** The `[boot_snippet]` table carries an explicit `source` field naming the path inside the package (conventionally under `boot/`), while the target is always the fixed `spec/boot/<filename>`. @status:spec/done

- @fact:MIRROR-WHY-DRIFT **Why:** a single source of truth for source-and-target paths eliminates a whole class of authoring bug where the package layout drifts from the declared writes. @status:spec/done
- @fact:MIRROR-WHY-READABLE It also makes a package directory instantly readable — a human looking at the tree knows exactly what will appear in a consumer's project without cross-referencing a separate mapping table. @status:spec/done
- @fact:mirror-rejected **Considered and rejected:** a **per-entry `target = "…"` field** in `writes.files` — rejected because `writes.files` would stop being the single source of truth for *"where does this file go?"* (`##NO-TARGET-FIELD`), reviving the authoring-drift bug `##MIRROR-WHY-DRIFT` names and costing the package directory its at-a-glance readability (`##MIRROR-WHY-READABLE`). `[boot_snippet].source` is the **one retained exception**, not a rejection: its target is the fixed `spec/boot/<filename>` (`##BOOT-SNIPPET-EXCEPTION`). @status:spec/done
- @fact:mirror-revisit **Revisit when:** a **second** source/target exception is proposed — i.e. any manifest table beyond `[boot_snippet]` needing an install path that differs from its in-package path. Observation point: the manifest schema in [`crates/vibe-core`](../../crates/vibe-core/) — a per-entry target field appearing there *is* the fired state. One exception stands today; a second means the mirror rule is carrying less than `##MIRROR-LAYOUT` claims. @status:spec/done

- @fact:MIRROR-PINNED **Where pinned:** `VIBEVM-SPEC.md` §13.1 shows the mirror-layout diagram and §13.2 the matching manifest. This PROP-000 entry is the decision record; the spec carries the operational definition. @status:spec/done
- @fact:INSTALL-RELIES `vibe-install` relies on this convention — the source path of a planned write is computed by joining `cache_dir` with the manifest's declared target path. @status:spec/done

---

## 14. Manual-test protocol {#manual-tests}

- @fact:MT-LOCATION **Decision:** human-runnable smoke-tests live in [`manual-tests/`](../../manual-tests/) at the repo root, one Markdown file per scenario, named `<milestone>-<slug>.md` (e.g. `M1.1-git-registry-smoke.md`); the directory's own [`README.md`](../../manual-tests/README.md) carries the index. @status:spec/done
- @fact:MT-FLOW-POINTER The tier itself — why a project keeps a second, human-run test layer alongside the automated suite, what a manual test is and is not, when to run one, and who signs it off (a human, over an agent's pre-run) — is the `manual-tests` flow: `spec://org.vibevm.world/manual-tests/flows/manual-tests/MANUAL-TESTS-PROTOCOL#root`, with the four authoring rules in its `authoring-rules` document and the copy-ready skeleton in `test-template`. @status:spec/done

- @fact:MT-HERMETIC-SUITE **vibevm's real-world surfaces.** `cargo test --workspace` uses fakes, tempdirs, and local bare repositories for speed and hermeticity (the flow's `spec://org.vibevm.world/manual-tests/flows/manual-tests/MANUAL-TESTS-PROTOCOL#why-second-tier`). @status:spec/done
- @fact:MT-LAST-MILE The manual tier is the last mile for what only the real world has here: SSH auth against GitVerse, the lockfile `source_uri` exactly as a downstream consumer receives it, the `~/.vibe/` layout on a user's actual filesystem, and a human confirming the CLI output says what they meant. @status:spec/done

- @fact:MT-ISOLATION **vibevm's bindings.** Every test isolates state with `mktemp -d` for the project and `VIBE_REGISTRY_CACHE` pointing inside the scratch dir for the registry cache — the user's real `~/.vibe/` is never touched by a run. @status:spec/done
- @fact:MT-GIT-BASH Git Bash on Windows is the primary smoke-test environment (macOS and Linux must work too); where platform output differs (path separators, `.exe` suffix), the Windows form comes first with a portable note. @status:spec/done

@fact:mt-when-lead **When to run** (the flow's `spec://org.vibevm.world/manual-tests/flows/manual-tests/MANUAL-TESTS-PROTOCOL#when`, against vibevm's surfaces): @status:spec/done

- @fact:MT-WHEN-TAGGING before tagging any milestone; @status:spec/done
- @fact:MT-WHEN-CHANGES after changes to the git backend, CLI arg parsing, or lockfile format even when `cargo test` stays green; @status:spec/done
- @fact:MT-WHEN-REPRO and as reproducers whenever a user files an integration bug. @status:spec/done

@fact:MT-WAL-NAMES [`spec/WAL.md`](../WAL.md) names the outstanding manual runs for the current milestone — a practice that must not lapse: MT-02 and MT-03 have been awaiting owner sign-off since the TUI work landed, and a WAL that names none reads as "nothing pending". @status:spec/done

---

## 15. Dependency weight is not a decision factor {#dep-weight}

- @fact:DEP-WEIGHT-NOT-FACTOR **Decision:** Binary size, crate count, transitive dep weight are NOT decision factors when selecting third-party libraries. @status:spec/done
- @fact:PICK-STRONGEST Pick the strongest available library for the job — for both the Rust CLI and any future Java / frontend side. @status:spec/done

- @fact:WHY-PRECEDENT **Why:** Software of comparable surface area (Chrome, modern IDEs, production package managers) routinely ships tens to hundreds of dependencies and remains fast and capable. @status:spec/done
- @fact:WHY-DEBT Under-specifying a load-bearing component to save megabytes creates ongoing architectural debt that is much more expensive to repay than the weight it saves. vibevm intends to be best-in-class, and best-in-class means using best-in-class primitives. @status:spec/done

@fact:reject-reasons-lead **Legitimate reasons to reject a dep:** @status:spec/done

- @fact:REJECT-LICENSE non-permissive license (see §3 — MIT / Apache-2.0 / BSD / Unlicense only; GPL / AGPL / LGPL forbidden; MPL-2.0 allowed case by case, since its weak copyleft does not taint consumers), @status:spec/done
- @fact:REJECT-ABANDONED abandoned upstream, @status:spec/done
- @fact:REJECT-SECURITY demonstrated security issues (CVE history, unpatched known exploit), @status:spec/done
- @fact:REJECT-ERGONOMICS fundamentally bad API ergonomics that would propagate into our own interfaces. @status:spec/done

@fact:TOO-HEAVY-NOT-REASON "Too heavy" alone is **not** a reason. @status:spec/done

- @fact:READMISSIBLE **Concrete consequences:** libraries previously rejected on footprint grounds are re-admissible. Notable: `libsolv` (C, with Rust bindings), `git2` (wrapping `libgit2`), bundled native C deps, embedded interpreters when justified. @status:spec/done
- @fact:PROP-001-PRUNE The size-based argument in [PROP-001 §2.1](../modules/vibe-registry/PROP-001-git-backend.md#backend) against `git2` is to be pruned — the remaining arguments (Windows SSH auth, shell-out diagnostic clarity) may still carry that decision, but not the size one. @status:spec/done
- @fact:dep-weight-rejected **Considered and rejected:** the **predecessor policy — reject a dependency on footprint** (binary size, crate count, transitive weight) — rejected, and its consequences already executed in this section: libraries previously refused on footprint grounds are re-admissible, `libsolv` and `git2` named (`##READMISSIBLE`), and PROP-001 §2.1's size-based argument against `git2` is marked for pruning (`##PROP-001-PRUNE`). Four grounds survive and are the *only* ones — licence, abandonment, demonstrated security issues, API ergonomics (`##REJECT-LICENSE` … `##REJECT-ERGONOMICS`); *"too heavy" alone is not a reason* (`##TOO-HEAVY-NOT-REASON`). @status:spec/done
- @fact:dep-weight-revisit **Revisit when:** the premise of `##WHY-PRECEDENT` — that weight does not cost us — stops holding: a dependency is admitted whose weight measurably degrades a user-visible surface (install time, first-run latency, release binary size), recorded as a finding. Observation points exist today — the release artefact's size and `cargo build --timings`; numeric thresholds were offered and not set (owner, 2026-08-01), so the trigger is event-shaped until numbers exist. @status:spec/done

---

## 16. JTD + codegen for wire contracts {#jtd}

- @fact:JTD-SSOT **Decision:** JSON Type Definition (RFC 8927) schemas are the single source of truth for every client/server and machine-to-machine contract in this project. @status:spec/done
- @fact:JTD-CODEGEN Rust types — and types in any future non-Rust clients — are **generated** from JTD schemas via `jtd-codegen`, not hand-maintained. @status:spec/done
- @fact:NO-DUPLICATION No client/server duplication is permitted on contracts. @status:spec/done

- @fact:JTD-WHY-SKEW **Why:** duplication between a server contract and a hand-written client is a classic source of version-skew bugs; schema-first codegen eliminates that class of bug categorically. @status:spec/done
- @fact:JTD-OVER-JSONSCHEMA JTD specifically (over JSON Schema / OpenAPI alone) because JTD is deliberately narrower: its schema grammar is constructed so every JTD schema maps to a clean static type in every target language, with no language-specific escape hatches. @status:spec/done
- @fact:jtd-rejected **Considered and rejected:** **JSON Schema / OpenAPI alone** — rejected: JTD is *"deliberately narrower: its schema grammar is constructed so every JTD schema maps to a clean static type in every target language, with no language-specific escape hatches"* (`##JTD-OVER-JSONSCHEMA`); **a hand-written client against each server contract** — rejected: that duplication is *"a classic source of version-skew bugs"* which codegen eliminates categorically (`##JTD-WHY-SKEW`, `##NO-DUPLICATION`). The boundary in the other direction is not a rejection: human-edited manifests stay TOML via `serde` — *"JTD is for wire, not for configs humans hand-edit"* (`##JTD-OUT-OF-SCOPE`). @status:spec/done
- @fact:jtd-revisit **Revisit when:** either upstream fails us — **`jtd-codegen` ships no release for 24 months** (observation point: its upstream repository, version-pinned in `tools/jtd-codegen/` per `##TC-BINARY`) — **or** a contract listed in `##JTD-IN-SCOPE` proves inexpressible in JTD's grammar and would need an escape hatch, which is the property `##JTD-OVER-JSONSCHEMA` bought the narrowness for. Observation point for the second: the first schema in `schemas/` that cannot be written. @status:spec/done

@fact:JTD-IN-SCOPE **In scope:** LLM provider API wrappers (Anthropic, OpenAI, OpenRouter, Ollama), GitVerse public-API client, `vibe --json` CLI output, telemetry / event log formats, future hosted-registry HTTP surface. @status:spec/done

@fact:JTD-OUT-OF-SCOPE **Out of scope:** human-authored manifests — `vibe.toml`, `vibe.lock`, `vibe-package.toml` — stay TOML via `serde`. JTD is for wire, not for configs humans hand-edit. @status:spec/done

@fact:toolchain-lead **Toolchain placement:** @status:spec/done

- @fact:TC-BINARY `jtd-codegen` binary in project-local `tools/jtd-codegen/` (gitignored; version pinned). @status:spec/done
- @fact:TC-SCHEMAS Schemas in `schemas/` at repo root, one `.jtd.json` file per contract, committed. @status:spec/done
- @fact:TC-GENERATED Generated Rust code in `crates/vibe-wire/src/generated/`, committed, with a `// DO NOT EDIT — regenerate via cargo xtask codegen` header on every file. @status:spec/done
- @fact:TC-REGEN Regeneration via `cargo xtask codegen`. CI enforces zero drift (`cargo xtask codegen && git diff --exit-code`). @status:spec/done

- @fact:TC-AGENT-SETS-UP **Toolchain install ownership:** the coding agent sets up the codegen toolchain itself. @status:spec/done
- @fact:TC-RUNAS Machine-global changes (PATH mutation, admin-level installs, env-var additions) go through `runas` with an operator confirmation at the moment of the change. @status:spec/done

---

## 17. Production architecture in the prototype phase {#prod-arch}

- @fact:PROD-QUALITY-DAY-ONE **Decision:** Load-bearing surfaces — lockfile schema, registry protocol, dep-resolver semantics, wire formats, identity model — are designed to production quality from day one. @status:spec/done
- @fact:FORMATS-BIND The project is a prototype today; the formats and protocols it chooses today are the ones its future users will be bound to. Changing them later is orders of magnitude more expensive than designing them correctly now. @status:spec/done

- @fact:PRINCIPAL-LENS **Lens:** "a principal engineer at a top-tier infrastructure company, designing a format or protocol that will be used by millions" is one of the reflection lenses to reach for when a design decision lands. @status:spec/done
- @fact:LENS-NOT-ONLY It is **not** the only lens — "the simplest thing that works" remains valid for leaf features — but architecture-heavy surfaces prefer the principal-engineer lens. @status:spec/done

@fact:prod-consequences-lead **Consequences:** @status:spec/done

- @fact:PC-PREFER-DESIGNED Prefer a recent-but-well-designed library over a tactical shortcut, even when the shortcut is cheaper in the short term. @status:spec/done
- @fact:PC-EXTENSION-POINTS Extension points, versioning markers, and forward-compatibility hooks land with the initial cut, not in a later "hardening" pass. @status:spec/done
- @fact:PC-REVERSIBILITY Reversibility matters: if a format or protocol decision is hard to reverse (lockfile schema, registry URL scheme, identity hash), lean heavier into design rigour before first commit. @status:spec/done
- @fact:PC-FIX-LATER-SCOPE "We'll fix it later" is a valid stance only for implementation quality inside a well-chosen architectural surface — not for the surface itself. @status:spec/done

---

## 18. Complexity expectation: higher than RPM {#complexity}

- @fact:RPM-CLASS-TARGET **Decision:** The dependency / package model is designed to handle complexity **at least** matching RPM-class systems (zypper, DNF), and in several dimensions greater. @status:spec/done
- @fact:RICH-DEPS-DAY-ONE Manifest grammar and lockfile schema reserve fields for — and the resolver implements — capabilities, provides / requires / obsoletes / conflicts / supplements / recommends, disjunctions (`A or B`), boolean rich-dep syntax, capability-based resolve, and multi-kind cross-deps. **Semantic (LLM-reviewed) conflicts are the one exception:** the heuristic static check ships ([PROP-003](../modules/vibe-resolver/PROP-003-dep-evolution.md) `CHECK-ACTIVATION-CONFLICT`) while the LLM lane waits on the unbuilt `vibe-llm` emission engine. All of it is designed in from day one, not deferred. @status:impl/done

- @fact:WIDER-THAN-RPM **Why:** vibevm's dependency surface is not simpler than RPM — it is wider. A `feat` package may require a `stack` providing a specific capability, `flow`s may declare semantic compatibility with other `flow`s, LLM-backed review adds a non-mechanical conflict dimension RPM never had. @status:spec/done
- @fact:UNDERSHOOT-COST Undershoot — picking a resolver that lacks virtual packages or disjunctions, or a manifest that cannot express capability-based requires — would force an incompatible schema migration after users exist. @status:spec/done

- @fact:RESOLVER-RESOLVO **Resolver choice** (pinned in the module PROP): `resolvo` crate as the primary depsolver. @status:spec/done
- @fact:RESOLVER-LIBSOLV-FALLBACK `libsolv` as an explicit FFI-backed fallback behind a `DepSolver` trait (analogous to [PROP-001 §2.2](../modules/vibe-registry/PROP-001-git-backend.md#backend-trait)'s `GitBackend` pattern). @status:spec/done
- @fact:RESOLVER-PUBGRUB-REJECTED PubGrub is rejected for the *primary* role — its algorithm does not handle virtual packages or disjunctions — but is acceptable for explanatory rendering of conflicts in CLI output if it proves superior there. @status:spec/done

---

## 19. Load-bearing setup documentation {#setup-docs}

@fact:SETUP-DOCS **Decision:** Two files at the repo root are load-bearing for the project: @status:spec/done

- @fact:DOC-DEV-GUIDE [`DEV-GUIDE.md`](../../DEV-GUIDE.md) — contributor-facing: everything to install on a fresh machine to clone, build, test, contribute to, and (if authorized) publish from this repository. @status:spec/done
- @fact:DOC-RUNTIME-GUIDE [`RUNTIME-GUIDE.md`](../../RUNTIME-GUIDE.md) — user-facing: everything to install and env-configure to run the shipped `vibe` CLI. @status:spec/done

@fact:SETUP-DOCS-FLOW vibevm's setup docs are [`DEV-GUIDE.md`](../../DEV-GUIDE.md) (contributor / build) and [`RUNTIME-GUIDE.md`](../../RUNTIME-GUIDE.md) (runtime / user). The same-commit obligation that binds them is the `dev-runtime-docs` flow: `spec://org.vibevm.world/dev-runtime-docs/flows/dev-runtime-docs/DEV-RUNTIME-DOCS-PROTOCOL#obligation`. @status:spec/done

---

## 20. Token secrecy and adapter scope {#token-secrecy}

@fact:req-token-secrecy `req r1` @status:spec/done

- @fact:TOKEN-SURFACE-SECRET **Decision.** Publish tokens, registry-API tokens, and any LLM-provider keys handled by vibevm are **surface secrets** in the sense of the `secrets-hygiene` flow (`spec://org.vibevm.world/secrets-hygiene/flows/secrets-hygiene/SECRETS-HYGIENE-PROTOCOL#surface-secret`): their **value** MUST NOT appear on any surface vibevm produces, though their **source** (env-var name, file path) may be printed. @status:spec/done
- @fact:token-bindings-lead vibevm's bindings of the flow's four laws (`spec://org.vibevm.world/secrets-hygiene/flows/secrets-hygiene/SECRETS-HYGIENE-PROTOCOL#laws`): @status:spec/done

- @fact:TS-NEVER-PRINTED **Never printed.** Not to stdout, stderr, the CLI log, the `--json` event stream, error messages, panic traces, telemetry, or the lockfile. The CLI prints the *source* of a token (explicit / env-var name / file path) but never the value. The in-process wrapper types (`vibe_publish::Token`, future `vibe_llm::ApiKey`) MUST redact on `Display` and `Debug` — verified by unit tests (the flow's `spec://org.vibevm.world/secrets-hygiene/flows/secrets-hygiene/SECRETS-HYGIENE-PROTOCOL#law-tested`). @status:spec/done
- @fact:TS-NEVER-PERSISTED **Never persisted.** Not committed to the repository, not written into the lockfile, not embedded in cache files, not landed in the `.vibe/` tree. The single sanctioned at-rest location is the operator's `~/.vibe/<host>.publish.token` file (per-user, chmod-protected). @status:spec/done
- @fact:TS-BOUNDARIES **Sanctioned process boundaries.** The token may cross a process boundary only via: (a) the host API's `Authorization: Bearer …` header, sent over TLS; (b) a single `git remote add` / `git push` invocation where the token is embedded in the URL as `https://x-access-token:<TOKEN>@host/…` (modern git ≥ 2.31 redacts URL passwords in its own log output to `***`). No other path is allowed — in particular, never into a spawned third-party hook's environment (the flow's `#law-boundaries`). @status:spec/done
- @fact:TS-ADAPTER-SCOPE **Adapter scope.** A `RepoCreator` impl MUST refuse to operate outside the organization specified in the project's `[[registry]].url`. A publish run targeting `github.com/vibespecs` may not create, modify, or even probe a repository under a different `github.com` org or under any user namespace. Adapter implementations carry an explicit org-prefix check and surface a `PublishError` on attempted scope escalation. (The general integration-scope discipline is the flow's `scope-discipline` document.) @status:spec/done

- @fact:BLAST-RADIUS **Why global, not module-local.** The blast radius of a leaked publish token is the entire organization it can reach (cross-repo writes, branch deletes, CI-secret reads); of an escalated adapter, the entire host account (the flow's `spec://org.vibevm.world/secrets-hygiene/flows/secrets-hygiene/SECRETS-HYGIENE-PROTOCOL#blast-radius`). Both are catastrophic beyond what module-local discipline can bound, so the rules are global and every code path touching a `Token` or a `RepoCreator` is audited. @status:spec/done
- @fact:ROTATE-FIRST On any suspected leak, rotate first (the flow's `#leak-drill`). @status:spec/done

@fact:TOKEN-PINNED-WHERE **Where pinned (operationally):** [PROP-002 §2.10](../modules/vibe-registry/PROP-002-decentralized-registry.md#publish) carries the publish-side mechanics; [`spec/boot/90-user.md`](../boot/90-user.md) carries the operator-facing rule for this machine. Both are subordinate to this PROP-000 entry. @status:spec/done

---

## 21. Surface floor — which channels a capability owes {#surfaces}

@fact:SURFACE-DISCIPLINE-IS-THE-OMNICHANNEL-FLOW **Decision:** a capability lives in a **library**; the CLI, the TUI and the MCP server are thin surfaces over it. The rule and its vocabulary are the installed `omnichannel` flow: `spec://org.vibevm.world/omnichannel/flows/omnichannel/OMNICHANNEL-PROTOCOL#root`. This section declares only vibevm's own floor, which is what that flow asks each project to state for itself. @status:spec/done

@fact:VIBEVM-DECLARES-LIBRARY-CLI-MCP **vibevm's declared floor: library + CLI + MCP**, plus **TUI** where one exists (`vibe tree` has one today). A new capability ships with those, or with a recorded reason why one sufficed. @status:spec/plan

@fact:LSP-AND-IDE-ARE-NOT-DECLARED **LSP and IDE extensions are deliberately NOT declared** (owner, 2026-08-06: he will open that work himself). By the flow's own rule an undeclared surface is not a debt, so their absence is a choice and not a gap to be closed. @status:spec/done

@fact:THE-FLOOR-IS-A-TARGET-NOT-A-DESCRIPTION **This is a target, not a description of today.** The census that motivated the decision (`campaigns/packages-2026-09/harvest/g6-b047-surfaces-census.md`) measured the opposite in places: of 29 top-level commands, 19 keep their substance in a separate crate and **10 keep it inside `vibe-cli`** — the largest being the whole `vibe self` version manager. Of 5 MCP tools, 2 share a library function with their CLI twin, 2 have no CLI twin, and 1 reads the same data as `vibe list` while building its output by hand. @status:spec/done

@fact:THE-DIVERGENCE-HAS-ALREADY-BEEN-PAID-FOR **The gap is not theoretical.** `vibe list --json` and the MCP `query_package` printed different values for one field on Windows until 2026-08-06, because each rendered the path itself — two surfaces of one capability answering one question differently, which is exactly the failure this floor exists to prevent. @status:spec/done

---

## Invariants {#invariants}

@fact:INVARIANTS-STOP-RULE (These restate the most load-bearing rules from the spec and the book. If anything below seems violated in practice, stop and reconcile before proceeding.) @status:spec/done

1. @fact:INV-VOCABULARY **Vocabulary lock.** Never use Maven's "lifecycle/phase/goal" or Bazel's internal terminology in user-facing or internal code. The installable kinds are `flow`, `feat`, `stack`, `tool`, `mcp`, `lang` — the register grows only by owner amendment to `VIBEVM-SPEC.md` §4.1 (`app` is anticipated). The canonical process discipline vocabulary is the one in `VIBEVM-SPEC.md` §4 and the book. @status:spec/done
2. @fact:INV-SPEC-DIR **`spec/` is fixed.** The directory name and role cannot be configured away in v1. @status:spec/done
3. @fact:INV-USER-FILES **User-owned files are never written by `vibe`.** `spec/boot/00-core.md` and `spec/boot/90-user.md` are off-limits to install/uninstall/update. @status:spec/done
4. @fact:INV-ATOMIC-COMMITS **One commit = one logical unit.** Commit messages follow the git-practices family (§12) and reference `spec://…` URIs where relevant. @status:spec/done
5. @fact:INV-DOGFOOD **Dogfood.** vibevm is being built using the same discipline it enforces. The `spec/` tree in this repo IS `vibe init`'s reference output. @status:spec/done
6. @fact:INV-HUMAN-AUTHORSHIP **Human authorship is the only attribution.** The posture is the `attribution-policy` flow (a git-practices member, §12); everywhere else assume human authorship only. @status:spec/done
7. @fact:INV-TOKEN-SECRECY **Tokens never appear in vibevm output.** See §20. Audited in unit tests; any new code path touching a `Token` or `RepoCreator` is reviewed for redaction and scope-escalation safety. @status:spec/done
