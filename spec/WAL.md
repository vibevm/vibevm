# WAL — Project Continuation State
_Updated: 2026-06-13 — **CONVERT-PLAN v0.1 IS IN PROGRESS** (owner goal: «выполнить CONVERT-PLAN-v0.1 до конца, фаза 7 тоже разгейчена» — execute the full-depth conversion plan, Phase 7 owner-un-gated). **Phase 0 complete** (8 commits `173bb15`…`616e9db`): PROP-014 self-marks its six units (specmap warnings 6→0), stub-status headers on vibe-graph/vibe-llm, ServerLock promoted to a crate-level `src/lock.rs` seam, `vibe search` reads its env at the composition root, short-name resolution drops its len-checked expects, and conform-frontend-rust + env-audit join the gate → **CONFORM_GATED 12** with a checked `CONFORM_EXEMPT` reasons table. **Phase 1 (vibe-core armor) partway**: **1.1 COMPLETE — all seven newtypes** (`RelPath`, `PackageName` [the 25-file compiler-led cascade, `7d4f041`], `CapabilityNamespace`, `CapabilityName`, `ContentHash` [`4c46eae`], `SourceUrl`, `TraceId` [`f99976e`]); all serde-transparent + `Deref<str>`-ergonomic — Deref collapses the read-site cascades so each lands as a handful of construction edits, with `from_validated`/`new` at trusted-reconstruction seams (the lockfile builders) and `parse` validating only at untrusted boundaries. **1.3 done** — nine vibe-core seam doctests (`0eac0cc`, `17f7344`: Group, VersionSpec, PackageRef, CapabilityRef, Manifest, Lockfile, UserConfig + PackageName, RelPath). 1.2's invariant witnesses are largely carried by the validating constructors plus those doctests. **Phase 1 COMPLETE** — 1.4 landed the `pub-doctest` Class-G rule (`647ce68`; it froze vibe-core's 55-entry type doc-debt shrink-only, the plan's single-digit-freeze prediction FALSIFIED — vibe-core carries 55 public types beyond the 9 primary seams 1.3 documented; the rule reads the `is_pub`/`has_doctest` facts the frontend has emitted since v2, no frontend bump), and 1.5 added Lockfile's explicit lockfile-schema edge (`00cf8c1`; the other foundation edges were already satisfied by `scope!` inheritance, so §10 left them). **Phase 2 COMPLETE** — vibe-publish's three RepoCreator adapters became `#[cell]`s with a seam-driving oracle and the R-001 construction moved to the registry module (`3bd4cfc`, cells 20→23); the token-redaction tests gained `#[verifies]` (`14ce2b0`); Publisher::publish got its error contract (`54446d0`); BootBand pins the effective-boot order (`8e65a1d`). vibe-install (2.4) was already full-depth from v0.2 (InstallSource + PlanObserver doctests + the conditional-fixpoint edge), so §10 left it untouched — no commit needed. **Phase 3 (vibe-index) COMPLETE.** **3.2** the rate-limiter's refill is a pure runnable model `refilled_tokens(...)` with a doctest, `Bucket::refill` applies it so the two can't drift (`e6c0ac1`); **3.4** the in-RAM Index teaches its full read/write lifecycle by doctest via a new public `VersionEntry::minimal(kind, group, name, version)` fixture builder (`eb85cbb`, `bc540db`); **3.5** the 21 RFC-7807 ApiError `detail` sites (routes/packages.rs + index_files.rs) gained the Class-F `(violates spec://…#http; fix:…)` tail where a spec unit exists, the clean 404 misses left bare (`4be55c6`; e2e does not assert on detail — no churn); **3.3** `types/entry.rs` (545 lines, 15 structs) split into a typed family — dependency relations / content+delivery / aggregate records by concern, VersionEntry kept in `entry/mod.rs`, tests-out, every child carrying the PROP-005#entry `scope!` (`38d6ae7`; all `crate::types::*` paths preserved by re-export, whole workspace builds untouched; gotcha — a non-`#[test]` helper in a standalone test file under `src/` needs its own `#[cfg(test)]` or its unwraps read as domain code, the idiom conform-frontend-rust documents on its own lib/tests.rs); **3.1** the server's two swappable deps became seams — `TokenStore`/`RateLimiter` traits, `AppState` holds each as `Box<dyn _>`, production variants renamed `FileTokenStore`/`TokenBucketRateLimiter` (the latter's docstring already names the foreseen v2), a new `AppState::with_seams` constructor and a fake oracle (tests/seam_fakes.rs) driving the write surface through injected fakes the e2e suites can't reach (a write authorised with no admin.tokens file on disk, a write refused, a read 429'd through the middleware); no `#[cell]` — one production variant each (§10); both new seam traits earned their Scaffold-G doctest (`73b43ca`). CONTINUE.md refreshed wholesale to the CONVERT-PLAN state (`e4919cf`). **Phase 4 (vibe-cli facade diet) STARTED — both independent small items landed.** 4.3: the manifest-mutation discipline `apply_git_source_flag` owned inline moved to vibe-install's pure `record_git_source(manifest, dep)`; the CLI now only validates the `--git*` flags, builds the typed `GitPackageDep`, and persists (`5a5833c`; 20 cli_pkg_cycle git-source tests unchanged). 4.4: `vibe init`'s three embedded templates became data files under `crates/vibe-cli/templates/` via `include_str!` (`97eae04`; the plan's ~300-line estimate FALSIFIED — ~55 lines across three; `.gitattributes` `eol=lf` keeps include_str! byte-stable, all 11 cli_init tests unchanged). **4.1 COMPLETE** — the search domain moved into a new gated `vibe-registry::search` family in three safe sub-steps, each panel-green: the TTL result cache (`c76537f`; was already gate-clean, froze nothing), the GitHub-org full-scan fallback with FullScanError converted to Class-F against PROP-002#registry-model — the same anchor the git_* registry code cites (`8ad4727`), and query tokenisation split into its own `query` module to stay under the 600-line budget, with doctests on the public query API (`1f2b3cd`). search.rs is now the thin CLI orchestration + render the plan wanted. (Finding: the plan's "scoring half of search.rs" was a misattribution — tokenise/score_manifest lived in search_full_scan.rs, so they moved with it; nothing scoring-related remained in search.rs to extract.) REMAINING: **4.2** the vendor (`commands/registry/vendor.rs` ~474) + redirect-sync (~310) domain → vibe-registry (the second big move; fresh-context work). **Key finding from this session's assessment: vendor is NOT a clean whole-file move like the search files.** `run_vendor`'s vendoring loop (vendor.rs:131-237) interleaves the clone domain (`refresh_package` → `bare_clone_from_clone` per lockfile entry) with `ctx.step`/`ctx.skipped` progressive output, so the extraction needs a **VendorObserver** (the vibe-install `PlanObserver` pattern from v0.2) to keep per-package progress, plus a typed `VendorError` (it is anyhow today) — closer in shape to the vibe-install orchestrator extraction than to 4.1's pure-domain search moves. The pure helpers (`bare_clone_from_clone`, `write_vendor_readme`, `file_url_for_dir`) move cleanly; the loop is the work. redirect-sync is a PARTIAL file split (the tag-sync half of sync.rs → beside `multi_registry_resolver::redirect_follow`). 4.1's template (git mv, intra-crate imports, Class-F, alias call sites, 600-line budget) still applies to the mechanics; **4.5** Class-F on the CLI's remaining thiserror enums, DEFERRED until after 4.2 carries the vendor error enums out; **4.6** the drain-then-flip of vibe-cli into CONFORM_GATED (→ 13) needs 4.1–4.5 done. Then Phases 5–7. Then Phases 4–7 (Phase 7 owner-un-gated): **4** vibe-cli facade diet (HUGE — move ~2.5k LOC search/vendor/sync domain to vibe-registry, then flip vibe-cli into CONFORM_GATED); **5** specmark/specmark-grammar gate flip (audit-verified zero-drain) + the new `ambient-env` rule (frontend EnvRead facts, gated, escape via fn-grain deviates); **6** spec layer truth pass (PROP kind audit + the implements edges the code already earns + PROP-010 [DEFERRED — M2] header — largely independent of 4/5, low code-risk, doable any time); **7** the MCP endgame (owner removed the DBT-0020 gate — spec home `spec/modules/vibe-mcp/PROP-0xx`, vibe-mcp tools.rs → McpTool trait + 3 cells, drain the 2638-line vibe-cli/commands/mcp.rs, baseline → 0, the parked file-length pair resolved). Each phase's commits cite their CONVERT-PLAN item; the git log is the authoritative per-item record. Per-commit detail lives in the git log (each commit cites its CONVERT-PLAN item) and the session task list; the plan is `spec/terraforms/CONVERT-PLAN-v0.1.md`. Five-gate panel green at every commit; full `self-check.sh` (fmt + workspace tests + doctests + clippy -D warnings + `vibe check`) green at the Phase-3-complete boundary (specmap --check clean 442 units / 419 items / 430 edges / 0 suspects / 0 warnings; conform 57 frozen [2 MCP file-length + 55 vibe-core pub-doctest] / 0 new). SHRINK-PLAN v0.2's record is retained below as superseded history. The 2026-06-11 history-rewrite question stays open (AUDIT 2026-06-12-01 rider). `CONTINUE.md` predates CONVERT-PLAN entirely — this WAL header supersedes it until the final session-end checkpoint._

## Prior phase (superseded by CONVERT-PLAN v0.1, in progress — see the Updated summary above and the git log)

**SHRINK-PLAN v0.2 — EXECUTED TO COMPLETION (2026-06-12, same-day execution).**
[`spec/terraforms/SHRINK-PLAN-v0.2.md`](terraforms/SHRINK-PLAN-v0.2.md) carries
the execution record in its header. Per move:

1. **The unsafe-gate posture (AUD-0016 → fixed).** Frontend v5: `UnsafeUse`
   gains `in_test` / `in_deviation` (the v4 `UnwrapUse` machinery applied to
   unsafe; unsafe impl methods extracted at all — they were invisible), the
   ordinal advances over testified uses so neighbour testimony never re-keys
   a fingerprint. Rule v2 honors fn-grain `#[spec(deviates, reason)]` per
   ENGINE-CONFORM §4; test-context unsafe is deliberately NOT exempt.
   **`env-audit`** is the designated audit crate: one process-global
   serialized, restoring `EnvGuard` behind a safe API replaced the three
   hand-rolled guards (output.rs ×2 + post_hook.rs temp_set) whose own
   SAFETY comment admitted a transient-observation race — the mutex closes
   it. The two immovable production boundaries testify in place
   (vibe-cli `promote_user_config_env` — pre-thread startup promotion;
   vibe-index `stop.rs` — `libc::kill` FFI), citing
   `ENGINE-CONFORM-v0.1#rules` per the settled deviates-target policy.
   Baseline **10 → 2** (pure shrink; the residual = the DBT-0020 MCP pair).
2. **`CONFORM_GATED` → vibe-core / vibe-index, then vibe-install — 11
   crates.** The entry queue (4 `error-enum-cites-req`, 21
   `error-message-cites-req`, 15 `no-unwrap-in-domain`; both crates'
   seams were already doctested) was drained BEFORE the gate flipped, so
   the baseline never widened: enum REQ edges landed with per-variant
   refinement (PROP-008#pkgref, #four-installable-kinds,
   PROP-002#capability/#git-source, VIBEVM-SPEC#lockfile-schema/#directory-layout,
   PROP-005#cli/#persistence…); all 15 unwraps fell to restructures —
   0 testimonies (two more latent `VersionReq::parse("={v}")`
   build-metadata panics killed by structural `semver::Comparator`; the
   rate limiter got one poison-recovering lock helper + `total_cmp`;
   metrics went `format!`-infallible; headers `HeaderValue::from_static`).
   Zero test expectations moved; one live error path eyeballed.
3. **The `vibe-install` orchestrator crate** (the audit's sketch, named in
   docs/architecture since M0, folded away by M1.18, now rebuilt): the CLI
   pipeline split at its natural joint — `plan()` (root derivation +
   case-c migration, PROP-011 freshness fast path, solve with held-pin
   fallback, fetch + feature pinning, the PROP-003 §2.6.1 conditional
   fixpoint) and `apply()` (manifest merge, materialisation, wholesale
   lockfile rebuild) — with the caller's confirmation between them.
   Cells arrive via the `InstallSource` seam (R-001 construction stays in
   vibe-cli's registry module); progress crosses as typed `PlanEvent`s;
   `PROP-003#req-conditional-fixpoint` carries its first implements edge
   (PHASE1-PILOT's honest zero, filled). os-740 answered structurally:
   `[lib] test = false`, integration tests under a safely named binary,
   doctest runner verified green. The CLI's install command is now a thin
   layer (mod/report/resolver); update/reinstall consume the seam trait;
   the exit-code mapper sees through the orchestrator's transparent
   envelope (MalformedRedirectBlock keeps exit 3). docs/architecture.md's
   five-milestone-stale vibe-install row now tells today's truth.

**Gate panel at close (each gate's own exit code, on the final tree):**
`specmap --check` clean — 442 units / 407 items / 417 edges / 0 suspects /
0 gated orphans (10 dispositioned, 7 exempt); `conform check` — 2 frozen /
0 new (9 rules, **11 gated crates**; residual = the 2 MCP file-length);
`test-gate` — 1132 results / 0 failed / 3 skipped, xfail-strict;
`fast-loop --enforce-budget` — **20/20** < 60s (env-audit and vibe-install
joined); `tools/self-check.sh` — fmt, workspace tests (doctests included),
clippy -D warnings, `vibe check` 0/0/0.

**Open after v0.2 (owner court, unchanged):** the history-rewrite question
(audit -01 rider); publishing the two Discipline packages; production
solver selection (`solver=sat`); the PROP-010 design session; DBT-0020
(MCP spec home; the parked file-length pair is now the WHOLE baseline);
the four open-instrument predictions; the PROP-014 external-namespace
amendment; Discipline v0.3 inputs. New small candidates born this session:
AUD-0014/0015 (the two doc-string one-liners) remain the cheapest open
items; `CONTINUE.md` refresh rides the next session-end checkpoint.

---

## Prior phase (superseded same day): SHRINK-PLAN v0.1

**THE SHRINK PLAN — EXECUTED TO COMPLETION (2026-06-12, same-day execution).**
[`spec/terraforms/SHRINK-PLAN-v0.1.md`](terraforms/SHRINK-PLAN-v0.1.md) carries
the execution record in its header. Per phase:

0. **Phase 0** — the stale-trio premise **falsified** (566/556/554 were
   non-blank counts; the rule counts physical lines — real sizes 609/612/608;
   the trio moved to Phase 4, active set 26 not 23); the `GitBackend` seam
   doctest landed (runs, not just compiles); **frontend v4** — `UnwrapUse`
   gains fn-grain `in_deviation` via `#[spec(deviates = …, reason)]` on the
   carrying fn (deliberately NOT impl/struct/mod grain: the live solver-choice
   deviates edges on `Sat`/`NaiveDepSolver` must not grant unwrap amnesty).
1. **Phase 1** — R-001 wiring: `registry.rs` owns `local_registry()`, the
   Registry-cell construction site; install.rs threads the instance (+18/−6,
   the ≤50-line prediction held).
2. **Phase 2** — all 24 unwrap sites drained, `no-unwrap-in-domain` = 0.
   Split: 18 restructures (types carry the invariants — split-first
   `package_urls (primary, mirrors)`, let-else, `next_if`, read-then-advance
   counters, parser early-returns), 3 honest (a)-conversions (two "invariants"
   were NOT invariants: var-dep names are unvalidated at parse, pinned_ref is
   reachable via pub construction; plus the latent `=<version>+build` panic in
   `hold_pins` fixed by typed `Comparator`), 3 (b)-testimonies
   (`fetch_with_expected_hash`, `package_meta`, `sarif::render`). **Prediction
   "≥1/3 land as (b)" falsified — 3/24.** Deviates target settled:
   `ENGINE-CONFORM-v0.1#rules` (the grammar admits only resolvable spec://
   units; the ban itself lives in the package guide, outside the specmap).
3. **Phase 3** — all 68 messages in the Class-F grammar «human text
   (violates spec://…; fix: hint)», `error-message-cites-req` = 0; four
   parallel agents, central gates; only 3 doctest expectations moved
   (prediction <10 held); zero goldens coupled. One live error path eyeballed.
4. **Phase 4** — all 26 active over-budget files ≤ 600 physical lines; six
   parallel agents, ~40 new modules, every new production module carries its
   parent's `scope!` URI; `file-length` = 2 (the MCP pair, parked). Lessons
   now in the tree: the conform frontend parses files standalone, so
   tests-out files wrap fixtures in `#[cfg(test)] mod fixtures`; output.rs's
   frozen unsafe-gate ordinals pinned its env-guards in place; `pub(super)`
   items cannot be re-exported wider (E0364).
5. **Phase 5** — the `PackageScanner` seam: trait + doctest, `from-clones` /
   `from-github` cells (`#[cell]` + `implements = PROP-005#reindex`), the
   shared walk extracted to `org_walk.rs` so no cell imports a sibling (the
   R-002 lesson applied at design time), selection at the reindex composition
   root, GitVerse stays an error stub. Direct seam-driving oracles added
   inside the existing e2e suites (zero new test files — prediction held).
   **cell-has-oracle green at 20 cells.** Audit -09's seam half closed.

**Gate panel at close (each gate's own exit code, on the final tree):**
`specmap --check` clean — 442 units / 394 items / 404 edges / 0 suspects /
0 gated orphans (10 dispositioned, 7 exempt); `conform check` — 10 frozen /
0 new (9 rules; the residual ten = 8 unsafe-gate + 2 MCP file-length);
`test-gate` — 1123 results / 0 failed / 3 skipped, xfail-strict;
`fast-loop --enforce-budget` — 18/18 < 60s; `tools/self-check.sh` — fmt,
workspace tests, clippy -D warnings, `vibe check` 0/0/0.

**Open after the shrink (the owner court + the next plan):**
`CONFORM_GATED` expansion to vibe-core / vibe-index is the NEXT plan's
opening move (vibe-index now carries cells + a seam doctest ahead of its
gate). Owner items unchanged: the history-rewrite question (audit -01 rider);
publishing the two Discipline packages; production solver selection
(`solver=sat` flag); PROP-010 design session; DBT-0020 (MCP spec home; the
parked pair); the four open-instrument predictions; PROP-014
external-namespace amendment (new input: the deviates-target compromise —
unwrap testimonies cite ENGINE-CONFORM#rules because discipline:// is not
addressable in specmark); Discipline v0.3 inputs.

---

## Prior phase (superseded same day): the depth program

**THE DEPTH PROGRAM — COMPLETE (2026-06-12, same-day execution).**
Headline numbers, before → after: tagged items **190 → 337**, edges
**198 → 347**, `#[verifies]` **40 → 104**, typed REQ fabric **5 → 72**
units (59 req + 13 design), `#[cell]` manifests **4 → 18**, spec units
**352 → 442** (the 90 VIBEVM-SPEC anchors). What landed, per program
point:

1. **DBT-0019 closed** — mdspec scans `VIBEVM-SPEC.md` (90 additive
   anchors, `spec://vibevm/VIBEVM-SPEC#…`); vibe-core trio tagged;
   vibe-cli left the ratchet exemption (21 module markers; 7 crates
   exempt now); the MCP surface honestly filed as **DBT-0020** with 10
   dispositions instead of a wrong edge.
2. **Unit typing** — 67 kind lines across PROP-002/005/007/008/012;
   PROP-008/PROP-012 stale DRAFT statuses corrected (Phases 5/6/8
   shipped with M1.19 — back-filled into PROP-008 §7).
3. **Affirmation sweep** — 27 `#[spec(implements)]` item-grain tags
   (boot_artifacts → PROP-012 co-tenant/markers/create/plan-time/
   content/migration; Workspace/publish → PROP-007; vibe-index types/
   persistence/search/server → PROP-005; RedirectSection → PROP-002).
4. **Verifies sweep** — 64 new `#[verifies]`, r-pinned, across the
   strongest e2e and unit suites of six crates.
5. **Registry seam cell-ified** — local / git-monorepo /
   git-per-package manifests + oracle tests; R-002 fired live on a
   sibling-import and was fixed by extracting `registry_cache.rs`.
6. **Six god-file cuts** — CLI registry.rs → 6 modules; mrr → 5; gpr
   → 4; vibe-check → root + 11 `Check`-seam cells (one `all_checks()`
   registration point, oracle test, every file ≤ 600); package.rs →
   597-line hub + when/deps/features/wire; conform-core → 7 modules;
   cli_e2e.rs → 4 feature binaries + common (109/109 green; the
   install cluster is `cli_pkg_cycle.rs` — Windows UAC blocks
   *install*-named exes, the PROP-007 §9.5 lesson again).
7. **Conform rule wave** — `error-message-cites-req` (68 frozen),
   `file-length` 600 (28 frozen), `no-unwrap-in-domain` (24 frozen —
   the honest domain count with real cfg(test) scoping; frontend v3),
   `seam-has-doctest` widened past lib.rs (+`GitBackend`); new
   `cargo xtask conform freeze`; baseline 130 entries, shrink-only.

**Gate panel at close (run on the final tree, own exit codes):**
`specmap --check` green — 442 units / 337 items / 347 edges /
0 suspects / 0 gated orphans (10 dispositioned, 7 exempt); `conform
check` green — 130 frozen / 0 new (9 rules); `test-gate` green —
1120 results / 0 failed / 3 skipped, xfail-strict; `fast-loop
--enforce-budget` — 18/18 cells within 60 s; `tools/self-check.sh` —
all four steps (fmt, workspace tests, clippy -D warnings, `vibe
check` 0/0/0). Specmap and conform re-certified after the final
`cargo fmt` pass — the gate-invocation lesson applied.

**Open after the program (the shrink backlog + owner items):**
the 130-entry conform baseline is the work queue, and
[`spec/terraforms/SHRINK-PLAN-v0.1.md`](terraforms/SHRINK-PLAN-v0.1.md)
(authored 2026-06-12, owner-requested) is its execution plan — six
phases, ~14 gated batches: hygiene + GitBackend doctest + frontend v4
deviates-awareness → R-001 wiring of Registry-cell construction → the
24 unwrap sites (convert / deviates-testify / cfg(test)) → the 68
messages to the fixed product grammar («… (violates spec://…; fix:
…)») → the 23 active over-budget files (tests-out lever first) → the
`PackageScanner` seam (audit -09). Exit state: baseline 130 → 10
(8 unsafe-gate owner-gated + the 2 MCP-parked files — DBT-0020
untouched per owner instruction). `CONFORM_GATED` expansion to
vibe-core / vibe-index is explicitly the NEXT plan's opening move,
not this one's. Plus the pre-program owner items below (publishing,
solver selection, PROP-010 session, predictions, PROP-014 amendment,
Discipline v0.3).

---

## Prior phase (superseded same day): the audit window

**AUDIT WINDOW 2026-06-12 — the discipline-depth sweep: COMPLETE.**
The owner opened the INT-0001 window with the question «насколько
глубоко код соответствует идеалам AI-Native Rust». The run added
category **E (discipline depth)** to PROP-013 §2.2 and recorded **12
findings** in `AUDIT.md` (1 P1 fixed in-run, 7 P2 filed, 4 P3).
Headline: **the adoption is ~one crate deep** — vibe-resolver holds
80/198 edges, 42/50 `#[verifies]`, all 4 `#[cell]` manifests and the
only differential oracle; 347/352 spec units are untyped anchors (the
formal REQ fabric is PROP-003's pilot five); `VIBEVM-SPEC.md` (1190
lines, 0 units) keeps 8 crates ratchet-exempt (DBT-0019 escalated
P3→P2); PROP-012 is shipped with 0 edges; `seam-has-doctest` audits
lib.rs only and `error-enum-cites-req` checks the attribute, not the
Class-F message grammar; 23 src files exceed 600 lines (top: CLI
`commands/registry.rs` 3245, `multi_registry_resolver.rs` 2870,
`git_package_registry.rs` 2539; `vibe-check/lib.rs` is the whole crate
in one file). **The P1 (2026-06-12-01):** the committed `specmap.json`
had every `content_hash` emptied by the post-session history rewrite
of 2026-06-11 (all adoption-day commits re-hashed, e.g.
`1792c14`→`3ab0986`; pre-rewrite objects gone) — gate #1 was red on a
clean `main` while believed green; the close-out panel had certified
the pre-rewrite tree. Fixed by regeneration (`9f06fbf`); panel
re-certified on the live tree: specmap --check green (352/190/198/0),
conform 8 frozen / 0 new, test-gate 1109 results / 0 failed / 3
skipped, xfail-strict (fast-loop budget figures inherited). **Open
owner question:** what tool performed the rewrite — anything that
re-serializes committed derived artifacts must regenerate them or
leave them alone.

**The depth program (the audit's filed P2s, in dependency order):**
(1) DBT-0019 — unit-ify `VIBEVM-SPEC.md` (now P2; unblocks tagging for
vibe-cli/mcp/wire/xtask, half the workspace); (2) type the implemented
modules' PROPs (002/005/007/008/012) — kind/revision/status lines at
REQ grain; (3) affirmation sweeps, PROP-012 first (shipped, 0 edges),
then PROP-007 / PROP-005 item-grain; (4) `#[verifies]` tagging of the
strongest existing tests outside the resolver; (5) cell-ify the
`Registry` seam (3 proven production variants) with `#[cell]`
manifests + R-001 registration; (6) the god-file decomposition backlog
(CLI registry.rs → 4 cells; the two vibe-registry files; vibe-check
gains a `Check` seam; `manifest/package.rs` 5-way split; conform-core
engine split; `cli_e2e.rs` → per-feature files); (7) the conform rule
backlog (seam-doctest beyond lib.rs; Class-F message grammar;
file-length warn per guide §2; unwrap-in-domain with cfg(test)
exclusion) — each lands ratcheted.

---

## Prior phase (superseded 2026-06-12): the v0.3 adoption

**THE v0.3 ADOPTION IS COMPLETE (2026-06-11).** The owner dropped the
Discipline v0.2 package and TERRAFORM-PLAN-v0.3; the plan ran to its
§5 exit criteria in one continuous effort:

- **Phase 0** — self-hosting: the Discipline became two installed
  vibevm packages (`flow:org.vibevm/discipline-core@0.2.0`,
  `stack:org.vibevm/rust-ai-native@0.2.0`) resolved from the in-repo
  `packages/` local registry (`vibe install … --registry ./packages`);
  slots committed under `vibedeps/`; boot = 00-core → discipline-core
  → rust-ai-native → 90-user; `vibevm.discipline.lock` pins the
  pilot; the mechanisms (PROP-014, BROWNFIELD, ENGINE-CONFORM,
  LEDGER-INTENT) relocated to `spec/discipline/` with URIs
  re-anchored suspect-free; `spec/neworder/` is a shim.
- **Phases 1–6** — the nine-card catalog applied: the fast-loop
  checker (`cargo xtask fast-loop`, 18/18 cells <60s); the REQ-citing
  diagnostics grammar + `seam-has-doctest` / `error-enum-cites-req`
  rules; `CapabilityTag` types the activation seam (+trybuild
  compile-fail); contracts witnessed at use sites (roots-first,
  lockfile uniqueness; AUD-0014/0015 closed); the property net +
  the differential socket + `cell-has-oracle`; the `fixpoint_model`
  simulator with model-vs-production conformance; `cargo xtask
  codemod add-cell` (atomic, rollback proven live on its own
  template bug).
- **Phase 7** — **DBT-0011 fixed**: the `Sat` cell (chronological
  backtracking over version bounds, the naive solver as branch
  checker so semantics cannot drift) passes the dominance
  differential — the oracle found naive's first-pick trap in a
  generated world before any human enumerated one; resolvo stays an
  owner option behind the recorded deviates edge. Composition
  predicates (`and`/`or`/`not`, parens, precedence) ratified
  PROP-003 `#req-conditional-composition` r1-planned → r2.
  DBT-0016 also closed (its subject dissolved with the v0.2
  package).
- **Sweep** — 25 seam doctests + REQ-edged error enums across
  vibe-registry / vibe-workspace / vibe-check / vibe-publish
  (authored by four parallel agents, verified centrally by the
  widened gates — which immediately caught the one enum the agents'
  briefs excluded).

**Gate panel at close (all green):** `cargo xtask specmap --check` —
352 units / 190 items / 198 edges / 0 suspects; `cargo xtask conform
check` — 8 frozen / 0 new (six rules, seven gated crates);
`cargo xtask test-gate` — xfail-strict green; `cargo xtask fast-loop
--enforce-budget` — 18/18; `tools/self-check.sh` — all four steps.

**Open after the adoption (owner- or measurement-gated):** publishing
the two Discipline packages to the public `vibespecs` registry
(token, outward-facing); resolvo adoption + production solver
selection via the R-001 registry flag; the PROP-010 design session
(new input: directory registries are `--registry`-flag-only);
`VIBEVM-SPEC.md` unit-ification (DBT-0019); the PROP-014
external-namespace amendment (new precedent: the `discipline://`
citation namespace in conform diagnostics); the four open-instrument
predictions (P2-1, P4-1, P5-1, P6-1) awaiting a measured weak-agent
run; M1.23 (vibe-tcg Stage 1) gated on M1.5. The lockfile's
machine-absolute `file:///` source_url for local-registry installs
is a recorded debt candidate. The REPORT's eight-item honest list
feeds Discipline v0.3 — the discipline content now lives in
`packages/org.vibevm/*` (the owner's tree by the same convention
that governed `spec/neworder/`).

---

## Prior phase (superseded 2026-06-11): the v0.2 terraform

**THE BIG REFACTORING IS COMPLETE — branch policy retired (2026-06-10).**
The owner declared the refactoring complete in-session («рефакторинг
завершен, все фазы PLAYBOOK-TERRAFORM-VIBEVM выполнены»); `new` merged
back to `main` with `--no-ff` — merge commit **`e1da0c4`** (181 files,
+19 247 / −389), pushed to `origin/main`. The branch-isolation notice
that stood here is retired per its
own instruction; `new` is retained (merged, not deleted — the
`m1.17-workspace` precedent).

**The terraform in one breath.** Phases −1/0/1 (inventory, tooling
skeleton, pilot + drift drill) closed earlier the same day — their
detail stands below unedited. This checkpoint adds Phases 2–6:

- **Phase 2 — backfill `vibe-resolver`: DONE.** 54 proposals
  (`terraform/specmap-proposals.json`), every one owner-APPROVED in
  conversation; six per-module affirmation commits; the freshly-built
  **orphan ratchet** (`specmap-ratchet.json` + the gate inside
  `cargo xtask specmap --check`) caught the one item the sweep missed
  (PRP-0054, PredicateError). Three deviates edges record the honest
  gaps at their seams: resolvo-primary absent (DBT-0011),
  `pin_preferences` absent, `if_os` unprobed. Coverage of the crate's
  ratified non-disputed req units: grammar + host-invariance
  implemented-and-verified; fixpoint stays an explained zero from this
  crate (the re-solve loop lives in workspace orchestration —
  pilot judgment call 2, owner-upheld); `composition` is `planned` and
  reported separately.
- **Phase 3 — cells v0: DONE.** `#[cell(...)]` manifests (new
  specmark attribute + shared grammar) on NaiveDepSolver and the real
  DepProvider pair (local-registry / multi-registry — the playbook's
  "next real seam pair", SatDepSolver not being in tree); the
  cell-selection registry `crates/vibe-cli/src/registry.rs` (R-001 —
  the ONLY module reading selection flags; flags are data with
  provenance, birth, sunset); a **hermetic differential oracle**
  driving both provider cells over real bare `file://` git
  repositories to the same resolved graph — simultaneously the first
  brick of the AUDIT P1 git harness; interim `conform-lite` lints.
- **Phase 4 — conform engine MVP: DONE.** `conform-core` +
  `conform-frontend-rust` (syn T-syn): fact model, content-addressed
  store keyed `(file content-hash, producer)` under `target/conform/`
  (a 1-file diff re-extracts exactly 1 file — proven by producer-log
  test), rules-as-queries, byte-stable SARIF, ratchet baseline
  `conform-baseline.json` (six pre-existing unsafe findings frozen;
  the file may only shrink). Gate: `cargo xtask conform check
  [--scope …]`; conform-lite retired. Scope `crates/vibe-resolver`:
  0 findings.
- **Phase 5 — ledger MVP: DONE, local only.** `.ledger/` (git-ignored)
  holds the interpretations class; facts class = the conform store,
  proven epoch-immune. `cargo xtask trace explain --prose`: epoch-keyed
  cache (epoch = H(Cargo.lock, vibe.lock, wire schema, discipline
  README, rustc)), provenance line on every render, telemetry counters.
  Producer is a deterministic template — no LLM in the path.
- **Phase 6 — expansion + reconciliation + report: DONE.** Scope-grade
  backfill: 98 modules gained `specmark::scope!` edges sourced from
  their own module-doc PROP citations; ratchet exemptions 15 → 8 (each
  with a recorded reason), gated orphans 538 → 0 with 6 dispositioned
  under the new **DBT-0019** (vibe-core error/timestamp/values have no
  scannable home until `VIBEVM-SPEC.md` is unit-ified). Intent
  reconciliation: **0 unaccounted** of 31 (3 done / 27 rescoped /
  1 rejected — the CI matrix, no-CI being a standing Rule-4 owner
  decision). Instrumented category-C audit appended to `AUDIT.md`
  (AUD-0014..0017). **`terraform/REPORT.md` delivered** — phase
  ledger, metrics vs BASELINE, the eight-item honest list feeding the
  package v0.2.

**Gate panel at the merge** (all green): `cargo xtask specmap --check`
— 489 spec units / 170 tagged items / 177 edges / 0 suspects / six
known pin-into-unmarked warnings; orphan ratchet 0 gated, 6
dispositioned, 8 reasoned exemptions; `cargo xtask conform check` —
0 new findings (6 frozen); `cargo xtask test-gate` — 1075 results,
0 failed, 3 skipped, xfail-strict; golden characterization
byte-identical; full `tools/self-check.sh` green (fmt, tests, clippy
-D warnings, `vibe check` 0/0/0).

**Owner inputs that remain open after the terraform:** the PROP-010
design session (INT-0003); the SAT solver (DBT-0011 — now visible as
deviates edges at the seam); the next full PROP-013 audit window
(INT-0001); `VIBEVM-SPEC.md` unit-ification (DBT-0019 — unblocks
vibe-cli's item-grain backfill); the discipline-package v0.2 revision
fed by REPORT.md; the pending PROP-014 amendment for external
read-only namespaces (`misra://`, spec/neworder/README).

**The Big Refactoring = the Discipline terraform pilot (2026-06-10).** The
owner directed execution of [`spec/neworder/PLAYBOOK-TERRAFORM-VIBEVM-v0.2.md`](neworder/PLAYBOOK-TERRAFORM-VIBEVM-v0.2.md)
(the v0.2-beta discipline package in `spec/neworder/`). **Phase −1 —
inventory: freeze reality — is executed.** Build gate exit 0; record-only
test run nextest **998/998 passed + 3 skipped** (the `#[ignore]`d live trio
— now the only entries in `terraform/registry/tests-baseline.json`, the
xfail-strict baseline); **debt registry** seeded
(`terraform/registry/debt.json` + `DEBT.md` — 18 entries: 1 P1 / 7 P2 /
10 P3; the 11 non-fixed AUDIT findings imported 1:1, plus 5 conflict-scan
disputes, plus 2 new from the inventory itself); **intent registry**
harvested (`terraform/registry/intent.json` + `INTENT.md` — 31 aspirations
from WAL / CONTINUE / ROADMAP; `TASKS.md` confirmed absent); **conflict
scan** over `spec/**` recorded 5 disputed pairs, resolved none (DBT-0012
PROP-002 vs PROP-008 naming default; DBT-0013 boot `00-core` vs `90-user`
registry host; DBT-0014 `90-user` repo shape vs PROP-008 / live org;
DBT-0015 PROP-003 duplicate `{#phases}` anchor — the Phase 1 pilot PROP;
DBT-0016 PLAYBOOK vs BROWNFIELD marker homing); **characterization**
captured (`terraform/golden/` — 5 hermetic flows / 12 steps, byte-
deterministic across double runs via `capture.sh`); snapshot in
`terraform/BASELINE.md`; session log in `terraform/LOG.md`.

**Phase −1 acceptance closed (2026-06-10).** The owner confirmed the P1
disposition and all five disputed-spec existences, and granted in-session
sanction to edit frozen surfaces. **Four disputes adjudicated immediately**
(all supersede): PROP-002 naming reconciled to PROP-008 fqdn (`aa54ab4`,
DBT-0012); boot `00-core.md` / `90-user.md` reconciled to split-host +
fqdn reality (`0e57f0f`, DBT-0013/0014); PROP-003's duplicate `{#phases}`
anchor disambiguated — §3.2 is now `{#solver-phases}` (`d090cb0`,
DBT-0015). DBT-0016 stays open by design (feeds the package v0.2).

**Phase 0 — tooling skeleton: DONE (2026-06-10).** Three new crates —
`specmark-grammar` (the single source of the PROP-014 §2.3 tag grammar),
`specmark` (inert `#[spec]` / `#[verifies]` / `scope!` proc-macros:
compile-time validation, rustdoc `Spec:` injection, item unchanged),
`specmap-core` (markdown unit parser with kind/revision/status lines and
CRLF-invariant hashes; syn-based attribute scanner; canonical index;
xfail-strict test-gate engine; debt tripwires) — plus
`schemas/specmap.jtd.json` → `vibe-wire` types, and three xtask
subcommands: `specmap [--check]`, `test-gate`, `tripwire`. The first
committed `specmap.json` inventories **408 spec units** (zero production
edges yet — Phase 1 lands the first). Acceptance green: `specmap` +
`--check` ×2 deterministic; `test-gate` 1044 parsed / 0 failed / 3
skipped (the quarantined live trio), xfail-strict; `cargo test -p
specmark` green; full `self-check.sh` green with `vibe check` 0/0/0.
**The CI bullet is deferred with cause:** the repo has no CI
infrastructure at all, so introducing it is a Rule 4 owner decision, not
a playbook line item — acceptance commands run locally. Same-day field
results: `tripwire` caught the owner-dropped `GUIDE-TYPESCRIPT` /
`GUIDE-PYTHON` files via DBT-0016's watch (now committed, README map
updated), and the unit parser gained fenced-block exclusion after two
sample headings from `GUIDE-SPEC-AUTHORING` leaked into the inventory.

**Phase 1 — pilot: EXECUTED (2026-06-10), review in-conversation.** Per
the owner's live direction ("без отдельных PR, работаем в new, всё
решаем здесь") the pilot landed directly on `new`; the full dossier is
[`terraform/PHASE1-PILOT.md`](../terraform/PHASE1-PILOT.md). Engine prep
first (`40077bf`, `dc79001`): canonical house-style URIs (the indexer's
full-path URIs would never have joined the repo's citation style — caught
before the pilot tripped on it), `spec_unit.file`, the suspects table,
dangling-edge / pin-ahead / pin-into-unmarked warnings, drift
classification on `specmap`/`--check` (revision bumps with their
suspects; unbumped-hash with the `spec-editorial:` convention), and
`cargo xtask trace explain <symbol|uri> [--text|--json]`. Then the pilot
(`4395d3b`): PROP-003 §2.6.1 unit-ified additions-only — four `req`
units (`grammar`, `fixpoint`, `host-invariance` ratified r1;
`composition` **planned**) plus one `design` unit; `conditional.rs`
carries the first production tags (implements ×3, the recorded
`deviates` into the planned unit, `#[verifies]` ×6); index: **413 units,
17 items, 19 edges, 0 suspects**. The drift drill ran end-to-end and
stays in history (`b3a947c` bump → 6 suspects → re-affirm; `73b6e81`
editorial → unbumped-hash → `spec-editorial:` marker; `4afe716` revert
to byte-identical pilot state). Acceptance: `trace explain` renders the
planned/deviates subgraph; `test-gate` green (1051 results, xfail-strict);
full `self-check.sh` green. Tripwire on the change set: DBT-0011 fired
(`touch:crates/vibe-resolver/**`) — addressed: tags only, solver debt
untouched.

**Phase 2 — backfill `vibe-resolver`: superseded by the COMPLETE
checkpoint above.** (This slot held the "STARTED, staged for the next
session" notice; the staged sweep ran and the phase closed the same
day — see the Current-phase block and `terraform/REPORT.md`. The
mid-session owner drops continued through the close-out: after the
three C++ guides (`630ba3b`), the session committed Go, four Java
guides, and Kotlin the moment the DBT-0016 watch surfaced them. The
pilot's three judgment calls were upheld by the owner's blanket
APPROVE; the CI decision stays with the owner — INT-0017 rejected
accordingly.)

**M1.19 — qualified package naming (PROP-008): SHIPPED 2026-05-22, under MFBT.** The qualified-naming refactor — [PROP-008](modules/vibe-registry/PROP-008-qualified-naming.md), design lore in [`spec/design/workspace-and-qualified-naming.md`](design/workspace-and-qualified-naming.md) — is **complete**: all eight phases on `main`, `bash tools/self-check.sh` green on all four steps. Exhaustive per-phase detail is in `CHANGELOG.md`'s M1.19 block and PROP-008 §7.

The identity core landed earlier this session — Phase 1 the `Group` newtype + the mandatory `[package].group`; Phase 2 the `PackageRef` identity refactor (`{ kind: Option<PackageKind>, group: Option<Group>, name, version }`, identity `(group, name, version, content_hash)`, `kind` demoted to pure metadata, pkgref grammar `[kind:][group/]name[@version]`); Phase 3 the lockfile `group` field at `CURRENT_SCHEMA_VERSION` 5; Phase 4 the group-native registry with `NamingConvention::Fqdn` the default; Phase 7 the group-native package index (`by-name/<name>.json` candidate sets). The identity core was the squashed `feat(core)` `c5c4fe6`; Phase 7 was `59355d3`.

This checkpoint adds the closing phases — order 8 → 5 → 6, per the owner:

- **Phase 8 — docs/spec close-out** (`a54fbea`, `503f912`, `1d66822`). `VIBEVM-SPEC.md` §7–§8 rewritten for group-qualified identity under the owner sanction in the PROP-008 header — the identity tuple, the `[kind:][group/]name[@version]` pkgref grammar, `name` unique within `group`, `kind` as metadata, lockfile schema v5, `naming = "fqdn"` the default. `docs/` (glossary, lockfile-format, architecture, install, version-syntax, git-source-dependencies, registry-add/publish) reconciled; PROP-008 §3 corrected v4→v5. The canonical `fixtures/registry/` packages already carried `group` (Phases 2/4), so no in-repo package migration was needed.
- **Phase 5 — index-backed short-name resolution** (`f4e8ee2`). `vibe install wal` (bare) resolves to `org.vibevm/wal` at the CLI input boundary, before the depsolver; manifests and the lockfile store only the qualified form. Lockfile-first, then candidate enumeration — `LocalRegistry::candidate_groups` (a directory scan, no index needed), `MultiRegistryResolver::resolve_name_candidates` (an index walk via `by-name/<name>.json`), `IndexClient::name_candidates`. The CLI-boundary orchestration is a new `crates/vibe-cli/src/commands/short_name.rs` module.
- **Phase 6 — collision detection + exit code 7** (`cee8c4a`, `56c574e`). A short name matching two groups → `InstallError::AmbiguousPackage`, the new exit code **`7`** ("ambiguous package", distinct from `3` — a collision is a naming ambiguity, not a dependency conflict), with the numbered qualified alternatives printed. `VIBEVM-SPEC.md` §9.4 records the code — closes PROP-008 §5 open question 1.

**Registry-org migration — GitHub `vibespecs` DONE this session; GitVerse + test orgs remain.** With the owner's explicit token authorisation, the three canonical packages were re-published from `fixtures/registry/org.vibevm/<name>/v0.1.0/` to the new fqdn repos `vibespecs/org.vibevm.{wal,sync-from-code,atomic-commits}` (via `vibe registry publish`, tag `v0.1.0`), and the legacy `vibespecs/flow-*` repos were archived — read-only, reversible, not deleted. A live smoke (a fresh `vibe init` + `vibe install org.vibevm/wal` against the real registry) installs cleanly at lockfile schema v5 with `group`. The smoke also surfaced and fixed a PROP-008 propagation miss — `vibe init` and `vibe registry add` still scaffolded `naming = "kind-name"`, now `fqdn`. The GitVerse side and the GitHub test orgs `vibespecstest1/2` remain owner-only — see Known issues.

**PROP-005 — the package index: found IMPLEMENTED, de-rotted, then folded into the workspace (2026-05-22).** A state review opening the planned PROP-005 work found the index was not pending at all: slices 1–8 (the `vibe-index` server + CLI), slices 9–10 (the `vibe-publish` post-publish hook and the `vibe-registry` consumer-side `IndexClient` fast path), and M2.10 `vibe search` had all shipped in earlier sessions. But `vibe-index` was a standalone Cargo workspace, outside the routine `cargo test --workspace` gate, and it had silently rotted: its duplicated `vibe.toml` parser still expected the pre-M1.17 schema (`[writes]`, `[dependencies]`, `[boot_snippet].filename`) and could not parse a current manifest, and its `content_hash` parity test had drifted off a fixture renamed by the M1.17 manifest unification — the suite was red. **The de-rot** rewrote `scanner/manifest.rs` for the unified `vibe.toml` (M1.17) + loading model (M1.18), fixed `BootSnippetEntry` (`filename` → `source` / `category`), refreshed the golden fixture + parity hash (cross-checked against the canonical `vibe-registry::compute_content_hash`), added a current-schema scanner regression test, and retired the dead slice-1 scaffolding. **The fold** (the owner's call, taken after the de-rot landed) then moved `vibe-index` from `services/vibe-index/` into `crates/vibe-index/` as a member of the vibevm workspace and switched the scanner to parse through `vibe-core::Manifest` / `SubskillManifest` — the duplicated parser is deleted outright, so the index schema can no longer drift, and the routine `cargo test --workspace` gate now covers the crate (`tools/self-check.sh` drops its standalone special-case). `vibe-index` is green — **169 tests**, `cargo clippy --workspace -D warnings` clean, `cargo fmt` clean. PROP-005 spec reconciled — §2.6 entry schema, the `vibe.toml` filename, §3.2 / §6 (the reversed standalone-workspace decision), §9 item 11 (RESOLVED). The CHANGELOG records the PROP-005 milestone end to end. As a closing pass, the whole workspace was brought rustfmt-clean (`cargo fmt --all` — 69 files of drift that no gate had caught, since `self-check.sh` checked test / clippy / `vibe check` but never formatting), and `tools/self-check.sh` gained `cargo fmt --all --check` as its first, fail-fast invariant. `bash tools/self-check.sh` is green on all four steps.

**M1.21 — Incremental install (PROP-011): SHIPPED 2026-05-22.** `vibe install` is now incremental — it does the least work a change requires. Four phases, all on `main`:

- **Phase 1 — skip resolution when fresh** (`feat(install)` `d6c4248`). A new `vibe-workspace::freshness` module runs a `cargo`-style satisfiability check before the depsolver: is `vibe.lock` still a correct resolution of every node's `[requires]`? When it is, a bare `vibe install` skips the depsolver entirely — no registry walk, no network, just a whole-tree boot regeneration. `vibe install` is now **lockfile-respecting** — an unchanged `[requires]` honours the locked versions, ending the silent version drift.
- **Phase 2 — materialise only the diff** (`2b1b6cc`). `apply_resolution` skips re-copying a `vibedeps/` slot already present for the resolved (immutable) version. A `slot_integrity` key in the vibevm user config (`trust-presence` default, or `verify`) governs the skip; `vibe reinstall --force` passes `verify`.
- **Phase 3 — minimum-churn re-resolution** (`f22f629`). When `[requires]` changed, `vibe install` re-resolves but pins every still-satisfied registry root to its locked version, so an untouched dependency never drifts; a held-pin conflict falls back to a full re-resolve.
- **Phase 4 — docs** (this checkpoint). `VIBEVM-SPEC.md` §9.1 records the lockfile-respecting contract (owner sanction granted this session); PROP-011 reconciled to the implementation; CHANGELOG / ROADMAP register M1.21; `docs/commands/install.md` documents the incremental behaviour.

**Two implementation findings, reconciled into PROP-011 (Sync-from-Code).** (1) FU3's `vibe update <pkgref>` scoped resolution is correctness-relaxed — it never unifies the held and re-resolved subtrees — so it cannot serve `vibe install`'s unified contract; Phase 3 holds pins via constraint-tightening instead, and skipping the registry walk for an unchanged subtree is deferred to PROP-003's SAT solver. (2) `slot_integrity = verify` re-materialises rather than hash-comparing — the cheaper `content_hash` spot-check waits until `compute_content_hash` is lowered out of `vibe-registry`.

**M1.18 — Loading model (PROP-009 + PROP-012): SHIPPED 2026-05-22, merged to `main`.** The flat `spec/boot/NN-*.md` boot model is gone; vibevm now boots from a computed loading model. `main` is at the `--no-ff` merge commit **`ffd5e1c`** — M1.17 (Workspace) and M1.18 (Loading model) both landed. Working tree clean; `cargo test --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, and `vibe check --path .` all green.

**The model in one breath.** Two physically separate trees — authored `spec/` (only the author writes it) and a committed `vibedeps/` (only `vibe` writes it; one slot `vibedeps/<kind>-<name>/<version>/` per resolved package, the package's published tree verbatim). The boot sequence is *computed* per node from the unified resolution — inherited foundation + own boot + dependency boot + overrides — and projected into `spec/boot/INLINE.md` (the verbatim `inline` priority lane) and `spec/boot/INDEX.md` (a TOML manifest of `static` paths + `dynamic` INCLUDE pointers). Three inclusion types — `inline` / `static` / `dynamic` — set per dependency via `link` (default `static`). The `NN-` filename prefix and `[writes]` are retired; `vibe` owns ordering by `[boot_snippet].category` band. `CLAUDE.md` / `AGENTS.md` / `GEMINI.md` are co-tenant files — vibevm owns only a `<vibevm>` block inside each, never the whole file (PROP-012). `vibe reinstall` regenerates without re-resolving.

**Shipped 2026-05-22, on top of the M1.18 merge — the dynamic-entry `when` gate.** PROP-009 §2.3 showed a `when` activation condition on a `dynamic` `INDEX.md` entry, but §2.6 pinned no field that declared it — the contract gap flagged at Phase 4. It is now closed. A package's `[boot_snippet]` may carry an optional `when`; for v1 the only condition is an operating-system match — the wire string `"os:<name>"` (`windows` / `macos` / `linux`), enough for OS-specific packages and subskills. `vibe-core` gains `WhenCondition` / `TargetOs` (`feat(core)`); the computed-view engine forces a `when`-bearing snippet to `dynamic` — a condition cannot be `inline`d or read as plain `static` — and carries the condition into `BootEntry`; `render_index` writes `when = "os:<name>"` into the `[[entry]]`, and the `INDEX.md` header documents the OS test for the agent, which evaluates it at boot (the committed `INDEX.md` stays OS-invariant). The same OS probe is reserved as `if_os` in the subskill `[activation]` vocabulary (PROP-003 §2.5.2) — one grammar across both mechanisms. Gate green: vibe-core 169 tests, vibe-workspace 87; `cargo clippy --workspace --all-targets -- -D warnings` clean; `vibe check --path .` clean.

**Shipped earlier 2026-05-22 — M1.18 Phase 7 + three follow-ups** (commits `78d9613` … `56d7a5f`, then merge `ffd5e1c`):

- **PROP-012 — the managed `<vibevm>` block** (`78d9613`, `651a57d` design; `55f24cd` impl). The Phase-4 redirect code overwrote the *whole* of `CLAUDE.md` / `AGENTS.md` / `GEMINI.md` on every install — destroying hand-authored content. `vibe` now owns only a delimited `<vibevm>` … `</vibevm>` block: locate / classify / splice / append / migrate; exactly one block per file (malformed → hard error, validated at plan time). `vibe init` now generates the boot artifacts, so a fresh project is bootable at once.
- **`vibe check` aligned with the loading model** (`ee117f4` boot-directory; `f35c557` check 8 + the malformed-block check). The `NN-` enforcement is retired; check 8 (`lockfile_files`) now verifies `vibedeps/` slot consistency; new `CheckId::RedirectBlock` reports a malformed `<vibevm>` block.
- **vibevm self-migration** (`2981970`). The repo migrated to its own loading model — `spec/boot/INDEX.md` generated, a `<vibevm>` block appended to its instruction files (every hand-authored line, the four rules included, preserved).
- **`VIBEVM-SPEC.md` consistency pass** (`bcb09fe`) — owner sanctioned a full pass; §6 rewritten, the retired-model footprint cleared across ~18 sections.
- **Docs + status** (`2028699` register M1.18; `09592af` docs sweep; `56d7a5f` 00-core.md). New `docs/loading-model.md`, `docs/commands/reinstall.md`; the `docs/` sweep; ROADMAP / CHANGELOG flipped M1.18 to shipped; `00-core.md` updated under owner sanction.

**Earlier in M1.18 (Phases 1–6, pre-this-session)** — the schema, the `vibedeps/` tree, the computed-view engine, `INLINE.md` / `INDEX.md` generation, workspace-aware `vibe install` (+ five follow-ups), `vibe reinstall`, published-copy boot regeneration. Full detail in `CHANGELOG.md` and PROP-009 §8.

**Branch state.** On `main`, pushed to `origin/main`. The M1.21 (PROP-011) commits — `d6c4248`, `2b1b6cc`, `f22f629`, plus this Phase-4 docs checkpoint — land on top of the `when`-gate commits (`fef37e5` … `0164a20`, `00bdd48`) and the M1.18 session-end checkpoints (`ffd5e1c` merge → `c74b2a5`). The `m1.17-workspace` feature branch is retained (merged, not deleted). Gate green — test counts: vibe-cli bin 124 / e2e 106 / cli_init 11 / cli_search 15 (3 ignored), vibe-core 173, vibe-workspace 103, vibe-check 27, vibe-registry 106 + 5 + 7, vibe-publish 51 + 5, vibe-resolver 48, vibe-mcp 22.

**Next — base-machinery-first, per the owner (2026-05-22).** The owner **deferred M1.5 (LLM Generation)** to a later phase: the base package machinery is to be brought to relative stability first — covered with tests, ready for large structural refactors — before *any* generation (not only LLM generation) is layered on top.

The dependency-correct sequence for the base, each under MFBT (PROP-006 §2):

- (a) **PROP-005 — the package index. ✅ DONE.** The state review that opened this work found PROP-005 already implemented — slices 1–10 plus M2.10 `vibe search`. It was de-rotted, then folded into the `crates/` workspace (see the PROP-005 entry above); PROP-005 §9 item 11 is resolved.
- (b) **PROP-008 — qualified naming (M1.19). ✅ DONE.** All eight phases shipped 2026-05-22 under MFBT — see the Current-phase block. The only residue is the owner-only outward-facing registry-org migration (Known issues); it gates nothing in-repo.
- (c) **PROP-010 — the local package cache (M1.20).** §2.3 keys it by PROP-008 identity; its five §5 open questions need an owner design session before implementation.

Then M1.5. No blocker.

**Test hardening before the next layer.** PROP-013 — the periodic health audit ([`common/PROP-013`](common/PROP-013-periodic-health-audit.md)) — is now an established process; findings live in [`AUDIT.md`](../AUDIT.md). Its seed run (2026-05-23) flags one **P1**: the production git-registry + naming path is under-tested — the gap that let the `vibe init` defect ship green through all eight phases of M1.19. Per the owner's base-machinery-first principle, the first full audit run and that P1 (a hermetic harness driving `GitPackageRegistry` against real `file://` git repositories, plus a default-path `vibe init` → `vibe install` e2e) should be weighed before, or run in parallel with, PROP-010 — laying the cache on an under-tested base only compounds the risk.

**Known issues / open items.**

- **Health audit (PROP-013).** A periodic defect / rot / drift inventory is now an established process — [`common/PROP-013`](common/PROP-013-periodic-health-audit.md), written to [`AUDIT.md`](../AUDIT.md). The seed run (2026-05-23) catalogued **13 findings** (2 P1, 4 P2, 7 P3; 2 already fixed). The items in this list are mirrored there with severities and stable IDs; `AUDIT.md` is the canonical inventory and the durable health record. Re-run per PROP-013 §3 — floor: once per milestone.
- **Registry-org migration — GitHub `vibespecs` done 2026-05-22; GitVerse + test orgs remain.** The canonical GitHub org is migrated under the owner's token authorisation: `org.vibevm.{wal,sync-from-code,atomic-commits}` published in the `fqdn` shape (tag `v0.1.0`), the legacy `flow-*` repos archived (read-only — reversible; the owner can delete them outright if a fully-clean org is wanted). The `vibe init` / `vibe registry add` naming-default bug the live smoke surfaced is fixed. Remaining: **(a)** the GitVerse side — `vibespecs-gitverse` and `vibespecstest3` — the GitHub token does not apply and GitVerse has no API DELETE, so this is owner web-UI / owner-token work; **(b)** the GitHub test orgs `vibespecstest1/2`, whose re-layout is coupled to the `#[ignore]`d `cli_live_e2e` tests — re-laying those fixtures means updating what the live tests expect, a unit of work best done together. Gates nothing in-repo — every hermetic test is self-contained and green.
- **`fixtures/manual-test-packages/` rot.** `flow-vibevm-github-smoke` (and likely `flow-vibevm-direct-push-smoke`) carry retired schema — `[writes]`, `[boot_snippet].filename`, no `[package].group`. Stale since M1.18 / PROP-008; not parsed by any hermetic test (manual-test fixtures only), so the gate stays green. A small de-rot pass, out of M1.19 scope.
- **PROP-010** — DRAFT; needs an owner design session to close its §5 open questions before implementation. PROP-011 is shipped (see Current phase).
- **Deferred PROP-011 refinements** (recorded in PROP-011 §5/§8) — the `content_hash` slot spot-check for `slot_integrity = verify` (needs `compute_content_hash` lowered out of `vibe-registry`); true incremental re-resolution that skips the registry walk for an unchanged subtree (needs PROP-003's SAT `pin_preferences`).
- **Parked backlog** — `version = { workspace = true }` member-version inheritance (PROP-007 §6 q4); the publish-signalling polish (`--archive`, `has_issues`).

**Resolved 2026-05-22 / 2026-05-23 (M1.19 session).** PROP-008 / M1.19 shipped end to end — Phase 8 (docs/spec close-out), Phase 5 (index-backed short-name resolution), Phase 6 (collision detection + exit code 7), on top of Phases 1–4 + 7 earlier the same day. `VIBEVM-SPEC.md` §7 / §8 / §9.4 reconciled under the standing owner sanction; PROP-008 §5 open question 1 (exit code 7 assignment) closed. The canonical GitHub `vibespecs` registry org was then migrated to the `fqdn` shape — new `org.vibevm.*` repos published, legacy `flow-*` archived — and a `vibe init` / `vibe registry add` naming-default bug surfaced by the live install smoke was fixed (`fix(cli)`). On 2026-05-23 a new process PROP was authored — [PROP-013](common/PROP-013-periodic-health-audit.md), the periodic health audit — and seeded with 13 findings in [`AUDIT.md`](../AUDIT.md); the audit is now the durable inventory the next session carries forward, with its first full sweep recommended before, or in parallel with, PROP-010.

---

## Earlier checkpoint (kept for context — M1.18 Phases 1–6, 2026-05-21)

**M1.18 Phases 1–6 landed on `m1.17-workspace`** before this session. PROP-009's loading model implemented in six phases: Phase 1 schema (`LinkType`, `BootCategory`, `[boot_snippet].category`, the `[boot]` table — commit `ce14877`); Phase 2 the `vibedeps/` materialisation tree (`e0a8d75`); Phase 3 the computed-view engine `compute_effective_boot` (`4e488e1`, `15dbefe`); Phase 4 boot-artifact generation — `INLINE.md` / `INDEX.md` / the redirect (`e06a5ff`); Phase 5 workspace-aware `vibe install` switch-over, `[writes]` deleted, plus five follow-ups FU1–FU5 (`f4d45a4` … `85dbc9a`); Phase 6 `vibe reinstall` + published-copy boot regeneration (`4606132`, `0706ae2`). PROP-010 (local package cache) and PROP-011 (incremental install) were drafted and registered as DRAFTs. Full detail: `CHANGELOG.md` M1.18 entry, PROP-009 §8.

- [`spec/design/loading-and-boot-model.md`](design/loading-and-boot-model.md) — non-normative rationale: the static/dynamic-linking metaphor, the four principles, the fork-by-fork record (commit `b48ba7f`).
- [PROP-009](modules/vibe-workspace/PROP-009-loading-model.md) — the contract; DRAFT, but every §5 open question is resolved — ready for M1.18 implementation (commits `1c1c19c`, `72ac624`).
- **Phase 1 — schema** (commit `ce14877`). `vibe-core` gains `LinkType` (the inclusion type — `inline` / `static` / `dynamic`, §2.4), `BootCategory` (the ordering band that retires the `NN-` prefix, §2.5), optional `category` + a suggested `link` on `[boot_snippet]`, the `Requires.links` side map (`<kind>:<name>` → `LinkType`; a side map, not a field on `PackageRef`, so `PackageRef` and its ~40 call-sites stay pristine — `link` is consumer config, not identity), and the project-level `[boot]` table (§2.6). All **additive** — nothing retired, the build stays green. Lockfile assessed: no bump, `vibe.lock` stays schema v4 (`link` does not affect resolution; materialisation slots are Phase 2). vibe-core 161 tests (+19).
- **Phase 2 — the `vibedeps/` tree** (commit `e0a8d75`). A new `vibedeps` module in `vibe-workspace` owns the materialisation layout (§2.1): `materialise` copies a resolved package's published tree verbatim into `vibedeps/<kind>-<name>/<version>/` at the absolute workspace root — idempotent (it clears the slot first, so stale files never linger), skipping `.git` and symlinks. Plus `slot_rel_path` / `slot_abs_path` / `is_materialised` / `remove_slot` and `Workspace::vibedeps_root` / `vibedeps_slot`. **Additive** — the legacy `[writes]` mirror layout is untouched; it retires at the Phase 5 switch-over. vibe-workspace 39 tests (+8); a `semver` dependency added.
- **Phase 3 — the computed-view engine** (commits `4e488e1`, `15dbefe`). A new `boot` module in `vibe-workspace`: `compute_effective_boot` composes a node's effective boot sequence (§2.2) — inherited foundation + own boot + dependency boot + user overrides — as a pure function over already-discovered inputs (no depsolver, no disk, no artifacts), so the algorithm is exhaustively unit-testable. Four-band ordering (§2.5), topological sort of the dependency band (a dependency before its dependents; a cycle → `BootDependencyCycle`), link precedence (§2.4: per-dep declared > package suggestion > `[boot].default_link` > `static`), and `EffectiveBoot::inline_entries` / `indexed_entries` for Phase 4. A discovered prerequisite shipped first as `fix(core)` `4e488e1`: Phase 1 elided an explicit `link = "static"`, which would silently lose a consumer's override of a workspace default — `Requires.links` now stores every declared link, and `Requires::declared_link` distinguishes explicit from absent. **Additive** — nothing calls the engine yet; Phase 5 wires it. vibe-workspace 53 tests (+14), vibe-core 162 (+1).
- **Phase 4 — boot artifact generation** (commit `e06a5ff`). A new `boot_artifacts` module in `vibe-workspace` projects an `EffectiveBoot` into the session-start files (§2.3): `render_index` (the `INDEX.md` TOML manifest — `schema`, an `inline` pointer, ordered `[[entry]]` tables with `path` + `kind`), `render_inline` (`INLINE.md` — verbatim concatenation of the `inline`-linked contributions), `render_redirect` (the `CLAUDE.md` / `AGENTS.md` / `GEMINI.md` thin redirect), and `write_boot_artifacts` (writes all three for a node; removes a stale `INLINE.md`). **Additive** — nothing calls it, and it never touches a hand-authored `CLAUDE.md`; `serde` + `toml` deps added. vibe-workspace 63 tests (+10). Contract gap flagged: §2.3 shows `when` on a `dynamic` `INDEX.md` entry but §2.6 pins no field declaring it — the renderer is `when`-ready but leaves it unset (see Owner attention).
- **Phase 5 — workspace-aware `vibe install` (the switch-over).** `vibe install` / `vibe uninstall` / `vibe update` now drive the computed loading model; the legacy `[writes]` mirror layout is gone. Commits: `f4d45a4` — the `install` orchestrator `apply_resolution` (materialise + regenerate boot, decoupled from the registry via `ResolvedDep`); `440a88c` — the `vibe install` CLI rewired onto it; `93fd043` — `vibe uninstall` (remove the `vibedeps/` slot + `regenerate_boot`) and `vibe update` (re-resolve, delegating to install-from-manifest); `7347208` — the `[writes]` machinery deleted (`vibe-install` gutted ~2300 lines → just `InstallError`); `a6e20db` — `fix(cli)` for a discovered ordering bug (boot was regenerated before `[requires]` was merged, so a CLI install's `INDEX.md` dropped the new package's own boot); `72b87b9` — `build(install)` `[lib] test = false`; `682e06d` — the e2e suite rewritten for the `vibedeps/` model (26 tests touched — 11 retired-behaviour deletions, 15 rewrites). `cargo test --workspace` (no exclude) and `cargo clippy --workspace --all-targets` are green.
- **Phase 6 — `vibe reinstall` + published-copy regeneration** (commits `4606132`, `0706ae2`). `vibe reinstall [<path>] [--force]` (PROP-009 §2.10) recomputes a workspace's materialised state and boot artifacts **without re-resolving** — the versions stay exactly as `vibe.lock` pins them; it is not `vibe update`. Without `--force` it regenerates every node's boot from the materialised `vibedeps/` tree on disk (no fetch, no network — the fix for a stale or hand-edited `INDEX.md`); a locked package whose slot is missing is reported and the operator pointed at `--force`. With `--force` it re-fetches every locked package's content from source at the pinned version, wipes the project `.vibe/cache`, then re-materialises `vibedeps/` and regenerates boot — the escape hatch for a corrupted subtree. Published-copy regeneration (PROP-009 §2.11): `vibe workspace publish`'s `stage_node` now regenerates each staged copy's boot artifacts for the published shape — a standalone node with its own authored boot only, no inherited foundation and no materialised dependencies — so the published `INDEX.md` never dangles on the dev tree's workspace `vibedeps/` slots. vibe-cli e2e 104 (+5), vibe-workspace 69 (+1).

**The model in one breath.** Two physically separate trees — authored `spec/` (only the author writes it) and a committed `vibedeps/` (only `vibe` writes it; one slot `vibedeps/<kind>-<name>/<version>/` per resolved package, the package's tree verbatim). The boot sequence is *computed* per node from the unified resolution — inherited foundation + own boot + dependency boot + overrides. `vibe install` generates, per entry-point node, `spec/boot/INLINE.md` (verbatim concatenation of `inline`-typed contributions, read first — the priority lane) and `spec/boot/INDEX.md` (a TOML manifest of `static` paths + `dynamic` INCLUDE pointers). Three inclusion types — `inline` / `static` / `dynamic` — set per dependency in `vibe.toml` (`link = …`, default `static`). The `NN-` prefix is retired; `vibe` owns ordering by category. `[writes]` is retired. `vibe reinstall [<path>] [--force]` regenerates. One computed-view engine serves both boot and the effective spec. The model is uniform — a single-package project is a degenerate workspace.

**Next — M1.18 Phase 7: migration + docs.** PROP-009 §7 phase 7 — existing-project migration, the vibevm self-migration (`spec/boot/` becomes categorised authored boot plus the generated `INLINE.md` / `INDEX.md` and the thin `CLAUDE.md` / `AGENTS.md` / `GEMINI.md` redirects), the `VIBEVM-SPEC.md` edits (§6, §4.2, §4.6, §13.1 — **explicit owner sanction required, not yet granted**), `ROADMAP.md` / `CHANGELOG.md`, and the `docs/` sweep (a new `docs/commands/reinstall.md` among them). Then phase 8 (the effective-spec view — v1.5 scope).

**Phase 5 follow-ups — all five landed** (commits `2f42776`, `1a55409`, `6ec47d2`, `b313829`, `85dbc9a`):

- **FU1** — `[writes]` and `[boot_snippet].filename` are retired from the `vibe-core` manifest schema (`WritesSection` / `Manifest.writes` / `BootSnippet.filename` deleted); the six `fixtures/registry/` manifests and every embedded test manifest migrated; `vibe-check`'s i18n-coverage check and `vibe-publish`'s index payload adjusted off the removed fields.
- **FU2** — `vibe install` run from the manifest unifies resolution across **every** workspace member's `[requires]`, not just the entry node (PROP-009 §2.7); a standalone project is a one-node workspace, so it degenerates cleanly.
- **FU3** — `vibe update <pkgref>…` is scoped: only the named packages and the subtree each pulls are re-resolved (against the manifest `[requires]` constraint) and re-materialised; everything else holds its lockfile pin. No-arg / `--all` still refresh the whole graph.
- **FU4** — `apply_resolution` prunes `vibedeps/` slots that fall out of the resolution, so a version bump or a dropped dependency leaves no orphan slot; `InstallOutcome.pruned` reports them.
- **FU5** — the vestigial one-enum `vibe-install` crate is folded into `vibe-cli` (`InstallError` now lives in `exit_code.rs`) and removed from the workspace.

`cargo test --workspace` and `cargo clippy --workspace --all-targets -- -D warnings` are green. Still open, now due in Phase 7: the `dynamic`-entry `when` contract gap (Owner attention §5) and the `VIBEVM-SPEC.md` sanction (Owner attention §2).

**Branch state.** `m1.17-workspace` — M1.17 (the shipped workspace milestone — see the earlier checkpoint), the PROP-009 design + contract docs, M1.18 Phases 1–6 (`ce14877` … `0706ae2`; Phase 6 in `4606132` + `0706ae2`), all five Phase-5 follow-ups (`2f42776`, `1a55409`, `6ec47d2`, `b313829`, `85dbc9a`), the DRAFT proposals PROP-010 (`9069f13`) and PROP-011 (`040c8c3`) plus their registration (`987e4d4`), the `docs(wal)` / `docs(continue)` checkpoints, and this session-end checkpoint. **Pushed** to `origin/m1.17-workspace` — the owner authorised push from this point on; not merged to `main`. Working tree clean (`.claude/settings.local.json` is git-ignored). Gate green: `cargo clippy --workspace --all-targets -- -D warnings` clean; `cargo test --workspace` — no `--exclude` needed (the os-740 issue is gone — the `vibe-install` crate no longer exists) — vibe-cli 253 (bin 123, e2e 104, cli_init 11, cli_search 15, 3 ignored), vibe-core 161, vibe-workspace 69, vibe-registry 106 + 5 + 7, vibe-publish 51 + 5, vibe-resolver 48, vibe-check 25, vibe-mcp 22.

**Resolved — the `os error 740` test-environment issue.** `cargo test` built a test-harness binary for `vibe-install` named `vibe_install-<hash>.exe`, and Windows UAC installer-detection refuses to launch any unsigned, unmanifested executable whose name contains `install` — so `cargo test -p vibe-install` (and therefore `--workspace`) failed with `os error 740`. It was never Windows Defender — that was an earlier misdiagnosis, corrected once already. Phase 5 dissolved the problem: `vibe-install` was gutted to a single error enum with no tests (`7347208`), and `[lib] test = false` (`72b87b9`) stops the empty harness binary being built at all. `cargo test --workspace` now runs clean with no exclude. Linux / CI were never affected. (Kept here as the record; it can drop out of the WAL at the next full rewrite.)

**Backlog — parked behind PROP-009.** PROP-007 §9.3's deferred items: workspace-aware `vibe install` in the old framing is **subsumed** by PROP-009 (it is the install half of the loading model); `version = { workspace = true }` member-version inheritance and the publish-signalling polish (`--archive`, `has_issues`, `published_repos`) are **parked** behind M1.18 — recorded, not dropped. PROP-008 (qualified naming) is unchanged — it follows PROP-005 (index) — and the milestone numbering shifts: PROP-009 takes M1.18, PROP-008 moves to M1.19.

**New design proposals — PROP-010, PROP-011.** Two DRAFT proposals from a design discussion this session on `vibe install` cost and offline use — both committed and pushed, neither implementation-ready. **[PROP-010](modules/vibe-registry/PROP-010-local-package-cache.md)** — the local package cache: the registry cache elevated to a machine-global, accretive, identity-keyed store, with a `--offline` policy flag and a user-level default registry configuration, so new modules and new projects resolve their dependencies offline; depends on PROP-008, provisionally M1.20. **[PROP-011](modules/vibe-workspace/PROP-011-incremental-install.md)** — incremental install: skip the depsolver when `vibe.lock` is fresh (so `vibe install` becomes lockfile-respecting) and re-materialise only the changed `vibedeps/` slots, so `vibe install` on a large workspace stops paying whole-tree cost; no dependency beyond shipped PROP-009, M1.21 nominal. Each carries a small set of §5 open questions for an owner design session; both are registered in `ROADMAP.md` and `spec/modules/README.md`.

**Owner attention.** (1) Branch `m1.17-workspace` is pushed to `origin` — the owner authorised push from this point on; merging to `main` remains the owner's call. (2) `VIBEVM-SPEC.md` edits for PROP-009 (§6, §4.2, §4.6, §13.1) need explicit owner sanction — not yet granted; **required now: Phase 7 is the next unit of work and cannot land its `VIBEVM-SPEC.md` edits without it.** (3) `spec/boot/00-core.md` line 38 still reads `package manifest = vibe-package.toml` — stale since M1.17 Phase 1; it is a user-owned boot file vibevm tooling must not edit, so the owner should change it to `vibe.toml`. (4) (carried from 2026-05-12) delete `https://gitverse.ru/vibespecs/vibevm-direct-push-smoke` via the GitVerse web UI (no API DELETE endpoint). Not blocking. (5) PROP-009 §2.3 shows a `when` activation condition on a `dynamic` `INDEX.md` entry, but §2.6 pins no manifest field declaring it; Phase 4's renderer is `when`-ready but leaves it unset. The contract needs a small decision on where a dynamic boot contribution's `when` is declared (likely `[boot_snippet]` or the `[requires.packages]` entry) — best taken alongside the Phase 7 `VIBEVM-SPEC.md` work, now imminent. (6) The two new DRAFT proposals above — [PROP-010](modules/vibe-registry/PROP-010-local-package-cache.md) and [PROP-011](modules/vibe-workspace/PROP-011-incremental-install.md) — need an owner design session to close their §5 open questions before either can be scheduled for implementation.

---

## Earlier checkpoint (kept for context — M1.17 Workspace shipped, 2026-05-21)

**M1.17 — Workspace: Phases 1–5 shipped (2026-05-21).** PROP-007 (multi-package workspaces) implemented across five phases on branch **`m1.17-workspace`** — not yet merged to `main`. Commits `b794e7a..b673d2b` plus the Phase 6 docs commits:

1. **Phase 1 — unified manifest** (`b794e7a`, `9a190ff`). One `vibe.toml` per node replaces `ProjectManifest` + `PackageManifest`; the role is set by section (`[project]` ⊕ `[package]`, `[workspace]`). All manifest legacy deleted — the `vibe-package.toml` filename, `[dependencies]`, array-form `packages`, singleton `[registry]`. ~190 call-sites + 8 fixtures migrated. `VIBEVM-SPEC.md` §7 rewritten.
2. **Phase 2 — workspace model** (`ece30a6`). New `vibe-workspace` crate: `Workspace::discover` bubbles to the absolute root, recursive nesting, glob members, cycle detection. No absolute path is ever persisted — members carry a portable `rel_path`.
3. **Phase 3 — path-source + lockfile v4** (`ff21de3`, `e9a15d2`). `{ path = "../sibling" }` deps; resolver priority `override > path > git > registry`; `vibe.lock` schema v4 (`source_kind = "path"`), legacy v1/v2/v3 readers removed.
4. **Phase 4 — `[workspace.versions]`** (`98795e8`). Named version placeholders; `{ version.var = "core" }`; recursive matryoshka resolution in the workspace loader (nearest enclosing `[workspace.versions]` wins).
5. **Phase 5 — selective publish** (`b673d2b`). `vibe workspace publish` — topological walk of self-publishing members, `[origin]` marker + "contribute upstream" signalling, non-atomic stop-on-first-failure.
6. **Phase 6 — docs.** `VIBEVM-SPEC.md` §4.2 / §7.6, `PROP-007` status, ROADMAP / CHANGELOG, docs sweep, this WAL.

**State.** Branch `m1.17-workspace`, working tree clean (only `.claude/settings.local.json` untracked, pre-existing). Every phase landed clippy-clean (`cargo clippy --workspace --all-targets -- -D warnings`) with its test suite green. Test counts: vibe-core 142, vibe-workspace 24, vibe-registry 106, vibe-cli bin 124 + e2e 111, vibe-publish 51, vibe-resolver 48, vibe-check 25, vibe-mcp 22. `vibe check --path . --quiet` 0/0/0.

**Known environment issue (corrected — see the current phase):** `cargo test -p vibe-install` fails on this machine with `os error 740` — Windows UAC installer detection, not Windows Defender. `vibe-install` was touched this milestone (the `SourceKind::Path` lockfile mapping); its 18 tests pass when run under a binary name without the substring `install`.

**Next — the remaining M1.17 piece.** Wire `vibe install` / `vibe build` to discover the workspace and run unified multi-member resolution (PROP-007 §6 question 3). It is gated on a per-member **materialisation-target** decision PROP-007 §2.4 / §3 leaves open — a genuine spec fork that wants owner input (when a dependency is resolved for member M, which member's `spec/` does its content land in?). The path-source resolver capability it builds on is already implemented and tested. Also deferred: `version = { workspace = true }` member-version inheritance (PROP-007 §6 q4) and the `--archive` publish lockdown. Then: merge `m1.17-workspace` to `main`; M1.18 (PROP-008, qualified naming) follows, after PROP-005 (index).

**Owner attention (M1.17).** Three items want the owner: (1) `spec/boot/00-core.md` line 38 still reads `package manifest = vibe-package.toml` — factually stale after Phase 1, but it is a user-owned boot file vibevm tooling must not edit; the owner should change it to `vibe.toml`. (2) Branch `m1.17-workspace` is local — not pushed to origin, not merged to `main`; it awaits review. (3) The materialisation-target decision (PROP-007 §6 q3) gates workspace-aware `vibe install`.

**Outstanding manual step (owner-only, carried from 2026-05-12):** delete `https://gitverse.ru/vibespecs/vibevm-direct-push-smoke` via the GitVerse web UI (no API DELETE endpoint). Not blocking.

---

## Earlier checkpoint (kept for context — redirect-update + workspace/naming design, 2026-05-20)

**Session-end checkpoint (2026-05-20).** Two slices, both on `main`:

1. **`vibe registry redirect-update` shipped.** Four commits (`f8af587..b44729d`, pushed mid-session) closed the one remaining M1.16 deferred-list item — a CLI command to rewrite an existing redirect stub's `vibe-redirect.toml` in place (retarget via `--to`, switch `--ref-policy`, edit description), replacing the manual `git clone` / edit / push procedure. New `vibe_publish::git_publish::commit_and_push` helper (fast-forward push on an existing clone, refuses an empty commit). Trust model per PROP-002 §2.4.2 — `target_url` / `ref_policy` / `pinned_ref` changes require `--trust-redirect`; operator metadata does not. 15 unit tests on `compute_updated_redirect_section` + helpers, 2 on `commit_and_push`, 4 hermetic e2e on args-level guard rails. New `docs/commands/registry-redirect-update.md`. **The M1.16 deferred-list is now empty.**

2. **Workspace + qualified-naming design session.** Two commits (`ff23a0f`, `4d6775a`) record a multi-fork design discussion with the owner — the largest refactor proposed so far. Produced **PROP-007** ([workspace](modules/vibe-workspace/PROP-007-workspace.md) — multi-package projects, recursive nesting, unified `vibe.toml`, `path`-source, `[workspace.versions]`, selective publish) and **PROP-008** ([qualified naming](modules/vibe-registry/PROP-008-qualified-naming.md) — reverse-FQDN `group`, identity `(group, name, version, content_hash)`, short-name aliases, collision detection), both `DRAFT` — requirements locked, **implementation deliberately deferred to a fresh session**. Also: a new non-normative documentation genre `spec/design/` (genre recorded in `spec/design/README.md`), with the full fork-by-fork lore in `spec/design/workspace-and-qualified-naming.md`. ROADMAP gains M1.17 / M1.18 stubs + an M3+ registry-explorer entry. The owner granted explicit sanction to edit any specification, including the owner-frozen `VIBEVM-SPEC.md`, for this refactor (recorded in the PROP-007/008 headers); the `VIBEVM-SPEC.md` edits land at implementation time, not yet.

**HEAD `4d6775a`.** Working tree clean (only `.claude/settings.local.json` untracked). `cargo clippy --workspace --all-targets -- -D warnings` clean; `vibe check --path . --quiet` 0 errors. Test counts this session: vibe-publish **51 hermetic** (+2 `commit_and_push`); vibe-cli bins **118 hermetic** (+15 redirect-update unit); vibe-cli e2e **101 hermetic** (+4 redirect-update guard-rail).

**Known environment issue (not a code bug):** `cargo test -p vibe-install` — and therefore `cargo test --workspace` — fails on this machine with `os error 740` ("requires elevation"). Windows Defender / Smart App Control blocks the freshly-compiled unsigned `vibe_install-<hash>.exe` test runner; `cargo clean` does not help. The `vibe-install` crate was not touched this session. The owner is resolving the AV side himself. `cargo build -p vibe-install --tests` type-checks cleanly.

**Next session:** implement **M1.17 — Workspace** ([PROP-007](modules/vibe-workspace/PROP-007-workspace.md)). It has no dependency on the index and delivers the bulk of the request. Read `spec/design/workspace-and-qualified-naming.md` first — it carries the design reasoning. M1.18 (PROP-008) follows, but depends on PROP-005 (index) being implemented for short-name resolution.

**Outstanding manual step (owner-only, carried from 2026-05-12):** delete `https://gitverse.ru/vibespecs/vibevm-direct-push-smoke` via the GitVerse web UI (no API DELETE endpoint). Not blocking.

---

## Earlier checkpoint (kept for context — M1.15 + M1.16 ship + test re-home, 2026-05-12)

**Session-end checkpoint (2026-05-12).** The day's work split into three slices, all on `main`, all pushed to `origin/main`:

1. **M1.16 finalisation (2026-05-10).** Seven commits (`5b9a2dc..9b22adb`) closed the M1.16 deferred-list: `vibe registry redirect` + `vibe registry redirect-sync` CLI commands, four hermetic redirect e2e tests, four git-source corner-case e2e tests, two bug fixes (uninstall git-source cleanup; `fetch_manifest_at_ref` archive→clone fall-back on GitHub), and a redirect-aware `MultiRegistryResolver::fetch_manifest` (depsolver path now sees stub-only repos). M1.15 also gained its deferred production smoke walk along the way.

2. **Test-fixture re-homing (2026-05-12).** Commit `dbba8d7` plus the docs catch-up in `4e852f0`. Five GitHub repos + one GitVerse repo migrated out of canonical `vibespecs` + `olegchir` personal namespace into three dedicated test orgs: `vibespecstest1` (GitHub, registry-side fixtures), `vibespecstest2` (GitHub, external-target fixtures), `vibespecstest3` (GitVerse, GitVerse-side fixtures). Migration via `git clone --mirror` + `git push --mirror` for five GitHub repos; `vibe registry publish --repo-url` from a local fixture for the GitVerse leg. The `feat-helper` stub marker rewritten + retagged to point at `vibespecstest2/vibevm-m1-smoke-feat-helper`. `cli_live_e2e.rs` rewritten to overwrite `vibe.toml` after `vibe init` with explicit test-org `[[registry]]` blocks; M1.15 / M1.16 manual-test recipes reprovision via `/orgs/vibespecstest2/repos`. Five old smoke artefacts deleted via GitHub API (`HTTP 204` for all). `github.com/vibespecs` now hosts only real packages: `flow-wal`, `flow-sync-from-code`, `flow-atomic-commits`. All three live e2e tests pass.

3. **Documentation catch-up.** Commits `ad9b8b3` + `9b22adb` + `4e852f0` covered CHANGELOG / ROADMAP / WAL / CONTINUE / `docs/registry-redirect.md` / `docs/commands/registry-redirect{,-sync}.md` and the two new `manual-tests/M1.{15,16}-*-smoke.md` recipes. ROADMAP flips M1.15 + M1.16 to `✅ SHIPPED (2026-05-10)`.

**HEAD `4e852f0`**. Workspace clean (only `.claude/settings.local.json` untracked). `cargo test --workspace` all green; clippy `-D warnings` clean; `vibe check --path . --quiet` 0/0/0. **No active blockers.**

vibe-cli e2e: **97 hermetic + 3 ignored** (was 89; +8). vibe-cli bin: **103 hermetic** (was 93; +10). vibe-registry: **102 hermetic**. vibe-core: **139 hermetic**.

Outstanding manual step (owner-only): delete `https://gitverse.ru/vibespecs/vibevm-direct-push-smoke` via the GitVerse web UI. GitVerse has no API DELETE endpoint vibevm could call; the equivalent GitHub cleanup completed via `curl -X DELETE`.

**Test-org map (live):**

- `https://github.com/vibespecstest1` — `flow-vibevm-github-smoke` (live-e2e GitHub leg), `feat-helper` (M1.16 redirect stub).
- `https://github.com/vibespecstest2` — `vibevm-m1-smoke-flow-internal` (M1.15 target), `vibevm-m1-smoke-feat-helper` (M1.16 target), `vibevm-private-probe` (M1.14.4 private target).
- `https://gitverse.ru/vibespecstest3` — `vibevm-direct-push-smoke` (live-e2e GitVerse leg, SSH-only).

Operational notes carried into this session:

- **GitHub `upload-archive` refusal** is a host policy. Any code path that wants to read a single file from a GitHub repo without cloning must fall back to a shallow clone. Three call sites needed it this session: `fetch_dep_manifest` (already had it), `fetch_manifest_at_ref` (added), `try_fetch_redirect_for_url` (added).
- **`MultiRegistryResolver::fetch_manifest`** is now the canonical depsolver-side manifest read. Pre-this-session DepProvider walked registries directly and missed stub-only repos / pinned redirects / git-source declarations. The new method delegates to `resolve()` and reads from whichever URL the resolution recorded.
- **Pinned-policy redirects** decouple stub-tag from target version. The depsolver pins on target version, but the stub may not have that tag. Fall-back: re-resolve `latest` and verify version match.
- **GitVerse HTTPS-vs-SSH**: canonical `vibespecs` happens to be HTTPS-readable; new orgs are not. Live tests use SSH form (`git@gitverse.ru:vibespecstest3`); operator docs and manual-tests do the same.
- **Token-discipline invariant remains intact** through M1.15, M1.16, and the migration recipe. `grep -r x-access-token ~/.vibe/registries/` empty after every production walk this session. `.git/config` in all redirect / git-source / migrated clones carries plain (credential-free) URLs.

What's deferred:

- **`vibe registry redirect-update`** — editing an existing stub's marker is a manual clone/edit/push procedure for v0. `feat-helper` retargeting in this session was done by hand. ~3-5 commits to deliver a CLI affordance.
- **Pinned-policy bridging in install pipeline** — pure `stub_tag != pinned_ref` case works at the resolver level (FakeBackend hermetic test) but not through the install pipeline. Bridging needs the install pipeline to remember the redirect-discovery rather than re-resolve through `=<version>`.
- **Manual-test recipe for M1.14.4 private-probe** — target migrated to `vibespecstest2/vibevm-private-probe` but no recipe file exists yet. ~150 lines.

---

## Earlier checkpoint (kept for context — M1.16 +1, 2026-05-10)

Out-of-line discovery: GitVerse https requires credentials even for public reads against new orgs (canonical `vibespecs` happens to be publicly readable over https, but a fresh org isn't). `cli_live_e2e` now uses SSH form `git@gitverse.ru:vibespecstest3` for the GitVerse registry URL, matching the operator path documented in `spec/boot/90-user.md`.

What still needs to land: cleanup of old smoke artefacts (delete `vibespecs/feat-helper`, `vibespecs/flow-vibevm-github-smoke`, `olegchir/vibevm-m1-smoke-flow-internal`, `olegchir/vibevm-m1-smoke-feat-helper`, `olegchir/vibevm-private-probe` via GitHub API; ask owner to delete GitVerse counterparts if any are left over). This is the safe-after-migration step — new test orgs verified working before old ones get removed.

**Working checkpoint (2026-05-10 +2, M1.16 ship-complete — CLI helper + redirect-sync + hermetic e2e + production walk).** Seven commits close the M1.16 deferred-list from the +1 checkpoint. The two missing CLI helpers (`vibe registry redirect <pkgref> --to <url>` and `vibe registry redirect-sync <pkgref>`) are implemented and documented; four hermetic e2e tests in `vibe-cli/tests/cli_e2e.rs` cover the install-via-redirect path end-to-end; and a production smoke walk against `vibespecs/feat-helper` (stub) → `olegchir/vibevm-m1-smoke-feat-helper` (target) on real GitHub validates the full path. The same push also closes M1.15's deferred production walk against `olegchir/vibevm-m1-smoke-flow-internal`. Two bug fixes hit along the way: `fetch_manifest_at_ref` now falls back to `refresh_package` on `ArchiveUnsupported` (GitHub case), and `vibe uninstall` correctly drops git-source declarations from `requires.git_packages`. **HEAD `<pending>`**, vibe-cli e2e at **97 hermetic + 3 ignored** (was 89; +8 — 4 git-source corners + 4 redirect cases), vibe-cli bin at **103 hermetic** (was 93; +10 redirect/redirect-sync helper tests), workspace `cargo test --workspace` all green, `cargo clippy --workspace --all-targets -- -D warnings` clean, `vibe check --path . --quiet` reports 0/0/0.

Seven commits land the M1.16 finalisation slice (newest-first):

- `<pending> docs(continue,wal): M1.15 + M1.16 finalisation checkpoint` — this WAL block + `CONTINUE.md` rewrite.
- `<pending> docs(commands,registry-redirect,manual-tests,changelog,roadmap): M1.15 + M1.16 ship reference` — new `docs/commands/registry-redirect{,-sync}.md`, `docs/registry-redirect.md` rewritten with the CLI workflow, `manual-tests/M1.{15,16}-*-smoke.md` recipes, ROADMAP / CHANGELOG flips PROPOSED → SHIPPED.
- `<pending> test(vibe-cli): hermetic e2e for git-source repeats + redirect resolves` — 4 new git-source tests (repeat-install rejection, uninstall removes-from-both-lists, plus a comment explaining why `--rev` is not exercised hermetically) + 4 new redirect tests (pass-through-tag, pinned, identity-mismatch reject, hop-2 chain reject).
- `<pending> feat(vibe-cli): vibe registry redirect + redirect-sync commands` — two new `RegistrySubcommand` variants. `vibe registry redirect` builds stub source dir, runs `RepoCreator::create_repo` against the registry org, pushes via `git_publish::push_initial`. `vibe registry redirect-sync` shallow-clones the stub, reads `vibe-redirect.toml`, ls-remotes both sides, pushes missing target tags onto the stub's marker-file commit. Refuses for pinned-policy stubs (semantically meaningless to sync). Plus 10 helper unit tests.
- `<pending> feat(vibe-publish): publish helpers for stub creation + tag mirroring` — `push_initial` (init + commit + push, no tag), `ls_remote_tags` (with redaction), `push_tag_only` (annotated tag at HEAD + push), `shallow_clone` (depth=1 single-branch=main TempDir). All factored from the existing `push_release` infrastructure with the same `redact_credentials` + `push_with_classification` machinery.
- `<pending> feat(vibe-registry,vibe-resolver): redirect-aware fetch_manifest` — `MultiRegistryResolver::fetch_manifest(kind, name, version)` is the new redirect-aware DepProvider entry point; re-runs `resolve()` to converge on the same `MultiResolution` and reads from target_url for redirect-resolved packages, dep.url for git-source, registry's URL otherwise. `MultiRegistryProvider::fetch_manifest` delegates. Pinned-policy fall-back: when `resolve(=version)` fails because the stub's tag list does not contain the target version (pinned semantics — stub-tag and pinned_ref are decoupled), retry with constraint-free latest and verify the result version matches what the depsolver pinned. Hop-limit check in `follow_redirect` swapped to fire BEFORE manifest fetch — was failing on stub-only target repos at hop-2 because manifest fetch returned `FileNotFoundInRef` first.
- `<pending> fix(vibe-registry): archive→clone fall-back in fetch_manifest_at_ref` — same shape as `fetch_dep_manifest`. Without this, GitHub-hosted git-source / redirect targets failed at resolution time.
- `<pending> fix(vibe-cli/uninstall): drop git-source declarations on uninstall` — `drop_from_manifest_requires` now retains-not on both `requires.packages` and `requires.git_packages`.

Operational notes carried into this slice:

- **`fetch_manifest` is now the canonical depsolver-side manifest read.** Pre-M1.16 the DepProvider walked `MultiRegistryResolver::registries()` directly, which sees only registry-served packages with full `vibe-package.toml` payload. Stub-only repos (M1.16) and `git_packages` declarations (M1.15) were both invisible to the manifest fetch path. The new `MultiRegistryResolver::fetch_manifest` uses `resolve()` as the single source of truth and then reads from whichever URL the resolution recorded. Same change pattern lives in `fetch_with_expected_hash` (already redirect-aware as of M1.16 +1); the depsolver-side now agrees with the install-side on every shape.
- **GitHub archive-protocol refusal applies to both manifest reads and marker probes.** `git archive --remote=https://github.com/...` is refused server-side because GitHub disables `upload-archive` by policy. Two-path read (`fetch_file_at_ref` first; `refresh_package` clone fall-back) now lives in `fetch_manifest_at_ref` AND `try_fetch_redirect_for_url`. After this, the marker probe reads `vibe-redirect.toml` from the working tree of a shallow clone instead of demanding archive support.
- **Production smoke walks remain in the repo as runnable recipes.** `manual-tests/M1.15-git-source-smoke.md` and `manual-tests/M1.16-redirect-smoke.md` are step-by-step for a human to walk before the next release, mirroring the M1.14.4 private-probe shape. Both recipes include cleanup steps that delete the GitHub test repos via the API; either run cleanup or leave the repos as smoke artefacts for re-runs.
- **Token-discipline invariant remains intact** through both M1.15 and M1.16 paths. The redirect smoke walk verified `grep -r x-access-token ~/.vibe/registries/` returns empty after a successful private install — same shape as the M1.14.4 invariant. The newly-introduced clone fall-backs reuse `set_remote_url(.., "origin", plain_url)` post-bootstrap so freshly-cloned `.git/config` carries the plain URL.

What's deferred out of M1.16:

- **Editing an existing stub via the CLI**. `vibe registry redirect` only creates fresh stubs; updating the marker file (e.g. to change `target_url`) is a manual `git clone` / edit / push procedure for v0. Closing this is a separate command (`vibe registry redirect-update <pkgref>`) — not blocked by anything, just not done.
- **Pure pinned-policy semantic in production walks**. The hermetic test `install_via_redirect_pinned_policy_uses_pinned_ref` works against a stub whose tag set equals `{pinned_ref}` because the install-pipeline's pinned re-resolve requires the stub to surface the resolved version. The "stub-tag != pinned_ref" case (operator wants every consumer to resolve to v1.0.0 of target regardless of stub's v9.9.9 tag) is exercised at the resolver level by `resolve_redirect_pinned_uses_pinned_ref` with FakeBackend. Bridging this would need the install pipeline to remember the redirect-discovery, not re-resolve through `=<version>` — a small but invasive refactor.
- **Signed redirect markers** — cryptographic attestation of `target_url` by the org owner's key. PROP-002 §2.4.2 keeps this for v1+.

**Working checkpoint (2026-05-10 +1, M1.16 — registry redirect resolver wired end-to-end).** Three commits land on top of M1.15 to deliver the PROP-002 §2.4.2 contract: a registry org's stub repo carrying `vibe-redirect.toml` redirects the resolver to an external target URL, with full token-discipline preserved through the redirect path. The resolver-side support is fully wired — consumers can `vibe install <pkgref>` against a project whose registry has stubs, and the redirect is followed transparently. Operator-side stub creation (CLI helper `vibe registry redirect <pkgref> --to <url>`) is documented as a manual procedure for v0; CLI tooling is a planned follow-up. **HEAD `3cf3b01`**, vibe-core at **139 hermetic** (was 128; +11 redirect parser tests), vibe-registry at **102 hermetic** (was 98; +4 redirect resolver tests), workspace `cargo test` all green, clippy `-D warnings` clean, `vibe check --path . --quiet` reports 0/0/0.

Three commits land the M1.16 slice (newest-first):

- `<pending> docs(commands,registry-redirect,wal): user-facing redirect reference + checkpoint` — new `docs/registry-redirect.md` (full operator reference: marker file shape, wire grammar, resolver behaviour, tag visibility, identity rules, lockfile shape, two-layer auth, manual stub creation procedure, comparison table); `docs/README.md` index entry; WAL block (this one).
- `6e861ac feat(vibe-registry): MultiRegistryResolver follows vibe-redirect.toml stubs` — resolver detects stub at registry-walk success, follows redirect via `try_fetch_redirect` + `follow_redirect`. New `MultiResolution` fields `redirect_target_auth` / `redirect_target_token_env` carry the redirect's auth declaration through the resolve→fetch boundary. New `fetch_via_redirect` synthesises a target-side `GitPackageRegistry` and clones into `<cache_root>/__redirects__/...`. Hop limit = 1 enforced. Identity check against target's `[package]`. Four hermetic tests: pass-through-tag dispatch, hop-limit-rejection, pinned ref, identity mismatch.
- `b37e1b3 feat(vibe-core,vibe-registry,vibe-install): vibe-redirect.toml parser + via_redirect lockfile field` — new `vibe-core::manifest::redirect` module with `RedirectFile`, `RedirectSection`, `RefPolicy` types. `parse_redirect_bytes` helper. `LockedPackage.via_redirect: Option<String>` and parallel `MultiResolution.via_redirect` / `CachedPackage.via_redirect` fields propagate the stub URL through the install pipeline. 11 unit tests covering all shapes + validation errors.

Operational notes:

- **Resolver dispatch is conservative.** The redirect probe runs only after the registry-walk leg succeeded (the registry returned a tag); a missing-package response from `list_versions` doesn't trigger a redirect probe. This matches PROP-002 §2.4.2's stance that stubs are full-on registry entries with their own tags, not fallback indicators.
- **Two-layer auth preserved.** Stub auth is the registry's `[[registry]] auth`; target auth is `[redirect].auth`. The fetch path synthesises a target-side `GitPackageRegistry` with the redirect's auth, ensuring token-discipline (M1.14 plumbing — `inject_token` + scrub-from-`.git/config`) applies to private targets.
- **Cache layout adds a third tier.** Registry-served clones live at `<cache>/<canonical-url-hash>/packages/...`. Override clones live at `<cache>/__overrides__/<kind>-<name>/clone/`. Git-source clones live at `<cache>/__git_sources__/<kind>-<name>/clone/`. M1.16 adds `<cache>/__redirects__/<kind>-<name>/clone/` so a package that flips between resolution modes (registry / override / git-source / redirect) does not share state across modes.
- **Hop limit = 1 is hard-coded by spec.** Stubs are flat indirection. A redirect chain (stub → stub → real) is rejected with "redirect chain not allowed" at the resolver layer; no operator override exists. If chains ever become useful, that's a future spec change.
- **CLI helper `vibe registry redirect` is the v0 gap.** The resolver works against any properly-formed stub repo; what's missing is a one-liner operator command that creates the stub. The manual procedure (git init / write marker / commit / push / tag) is documented and works. Closing the gap is a small follow-up commit reusing the existing `RepoCreator` infrastructure from `vibe registry publish`.

What still needs to land for full M1.16 ship:

- **`vibe registry redirect <pkgref> --to <url>`** — CLI helper that creates the stub repo automatically (analogous to `vibe registry publish` but commits a `vibe-redirect.toml` instead of package content).
- **`vibe registry redirect-sync <pkgref>`** — convenience tool that mirrors target tags into the stub for ergonomic version gating. Opt-in (operators can equally manage stub tags by hand).
- **Production smoke walk** against a real GitHub stub→target pair. Recipe shape analogous to M1.14.4's private-probe walk; deferred to when an operator session has the appropriate token loaded.
- **e2e CLI test** in `vibe-cli/tests/cli_e2e.rs` exercising the redirect path end-to-end through a real shell invocation. Hermetic resolver tests already cover the dispatch; an e2e test would lock in the install→lockfile shape across a real CLI run.

**Working checkpoint (2026-05-10, M1.15 — `[requires.packages]` table-form schema + git-source dispatch end-to-end).** Six implementation commits land the M1.15 spec from PROP-002 §2.4.1. The schema, single-package registry constructor, resolver dispatch, lockfile field, CLI wiring, and CLI flags are all in place; the workspace builds clean, every existing test passes, two new resolver hermetic tests + 12 new schema-parser tests cover the new surfaces. Production smoke walk (against a real GitHub repo) and full doc set follow in the next session-end.

Six commits land the slice (newest-first; on top of the two PROPOSED spec commits from yesterday):

- `<pending> docs(commands,git-source,readme): user-facing reference for git-source declarations` — new `docs/git-source-dependencies.md`, `docs/commands/install.md` flag-table extension, `docs/README.md` index entry. WAL block (this one).
- `90bf10b feat(vibe-cli): vibe install --git/--tag/--branch/--rev for git-source declarations` — Cargo-shape CLI affordance for adding a git-source dep without hand-editing `vibe.toml`. New `--git <URL>`, `--tag/--branch/--rev`, `--git-auth`, `--git-token-env` flags on `InstallArgs`. New `apply_git_source_flag` helper validates flag combinations, builds `GitPackageDep`, persists the manifest before resolving. `merge_manifest_requires` extended to skip CLI roots already declared as git-source (avoids `(kind, name)` duplicate that the parser would reject).
- `a7dce7f feat(vibe-core,vibe-registry,vibe-install): lockfile source_kind field for git/override discriminant` — new `SourceKind` enum (`Registry` / `Git` / `Override`) on `LockedPackage`, derived from `cached.overridden` / `cached.is_git_source`. `CachedPackage.is_git_source` propagates through five construction sites in vibe-registry. Wire-compatible — `Option<SourceKind>` defaults to `None` for pre-M1.15 lockfiles.
- `153f3a2 feat(vibe-cli): wire git-source declarations through install/update/outdated` — three `MultiRegistryResolver::open` call-sites chain `.with_git_packages(manifest.requires.git_packages.clone())`. `install::run` roots derivation combines `requires.packages` + `requires.git_packages` into one `Vec<PackageRef>`.
- `161b7b1 feat(vibe-registry): MultiRegistryResolver dispatches to git-source declarations` — resolver short-circuits the registry walk for any pkgref in `git_packages` map. New `resolve_git_source` synthesises a single-package registry, fetches manifest at the declared ref via `fetch_manifest_at_ref` (tag/branch/rev), verifies `(kind, name)` and optional `version` constraint. New `fetch_git_source` mirrors `fetch_override` but threads `dep.auth`/`dep.token_env` through M1.14 token-injection + scrub plumbing. `MultiResolution.is_git_source: bool` discriminates downstream. Two new hermetic resolver tests (`resolve_dispatches_to_git_source_short_circuiting_registries`, `resolve_git_source_rejects_kind_name_mismatch`).
- `c313ebd feat(vibe-registry): GitPackageRegistry::open_single_package for git-source` — new constructor that wraps `open_with_auth` and flips a `single_package_url: Option<String>` field. `package_repo_url` / `package_urls` consult the field and return the URL verbatim instead of applying `naming` to compose `<org>/<kind>-<name>.git`. New `is_single_package() -> bool` predicate. Two unit tests.
- `2544d76 feat(vibe-core): [requires.packages] table-form schema with git-source slot` — schema bumps. New `GitPackageDep`, `GitRefKind` types. `Requires.packages` keeps `Vec<PackageRef>` for back-compat (~40 downstream call-sites untouched); new `git_packages: Vec<GitPackageDep>` field stores git-source declarations separately. Custom Deserialize accepts both legacy array-of-strings shape (M1.13) and modern map shape (M1.15) — manual `Visitor` for clean inner-error propagation. Round-trip writes the modern map form. New `Error::BadDependencyDecl` variant. 12 new tests covering tag/branch/rev variants, auth, version-constraint, missing-ref / multiple-refs / `@`-in-key validation, full round-trip.

Workspace state at HEAD `90bf10b`:

- vibe-core: **128 hermetic** (was 116; +12 git-source schema tests).
- vibe-registry: **98 hermetic** (was 94; +2 single-package constructor + 2 resolver dispatch).
- vibe-install: **22 hermetic** (unchanged in count; +`source_kind` field touched 3 test fixtures).
- vibe-cli e2e: **89 hermetic + 3 ignored** (unchanged; one fixture string updated for new `[requires.packages]` map-form output).
- vibe-cli bin: **93 hermetic** (unchanged).
- `cargo test --workspace` all green; `cargo clippy --workspace --all-targets -- -D warnings` clean; `vibe check --path . --quiet` reports 0/0/0.

Operational notes:

- **Wire-form back-compat is dual-direction.** Legacy `packages = ["flow:wal@^0.3"]` array still parses for any vibe.toml file produced before M1.15. Round-trip writes the modern map form. Both shapes are read forever; only the map form is written.
- **`(kind, name)` collision rejected.** A pkgref cannot appear simultaneously in `packages` (registry-resolved) and `git_packages` (git-source). TOML's no-duplicate-keys grammar already enforces this through the wire form; the `TryFrom<RequiresWire>` validation is defence-in-depth for any future Vec-based wire form.
- **Resolution priority: override > git-source > registry.** The order matches Cargo's `[patch] foo` overriding `[dependencies] foo = { git = "..." }` overriding `[dependencies] foo = "*"`. The git-source layer is the *primary declaration* (long-lived architecture); override is a *patch* (short-lived fix).
- **`#[error(transparent)]` chain-walk quirk reused.** The structured-error envelope from M1.14.4 already documented that `cause.downcast_ref::<DepProviderError>()` does not propagate through `#[error(transparent)]` wrappers; the new git-source error path goes through the same `RegistryError::MalformedMeta` channel and inherits the manual destructure-on-`SolveError::Provider` plumbing.
- **Token-discipline preserved.** `fetch_git_source` synthesises a single-package `GitPackageRegistry` to leverage its `credentialed_url` plumbing for token injection, then immediately calls `set_remote_url(.., "origin", plain_url)` after `ensure_clone_at` to scrub the token from the freshly-bootstrapped `.git/config`. Same M1.14 contract; same hard invariant ("no token bytes on disk").

What still needs to land (planned for the next session):

- **Production smoke walk** against a real GitHub repo as a git-source target. Verify (a) install succeeds with `tag = "v..."`, (b) lockfile records `source_kind = "git"` + correct `source_url`, (c) `grep -r x-access-token ~/.vibe/registries/` empty if `auth = "token-env"`, (d) re-run is `unchanged`. Recipe analogous to M1.14.4's private-probe walk.
- **Branch-resolve test** — exercise `branch = "main"` end-to-end, verify `vibe install` sticks to lockfile commit, `vibe update` walks HEAD.
- **Hermetic e2e test** in `vibe-cli/tests/cli_e2e.rs` covering the `vibe install <pkgref> --git ... --tag ...` happy path — currently the wiring is exercised through unit tests at the resolver layer; an end-to-end CLI test would lock in the manifest+lockfile state across a real shell invocation.
- **VIBEVM-SPEC.md §7 update** if the wire-form or terminology shifts after the smoke walk.

**Working checkpoint (2026-05-09, M1.14.4 — production walk against a live private GitHub repo + the last three deferred-list items closed).** This is the slice that takes M1.14 from "all the moving parts pass hermetic tests" to "validated end-to-end against a real private vibevm package on a real GitHub org." The walk produced one operationally-significant insight, three small UX closers, and a new diagnostic command. **HEAD `<pending>`**, vibe-core at **116 hermetic** (was 115; +1 inline-kv comment preservation test), workspace `cargo test --workspace` all green, `cargo clippy --workspace --all-targets -- -D warnings` clean, `vibe check --path . --quiet` reports 0/0/0.

Production-walk procedure (kept here for the next time we need to validate a real-network change):

1. Created a minimal test repo `olegchir/vibevm-private-probe` (private) on GitHub with a `vibe-package.toml`, `boot/10-flow-private-probe.md`, `spec/flows/private-probe/PROBE.md`, tagged `v0.1.0`.
2. Configured a fresh consumer project's `vibe.toml` with `[[registry]] auth = "token-env"` pointing at `olegchir`.
3. Ran the cycle: `vibe registry test` (no token) → expect `missing-token`; `export VIBEVM_REGISTRY_TOKEN_GITHUB_COM=...` → `vibe registry test` → expect `reachable`; `vibe install flow:vibevm-private-probe` → expect a clean install with the file materialised under `spec/flows/private-probe/PROBE.md`; `grep -r x-access-token ~/.vibe/registries/` → expect zero hits (token-discipline invariant); inspect `.git/config` of the cloned bucket → URL must be the plain credential-free form. All five steps passed.

Three commits land the closing slice (newest-first, planned):

- `<pending> docs(commands,registry-auth,readme,wal): registry-test reference + JSON error doc` — new `docs/commands/registry-test.md` (full reference for the diagnostic command — usage, flags, human + JSON output shapes, exit codes, "how it works"); `docs/registry-auth.md` gains a "Diagnosing reachability before an install" section pointing at `vibe registry test` and a "Machine-readable resolution failures" section documenting the new `error_kind` / `package` / `attempts` JSON envelope; `docs/README.md` index gets the new command row. WAL block (this one).
- `<pending> feat(vibe-cli): vibe registry test diagnostic command` — read-only probe of every `[[registry]]` via a single `git ls-remote` (using `MultiRegistryResolver` with a guaranteed-not-to-exist pkgref). Classifies status as `reachable` / `auth-required` / `missing-token` / `unreachable`. Aligned 4-column table in human mode; structured `{ ok, command, summary, registries[] }` envelope in `--json` mode; `vibe registry test: <ok>/<total> reachable` one-liner in `--quiet` mode. Exit code is non-zero on any non-reachable registry — clean precondition gate for CI. Token discipline matches `vibe install` (read once, in-memory, never on disk).
- `<pending> feat(resolver,registry,cli): structured per-registry attempts in JSON error envelope` — `RegistryWalkAttempt` and `WalkAttemptStatus` made public with `serde::Serialize` (kebab-case discriminant). `RegistryError::PackageNotFoundEverywhere` gains an `attempts: Vec<RegistryWalkAttempt>` field alongside the existing `summary: String`. `DepProviderError::AggregateNotFound { kind, name, summary, attempts }` carries it through the resolver chain (replacing the lossy `Other(string)` fall-back). `vibe-cli/src/output.rs::stamp_structured_error` walks the anyhow chain (manually destructures `SolveError::Provider(d)` because `#[error(transparent)]` doesn't propagate `downcast_ref` into deeper indices) and stamps `error_kind: "package_not_found_everywhere"`, `package: { kind, name }`, and `attempts: [...]` onto the JSON envelope. The legacy single-line `error` field is preserved for backward compatibility.
- `<pending> feat(vibe-core): preserve inline-key comments inside vibe.toml writes` — closes the M1.14.2 deferred corner: comments **inside** an `[[registry]]` block (between two field lines like `name = "x"` and `url = "..."`) now survive a manifest rewrite. New `copy_inline_kv_decor` walks each (key, Item::Value) pair and clones BOTH `Key.leaf_decor` (carries between-key comments up to the `=`) AND `Value.decor` (carries post-`=` and same-line trailing comments) — they are stored on different parents in toml_edit, so cloning only one is insufficient. Invoked for both `Item::Table` and `Item::ArrayOfTables` branches of `merge_preserving_comments`. One new unit test: `inline_kv_comments_survive_inside_array_of_tables`.

Operational notes:

- **Token-discipline invariant is now end-to-end-verified, not just unit-verified.** The hermetic tests of M1.14 confirmed that `set_remote_url` rewrites the recorded URL after bootstrap; the production walk additionally confirmed that on the actual filesystem, `~/.vibe/registries/<hash>/packages/flow-vibevm-private-probe/clone/.git/config` carries no token bytes after a successful private install. This is now the canonical smoke recipe; rerun whenever the auth pipeline is touched.
- **`vibe registry test` is the cheapest CI gate for "are my tokens set right?"** Single `git ls-remote` per registry, ~50ms each. Call it before `vibe install` in non-trivial pipelines; the structured exit code lets you fail the job at the right step rather than letting a downstream resolver error mislead the operator.
- **`#[error(transparent)]` quirk under anyhow.** The chain walk in `stamp_structured_error` cannot rely on `cause.downcast_ref::<DepProviderError>()` finding the inner type at any depth — the transparent wrapper at `SolveError::Provider(...)` makes anyhow's chain stop at `SolveError`. The fix is to `downcast_ref::<SolveError>()` and pattern-match the variant explicitly. Same shape will apply to any future structured-error work that crosses a transparent error wrapper.
- **The M1.14 deferred-list is now empty.** Aggregated per-registry error report (M1.14.2): ✅ structured-JSON form (M1.14.4). Comment-preserving writes (M1.14.2): ✅ + inline-kv preservation (M1.14.4). `--auth-required` strict gate (M1.14.2): ✅. New `vibe registry test` diagnostic (M1.14.4): ✅. Production walk against live private GitHub repo (M1.14.4): ✅. Full registry-auth surface is feature-complete for v0.

Test-repo housekeeping: `olegchir/vibevm-private-probe` is left up as a permanent smoke artefact for re-running the production walk on future auth-pipeline changes. Delete via GitHub API if you want it gone (`gh api -X DELETE repos/olegchir/vibevm-private-probe`); recreate from `manual-tests/` recipe (to be added if the repo ever needs to be reproduced).

**Session-end checkpoint (2026-05-08).** The day closed M1.12 + M1.13 + M1.14 (with three half-step closers .1 / .2 / .3) across 25 commits. Workspace is at HEAD `8ab5c9c`, working tree clean, `cargo test --workspace` all green, clippy `-D warnings` clean, `vibe check --path . --quiet` 0/0/0. No active blockers. See `CONTINUE.md` at the repo root for the cold-resume snapshot — exhaustive non-obvious findings, per-crate file map, repo-wide policy reminders, six "what to do first" options, full commit chain. The blocks below remain the canonical living history.

**Working checkpoint (2026-05-08 +2, M1.14.3 — surface consistency: MCP `--yes` actually wired, `--auth-required` reach extends to `update` + `outdated`, `--exact` extends to `update`).** Closes the four CLI-surface consistency gaps surfaced by the audit after M1.14.2 landed: (a) `--yes` on `mcp install/upgrade/uninstall` was a vestigial flag that never gated anything; (b) `--auth-required` only existed on `vibe install`, not `vibe update` / `vibe outdated`; (c) `--exact` only existed on `vibe install`, not `vibe update` (cargo has the equivalent as `cargo update --precise X.Y.Z`); (d) MCP commands accepted `--yes` but not `--assume-yes`, splitting the operator's mental model from the package commands. **HEAD `<pending>`**, vibe-cli e2e at **89 hermetic + 3 ignored** (no count change — the existing tests exercise the new code paths through their existing flags), workspace `cargo test --workspace` all green, `cargo clippy --workspace --all-targets -- -D warnings` clean, `vibe check --path . --quiet` reports 0/0/0.

Three commits land the slice (newest-first, planned):

- `<pending> docs(commands,wal): surface-consistency closing slice` — `commands/mcp-install.md`, `mcp-upgrade.md`, `mcp-uninstall.md` flags tables now describe the TTY-only confirm policy + `--assume-yes` alias; `commands/update.md` gains `--exact` and `--auth-required` rows. WAL block (this one).
- `<pending> feat(vibe-cli): --auth-required + --exact reach to update + outdated` — `UpdateArgs` and `OutdatedArgs` accept `--auth-required`; both pass it through `MultiRegistryResolver::open(...).with_strict_auth(args.auth_required)`. `UpdateArgs` additionally gains `--exact`: after a successful apply, walks `manifest.requires.packages` and tightens each updated root's constraint to `=<resolved-version>` before persisting. `vibe-cli/src/commands/update.rs::run` flips `manifest` to `mut` and writes only on actual diff. The flag is no-op when no plans landed (already up-to-date) — symmetric with `vibe install --exact`.
- `<pending> feat(vibe-cli/mcp): wire --yes to apply-confirm prompt + --assume-yes alias` — `--yes` on `mcp install/upgrade/uninstall` was previously a declared-but-unread flag (clap accepted it; `args.yes` was never consulted). This commit makes it functional. Three new helpers (`walk_install`, `walk_upgrade`, `walk_uninstall`) extract the per-(agent × scope) inner loop so `run_install/upgrade/uninstall` can call it twice — first as `dry_run = true` to gather the plan, then (after the operator approves) as `dry_run = false` to actually write. The confirm prompt is **TTY-gated**: skipped when `args.yes`, `--auto`, `--unattended` / `VIBE_UNATTENDED`, `--json`, OR when stdin is not a TTY (CI / opencode harness — pre-this-commit behaviour for those callers preserved). Operators on a real TTY without a skip-flag now get an interactive `[y/N]` summary before any MCP-config / SKILL.md write. The three `pub yes: bool` declarations gain `alias = "assume-yes"` so package-command muscle memory transfers.

Operational notes:

- **Backward compatibility for non-TTY scripts.** Pre-existing CI / opencode workflows that called `vibe mcp install --agent X --scope Y --what Z` (without `--yes`) continue to work. The TTY-gate condition (`!console::user_attended()` short-circuits to "approved") preserves that. `--yes` is the documented way to skip the prompt **on a TTY**; the env-var-driven `--unattended` is the cleaner path for "I am scripting this regardless of TTY status."
- **MCP commands now do real two-pass walks.** Slight perf cost: every `install`/`upgrade`/`uninstall` now runs the walk twice (once dry, once apply) when there are pending changes. The walk is in-memory diff vs disk reads; ~10–50 ms total even with five agents. Acceptable cost for the safety win on `mcp uninstall --scope both`.
- **`--exact` on update is cargo's `cargo update --precise X.Y.Z` shape.** Cargo separates the verbs: `cargo update` re-resolves and bumps the lockfile; `cargo update --precise X.Y.Z` additionally tightens the manifest. We collapse the two into `vibe update --exact` for symmetry with `vibe install --exact`. The non-`--exact` path of `vibe update` does not touch `vibe.toml` — only the lockfile, mirroring cargo's default behaviour.
- **`--auth-required` reaches `outdated` even though it is read-only** because the same fall-through logic applies: a 401 from a private registry that's been re-classified as `UnknownPackage` would silently miss "yes, the new version is here" answers. CI gating on `vibe outdated --auth-required --json` lets monitoring pipelines distinguish "no updates" from "private registry unavailable."

**Working checkpoint (2026-05-08 +1, M1.14.2 — `--auth-required` strict gate, aggregated per-registry error report, comment-preserving `vibe.toml` writes).** Three deferred enhancements from the M1.14-final WAL ("out of M1.14" list) all land in this slice. Three commits, all on top of the M1.14 production-ready runtime; together they close the deferred-list to zero and constitute the final UX polish on the registry-auth surface. **HEAD `<pending>`**, vibe-core at **115 hermetic** (was 110; +5 toml_edit merge tests), vibe-registry at **94 hermetic** (was 93; +2: `resolve_strict_auth_halts_on_public_401_instead_of_walking` and the renamed `resolve_aggregates_walk_attempts_when_no_registry_has_it` covering both the strict-auth halt and the new `PackageNotFoundEverywhere` aggregate-report shape — the latter replaces an existing test rather than adding a new one), workspace `cargo test --workspace` all green, `cargo clippy --workspace --all-targets -- -D warnings` clean, `vibe check --path . --quiet` reports 0/0/0.

Three commits land the closing slice (newest-first):

- `<pending> docs(commands,wal): document closing-slice landings` — `docs/commands/install.md` flags table grows the `--auth-required` row; `docs/registry-auth.md` gains a "Strict-auth posture" section with CI / env-var examples; WAL block (this one).
- `<pending> feat(vibe-core): toml_edit-based comment-preserving writes for vibe.toml` — `vibe-core::manifest::write_toml` now layers `toml_edit::DocumentMut` on top of the existing serde-driven render path. Three layers of decoration (document-level prefix, per-table prefix, document-level trailing) are copied from the existing-file representation onto the freshly-rendered one before save. `[[registry]]`-shaped arrays of tables get per-element prefix preserved up to the shorter of the two arrays — strict index-pairing is the simplest defensible approximation. Falls back to the unmerged rendering on any parse failure (worst-case = prior behaviour, so the change strictly improves UX). 5 new unit tests (`header_comments_survive_full_rewrite`, `pre_table_comments_survive_for_unchanged_sections`, `trailing_comments_survive`, plus 2 fall-back-on-malformed-input tests). Workspace gains `toml_edit = "0.23"` as a workspace dep.
- `<pending> feat(vibe-registry,vibe-cli): --auth-required + aggregated per-registry error report` — combined slice for the two remaining auth UX wins from the M1.14 deferred list. `MultiRegistryResolver::with_strict_auth(bool)` flips the public-401 walk-past behaviour from §2.3.1 default to halt; `vibe install --auth-required` plumbs through. `RegistryError::PackageNotFoundEverywhere { kind, name, summary }` carries a pre-formatted multi-line per-registry report (registry name, URL, auth regime, outcome) — `Display` renders it inline so the standard `error: ...` chain shows operators exactly what each configured registry said about the missing package, with a hint pointing at `auth = "token-env"` if any registry returned a walked-past 401. Renamed `resolve_unknown_package_when_no_registry_has_it` to `resolve_aggregates_walk_attempts_when_no_registry_has_it` and updated the assertion to match the new shape; old simpler `UnknownPackage` variant is preserved for the no-registries-configured path so downstream pattern-matchers still compile. Two new tests (the strict-auth halt + the aggregate-report content check).

Operational notes:

- **Strict-auth is opt-in.** Default behaviour (without `--auth-required` and without `VIBEVM_GIT_SILENCE_HELPERS` overrides) is unchanged from M1.14 — public-401 walks past, authenticated-401 halts. The flag exists for the narrow class of CI runs where a fallback to a public substitute would be wrong.
- **`PackageNotFoundEverywhere` flows through the DepProvider chain via `Other(string)`.** `multi_registry_provider::resolve_version` already had a generic `Err(other) => Err(DepProviderError::Other(other.to_string()))` fall-through; the new variant's multi-line `Display` rides through that path unchanged. No cross-crate API churn was needed; downstream `vibe-cli/install.rs` sees the multi-line message in the standard error chain.
- **Comment preservation is best-effort.** Inline comments inside an `[[registry]]` block (between `name = ...` and `url = ...` for example) are not preserved across writes — only **prefix** comments on the table line itself, plus document header / trailing. Operators wanting full inline-comment preservation should hand-edit `vibe.toml` instead of using `vibe registry add`. The 80%-case (header at top, comments above each `[[registry]]` block, footer notes) is fully covered.
- **Aggregate-report is text-mode-only today.** The structured `attempts` are pre-formatted into a `summary: String` at error-construction time; JSON envelope still flows through `DepProviderError::Other(string)` rather than carrying the structured array. JSON-aware aggregation would require new variants in the DepProvider error chain — left as a small future follow-up if anyone needs to programmatically inspect per-registry status.

**M1.14 deferred-list status:** all three closed.

  | Item | Status |
  | --- | --- |
  | Aggregated per-registry error report | ✅ this commit |
  | `toml_edit`-based comment-preserving writes | ✅ this commit |
  | `--auth-required` flag for strict CI gating | ✅ this commit |

The registry-auth surface is now feature-complete for v0. Next surface to refine is independent: comment-preserving extends naturally to mirror / override blocks if a future case asks; per-element comment preservation inside arrays of tables is a corner-case enhancement; structured-attempts in JSON envelope is the same.

**Working checkpoint (2026-05-08 final, M1.14 — full registry-auth runtime: token injection, 401 classification, walk-vs-halt, production-ready private registries).** First half of M1.14 (committed earlier today as `5f296d9..41efc0c`) landed the spec contract, the schema (`AuthKind` + `RegistrySection.auth/token_env`), the `vibe registry add --auth --token-env` CLI flags, and TTY-aware credential-helper silencing in `apply_common_env`. Second half (this checkpoint) plumbs the rest of PROP-002 §2.2.1 / §2.3.1 end-to-end so `auth = "token-env"` actually authenticates fetches at runtime, `MissingToken` surfaces before any git invocation, 401 / 403 walk past public registries but halt against authenticated ones, and the token never persists on disk inside the cloned `.git/config`. **HEAD `<pending>`**, vibe-registry at **93 hermetic + 0 ignored** (was 81; +12: 2 classifier, 4 inject_token + 1 host-extraction, 1 MissingToken precheck, 1 bootstrap-with-scrub, 3 resolver walk-vs-halt), workspace-wide `cargo test --workspace` all green, `cargo clippy --workspace --all-targets -- -D warnings` clean, `vibe check --path . --quiet` reports 0/0/0.

Five commits land the second half (newest-first, planned):

- `<pending> docs(registry-auth,wal): user-facing reference + checkpoint` — `docs/registry-auth.md` covers the four regimes, env-var conventions, the walk-vs-halt matrix, token-discipline checks, troubleshooting; `docs/README.md` index gains a "Registry authentication" section. WAL block (this one).
- `<pending> feat(vibe-registry,vibe-resolver): per-auth walk-vs-halt rules in MultiRegistryResolver` — `MultiRegistryResolver::resolve` matches on `RegistryError::Git(GitError::AuthFailed)` and consults `reg.auth_kind()`: `None` → reclassify as `UnknownPackage` and walk; any other regime → propagate the halt unchanged. `MissingToken` is propagated unchanged for any registry — silently walking past would mask a setup mistake. `from_manifest` plumbs `RegistrySection::resolve_token_env_name()` (or the explicit `token_env`) into `GitPackageRegistry::open_with_auth`. 3 new tests: `resolve_walks_past_auth_failed_when_registry_is_public`, `resolve_halts_on_auth_failed_against_authenticated_registry`, `resolve_halts_on_missing_token_for_authenticated_registry`.
- `<pending> feat(vibe-registry): token injection + bootstrap-with-scrub for auth=token-env` — `RegistryError::MissingToken { registry, env_var }` variant for the precheck-before-spawn case; `inject_token(plain_url, token)` helper applies `https://x-access-token:<TOKEN>@host` shape only to https URLs that aren't already credentialed; `GitPackageRegistry` gains `auth: AuthKind` + `effective_token: Option<String>` + `token_env_name: Option<String>` fields, `open_with_auth` resolves the env-var at construction time, `open_with_explicit_token` is the test-only constructor that takes a resolved token directly (avoids the `unsafe`-blocking `set_var` problem under Rust 2024+'s `forbid(unsafe_code)`); `ensure_token_loaded` short-circuits with `MissingToken` before any git invocation; `list_versions` / `fetch_dep_manifest` / `fetch_with_expected_hash` all call `ensure_token_loaded()?` then capture the token into the closure for `inject_token` on the `&url` parameter. The bootstrap path adds a critical token-discipline step: after `backend.bootstrap(credentialed_url, ...)` succeeds, `backend.set_remote_url(clone_dir, "origin", plain_url)` immediately rewrites the recorded origin URL to the credential-free form, so the freshly-cloned `.git/config` does NOT carry the token on disk. 7 new unit tests (4 inject_token edge cases, 1 host extraction, 1 MissingToken precheck, 1 end-to-end token-injection-and-scrub through the bootstrap path).
- `<pending> feat(vibe-registry): GitBackend::set_remote_url + ShellGit impl` — new method on the `GitBackend` trait wired through `git -C <dest> remote set-url <remote> <url>`. Default impl provided as `Ok(())` so non-shell test backends don't need to stub it explicitly. Used by the bootstrap-scrub flow above to keep tokens out of persistent `.git/config`.
- `<pending> feat(vibe-registry): classify credential-prompt + http-status patterns as AuthFailed` — the original opencode walk's stderr (`fatal: User cancelled dialog.\nfatal: could not read Username for ...`) now classifies as `GitError::AuthFailed` instead of falling through to `CommandFailed`. New patterns: `"could not read username"`, `"could not read password"`, `"user cancelled dialog"`, `"http 401"`, `"http 403"`, `"401 unauthorized"`, `"403 forbidden"`. Two new tests: `classify_credential_prompt_failure_after_silencing` (the verbatim output we saw against GitVerse with our credential helpers silenced), `classify_http_status_codes` (the proxy / CI-runner-direct paths).

Architectural notes carried into M1.14:

- **Token never lives on disk** through any vibevm-controlled persistence path. Read once from env at registry-open; held in memory in `GitPackageRegistry::effective_token`; injected into per-package URLs only at git-invocation time; scrubbed out of `.git/config` immediately after the clone via `set_remote_url(.., "origin", plain_url)`. The `cargo test` walk confirms the URL recorded post-bootstrap is the plain (token-free) form. Subsequent `update` calls hit the plain origin — if that returns 401 (still-private host), `ensure_clone_against_sources` wipes the clone and re-bootstraps with a fresh credentialed URL. Slight perf cost on stale-cache-against-private-host paths, accepted in exchange for "no token bytes on disk" as a hard invariant.
- **`MissingToken` is a halt, not a walk.** PROP-002 §2.3.1 is explicit on this: walking past a missing-token registry would silently downgrade a private declaration to "not present here", which masks the operator's setup mistake. `MultiRegistryResolver::resolve` propagates `MissingToken` unchanged from `reg.resolve()`; only `Git(AuthFailed)` on `auth = None` triggers the walk-past behaviour.
- **`AuthFailed` on `auth = None` is the GitVerse-fix path.** GitVerse returns 401 for missing public repos as a security-through-obscurity policy. With public-401-as-walk, the resolver moves past GitVerse to the next registry (typically GitHub which returns clean 404), and the install completes normally with `UnknownPackage` if neither host has the package. This is the closure of the original opencode + glm-flash walk that surfaced the GCM popup — the popup itself was killed in the first half of M1.14 by the silencing, but the underlying classification problem only fully closes here.
- **Test-only constructor (`open_with_explicit_token`) for env-write-free unit tests.** Rust 2024+ marks `std::env::set_var` `unsafe`, and vibe-registry has `#![forbid(unsafe_code)]` at the crate level. Production code reads the env-var via the regular `open_with_auth`; tests construct registries with the resolved token in hand. Same shape as `vibe-publish`'s test plumbing for its own publish-token env-var.
- **Default-impl on `GitBackend::set_remote_url`** makes the trait change source-compatible with every existing test backend (the multi-registry-resolver `FakeBackend` does not stub it; the production `ShellGit` overrides). Adding a method to a public trait without breaking downstream test fixtures is exactly the kind of compatibility hygiene PROP-000 §17 talks about (production architecture in prototype).

Out of M1.14 (deferred): aggregated per-registry error report on full resolution failure (currently the resolver returns the last `UnknownPackage` or the first non-walking error; an "I tried these registries and here's what each said" report is a UX win that lands as a follow-up against `vibe-cli`'s install error formatting). Comment-preserving `vibe.toml` writes around `auth = ...` (current `toml = "0.9"` round-trip preserves field values but discards comments — `toml_edit` migration is its own slice). `--auth-required` flag on `vibe install` (refuses to fall through 401 for any registry) — useful for CI gating private installs, fits naturally on top.

**Working checkpoint (2026-05-08 mid, M1.14 first half — `[[registry]] auth` schema + TTY-aware silencing).** Earlier today's serie (`5f296d9..41efc0c`):

- `5f296d9 docs(spec): per-registry auth axis (PROP-002 §2.2.1) + 401 classifier rules` — spec contract for the `auth` axis (none / token-env / credential-helper / ssh), the four-cell silencing matrix, `auth`-aware 401 classification.
- `97753f7 feat(vibe-core): AuthKind enum + RegistrySection.auth/token_env` — schema half. `AuthKind` enum (kebab-case wire form, default `none`), `auth` + `token_env` fields on `RegistrySection`, `resolve_token_env_name()` helper that derives the default env-var name from the registry's host. 8 unit tests round-tripping every shape and back-compat-parsing legacy manifests.
- `e65c73e feat(vibe-cli): --auth and --token-env on vibe registry add` — CLI flags so an authenticated registry can be added without hand-editing `vibe.toml`. Validation rejects `--token-env` paired with anything other than `--auth token-env`.
- `41efc0c feat(vibe-registry): TTY-aware credential helper silencing` — `apply_common_env` in `git_backend/shell.rs` now silences GCM / `credential.helper` / `core.askPass` in non-TTY / `--unattended` runs. The original GCM-popup-in-opencode case is closed here. Subordinate fix: every `ShellGit` method calls `apply_common_env(&mut cmd)` BEFORE `cmd.args(args)` so the silencing-layer `-c` flags land before the subcommand name.

Together with the second half (above) M1.14 closes the registry-authentication story end-to-end. Public registries: never prompt, never popup, walk past 401. Private registries: declare `auth = "token-env"` with an env-var, vibe injects, scrubs, classifies failures; or use `credential-helper` for interactive corporate SSO; or use `ssh` for ssh-agent. The full operator-facing reference lives at `docs/registry-auth.md`.

**Working checkpoint (2026-05-08 late, M1.13 — Cargo-shape version constraints: caret default + `--exact` flag).** M1.12 plumbed `[requires]` end-to-end but recorded pkgrefs verbatim — `vibe install flow:wal` (no version) wrote `"flow:wal"` with `VersionSpec::Latest`, which meant every subsequent `vibe install` / `vibe update` could potentially pull a breaking-change major. Out of step with cargo / npm / Poetry / Bundler — they all resolve at install time and write a caret constraint, so the manifest pins to a known-compatible range. M1.13 brings vibevm in line with that convention and also drops the bare-semver-as-exact parser quirk in favour of the Cargo shorthand (bare `0.3.0` ≡ `^0.3.0`; use `=0.3.0` for strict equal). **HEAD `<pending>`**, vibe-cli at **86 hermetic + 3 ignored** in the `vibe` bin (was 80; +6 unit on `finalize_pkgref_for_manifest`), vibe-cli e2e at **85 hermetic + 3 ignored** (was 83; +3 e2e: caret default / explicit preservation / `--exact`), vibe-core at **102 hermetic** (was 99; +3 on bare-semver-caret + tilde + eq across `package_ref` and `capability_ref`), `cargo test --workspace` all green, `cargo clippy --workspace --all-targets -- -D warnings` clean, `vibe check --path . --quiet` reports 0/0/0.

Three commits land the slice (newest-first, planned):

- `<pending> docs(spec,commands,roadmap,wal): cargo-shape version syntax + --exact` — `VIBEVM-SPEC.md` §7.1 rewritten as a six-row syntax table covering bare/caret/tilde/eq/range/`>=` forms; §7.5 example switches to `^0.1.0` shape with the comment `caret-default; bare semver = caret (Cargo)`. `docs/commands/install.md` grows a full pkgref-syntax table, an `--exact` flag row, and an `--exact` example. `ROADMAP.md` adds §M1.13 marked SHIPPED. WAL block (this one).
- `<pending> feat(vibe-cli/install): caret default constraint + --exact flag` — `install::run` now pairs each CLI-supplied root with its resolved version (read off `plans[i].cached.resolved.version`) and runs through `finalize_pkgref_for_manifest` before merging into `[requires].packages`. Three branches: `--exact` → `=<resolved>`; CLI had no version → `^<resolved>`; CLI had explicit constraint → preserve verbatim. The same finalized list mirrors into `lockfile.meta.root_dependencies` so the two files agree byte-for-byte. New `--exact` flag on `InstallArgs` (clap `bool`, default off). 6 unit + 3 e2e tests.
- `<pending> refactor(vibe-core,vibe-resolver): bare semver follows Cargo (caret) instead of exact` — `VersionSpec::parse` simplified to a single `semver::VersionReq::parse` call; the prior `format!("={version}")` shim is removed, so a bare semver like `0.3.0` now parses as caret `^0.3.0` (Cargo shorthand). `capability_version_for_provider` in `vibe-resolver::naive` updated to walk `req.comparators.first()` for the `(major, minor, patch)` anchor — covers bare/eq/caret/tilde/range uniformly without the `=`-prefix string trick. 3 unit tests in `package_ref` + 2 in `capability_ref` updated; one resolver test passed unchanged once the comparator-based anchor was in place.

Operational notes carried into M1.13:

- **Two-tier pkgref policy on writes.** Default = caret (resolved); `--exact` = strict equal. Operators who want different defaults set their preference once on the CLI: `vibe install --exact ...`. There is no per-project default-constraint config — keeping the surface small and matching cargo's discipline (`cargo add` is caret; `cargo add --no-default-features` doesn't change the constraint, just the features).
- **Explicit constraints are preserved.** `vibe install flow:wal@^0.1` writes `flow:wal@^0.1` (not `^0.1.0` — we do NOT tighten the operator's wider declaration). `vibe install flow:wal@~0.1.0` writes `~0.1.0`. `vibe install flow:wal@>=0.2, <1.0` writes the range verbatim. `--exact` is the only thing that overrides; without it, what the operator typed is what lands.
- **Pre-1.0 caret.** All vibevm packages today are `0.x.y`. semver caret on pre-1.0 is `>=0.x.y, <0.(x+1).0` — patch-only, not minor. So `flow:wal@^0.1.0` will pick up `0.1.5` automatically but stop at `0.2.0`. Once a package crosses 1.0, caret semantics widen to `>=1.x.y, <2.0.0` (minor-allowed), same as Cargo / npm.
- **Migration of legacy `"flow:wal"` records.** Pre-M1.13 manifests with bare-pkgref `[requires].packages` entries (no `@` at all) keep working — `VersionSpec::Latest` is still a valid shape and the resolver treats it as "any version". We do NOT auto-rewrite them on the next install. New installs write caret; legacy records sit until the operator explicitly re-runs the install or hand-edits.
- **`capability_version_for_provider` anchor change.** Previously the provider-side capability version was extracted by stripping `=` from the rendered `VersionReq` string. After the parser change `0.3.0` no longer renders as `=0.3.0`, so the string trick became unreliable. Replacement walks `req.comparators.first()` and assembles a concrete `Version` from `(major, minor.unwrap_or(0), patch.unwrap_or(0))`. Covers bare/eq/caret/tilde/range/`>=` uniformly. `*` (no comparators) falls back to the providing package's resolved version, which is the previous behaviour.

Out of M1.13: `vibe update --aggressive` that re-derives caret from current `Latest`, anything resembling a `vibe.toml`-level "version policy" config knob, opinion on whether to publish post-1.0 packages (PROP-002 leaves that to package owners). None are blocked by this slice.

**Working checkpoint (2026-05-08, M1.12 — `vibe.toml` `[requires]` section + cargo-shape install/uninstall + install-from-manifest mode).** First-time real-world walk of `vibe install` against a freshly-initialised project surfaced the gap: `vibe install <pkgref>` only wrote to `vibe.lock`, never updated `vibe.toml`. The project manifest carried registries / LLM config / language preferences but no list of installed packages — that lived only in the lockfile as `meta.root_dependencies`. Out of step with cargo / npm / Poetry / Bundler / Go modules. Made `vibe install` with no arguments a no-op (clap rejected empty packages list), made PR diffs unreadable (a one-line dep change ballooned into dozens of hash/source/ref lines in the lockfile), and made cloning a vibevm project from git unable to "just work" — the operator had to re-type every pkgref. Slice closes the gap: `[requires]` lands in `ProjectManifest`, install/uninstall keep manifest and lockfile in lockstep, no-args install is the install-from-manifest shape, lockfile's `meta.root_dependencies` reframed as a mirror of the manifest. **HEAD `<pending>`**, vibe-cli at **183 hermetic + 3 ignored** (+9 since slice 5's 174: 4 e2e + 4 unit + 1 across other crates), `cargo test --workspace` all green, `cargo clippy --workspace --all-targets -- -D warnings` clean, `vibe check --path . --quiet` reports 0/0/0.

Five commits land the slice (newest-first, planned):

- `<pending> docs(commands,roadmap,wal): refresh install/uninstall + checkpoint` — `docs/commands/install.md` rewritten to cover the two-file model (`vibe.toml` declaration ↔ `vibe.lock` materialisation), the no-arguments install-from-manifest mode, and the manifest-update step. `docs/commands/uninstall.md` updated to mention the `[requires]` cleanup. `ROADMAP.md` adds §M1.12 with the slice's scope marked SHIPPED.
- `<pending> feat(vibe-cli/uninstall): clean [requires] from vibe.toml` — `uninstall::run` now reads the project manifest, calls `drop_from_manifest_requires` (returns true iff an entry was actually removed), and writes the manifest only on change. `unregister_installed` continues to handle the lockfile side; the manifest write is symmetric. Pure transitives (never declared in the manifest) leave the manifest untouched.
- `<pending> feat(vibe-cli/install): write [requires] + install-from-manifest` — `install::run` now treats `manifest` as `mut`, builds the effective root list from three input shapes (CLI args / manifest declarations / lockfile snapshot for first-run migration), records CLI-supplied roots into `manifest.requires.packages` after a successful apply (de-dup by `(kind, name)`; constraint change overwrites the prior entry), and writes the manifest before the lockfile. `--required = true` removed from `InstallArgs::packages` so clap accepts no-arg invocations; the new `merge_manifest_requires` helper has 4 unit tests + 4 cli_e2e tests.
- `<pending> feat(vibe-core): [requires] in ProjectManifest` — adds `pub requires: Requires` (re-using the existing `vibe-core::manifest::package::Requires` type so the same shape covers package and project manifests) with `#[serde(default, skip_serializing_if = "Requires::is_empty")]` so empty sections round-trip cleanly. `ProjectManifestWire` and `From<ProjectManifestWire>` updated; `vibe init` initialises the field via `Requires::default()`. Two new tests: round-trip of a populated `[requires]`, parse-without-section back-compat for legacy manifests.
- `<pending> docs(spec): vibe.toml [requires] section + sync model` — `VIBEVM-SPEC.md` §7.5 example gains the `[requires]` section + a paragraph spelling out the two-file model (declaration vs materialisation, same shape as Cargo / npm / Poetry / Bundler). §5.6 install graph adds the `install:update-manifest` node + an explicit "install with no arguments" subsection. §7.4 reframes `meta.root_dependencies` as a mirror of the manifest. `PROP-002 §2.7` refactored to match: lockfile is self-contained snapshot, manifest is the source of truth, first-run migration path documented.

Operational notes carried into M1.12:

- **Manifest is authoritative for user intent; lockfile mirrors.** When the two diverge (operator hand-edits `[requires]`), `vibe install` re-resolves against the manifest and the lockfile follows. `vibe.lock` `meta.root_dependencies` never drives behaviour on its own — its only job now is to keep the lockfile a self-contained snapshot for tooling that reads only one of the two files.
- **First-run migration is silent and one-way.** A pre-`[requires]` `vibe.toml` parses cleanly (the field is `default`-initialised); a no-args `vibe install` on such a project copies `meta.root_dependencies` from the lockfile into `vibe.toml` `[requires].packages`, persists the manifest, then proceeds with the resolve / fetch / apply pipeline against the migrated input list. Subsequent runs see a non-empty `[requires]` and skip the migration. Operator never sees the migration as an interactive prompt.
- **Repeat-install with new constraint replaces.** `vibe install flow:wal@^0.3` then `vibe install flow:wal@=0.4.0` ends with `[requires].packages = ["flow:wal@=0.4.0"]` — the constraint is what matters, not the history. `merge_manifest_requires` returns `true` iff the in-memory shape diverged from disk so the manifest is only written on change (avoids spurious atime / VCS churn).
- **Empty `[requires]` skipped on serialize.** `Requires::is_empty()` plus `skip_serializing_if` keeps fresh `vibe init` output minimal — the section appears only after an actual install. A pre-existing `[requires]` that becomes empty after the last `vibe uninstall` is not re-rendered.

Out of M1.12: workspace-shape installs (cargo `[workspace.dependencies]` analogue), dev-only / build-only dependency markers (cargo `[dev-dependencies]` analogue), `vibe install --frozen` mode that refuses to update the manifest. None are blocked by this slice; they fit naturally on top.

**Working checkpoint (2026-05-07, M1.7 slice 5 — bootstrap-mode MCP + scope/what unification + upgrade/uninstall + two-state SKILL.md).** Closes the chicken-and-egg from slice 4: until now, `vibe mcp install` required a `vibe.toml` next to the install path and wrote everything project-tree-only. An agent invited to "create a vibevm project" had no skill loaded yet, because installing the skill required an existing project. Slice 5 moves install / upgrade / uninstall to a two-axis (`--scope project|user|both` × `--what mcp|skill|both`) model, makes user-scope the bootstrap path that does NOT require `vibe.toml`, lands SKILL.md in two-state form (Section A bootstrap, Section B inside-project, plus common rules), and adds `vibe mcp upgrade` (refresh stale installs after `cargo install`) + `vibe mcp uninstall` (zeroing out vibevm with foreign-key preservation). Status command extends with skill-drift report. **HEAD `55d22d9`**, vibe-cli at **174 hermetic + 3 ignored** (+16 since slice 4's 158), `cargo test --workspace` all green, `cargo clippy --workspace --all-targets -- -D warnings` clean, `vibe check --path . --quiet` reports 0/0/0.

Six commits land slice 5 (newest-first):

- `55d22d9 docs(commands,guides): refresh mcp-* docs + opencode quickstart for slice 5` — `mcp-install.md` rewritten under new `--scope` / `--what` shape; new `mcp-upgrade.md` and `mcp-uninstall.md` documenting scan-then-act semantics + status vocabulary including `not-installed` / `would-remove` / `removed`; `mcp-status.md` extended with skill_results documentation; `docs/README.md` index gains rows for upgrade/uninstall; `docs/guides/agent-mcp-quickstart-opencode.md` fully rewritten under bootstrap flow ("install MCP+skill at user-level once, then let the agent create vibevm projects on demand").
- `35cad9f docs(vibe-cli/mcp): SKILL.md two-state — bootstrap + inside-project` — `crates/vibe-cli/src/commands/skill_template.md` rewritten in two-state form: detect-step picks Section A (bootstrap, run `vibe init`, install starter packages, optionally land project skill) or Section B (inside existing project, follow boot protocol); common section covers MCP tools / `--invoked-by` / `vibe --help` discipline / four rules. Frontmatter description widened to trigger on "create vibevm project" intents, not just on `vibe.toml`-shaped signals. 4 unit tests lock the two-state contract + slice-5 subcommand mention.
- `3c7fced feat(vibe-cli/mcp): vibe mcp status — include skill drift report` — `mcp status` now emits `skill_results` array alongside the existing MCP `results`. Reuses `install_skill` with `dry_run=true` to avoid duplicating decide-then-(don't-)apply logic. Each row keyed on (agent, scope) so an agent with both scopes appears twice. CI drift gate becomes a one-liner watching both axes.
- `08f8260 feat(vibe-cli/mcp): vibe mcp uninstall — drop vibevm block + delete SKILL.md` — mirror of install. Same three axes (scope / what / agent). Drops only `vibevm` key from `mcpServers` / `mcp` / `mcp_servers` (foreign keys preserved); deletes SKILL.md + best-effort `rmdir` parent `vibevm/` skill subdir if empty. New status: `removed` / `would-remove` / `not-installed` (file or block absent — nothing to remove). Top-level config files never deleted. 5 e2e tests covering the contract.
- `f068a21 feat(vibe-cli/mcp): vibe mcp upgrade — refresh stale installs to current` — scan known places, compare on-disk shape to current binary's `SKILL_TEMPLATE` + `build_mcp_entry`, rewrite only the diverged ones. **Does not create new installations** (status `not-installed` for absent files / blocks — points at `vibe mcp install`). Two-step probe: file missing → not-installed; file exists but no `vibevm` key → not-installed; vibevm-key present → fall through to install-time decide-then-apply pipeline. `--config-only` / `--skill-only` toggles. Text-mode renderer uses distinct sigils (`✓` unchanged, `would`/`updated` drift, `·` not-installed). 6 e2e tests including drift detection + foreign-key preservation + dry-run no-write + scope-project-without-vibe-toml gate.
- `3f0e517 feat(vibe-cli/mcp): scope=project|user|both + what + bootstrap mode` — large refactor closing slice-5 phases C1–C4 in one commit (the intermediate states would be non-functional). New `Scope { Project, User, Both }` enum replaces slice-4's `SkillScope`; `--scope` axis covers BOTH MCP-config and SKILL.md (no longer split between `--config-scope` / `--skill-scope`). New `--what mcp|skill|both` axis replaces slice-4's `--with-skill` / `--without-skill` toggles. `Agent::config_path(scope, project_root) -> Result<Option<PathBuf>>` returns `Some(<path>)` for valid (agent, scope) pairs, `None` for combinations with no surface (Claude Desktop / Codex have no project surface). `Agent::build_mcp_entry(scope, project_root)` omits `--path` for user-scope so the server resolves CWD per invocation — this is what lets one global config serve every project. `vibe.toml` gate is now scope-conditional: required for `--scope project` / `--scope both`, optional for `--scope user` (the bootstrap path). Wizard expanded to 3 questions (Scope / What / Agents); each step skip-by-flag. Agents step always shows all 5 candidates with checkbox preselected for detected ones (slice-4's `--force`-gated pool was over-strict). Wire envelope grows `scope` + `what` + per-result `scope` field; `mode` vocabulary changed `auto / agent-flag / interactive` → `auto / flags / interactive`. Breaking-change: slice-4 `--with-skill` / `--without-skill` / `--skill-scope` are gone. 28 unit + 13 e2e tests cover the matrix.

Operational notes carried into slice 5:

- **User-scope MCP entry omits `--path`.** `["vibe", "mcp", "serve"]` (no `--path`). The server resolves CWD per invocation. Project-scope keeps `["vibe", "mcp", "serve", "--path", "<abs-project>"]`. The two-state SKILL.md treats this transparently: agent doesn't need to know which scope wired it.
- **Both-mode for user-only agents.** `--scope both` against Claude Desktop or Codex emits a `skipped` row for the project leg + the actual write for the user leg. JSON consumers see two entries per agent in Both-mode walks (one per concrete scope).
- **Upgrade vs install boundary.** Install creates new installations + refreshes existing ones (slice-4 behaviour, preserved). Upgrade refreshes existing installations only — `not-installed` rows are hints, never auto-promoted to install. Sharp boundary keeps cron-style `vibe mcp upgrade --yes` safe.
- **Uninstall preserves user property.** Foreign keys, sibling MCP servers, top-level scalars — all survive uninstall. The skill-dir's parent `vibevm/` folder is removed only if empty (best-effort). Hand-edits inside SKILL.md ARE clobbered (the file is ours; if you need to keep an edit, back it up first).

Out of slice 5: Gemini agent, Copilot CLI/VSCode, `query_capabilities` / `list_subskills` MCP-tools, comment-preserving Codex TOML edits via `toml_edit`. Plan preview + apply-confirm prompt before writes is in the wizard surface but not wired to a hard interactive confirm yet — currently `--yes` and `--auto` both implicitly bypass; future commit can add an explicit confirm step before `apply_install_mcp` calls.

**Working checkpoint (2026-05-07, M1.7 slice 4 — multi-agent MCP install + skill + invoked-by + opencode quickstart guide).** Five-agent matrix landed end-to-end. `vibe mcp install` now targets Claude Code, Claude Desktop, Cursor, OpenCode, Codex with per-agent config writers (JSON for the first four with `mcpServers` literal, JSON for OpenCode under `mcp` with command-array shape and `type: "local"` + `enabled: true`, TOML for Codex under `mcp_servers` snake-case section in `~/.codex/config.toml`). Skill artefact lands at `<scope>/<agent-skills-dir>/vibevm/SKILL.md` for the three agents that load filesystem skills (Claude Code, OpenCode, Codex); Cursor and Claude Desktop are reported as `skipped`. New global `--invoked-by <agent>` flag + `VIBE_INVOKED_BY` env-var stamps every JSON envelope with the calling agent's identity; the SKILL.md instructs each agent to pass it on every invocation. New install UX — interactive `dialoguer::MultiSelect` when no flags are present (TTY required), `--auto` for CI / first-run scripts, `--with-skill` / `--without-skill` toggle, `--skill-scope project|user`. New `docs/guides/` directory with `agent-mcp-quickstart-opencode.md` — dual-purpose tutorial + integration-test acceptance gate (12 boxes pinning every slice-4 surface). **HEAD `3bf2462`**, vibe-cli at **158 hermetic + 3 ignored** (+27 since slice 3's 131), `cargo test --workspace` all green, `cargo clippy --workspace --all-targets -- -D warnings` clean, `vibe check --path . --quiet` reports 0/0/0.

Six commits land the slice (newest-first):

- `3bf2462 docs(guides): opencode + vibevm hello-world quickstart + acceptance gate` — adds `docs/guides/` (new home for long-form walkthroughs, distinct from per-command reference under `docs/commands/`). The first inhabitant — `agent-mcp-quickstart-opencode.md` — is dual-purpose: copy-paste tutorial for new operators + machine-readable acceptance checklist for vibevm releases. Filename pattern `agent-mcp-quickstart-<agent>.md` scales to siblings (Claude Code / Codex / Cursor / Claude Desktop) without restructuring. Three demo prompts ship escalating from cheapest (bare `query_package` probe) through full hello-world (agent reads subskills, creates README + docs/hello.md, updates `spec/WAL.md` per WAL protocol) to fallback for tool-use-incapable models (summarise SKILL.md body). Maintenance section codifies "when slice 4 surface changes, this document must change with it" with a per-change-type lookup. `docs/README.md` index gains a "Guides" section.
- `7cb1f33 docs(commands,roadmap,wal): M1.7 slice 4 — multi-agent + skill + invoked-by` — three new reference files (`docs/commands/mcp-install.md`, `mcp-status.md`, `mcp-serve.md`); ROADMAP §M1.7 marked slices 1–4 ✅, §M1.11 (agent auto-detection) marked closed alongside slices 2 + 4; this WAL block.

- `71229eb feat(vibe-cli/mcp): interactive install + --auto + --with/without-skill` — closes the install UX surface. New CLI shape: `--agent <FILTER>` optional (was default-`all` in slice 2; legacy operators must pass `--agent all` explicitly now or use `--auto`), `--auto` (detect every supported agent + install MCP + skill), `--with-skill` / `--without-skill` (mutually exclusive; defaults: `--auto` → on, explicit `--agent` → off, interactive → asks), `--skill-scope project|user`. Non-TTY without flags refused with a hint pointing at `--agent` / `--auto` rather than panicking inside dialoguer. Wire shape grows `skill_results` array, `skill_scope`, `install_skill` boolean, `mode` (`auto` / `agent-flag` / `interactive`). Two slice-2 e2e tests updated to pass `--agent claude` / `--agent cursor` explicitly; eight new tests landed (with-skill / without-skill / cursor-skipped / opencode shape / auto-dry-run / clap-conflict / non-TTY hint / `--invoked-by` envelope stamp).
- `d384a96 feat(vibe-cli/mcp): vibevm SKILL.md template + per-agent writer` — `crates/vibe-cli/src/commands/skill_template.md` vendored via `include_str!` so the template ships byte-identical inside `vibe`. YAML frontmatter triggers on every vibevm signal (presence of `vibe.toml`, vibe subcommand mentions, `spec/`, `packages/`, lockfile/subskill references). Body pins the bootstrap protocol (`CLAUDE.md` → `spec/boot/*` → `spec/WAL.md` → relevant PROPs/FEATs), enforces "use the MCP server, do not guess" against `query_package` / `read_subskill` / `materialise_subskill`, requires `--invoked-by` on every CLI call, requires `vibe <subcmd> --help` consultation before suggesting commands, inherits the four non-negotiable rules. `Agent::skill_path(scope, project_root)` resolves per-agent / per-scope paths (`.claude/skills/`, `.opencode/skills/`, `.agents/skills/` for project; `~/.claude/skills/`, `<config-dir>/opencode/skills/`, `~/.agents/skills/` for user). `install_skill(agent, scope, project_root, dry_run)` is idempotent — byte-identical existing files report `unchanged`; drift is overwritten (the contract is set by the binary). `SkillInstallReport` mirrors the JSON-config writer's status vocabulary.
- `2eaf544 feat(vibe-cli): --invoked-by global flag + VIBE_INVOKED_BY env` — top-level CLI flag (clap `global = true`), resolution `flag > env > unset` with whitespace-only values treated as unset on either layer. `output::Context` extracted `render_json` from `emit_json` so the `invoked_by` stamp is testable without stdout capture. `Map::entry().or_insert` shape on the stamp so caller-supplied `invoked_by` on an inner envelope is preserved (flatten semantics). `Context::error` (JSON-mode error path) also stamps. `vibe show config` gains an `invoked_by_resolution` block with provenance (`cli-flag` / `env` / `default`); top-level stamp on the same envelope coexists thanks to the rename.
- `05ce2e4 feat(vibe-cli/mcp): claude-desktop, opencode, codex + JSON/TOML mergers` — the `Agent` enum extends from two variants (Claude Code, Cursor) to five. Per-agent profile via inherent methods (`presence_markers`, `config_format`, `config_location`, `mcp_section_key`, `build_mcp_entry`, `host_present`, `is_present`). Generic `merge_json` parameterised by `(section_key, server_name)` so the same code drives Claude Code/Desktop/Cursor (`mcpServers`) and OpenCode (`mcp`). New `merge_toml` for Codex (`mcp_servers` table) — preserves foreign top-level keys but strips comments because `toml = "0.9"` round-trips `Value` not `toml_edit::Document`; switching to `toml_edit` is a v1+ follow-up if a Codex operator with handcrafted comments asks for it. OpenCode's MCP entry uniquely uses a single `command: ["vibe", "mcp", "serve", ...]` array (not split `command + args`) plus mandatory `type: "local"` and `enabled: true` discriminators. OpenCode markers include `AGENTS.md` per the owner's request — every vibevm project ships `AGENTS.md` (the cross-agent copy of `CLAUDE.md`), so the false-positive is intentional: every vibevm project gets OpenCode provisioning by default if `--agent all` or `--auto`.

Operational notes carried into slice 4:

- **Codex / Claude Desktop are user-level only.** Their config files live outside the project tree (`~/.codex/config.toml`, `<config-dir>/Claude/claude_desktop_config.json`). Detection probes the existence of the parent dir (`~/.codex/`, `<config-dir>/Claude/`); presence_markers are empty for these agents. `--auto` will mutate user-level configs when those dirs exist, so `--dry-run` is the safe preview path.
- **Skill scope decision.** Project-scope skills (default) commit to git; every clone gets the same byte-identical skill. User-scope skills install once per machine but require re-installs after a vibevm upgrade. The interactive multi-select asks operators to pick when skill installation is active.
- **`--invoked-by` is opt-in but skill-mandated.** The CLI accepts envelopes without the field (logs and JSON consumers tolerate `invoked_by` absent). The SKILL.md text raises it from "nice-to-have" to "you MUST pass this" so once the skill loads the agent has no excuse to skip attribution.

Out of slice 4: Gemini agent, Copilot CLI/VSCode, `query_capabilities` / `list_subskills` MCP tools, comment-preserving Codex TOML edits via `toml_edit`. ROADMAP §M1.7 + §M1.11 updated to reflect the closure.

**Working checkpoint (2026-05-06, MFBT session, PROP-005 closed + trailing fixups + rate limiter).** All eleven slices of PROP-005 landed end to end plus PROP-006 codifying owner-invoked codewords. Trailing-fixup slices on the second MFBT pass closed file-shape gaps (slices 16–19): primary.jsonl.gz, by-cap/by-purl, init writes README/gitignore, structured stub envelope for --from-gitverse. Third MFBT pass landed the built-in rate limiter (PROP-005 §9 Q10) — token-bucket per-token + per-IP, opt-in via CLI flags, RFC 6585 / RFC 9596 wire shape. Remaining §9 open questions (GPG signing v1+, Merkle log v2+, OCI registry shape, --auto-commit-push, WebSocket notifications) parked until demand surfaces. **HEAD `039bd96`**, services workspace at **162 hermetic tests + 0 ignored**, main workspace tests green, `cargo clippy --workspace --all-targets -- -D warnings` clean across both workspaces, `tools/self-check.sh` green.

What's effectively complete (non-LLM, non-libsolv):

- PROP-003 r2 — schema + features + subskills (4 channels + 3 delivery modes, lazy-pull genuinely lazy via cache+MCP) + BCP-47 i18n + conditional dependencies (cascading fixed-point loop) + lockfile schema v3 with full provenance.
- M1.7 — `vibe-mcp` crate, `vibe mcp serve` CLI, agent auto-detection + `vibe mcp install/status` for Claude Code & Cursor, cache-precise `read_subskill`, on-demand `materialise_subskill`.
- M1.10 — `vibe outdated`.
- M1.11 — agent auto-detection (closed alongside M1.7 slice 2).
- vibe-check — three PROP-003 checks (`features_graph`, `subskill_structure`, `i18n_coverage`) + `activation_conflict` Jaccard heuristic.
- Three integration fixture packages exercising every PROP-003 r2 surface in combination, with omnibus e2e suite proving cross-cutting correctness.
- Publish-side: dual-registry default + GitVerse publish stub + per-host token env precedence + `--repo-url` no-API direct push + live cross-registry e2e suite.
- **PROP-005 (new, complete)** — standalone `services/vibe-index/` utility plus main-workspace integration. Eleven slices: skeleton+dispatch (1), types+persistence (2), scanner+reindex --from-clones (3), read CLI (4), read-only HTTP server (5), write CLI/HTTP+auth (6), incremental reindex (7), reindex --from-github via REST API (8), `vibe-publish` post-publish index hook (9), `vibe-registry` consumer fast path (10), docs+smoke (11). Plus trailing layout-completeness fixups: `primary.jsonl.gz` deterministic gzip sibling, `by-cap/<slug>.jsonl` + `by-purl/<slug>.jsonl` inverted-index files with HTTP routes, `init` writes `.gitignore` + `README.md`, `reindex --from-gitverse` structured stub envelope. Standalone Cargo workspace at `services/vibe-index/`; redistribution-ready (`cargo install --path .`). Identity invariant from [PROP-002 §2.1] preserved — `content_hash` still verified at fetch time regardless of how versions were enumerated.
- **PROP-006 (new)** — operating-modes catalogue. First codeword «move fast and break things» recorded verbatim from owner; behavioural rules + lifecycle + escape-hatch for non-routine red lines.

What's open: M1.5 LLM (big, non-routine, needs sign-off), M1.8 `vibe review` static, libsolv FFI (Phase A), `vibe update` feature-awareness, vibe-mcp follow-ups (Gemini/Codex/Copilot writers, `list_capabilities` tool), GitVerse publish unstub (whenever their API gains parity), GitHub publish SSH option, `reindex --from-gitverse` (still NotYetImplemented for the same upstream-API gap). Detailed forward queue lives in `CONTINUE.md`.

**PROP-005 §9 Q10 follow-up — built-in rate limiter (2026-05-06, third MFBT pass).** Owner asked for the rate-limit knob; remaining §9 open questions (GPG signing, Merkle log, OCI registry shape, --auto-commit-push, WebSocket notifications) explicitly parked for v1+/v2+ until concrete demand surfaces.

`039bd96 feat(services/vibe-index): per-token + per-IP rate limiter` — token bucket per key with capacity = configured RPM and refill = RPM/60 tokens/sec. Two parallel pools: per-token (keyed on Bearer header) and per-IP (keyed on peer IP for unauth reads). Lazy eviction when per-IP map approaches `max_buckets` (default 10_000); idle buckets drop first, then most-replenished. Routes `/healthz` `/readyz` `/metrics` exempt. 429 response carries RFC 6585 `Retry-After` + X-RateLimit-Limit / X-RateLimit-Remaining; allowed responses also stamp the X-RateLimit headers. CLI `vibe-index serve` gains `--rate-limit-per-token <RPM>` and `--rate-limit-per-ip <RPM>` flags (default 0 disables). `axum::serve` switches to `into_make_service_with_connect_info::<SocketAddr>` so the middleware sees the peer IP. 8 unit + 7 integration tests; workspace test count 162 hermetic. Production deployments behind a reverse proxy still use the proxy's own rate-limit; the built-in knob is for operators with no proxy.

**PROP-005 trailing-fixup slices 16–19 — file-shape completeness (2026-05-06, second MFBT pass).** Owner asked to "доделать" PROP-005 after slices 1–11 landed; the gap between the implemented utility and PROP-005 §2.4/§2.13 documented layout was the inverted-index files (`by-cap/`, `by-purl/`), the gzip primary sibling, and the auto-generated README/gitignore. Plus the `--from-gitverse` branch was a generic NotYetImplemented rather than a structured stub. Four topical commits close it:

- `867ab97 feat(services/vibe-index): structured stub envelope for --from-gitverse` — slice 19. JSON envelope `{ ok: false, command: "registry:reindex", host: "gitverse.ru", org, data_dir, stub: true, reason }` mirrors the `vibe-publish` GitVerse stub shape. Exit 0; consumers detect the limitation programmatically. Reason string points at `--from-clones` workaround. The `tests/help_smoke.rs` anchor (renamed `*_emits_stub_envelope`) asserts `stub: true`, so the moment GitVerse exposes the API the test fails and we notice.
- `6e7487d feat(services/vibe-index): init writes README.md + .gitignore` — slice 18. PROP-005 §2.13 layout includes both. `vibe-index init` now seeds them; both are skipped when already present so operator-edited content survives `init --force`. README points at PROP-005 + maintenance commands; gitignore excludes `state/`.
- `7665af2 feat(services/vibe-index): by-cap + by-purl inverted index files` — slice 17. New `src/index/inverted.rs` + write/read of `by-cap/<slug>.jsonl` + `by-purl/<slug>.jsonl` files. Filesystem-safe slug encoding (`:` and `/` and `@` → `--`; uniform across capabilities and PURLs because Windows reserves `:` for ADS / drive letters; PROP-005 §2.4's "PURL slug only replaces `/`" tightened to also cover `:` for cross-platform compat). PurlRow records `binding_site` (`"package"` vs `"subskill"`) so consumers see where the describes match originated. HTTP routes `/v1/index/by-cap/{slug}` + `/v1/index/by-purl/{slug}` serve the files. `Index::write_to` regenerates both inverted dirs from `iter_versions()` on every rewrite.
- `da25eca feat(services/vibe-index): primary.jsonl.gz sibling + serve route` — slice 16. Deterministic gzip (level 6, mtime=0, no filename in header) so the sha256 in `repomd.json` stays stable across machines. `primary::write` now returns `(plain, gz)` metadata; both land in the manifest. HTTP route `/v1/index/primary.jsonl.gz` serves with `Content-Encoding: gzip` so well-behaved clients transparently decode.

Trailing-fixup test count: +13 (3 gzip + 5 inverted-view + 2 inverted-files-on-disk + 3 init-completeness; renamed help-smoke anchor counted as part of the existing 2). Workspace test count after fixups: 155 hermetic in services workspace.

**PROP-005 slices 1–11 — standalone vibe-index utility + integration (2026-05-06, MFBT).** Per the [PROP-005 design proposal](modules/vibe-index/PROP-005-package-index.md), `services/vibe-index/` is the per-org metadata index utility for vibevm-shaped registries. Single binary, two modes (CLI + HTTP server). Standalone Cargo workspace deliberately outside `crates/` so an org owner can vendor just the subdirectory and `cargo install --path .` without pulling all 13 vibevm crates.

Slice landing chain (newest-first):

- `db26a63 docs(vibe-index): operator handbook + consumer protocol + format + smoke` — slice 11. `services/vibe-index/docs/{operator-handbook,consumer-protocol,format}.md` close the documentation surface; `manual-tests/M2.10-index-smoke.md` walks bootstrap → serve → consume in three scenarios (A: serve+read+write+auth-gate; B: vibe-registry consumer fast path; C: vibe-publish post-publish hook). Pass-line "TBD on first walk".
- `86e3a16 feat(vibe-registry): index-aware list_versions fast path (PROP-005 slice 10)` — `GitPackageRegistry::list_versions` consults an upstream index when `VIBEVM_INDEX_URL_<R>` is configured for the registry. `IndexClient::probe(base)` auto-detects server (`<base>/v1/index/repomd.json`) vs raw-file (`<base>/repomd.json`) shapes; on 200 attaches a client. Per-call: 200 → return versions, 404 → fall through to git, other → fall through with debug log. Identity invariant preserved (content_hash still verified at fetch time per [PROP-002 §2.1]). 5 hermetic tests via mock axum server; reqwest moves into vibe-registry main deps.
- `97cdb9d feat(vibe-publish,vibe-cli): post-publish index hook (PROP-005 slice 9)` — when `VIBEVM_INDEX_URL_<R>` AND `VIBEVM_INDEX_TOKEN_<R>` resolve for the registry being targeted, `vibe registry publish` POSTs the freshly-built entry to `<index_url>/v1/packages` after the successful push. Hook is opt-in per registry; failures are warnings (don't fail the publish itself per PROP-005 §2.14). New `vibe-publish::post_hook` module: `registry_env_suffix` munging, `HookConfig::from_env`, `build_payload` constructs JSON matching `VersionEntry`'s serde shape via `compute_content_hash` for byte-identical parity with consumer-side recording, `post_to_index` POSTs with bearer-auth. CLI envelope grows `index_hook: { fired, status, error }`. 5 hermetic tests against axum mock + 2 unit on env-suffix shape and dormant fall-through.
- `f217178 feat(services/vibe-index): reindex --from-github via REST API + clone (slice 8)` — `--from-github <org>` walks the GitHub REST API (Link-header pagination + 5000 req/h with PAT), clones every non-fork repo into `--clone-cache` (defaults to a tempdir destroyed at end of run), then runs the existing `from_clones` scanner. `clone_url_with_token` injects `https://x-access-token:<TOKEN>@…` for HTTPS clones (token discipline per [PROP-000 §20] — never logged). 3 hermetic tests via local-bare-repo mock + 5 unit on `parse_next_link` + `clone_url_with_token`. `--from-gitverse` remains stub-bound until GitVerse exposes org-scoped repo enumeration.
- `1ab0fb0 feat(services/vibe-index): incremental reindex via checkpoint (slice 7)` — `<data-dir>/state/checkpoint.json` records each repo's HEAD commit + tag list. `--incremental` skips repos whose snapshot is unchanged, copies forward existing entries, only re-walks deltas. Summary envelope grows `mode` field. `git_cli::head_commit` best-effort `rev-parse HEAD`. Tests: full-then-incremental no-op, then add tag → incremental picks up only delta.
- `07b0130 feat(services/vibe-index): write surface + bearer-token auth (slice 6)` — CLI `add` parses vibe-package.toml + computes content_hash from package directory + composes source_url from registry metadata + upserts. CLI `remove` drops one version or all versions. Both refuse to run while a server lock is held (single-writer discipline). HTTP `POST /v1/packages` (201 created / 200 upsert), `DELETE /v1/packages/{kind}/{name}` (whole package), `DELETE /v1/packages/{kind}/{name}/{version}` (one version). `src/server/auth.rs::TokenStore` loads `<data-dir>/state/admin.tokens` (one bearer token per line, `#`-comment-tolerant). `require_writeable` runs auth + read-only refusal + scope check (entry.registry == server.registry) before any mutation. 12 server-write tests + 6 CLI-write tests + 2 TokenStore unit.
- `223114b feat(services/vibe-index): HTTP server, read-only routes (slice 5)` — MVP marker. `vibe-index serve` boots an axum runtime over `Arc<RwLock<Index>>` with a PID-file lock at `<data-dir>/state/server.lock`. Read routes from PROP-005 §2.10: `/healthz`, `/readyz`, `/v1/index/{repomd.json,primary.jsonl,by-name/<kind>/<name>}` (mirror-friendly raw files), `/v1/packages` (list+search via `?q=`), `/v1/packages/{kind}/{name}`, `/v1/packages/{kind}/{name}/{version}`, `/v1/capabilities/{cap}`, `/v1/purls/{purl}`, `/v1/admin/status`, `/metrics` (Prometheus 0.0.4 text, six gauges/counters, no prometheus crate). Errors: RFC-7807 problem-details with `type/title/status/detail`. `stop` subcommand reads PID, sends SIGTERM (Unix) or prints `taskkill` hint (Windows). 16 server_e2e tests via axum's `oneshot` (no TCP listener needed).
- `769921d feat(services/vibe-index): read CLI subcommands (slice 4)` — get / list / search / capabilities / purls / outdated. `src/index/search.rs::tokenise` lowercases ASCII alphanumeric runs, ~30-stopword filter (matches vibe-check's `activation_conflict` discipline), drops ≤1-char tokens. `search` scores by query-token overlap with name+description+keywords+capabilities+purls; ties broken by `(kind, name)`. `lookup_capability` exact match or left-of-`@` match. `lookup_purl` matches package-level AND subskill-level `describes`, records binding site. `src/lockfile.rs` is a deliberately minimal vibe.lock reader — only `(kind, name, version)` per `[[package]]` consumed. 12 cli_read tests.
- `5761c26 feat(services/vibe-index): scanner + reindex --from-clones (slice 3)` — `src/content_hash.rs` ports `vibe-registry::compute_content_hash` byte-for-byte, with `tests/content_hash_parity.rs` locking the algorithm against `fixtures/golden-flow-wal-0.1.0/` (golden hash `sha256:e9fedc6326…`, verified against vibe-registry's Rust impl + Python reference impl on 2026-05-06). `src/scanner/git_cli.rs` shells out to `git` for `list_tags` / `resolve_commit` / `materialise_at_ref` (shallow clone + remove `.git` so the result is hash-clean per vibe-registry's `copy_dir_excluding_git` invariant). `src/scanner/manifest.rs` parses `vibe-package.toml` into VersionEntry-relevant fields + walks `subskills/<path>/vibe-subskill.toml`. `src/scanner/from_clones.rs` org-walks subdirs, skips non-git or non-`v<semver>` ones with `SkipNote`. 4 scanner_e2e tests + 1 parity + 4 git_cli unit + 4 manifest unit + 6 content_hash unit.
- `26d2648 feat(services/vibe-index): types + on-disk persistence (slice 2)` — `src/types/{entry,kinds,repomd}.rs` mirror the relevant subset of `vibe-core`'s manifest schema (PROP-005 §3.2 explained the duplicate-rather-than-import trade-off; parity test gates divergence). `src/index/{memory,persistence,primary,by_name,repomd}.rs` write-pipeline: atomic tmp+fsync+rename, `repomd.json` written LAST so partial views remain consistent. `Index::write_to` clears `by-name/` before rewrite (slice 7's incremental upgrade replaces this scorched-earth approach with per-repo diff). 7 cli_lifecycle tests covering init/dump/verify e2e + 33 unit.
- `babfcf0 build(self-check): include services/vibe-index workspace` — adds two new conditional steps to `tools/self-check.sh` so CI gates services + main workspaces in lockstep.
- `d45355e feat(services/vibe-index): skeleton crate + clap dispatch (slice 1)` — fourteen subcommand stubs (init / reindex / get / list / search / capabilities / purls / outdated / add / remove / verify / dump / serve / stop), each its own one-file module, each `Args` struct carrying the v1 flag surface so help text prints the planned shape from day one. `tests/help_smoke.rs` pins the dispatch surface as a regression invariant.

Standalone-workspace decision (PROP-005 §6) bears repeating: `services/vibe-index/Cargo.toml` carries its own `[workspace]` table to opt out of the parent vibevm workspace; this is what lets an org owner clone JUST the subdirectory. `tools/self-check.sh` runs both workspaces in lockstep so divergence is gated at CI time.

Operational env-var convention (slices 9 + 10):

- `VIBEVM_INDEX_URL_<REGISTRY>` — index URL for both publish-side hook and consume-side fast path. The publish hook treats it as a server root (POSTs to `<base>/v1/packages`); the consumer fast path auto-probes both `<base>/v1/index/repomd.json` and `<base>/repomd.json` so either server-root or static-file-root URLs work.
- `VIBEVM_INDEX_TOKEN_<REGISTRY>` — bearer token for write-side endpoints. Read-only consumers ignore it. Token bytes never logged anywhere in the toolchain (same discipline as `VIBEVM_PUBLISH_TOKEN_<HOST>`).

Carry-forward queue:

- **PROP-005 slice 8 follow-up — `reindex --from-gitverse`.** Currently `NotYetImplemented`. Lands when GitVerse exposes org-scoped repo enumeration in their public API (same gap that keeps `vibe registry publish --registry vibespecs-gitverse` stub-bound).
- **`--auto-commit-push`** (PROP-005 §2.9). Server-side option to `git add -A && git commit && git push` after every mutation against the index repo. Parked until operator demand surfaces; until then the operator commits + pushes manually or via a separate cron.
- **GPG signing of `repomd.json`** (PROP-005 §9). Tracked. v1+.
- **Merkle log (Go sumdb-style)** (PROP-005 §9). Tracked. v2+.
- **Integration of `vibe outdated --upstream`** with the index (in addition to the existing per-package `git ls-remote` upstream probe). Cheaper polling for large lockfiles. Naturally fits on top of the slice 10 IndexClient.

**PROP-006 — operating modes (codewords) (2026-05-06).** Codified the owner-invoked codeword pattern as a first-class project artefact. PROP-006 catalogues each codeword's trigger phrase, authoritative description (recorded verbatim from owner), operative interpretation, what it changes / what it does NOT change, and activation lifecycle. The four non-negotiable rules from CLAUDE.md survive every codeword unchanged; only Rule 4's "ask before routine large changes" subclause is suspended; Rule 4's red-line list (force-push / history rewrite / large blobs / CI / signing / secrets / irreversible ops) STILL gates non-routine work even under the most aggressive posture.

First codeword: «move fast and break things» — heads-down execution, maximum-version target, testable phased iterations, full test coverage, no mid-work asking, full reasoning depth (`/effort max`, ultrathink, superthink, think-like-mythos). Owner activated it for this PROP-005 push at session start.

A pointer block in `spec/boot/90-user.md` surfaces the codeword catalogue at session boot so future sessions discover the codeword without already knowing to look for it. Definition stays in PROP-006; 90-user.md just says "they exist; here is the index".

Two commits land it: `9fd0575 docs(spec): PROP-006 — operating modes + 'move fast and break things' codeword` (PROP doc + 90-user.md cross-ref).

**PROP-005 design proposal — optional per-org package index (2026-05-06).** Long-form proposal at [`spec/modules/vibe-index/PROP-005-package-index.md`](modules/vibe-index/PROP-005-package-index.md) (~700 lines). Per-org dedicated `index` git repository holds `repomd.json` (RPM-style manifest with sha256 of every file) + `primary.jsonl` (JSON Lines, one record per (kind, name, version), sorted) + `by-name/<kind>/<name>.json` (cargo-sparse-style per-package fetches) + `by-cap/` + `by-purl/`. Standalone `services/vibe-index/` utility, single binary with CLI + `serve` modes, single-writer in-RAM with atomic on-disk persistence, full CRUD via REST + bearer-token auth, full+incremental reindex, observability via `/metrics`. Identity/integrity invariants from PROP-002 §2.1 unchanged — `content_hash` still verified at fetch time; index is a hot cache, not source-of-truth. PROP-005 explicitly carries the "out-of-band research summary" of comparative inventories from a prior session (Maven Central / npm / PyPI / RPM / Deb / Cargo / Go modules / Nix flakes / Homebrew / OCI) so future readers see the design space without re-derivation. Eleven slices planned; slices 1–7 landed (this session); slices 8–11 carry forward. One commit lands it: `505a8cd docs(spec): PROP-005 — optional per-org package index utility`.

**Publish-side rework (2026-05-06).** Coherent slice across `vibe-core` / `vibe-publish` / `vibe-cli` reshaping how vibevm projects discover and publish packages. Two commits land it: `44a8c1c feat(core,publish,cli): two default registries + per-host tokens + no-API direct push` and `f6f4f0c test(cli): live e2e for cross-registry resolution + smoke fixtures`.

- **Dual-registry default** (`44a8c1c`). `vibe init` now provisions both `vibespecs` (GitHub, primary, `naming = "kind-name"`) and `vibespecs-gitverse` (GitVerse, secondary, `naming = "name"`). The asymmetric naming convention is deliberate — the GitVerse `vibespecs` org provisions repos under bare names (`vibespecs/vibevm-direct-push-smoke`) rather than the kind-prefixed form GitHub uses (`vibespecs/flow-vibevm-github-smoke`). Resolver walks them in priority order on `UnknownPackage` fall-through; a fresh project finds packages on either host without operator hints. New constants `DEFAULT_REGISTRY_GITVERSE_NAME` / `DEFAULT_REGISTRY_GITVERSE_URL` in `vibe-core::manifest::project`. `--registry-url` overrides to single-registry; `--no-registry` empty. Root `vibe.toml` updated to mirror the new shape so self-`vibe check` validates against the same layout fresh projects use.

- **GitVerse publish stub** (`44a8c1c`). The GitVerse public REST API does not yet expose org-scoped repo creation, so `vibe registry publish --registry vibespecs-gitverse <path>` short-circuits at host detection with a clear "not implemented" envelope (`ok: false, command: "registry:publish", host: gitverse.ru, registry, stub: true, reason`). No token is loaded, no HTTP call is made. Resolve-time reads against GitVerse continue to work via `MultiRegistryResolver`. `vibe registry publish` to GitHub (the default target without `--registry`) keeps working through the regular API path.

- **Per-host publish-token env vars** (`44a8c1c`). New precedence in `vibe-publish::token::load_token_for_host`: `VIBEVM_PUBLISH_TOKEN_<HOST>` (host-specific env, `_GITHUB` / `_GITVERSE` / etc.) → `VIBEVM_PUBLISH_TOKEN` (legacy host-agnostic env, kept so existing setups don't need a rename) → `~/.vibevm/<host-prefix>.publish.token` → `~/.vibevm/git.publish.token`. `TokenSource::EnvVar(String)` (was `&'static str`) since the var name is now computed. New `host_env_var(host) -> Option<String>` helper. `vibe show config` lists all three publish-token vars in `CONFIG_ENV_VARS` with `sensitive: true` → `redacted` provenance gating intact. CI can now hold tokens for several hosts in the same env without one clobbering the others.

- **`vibe registry publish --repo-url <git-url>`** (`44a8c1c`). New no-API direct-push path: pushes the freshly-built commit + tag straight to the supplied URL using the local user's git credentials (SSH agent / `credential.helper` / netrc). No host-API call, no token loaded, no organisation-scope plumbing. Implemented as a new `DirectGitCreator` in `vibe-publish::direct_git` declaring `direct_repo_url() -> Option<&str>` (default `None` on the `RepoCreator` trait); `Publisher::publish` short-circuits the `extract_org_segment` + `repo_exists` + `create_repo` dance when that hook returns `Some`, falling straight into `git_publish::push_release`. Repo presence is the operator's responsibility (the path is the right escape hatch for hosts without API adapters, for forks, and for ad-hoc test repos). `--repo-url` and `--registry` are mutually exclusive at the clap layer. Both SSH and HTTPS URLs supported equally — the URL is used verbatim. Outcome envelope `{ ok: true, command: "registry:publish", mode: "direct-git", host, repo_url, repo_name, tag, dry_run }` — `mode: "direct-git"` lets consumers distinguish from the registry path without parsing host strings.

- **Live e2e tests + manual-test fixtures** (`f6f4f0c`). Three `#[ignore]`-d tests in `crates/vibe-cli/tests/cli_live_e2e.rs`: `install_github_smoke_alone` (GitHub-only resolution), `install_gitverse_smoke_alone` (fall-through to GitVerse on GitHub `UnknownPackage`), `cross_registry_resolution_routes_each_package_to_correct_host` (both in one install, each to the correct host, distinct content_hashes). Run with `cargo test --test cli_live_e2e -- --ignored` (~22s combined). Two test packages published live to back the suite: GitHub `vibespecs/flow-vibevm-github-smoke@v0.0.1` (created via API path) + GitVerse `vibespecs/vibevm-direct-push-smoke@v0.0.1` (created via `--repo-url` direct push, SSH). Fixtures under `fixtures/manual-test-packages/` — throwaway no-op flows whose names scream "test" so the org page makes their nature obvious. Pinned at `v0.0.1` forever to keep them deletable. Walked successfully on this machine (Windows 11 / git 2.52.0): all three pass.

Tests landed: 14 hermetic (4 token unit, 7 DirectGitCreator unit, 3 e2e — GitVerse stub envelope, direct-push to local bare repo via `file:///`, mutual-exclusion gate) + 3 ignored live. Pre-existing `init_writes_default_registry` updated to assert dual-registry layout. Workspace state: 418 hermetic tests (+15 since previous checkpoint's 403; one consolidated test for the `--registry-url` override pattern was rewritten rather than added net-new, hence 15 not 17), `cargo clippy --workspace --all-targets -- -D warnings` clean, `tools/self-check.sh` green.

Out-of-scope follow-ups for future sessions:
- **GitVerse publish unstub.** When/if their API exposes `POST /orgs/<org>/repos` end-to-end, flip the stub branch in `run_publish` back to regular adapter dispatch. The stub message itself notes the limitation.
- **GitHub publish SSH option.** Currently HTTPS-token only via `GitHubCreator::push_url`. Could add SSH fallback for operators who prefer key-based push. Tied to broader publish-flow polish.
- **`docs/commands/{registry-publish.md, show.md}` refresh.** Mechanical translation of new `--help` text + the new env-var entries into reference shape.

**M1.7 vibe-mcp slice 3 — per-subskill files index + materialise_subskill (2026-05-05).** Closes the lazy-pull runtime promise from PROP-003 §2.5.0. Three coupled changes land together so `delivery=lazy-pull` subskills behave correctly end-to-end without polluting the project tree.

- **`LockedSubskill` schema** (`390fc3a`). Two new fields on the v3 lockfile entry. `files_written: Vec<PathBuf>` — project-relative paths a subskill specifically contributed (empty for lazy-pull). `cache_files: Vec<PathBuf>` — subskill-root-relative paths inside the package cache (populated for every delivery mode so MCP can resolve bytes via the cache regardless of mode). Both `#[serde(default)]` so legacy lockfiles parse.
- **`vibe-install` lazy-pull becomes truly lazy** (`390fc3a`). The install pipeline no longer materialises `delivery=lazy-pull` subskills into the project tree. `lazy-push` continues to degrade to eager until M2.8 ships the runtime push path. Both modes write their per-subskill files indices into the lockfile from day one so future tooling has the data without lockfile churn.
- **`vibe-mcp` cache-precise tools** (`3c9e710`). `read_subskill` upgraded — for `eager`/`lazy-push`, reads `files_written` from the project; for `lazy-pull`, reads `cache_files` from the package cache. Wire shape stays uniform across modes. New `materialise_subskill(package, subskill_path, force?)` tool promotes a lazy-pull subskill into the project tree on demand; refuses to overwrite existing files unless `force=true` (preserves user edits, same discipline as `vibe update`'s `UserEditedFile` gate). Eager/lazy-push subskills are no-ops on this tool.

Tests: 4 new (1 omnibus reflow for the lazy-pull behaviour shift + 3 new vibe-mcp unit on materialise paths) + 1 new e2e `mcp_materialise_subskill_promotes_lazy_pull_into_project` spawning `vibe mcp serve` and driving the JSON-RPC call to verify end-to-end materialisation through the MCP wire form.

Workspace state: 403 tests (+4 over slice 2's 399), `cargo clippy --workspace --all-targets -- -D warnings` clean, `tools/self-check.sh` green. M1.7 effectively complete for non-LLM scope: server + transport + tools + agent auto-config + lazy-pull runtime. Remaining slices: Gemini/Codex/Copilot agent writers, `query_capabilities` / `list_subskills` discovery tools, integration with the LLM virtual-capability emission story (Phase F, post-M1.5).

**M1.7 vibe-mcp slice 2 — agent detection + MCP config writers (2026-05-05).** Slice 1 shipped the server itself; slice 2 closes the integration loop so a fresh vibevm install hooks into the operator's existing coding-agent setup automatically. Combined with M1.11 (agent auto-detection at `vibe init` — overlap closed in this slice).

`vibe mcp install [--path] [--agent claude|cursor|all] [--dry-run] [--force]` (`98fec82`):

- Detects supported agents by probing for `.claude/` + `CLAUDE.md` (Claude Code) or `.cursor/` + `.cursorrules` (Cursor). Empty detection is legal; `--force` provisions even when the marker is absent.
- For each targeted agent, ensures `mcpServers.vibevm` in the per-project config file points at `vibe mcp serve --path <project-root>`. Per-agent paths: `.claude/settings.json` / `.cursor/mcp.json`. Foreign keys (other servers, top-level settings) preserved on merge.
- Idempotent: matching block → `unchanged`; divergent → `updated`; missing → `created`. Decision logic shared between `install` and `--dry-run` previews via a no-IO `decide_action`.
- JSON envelope: `command = "mcp:install"`, `detected[]`, `targeted[]`, `results[]` with per-agent status + note.

`vibe mcp status [--path]`: read-only counterpart, same JSON envelope shape (`command = "mcp:status"`). Useful in CI to assert configs haven't drifted.

12 new tests landed (7 library-side: detect-by-marker-dir, detect-by-CLAUDE.md, parse_filter known/unknown, merge into empty file, merge preserving existing keys, decide_action across created/unchanged/updated; 5 e2e: writes claude settings, idempotent on second run, dry-run produces no file, force provisions absent agent, status reports per-agent state) + 2 help-smoke entries for the new subcommands.

Workspace state: 399 tests (+14 over slice 1's 385). Clippy clean, self-check green. Out-of-scope deferrals: user-level config (`~/.config/claude/...`) and Gemini / Codex / Copilot agents land in follow-up slices.

**M1.7 vibe-mcp slice 1 — Model Context Protocol server crate + CLI plumbing (2026-05-05).** PROP-004's headline gap ("vibevm has no MCP server" — highest-impact item per §5.1) starts landing piece-by-piece. Slice 1 is a self-contained crate with the JSON-RPC 2.0 transport, MCP message shapes, two tools, and full CLI wiring through `vibe mcp serve`. Slice 2 will add agent-config writers (`vibe init` writing `.claude/settings.json` MCP entries based on auto-detected agent) and a per-subskill files-index so `read_subskill` can return precisely the subskill's content rather than the union of the package's files.

- **`vibe-mcp`** crate (`c2977fa`). Transport-agnostic `Server<T: Transport>` — production wires `StdioTransport` (line-delimited JSON-RPC over stdin/stdout, the canonical MCP shape for stdio servers); tests use `MemoryTransport` for deterministic round-trip checks without spawning subprocesses. `Server::dispatch` handles `initialize` (returns `protocolVersion = "2024-11-05"`, `serverInfo`, `capabilities.tools.listChanged = false`), `tools/list`, `tools/call`, `ping`. Unknown methods → JSON-RPC -32601, malformed JSON → -32700. Notifications (no `id`) accepted and silently ignored. Tool registry is `BTreeMap<name, RegisteredTool>` with `register_tool(descriptor, handler)` ergonomics. `ServerContext` reloads the lockfile fresh per tool call so concurrent `vibe install` runs surface without restart.
- **Two tools shipped.** `query_package(name)` returns the full lockfile entry (kind/name/version, content_hash, registry, source_url, source_ref, resolved_commit, files_written, features, subskills_active with delivery+describes, describes PURL, language). `read_subskill(package, subskill_path)` returns the concatenated text of the package's files_written (path-headed) when the named subskill is active. Both surface tool-level errors as `isError: true` payloads (vs. JSON-RPC errors that signal transport failures).
- **`vibe mcp serve`** (`416ac74`). New `Command::Mcp` with `Subcommand::Serve(McpServeArgs)` — enum-of-subcommands leaves room for `mcp config` / `mcp test` follow-ups. `--path` defaults to `.`. End-to-end test `mcp_serve_responds_to_initialize_and_query_package` spawns the binary, drives 3 JSON-RPC messages over stdin (`initialize` → `tools/list` → `tools/call query_package` against the omnibus alpha fixture), parses response lines, asserts protocol version + tool registry shape + lockfile-derived payload (describes/features/subskills_active populated). Same shape Claude Code / Cursor will speak.

Workspace state: 385 tests (+20: 19 vibe-mcp unit + 1 e2e), `cargo clippy --workspace --all-targets -- -D warnings` clean, `tools/self-check.sh` green.

**PROP-003 r2 omnibus integration fixtures + cross-cutting e2e (2026-05-04).** Slices 1–4 each locked one PROP-003 surface in isolation; the omnibus slice proves they actually compose correctly at the byte level. Three new fixture packages committed under `fixtures/registry/`, plus six end-to-end tests in `cli_e2e.rs` exercising every surface in combination. Two real integration bugs surfaced and fixed during the build.

- **Fixtures** (`25b8435`). `flow/integration-alpha/v0.1.0/` is the omnibus: `[package].describes = "pkg:cargo/sqlx@0.8.0"`, `[i18n] available = ["en", "ru"]` with Russian sidecars on `PROTOCOL.md` + boot snippet (and deliberate canonical-fallback for `overview.md`), `[features]` table with default + `extra-discipline` mapping `subskill:feature/extra-discipline`, `[features.exclusive]` group, conditional dep `[target."context(stack:integration-rust)".dependencies]`. Four subskills probing every channel: `feature/extra-discipline` (manual via parent feature, eager), `stack/rust` (`if_present`, lazy-push), `lang/ru-extras` (`if_language`, eager), `sqlx/v08` (subskill-level `describes` + `if_describes_match`, lazy-pull). `flow/integration-beta/v0.1.0/` is alpha's conditional-dep target — provides `interface:trace-discipline` and ships an `if-cargo` subskill via `if_files = ["**/Cargo.toml"]`. `stack/integration-rust/v0.1.0/` is the trigger.
- **Bug fixes uncovered by integration** (`ff38a89`). (1) Multi-root `--features X` aborted on roots that didn't declare X — same shape Cargo silently tolerates. Fix: `tailor_feature_request(request, table)` trims explicit features per package; post-phase-1 visibility warning surfaces if a requested feature matched no root. (2) The slice-4 fixed-point conditional-deps loop's re-fetch path didn't apply the same tailoring, so beta (pulled via conditional dep) inherited the raw `--features extra-discipline` from the original request and aborted. Fix: same call inside the loop. Both bugs would have shipped silently; only the omnibus e2e caught them.
- **Six omnibus e2e tests** (`ff38a89`). End-to-end byte-level verification: lockfile schema_version=3, language_chain=[ru, en], 3 packages total (beta via conditional dep), alpha's describes/language/features/active subskills, delivery modes preserved on round-trip (lazy-push/lazy-pull strings survive), Russian sidecars materialised under canonical target paths (`PROTOCOL.md` carries Russian content), canonical fallback for files without sidecars, beta's `if-cargo` subskill activation state toggling correctly with/without `Cargo.toml` in project root, conditional-dep dormancy without trigger, `--no-default-features` excluding default subskills, uninstall removing every subskill-sourced file, `vibe show features|subskills|purls` JSON envelopes carrying the right shape including subskill-level PURL bindings.

Workspace state: 365 tests (+6 omnibus over slice 4's 359), `cargo clippy --workspace --all-targets -- -D warnings` clean, `tools/self-check.sh` green. Cumulative PROP-003 r2 surface end-to-end proven through the omnibus suite, not just unit-level. Fixtures are LocalRegistry-shaped and drop-in publishable to `https://github.com/vibespecs` once we want a public integration-test harness.

**PROP-003 r2 implementation slice 4 — fixed-point conditional + activation-conflict heuristic (2026-05-04).** Slice 4 closes two follow-ups from slice 3: cascading conditional dependencies (slice 3 was single-pass; slice 4 promotes the expansion to a fixed-point loop with iteration cap) and the static `activation_conflict` check from PROP-003 §2.10 (Jaccard-keyword-overlap heuristic mirroring Tessl's review-rubric "activation distinctiveness" axis without needing an LLM judge).

- **Conditional-deps fixed-point loop** (`91c696f`). The single-pass slice-3 expansion replaced with a loop that re-evaluates predicates after each fetch round. Convergence guarantee: extras only ADD packages monotonically and predicate evaluation is a pure function of `present` + `provides` which only grow, so each iteration either produces no extras (terminates) or expands the graph by at least one package. Iteration cap = 5 with `bail!` past it, listing unconverged extras so authoring bugs surface loud. New e2e test `install_expands_cascading_conditional_dependencies` exercises a 3-level cascade (`flow:cascade-root` → `cascade-mid` → `cascade-leaf` via two predicates each waiting on the previous level).
- **`vibe-check` activation_conflict** (`4724b97`). New `CheckId::ActivationConflict` registered. For each locally-discoverable package, walks every subskill whose `delivery` is `lazy-push`/`lazy-pull`, tokenises each `description`, filters ~30 common English stopwords (the, this, with, when, for, etc.), computes pairwise Jaccard set similarity. Pairs ≥70% flag as warnings. Threshold tuned down from PROP-003's nominal 75% because practical Jaccard on short trigger descriptions saturates in the high-60s for content-equivalent pairs after stopword filtering. Two unit tests pin both polarities (overlapping triggers flag; distinct triggers stay clean).

Workspace state: 358 tests (+3 over slice 3's 355), `cargo clippy --workspace --all-targets -- -D warnings` clean, `tools/self-check.sh` green. End-to-end PROP-003 r2 surface that's landed across slices 1+2+3+4: PURL parser + describes binding (package & subskill), BCP-47 i18n with sidecar resolution, `[features]` table with cargo-shape semantics + `[features.exclusive]`, eight-channel subskill activation (manual + if_present + if_provides + if_files + if_command + if_env + if_describes_match + if_language), three delivery modes (eager working, lazy-* recorded with degraded materialisation pending vibe-mcp M1.7), conditional dependency cascading expansion with iteration cap, lockfile schema v3 with full provenance, `vibe show features` / `subskills` / `purls`, `vibe outdated`, vibe-check `activation_conflict` heuristic. Out-of-scope still deferred: libsolv FFI (Phase A), vibe-mcp lazy-push/lazy-pull runtime (M1.7), LLM-emitted virtual capabilities (Phase F), `vibe outdated --upstream` PURL probes against npm/pypi/cargo.io.

**PROP-003 r2 implementation slice 3 — conditional dependencies + `vibe outdated` (2026-05-04).** Continuing the deep-work session that landed slice 1 + slice 2 earlier today; slice 3 closes two more PROP-003 surfaces and lands the M1.10 roadmap entry.

- **Conditional dependencies — schema + predicate parser** (`3168de0`). PROP-003 §2.6.1's `[target."context(<key>)".dependencies]` lands as a `BTreeMap<String, ConditionalTarget>` field on `PackageManifest`. `ConditionalTarget` carries a `[dependencies]` block in `[requires]`-shape so the same `Vec<PackageRef>` / `Vec<CapabilityRef>` validation runs. New `vibe-resolver::conditional` module: `ConditionalPredicate` enum (today's only variant — `Present(String)` for `context(<key>)` covering capability/pkgref/interface lookups), `parse` accepts whitespace + rejects malformed + flags richer forms (`if_files = '...'`, boolean composition) as `Unsupported` so unrecognised authoring forms surface as typed errors rather than hard install failures, `evaluate(ctx)` checks `ctx.present` and `ctx.provides`. Six unit tests on the parser/evaluator.
- **Conditional-dep runtime in install** (`5d9e98e`). After phase 1 fetch + feature expansion, build a preliminary activation context, walk every package's `conditional_deps`, evaluate predicates, fold matched dependencies into a delta of extra roots. If non-empty, re-solve `(original_roots ∪ delta)` once and fetch newly-introduced nodes. Single-pass — cascading conditional chains (one conditional dep triggering another) defer to a follow-up slice with an explicit fixed-point loop. Predicates that fail to parse log `tracing::warn!` and skip rather than aborting install. The final activation context for plan-time gets rebuilt from the post-expansion graph so subskills in newly-pulled packages can probe against the full set.
- **`vibe outdated`** (`1c35c69`). Read-only registry-side update preview per PROP-003 §M1.10 / Tessl's `tessl outdated`. Walks the lockfile, calls `MultiRegistryResolver::resolve(<pkgref>@Latest)` per package, emits a status table (`text` / `--quiet` / `--json`). JSON envelope: `command = "outdated"`, `update_available` count, per-package `kind` / `name` / `installed` / `latest` / `status`. Per-ecosystem resolution failures degrade to `latest = null`, `status = "unknown"` rather than aborting the whole report. `--upstream` PURL probe (npm/pypi/cargo.io HTTP) deferred to follow-up — needs per-ecosystem clients.

E2E tests new in this slice (3 added, all in `cli_e2e.rs`):

- `outdated_reports_newer_version_available` — builds a per-package git registry with v0.1.0 + v0.2.0 of `flow:test-multi`, installs v0.1.0 pin, runs `vibe outdated --json`, asserts `update_available = 1` plus the `installed`/`latest`/`status` fields per package.
- `install_expands_conditional_dependencies_when_predicate_matches` — registry hosting `flow:dispatcher` (with `[target."context(stack:rust-cli)".dependencies]` pulling `flow:rust-helper`), `flow:rust-helper`, `stack:rust-cli`. Installing `stack:rust-cli` + `flow:dispatcher` together expands the conditional and pulls in `flow:rust-helper`; lockfile records all three.
- `conditional_dependencies_dormant_when_predicate_misses` — installing `flow:dispatcher` alone leaves `flow:rust-helper` out, confirming predicates don't fire when the context misses.

Workspace state: 355 tests (+10 over slice 2's 345), `cargo clippy --workspace --all-targets -- -D warnings` clean, `tools/self-check.sh` green. PROP-003 r2 surface that's landed end-to-end across slices 1 + 2 + 3: PURL parser + describes binding, BCP-47 i18n with sidecar resolution, `[features]` table with cargo-shape semantics + `[features.exclusive]` named groups, four-channel subskill activation (manual + if_present + if_provides + if_files + if_command + if_env + if_describes_match + if_language), three delivery modes (eager working, lazy-* recorded with degraded materialisation pending vibe-mcp M1.7), conditional dependency expansion, lockfile schema v3 with full provenance fields, `vibe show features` / `subskills` / `purls`, `vibe outdated`. Out-of-scope still deferred: libsolv FFI (Phase A), vibe-mcp lazy-push/lazy-pull runtime (M1.7), LLM-emitted virtual capabilities (Phase F), cascading conditional-dep fixed-point loop, `vibe outdated --upstream` PURL probes.

**PROP-003 r2 implementation slice 2 — feature-aware install + subskill materialisation (2026-05-04).** Slice 1 landed the schema + parser + static evaluator earlier today; slice 2 plumbs the runtime layer end-to-end. After this slice, `vibe install --features X,Y --language ru flow:foo` actually works: it expands features per package, walks the subskill tree under each fetched cache, evaluates context probes, materialises eager subskills, and writes the v3 lockfile fields with the full activation trail. Out-of-scope for this slice (deferred): libsolv FFI (Phase A), `vibe-mcp` lazy-push/lazy-pull runtime (M1.7 — manifest mode preserved in lockfile but materialisation degrades to eager), LLM-emitted virtual capabilities (Phase F).

- **`vibe-install`** (`71ba1b2`). `InstallOptions` extended with `feature_expansion`, `activation_context`, `describes`. `plan_install_with_options` gains a fourth phase: walk `<cache>/subskills/<path>/vibe-subskill.toml`, evaluate manual + context-based activation, enforce `[conflicts].subskills`, materialise `delivery=eager` files (lazy-push/lazy-pull degrade with `tracing::warn!`). `WriteKind` extended with `SubskillContent { subskill_path }` and reserved `SubskillBootSnippet`; subskill writes participate in the same boot-prefix uniqueness check as main package boot snippets. `InstallPlan` gains `active_subskills: Vec<ActiveSubskill>` recording path/delivery/describes/matched-channels for downstream consumption. New `register_installed_with_metadata` writes the v3 lockfile fields; old `register_installed` is a back-compat alias.
- **`vibe-cli`** (`e5d5845`). `vibe install` gains `--features` (repeatable + comma-separated, applied to root packages; transitives get default features per cargo's semantics), `--no-default-features`, `--all-features`. The install pipeline split into two phases: phase 1 fetches every graph node and runs `expand_features` per node; phase 2 builds the `ActivationContext` from the full graph (`<kind>:<name>` + capabilities + interface tags + PURL types + project root + language chain), then plans each node with options threaded through. After apply, `register_installed_with_metadata` writes the v3 lockfile fields per package; `[meta].language_chain` and `[meta].active_features` get populated from the cross-package union. Three new `vibe show` subcommands: `features`, `subskills`, `purls` — JSON-aware with `--json` / `--quiet` / text default. Five new e2e tests in `cli_e2e.rs` lock the wiring on bytes (feature → subskill activation, no-default skips default subskills, `if_files` glob activation, `show features` / `show subskills` / `show purls` JSON shape). Help-text smoke extended.

Workspace state: 345 tests (+5 over slice 1's 340), `cargo clippy --workspace --all-targets -- -D warnings` clean, `tools/self-check.sh` green. PROP-003 r2 features that landed end-to-end across slice 1 + slice 2: capability/interface activation, file-glob activation, manual feature → subskill mapping, BCP-47 i18n materialisation, three-mode delivery in the lockfile (with eager working at runtime and lazy-* recorded but deferred), `describes` PURL forwarded from manifest into lockfile, full `vibe show` inspector surface.

**PROP-003 r2 implementation slice 1 — schema + activation + i18n materialisation (2026-05-04).** Manifest, lockfile, resolver, install, CLI, and check layers all gain the parser-and-static-evaluation parts of PROP-003 r2. What does NOT land in this slice: libsolv FFI (Phase A — separate dedicated chunk), full subskill materialisation through `vibe-mcp` (M1.7), feature-aware install lockfile recording (next slice), LLM-emitted virtual capabilities (Phase F, post-M1.5). What DOES land:

- **`vibe-core` schema** (`c6d6e1a`). New modules `manifest::purl` (Package URL parser, npm-`@scope/name`-aware via `rsplit_once('@')`), `manifest::i18n` (BCP-47 sidecar pattern, fallback chain, project preference chain), `manifest::subskill` (`vibe-subskill.toml` with `[subskill]` / `[activation]` / `[recommends]` / `[conflicts]` / `[content]`, `DeliveryMode` enum, static `validation_findings`). Existing types extended: `PackageMeta` gains `describes: Option<Purl>`, `PackageManifest` gains `[features]: FeaturesTable` (with TOML-idiomatic `[features.exclusive]` named-group syntax replacing r1's underscore sigil) and `[i18n]: I18nDecl`, `ProjectManifest` gains `[i18n]: I18nDecl`. **Lockfile schema bumped to v3** — `[meta].language_chain` / `active_features` / `virtual_capabilities`, per-package `features` / `subskills_active` / `describes` / `language`. v2 lockfiles parse transparently and rewrite as v3 on next `vibe install`.

- **`vibe-resolver`** (`05ad417`). Two new modules. `features::expand_features` walks the cargo-shape feature DAG: `feat`, `dep:foo`, `foo/feat`, `foo?/feat`, `subskill:<path>`. Cycles terminate via seen-set; private `_`-prefixed features cannot be activated by name; exclusive groups enforced after expansion. `activation::evaluate` evaluates seven probe channels per subskill (`if_present` / `if_provides` / `if_files` / `if_command` / `if_env` / `if_describes_match` / `if_language`) and returns `ActivationOutcome { active, channels_matched }`. Tiny in-tree glob matcher avoids pulling a heavy crate; PATH probe is Windows-aware (`.exe`/`.cmd`/`.bat` suffixes).

- **`vibe-install`** (`29faf9f`). New `InstallOptions` struct hands `language_chain` into `plan_install_with_options`; legacy `plan_install` aliases through with empty chain (behaviour-preserving for non-i18n packages). When the chain is non-empty, both regular `[writes]` files and the boot snippet source resolve through `i18n::resolve_localised` (exact tag → region-stripped tag → canonical no-suffix). Target paths on the consumer's tree are always canonical — operators see `PROTOCOL.md` not `PROTOCOL.ru.md`, even when the bytes came from a Russian sidecar.

- **`vibe-cli` + `vibe-check`** (`9a08c3f`). `vibe install --language <bcp47>` plumbs through to `InstallOptions`; precedence is CLI flag > project `[i18n].preferred` > `[i18n].available[0]` > canonical, with registry-default `en` always last in the chain. `vibe init` populates default `[i18n]` so new projects parse under v3. Three new `CheckId` variants — `FeaturesGraph` (warn), `SubskillStructure` (error), `I18nCoverage` (mixed) — walk every locally-discoverable package under `packages/`, validate the relevant manifest sections, and surface findings actionably.

End-to-end tests in `cli_e2e.rs` lock i18n on bytes: a fixture flow shipping `PROTOCOL.md` + `PROTOCOL.ru.md` produces Russian content under `--language ru` and English without the flag; requesting Japanese against an English+Russian package falls through to English cleanly. Workspace state: 340 tests (+57 over prior 275), clippy `-D warnings` clean, `tools/self-check.sh` green. Schema migration costs zero — pre-release window, no operators to disrupt.

**PROP-003 r2 — eight architectural improvements after Tessl research (2026-05-04).** Re-read PROP-003 in light of the [PROP-004 Tessl comparative research](research/PROP-004-tessl-comparative-research.md) and folded eight improvements into the design proposal *before* implementation rather than retrofitting them later. Diff at the section level:

- §2.5 expanded with **three delivery modes** (`eager` / `lazy-push` / `lazy-pull`) as a primary axis of the subskill manifest, not a follow-up bolt-on. Mirrors Tessl's "rules eager-push / skills lazy-push / docs lazy-pull" framing — with the difference that vibevm makes the mode a **per-subskill choice**, not a per-content-type one. A single package can ship eager rules + lazy-push workflows + lazy-pull deep references and the consumer sees each at the right moment.
- §2.5.1 subskill manifest grows a required **`description` field** (natural-language activation trigger; required for `lazy-push` / `lazy-pull` subskills). This is Tessl's load-bearing pattern — the agent matches the description against task / files / conversation to decide which lazy-push subskill to load. `vibe review` will score this string under the "activation distinctiveness" axis.
- §2.5.2 context-based activation **broadened** with `if_files`, `if_command`, `if_env`, `if_describes_match` probes alongside the existing `if_present` / `if_provides` / `if_language`. File-system / machine-state / PURL-match triggers cover real-world use cases that don't require explicit capability/interface declarations from package authors.
- §2.5.3 LLM-inferred activation **refactored** from "LLM toggles subskills directly" into "LLM emits virtual capabilities into the dep graph" — same expressive power, but a single audit point at the spec layer (capability emission), and normal `if_present` / `if_provides` channels handle the actual toggle. The lockfile records every emission with `(name, emitter, trace_id, emitted_at)`. Static rules like `[[overrides]] reject_virtual_capability = …` give the consumer veto power over LLM-emitted dimensions.
- New §2.5.6 — **`describes` PURL on subskills** (not just packages). A `flow:wal` package as a whole may not bind to any one library, but its `subskills/sqlx-0.8/` cut binds specifically to `pkg:cargo/sqlx@0.8.0`. Different subskills coexist in the same package, and `if_describes_match` selects the right one for the consumer's actual library version. This is what makes vibevm's version-matched-documentation story stronger than Tessl's tile-only `describes`.
- New §2.6.1 — **Conditional dependencies** (`[target."context(...)".dependencies]`), Cargo-shape but predicated on vibevm's context probes. Distinct from subskill activation: subskill = content shaped to context; conditional dep = packages shaped to context. Choose subskills when content lives naturally inside an existing package; choose conditional deps when bringing in a separately-versioned, separately-authored package makes more sense. Solver evaluates conditional deps after the unconditional SAT solve, then re-solves with the new requirements; convergence guaranteed because each pass only adds requirements.
- §2.4's `__exclusive` sigil **replaced with named-group `[features.exclusive]`** table — TOML-idiomatic, no underscore-namespace dance.
- §2.10 `vibe check` gains an **activation-conflict** check that catches subskill `description` triggers that materially overlap (same package, both `lazy-push` or `lazy-pull`). Threshold 75% keyword-overlap; tightened by LLM-judge mode when available. Mirrors Tessl's review-rubric "activation distinctiveness" axis.

Lockfile schema v3 evolved at the same time: `[meta].virtual_capabilities = [...]` (LLM-emitted with audit trail), `[[package]] subskills_active` entries gain `delivery` field so the materialisation behaviour is reproducible across machines, both `[[package]]` and per-subskill entries gain optional `describes` PURL.

ROADMAP M2.8 retitled "Lazy-push / lazy-pull runtime plumbing" — the manifest-schema parts already land in PROP-003 phase C; M2.8 is now the wiring through `vibe-mcp` (M1.7) so lazy modes actually do something at runtime.

The first-revision text is preserved in place; revision-r2 additions are inline at their natural locations and tagged at the top of the document for future readers.

**PROP-004 Tessl comparative research + roadmap deltas (2026-05-04).** New self-contained research document at [`spec/research/PROP-004-tessl-comparative-research.md`](research/PROP-004-tessl-comparative-research.md) (~700 lines) — full inventory of Tessl's product surface (CLI commands, primitives, file formats, evaluation framework, MCP integration, registry model, workspace/RBAC, security gating, auto-update, GitHub integration), gap analysis vs vibevm with depth on each gap, recommended roadmap entries with priority and crate placement, an inverse list of what vibevm leads on (decentralised registry, content-hashed identity, SAT/feature/subskill model in PROP-003, strict provenance lockfile, manual-test smoke protocol, token-secrecy invariant, self-host capability, spec-corpus-as-runtime-input). Materials sourced verbatim from `https://docs.tessl.io/llms-full.txt` (Tessl publishes their docs in a concatenated LLM-targeted format) — quotes preserved in the doc; §7 of PROP-004 captures the full source URL list with re-fetch procedure so the research stays refreshable.

Created new `spec/research/` subdirectory + index README to separate research backgrounders from per-crate PROPs (in `spec/modules/`) and foundation policy (in `spec/common/`). `spec/modules/README.md` index updated to cross-reference.

ROADMAP gained five new M1.x milestones (M1.7 `vibe-mcp` Claude-native context provider via Model Context Protocol, M1.8 `vibe review` static quality scoring, M1.9 `describes` PURL linkage to upstream packages, M1.10 `vibe outdated`, M1.11 agent auto-detection at `vibe init`) plus four M2.x (M2.7 `--optimize` + multi-model A/B, M2.8 three-mode delivery eager/lazy-push/lazy-pull, M2.9 scenario generation from real commits, M2.10 `vibe search`) plus one M3.1 (security threat-model research). Each entry cross-references the PROP-004 §5.x section that motivates it. Top-of-roadmap status snapshot bumped to 2026-05-04.

Highest-impact gap surfaced: **vibevm has no MCP server** — agent integration today is purely file-system-side (writes `CLAUDE.md` etc., no live query path). Tessl's `query_library_docs` tool is what gives them lazy-pull doc loading at agent runtime. Mapping to vibevm: new `vibe-mcp` crate over stdio, tools `query_package` / `read_subskill` / `list_capabilities` / `materialise_subskill`, composes with PROP-003 §2.5 subskill activation channels. Targeted as M1.7.

**PROP-003 design proposal — dep-model evolution (2026-05-04).** Long-form proposal at [`spec/modules/vibe-resolver/PROP-003-dep-evolution.md`](modules/vibe-resolver/PROP-003-dep-evolution.md) covering four interlocking upgrades: (1) SAT-class solver behind the existing `DepSolver` trait via **libsolv** (BSD-3-Clause — passes the PROP-000 §3 permissive-only license gate; libdnf5 is LGPL and stays out of the dependency tree), keeping `NaiveDepSolver` as the small-graphs fast path; (2) **cargo-tradition features** (`[features]` table, default features, optional deps via implicit features and `dep:`/`?/` syntax, weak feature gating, additive-only invariant, mutual-exclusion sets via `__exclusive`); (3) **subskills** — vibevm-native optional content units inside a package (`subskills/<path>/` subtree with own `vibe-subskill.toml`), with four orthogonal activation modes (manual via parent feature, context-based by present capability, context-based by provided interface tag, LLM-inferred post-M1.5); (4) **BCP-47 sidecar i18n** — `README.ru.md` next to `README.md`, fallback chain region→canonical→hard-error, language preference at CLI/env-var/`vibe.toml`/package levels with the existing precedence model. New construct: **interface tags** (`interface:build-system`) — abstract role declarations distinct from capabilities, used by subskills to auto-activate against any package fulfilling a role. Lockfile schema bumps to v3 (`active_features`, per-package `features`/`subskills_active`/`language` fields, `[meta].language`/`language_fallback`). Phase plan covers six staged slices (A solver swap → B features → C subskills → D i18n → E SAT default → F LLM activation). Reference reading committed under `refs/study/{cargo,dnf,dnf5}/` (gitignored) — `cargo`'s `core/resolver/features.rs` and `core/summary.rs::FeatureValue` for the feature semantics, `dnf5`'s `libdnf5/solv/` and `libdnf5/comps/group/` for the libsolv usage and the comps/group analogue, plus dnf5's weak-deps surface (`Recommends`/`Suggests`/`Supplements`/`Enhances`) which we adopt unchanged at the manifest layer.

This is a **design proposal**, not implementation-locked. Schema changes pre-release per the explicit "no migration burden until release" policy. Implementation lands incrementally over six phases.

**JTD codegen wired end-to-end + first consumer migrated (2026-05-04).** `jtd-codegen 0.4.1` installed under `tools/jtd-codegen/` (per-host README install procedure followed; Windows asset name in README corrected from non-existent `x86_64-pc-windows-msvc.zip` to the actual `x86_64-pc-windows-gnu.zip`). `cargo xtask codegen` reworked to give each `*.jtd.json` schema its own subdirectory under `crates/vibe-wire/src/generated/<stem>/` and synthesise a deterministic top-level `mod.rs` listing each submodule alphabetically — necessary because `jtd-codegen` writes a single `mod.rs` per `--rust-out` and the previous one-call-per-schema layout collapsed all seven schemas onto the last one's output. Cleanup-before-codegen invariant added so a removed schema actually drops its submodule. Seven generated modules committed as source of truth (`init_report`, `install_plan`, `install_report`, `list_report`, `registry_publish_report`, `registry_sync_report`, `uninstall_report`); CI's `cargo xtask check-codegen` will keep them in sync with `schemas/`. First consumer migrated: `vibe init --json` now constructs `vibe_wire::generated::init_report::InitReport` directly instead of a `serde_json::json!{}` blob; the `init_json_output_parses` integration test still passes (parser-based, not byte-based, so the alphabetical key reorder is invisible). Migration of the remaining six consumers is incremental.

**Self-check tooling (2026-05-04).** `tools/self-check.sh` bundles the three tree-shippable invariants (`cargo test --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo run -p vibe-cli -- check --path . --quiet`) behind one entry point; CI wires the same line. Uses `cargo run` for the spec-linter step rather than a cached `target/release/vibe` binary so a stale binary built before a subcommand existed cannot fool the check. `tools/.gitignore` carved `!*.sh` / `!*.ps1` so vendored shell helpers travel with the repo (binaries still excluded).

**Dogfood: `vibe check` clean on vibevm self (2026-05-04).** Added a minimal repo-root `vibe.toml` (`name = "vibevm"`, single `[[registry]] name = "vibespecs" url = "https://github.com/vibespecs"`, no `[[package]]`-installed entries) and an empty-package `vibe.lock` so `vibe check --path .` can run against the project's own spec corpus. All six v0 checks pass: manifest validity (vibe.toml + vibe.lock parse), WAL freshness (mtime under 24h), WAL well-formedness (every canonical heading present — `## current phase` / `## constraints` / `## done` / `## next` / `## known issues`), boot directory (`spec/boot/00-core.md` + `90-user.md`, no `NN` collisions), lockfile/disk consistency (no orphans in `spec/flows/feats/stacks` since vibevm doesn't yet consume packages), REVIEW marker aging (no markers in scope).  Findings count zero across `text` / `--json` / `--quiet` formats. `vibe show config --json` correctly attributes the registry to `vibe.toml`, the env vars to `default`, and the user-config layer to `loaded = false` (no `~/.config/vibe/config.toml` on this machine). Manifest is human-edited, not the result of `vibe init` — DEV-GUIDE §6 documents both the self-check workflow and the warning that `vibe install` against this manifest would land package bytes in `spec/`. Full self-hosting under `packages/` (vibevm consuming vibevm) remains queued for post-M1 per ROADMAP.

**M1.6 Scenario A walked end-to-end against live GitHub (2026-05-04).** First formal walk of `manual-tests/M1.6-mirror-vendor-smoke.md` Scenario A (vendor → file:// mirror → broken-primary rescue install) against `https://github.com/vibespecs` plus a local file:// vendor mirror under tempdir. A1–A3 PASS: three `vibe install` runs land all three flows; `vibe registry vendor --out` produces `flow-wal.git` / `flow-sync-from-code.git` / `flow-atomic-commits.git` bare repos plus README; peeled SHAs from `git ls-remote --tags` against the vendor dir match the GitHub upstream byte-for-byte (`1c3a1355…` / `a620157d…` / `d7651203…`). A5 PASS: with primary URL rewritten to `https://invalid.example/vibespecs`, fresh project install of `flow:wal` succeeds via the file:// mirror; lockfile records `source_url = "https://invalid.example/vibespecs/flow-wal.git"` (the canonical primary URL — mirror is not leaked, per PROP-002 §2.3 step 3); `content_hash = sha256:8136ecdbc25d…` byte-identical to the primary install (cross-source identity). `VIBE_LOG=vibe_registry=info` capture shows three `lookup served by mirror` lines (list_versions / fetch_dep_manifest legs) plus one `fetch served by mirror` (cache-mutating bootstrap), all attributing the canonical primary plus the served-by mirror URL.

A4 surfaced one regression and one doc-bug. (1) `vibe registry set-mirror vibespecs "file:///<vendor-dir>"` is rejected with `cannot derive an organization segment from file:///…` — the mirror-URL validator runs the same `extract_*_segment` org-extractor that `[[registry]]` uses, but a vendor mirror is a *content path*, not an org root, so extraction is structurally meaningless. The same `vibe registry vendor` command that produces the vendor dir suggests this exact `[[mirror]] url = "file://…"` line in its summary, so the CLI contradicts itself. The runtime mirror dispatch path accepts a hand-written `[[mirror]]` block fine (A5 verified that on the same project), so the bug is scoped to the manifest-mutating CLI command, not to the resolver. Workaround documented in the smoke for now (append the `[[mirror]]` block manually). (2) Both smoke scenarios used `RUST_LOG=…` to gate trace capture, but `vibe-cli/main.rs::init_tracing` reads `VIBE_LOG`, not `RUST_LOG` — `vibe show config` already documents this. Smoke patched to `VIBE_LOG=vibe_registry=info` everywhere.

Pass-line for Scenario A recorded as "A1–A3, A5 PASS · A4 needs manual" so future walkers know to expect the regression until it lands. Combined with the prior B1–B4 PASS, M1.6 smoke is now end-to-end walked.

**M1.6 manual-test smoke — Scenario B PASS + registry-vs-mirror policy formalised (2026-05-04).** First formal walk of `manual-tests/M1.6-mirror-vendor-smoke.md` Scenario B (multi-`[[registry]]` priority walk, fully local, two file:// registries built from `fixtures/registry/flow/wal/v0.1.0`). B1–B3 pass: `flow:wal` installs through the `fallback` registry after `primary-empty` returns `UnknownPackage`; lockfile attributes the install to `registry = "fallback"`, `source_url` is the file:// URL composed from the fallback's org root, `content_hash = sha256:8136ecdbc25d4555cbab6e9574f153b252a05c62b55b5e0255def645458c9544` — byte-identical to the GitHub-served `flow:wal@0.1.0`, proving cross-source identity (the same payload at GitVerse, GitHub, and a local fixture all hash to the same `content_hash`).

The first walk surfaced a discrepancy on B4 (primary URL pointed at a dead host): the previous draft of the smoke expected fall-through, but the implementation halts. Triage made the policy explicit in [PROP-002 §2.3.1](modules/vibe-registry/PROP-002-decentralized-registry.md#failure-discriminator) — `[[registry]]` is a *distinct package source* (registry-walk falls through on `UnknownPackage` only, hard-fails on connect-/auth-/server-errors so typos surface), `[[mirror]]` is an *availability copy* (mirror-walk falls through on any availability failure). Two-part fix landed alongside: (a) `fix(vibe-registry): widen connect-failure classifier substrings` (`5c2e3d5`) — `classify_stderr_message` now matches `failed to connect` / `could not connect to` / `connection refused` / `connection timed out` / `operation timed out` so connect-failures land as `NetworkUnreachable` instead of `CommandFailed`; (b) smoke B4 reframed as a hard-fail-by-design check (mis-configured primary halts install, fall-through is `[[mirror]]`'s job — Scenario A). Classifier-aware mirror-walk fall-through on `NetworkUnreachable`/`AuthFailed` is the next slice.

B4 then re-run on the new shape (vibevm `5c2e3d5`): primary `https://invalid.example/empty` → install halts with `unable to reach … (network or DNS error)`, lockfile remains empty, no spurious `fallback` install. Scenario B PASS recorded in the smoke file's pass-line. Scenario A still TBD pending a live walk against `https://github.com/vibespecs`. Walked on Windows 11 / git 2.52.0.windows.1.

**M1.4 user-config runtime injection — LANDED (2026-05-04).** Closes the operator caveat from the previous user-config slice. `vibe-cli/main.rs` gained `promote_user_config_env()` running at the very top of `main` (before dispatcher, before any thread spawn): `UserConfig::load()` is consulted, and every `[env]` entry whose live env-var is unset is written into the process env via `std::env::set_var` — wrapped in a single localized `unsafe` block with a SAFETY comment explaining the single-threaded invariant. The crate-level lint is now `#![deny(unsafe_code)]` instead of `forbid` so that one block can carry an `#[allow(unsafe_code)]` override. The set of names actually promoted is stashed in a `OnceLock<BTreeSet<String>>` so `vibe show config` can distinguish operator-set live env (`provenance = "env"`) from promoted defaults (`provenance = "user-config"`) without re-reading the file mid-run; the parsed `UserConfig` value in `show config` is no longer used for env resolution (just for the summary block) since promotion baked the values in.

End-to-end consequence: every runtime consumer that reads env-vars — `vibe-registry::default_cache_root` (the `~/.vibe/registries/` override), `init_tracing` (the `VIBE_LOG` filter), future LLM-key paths in M1.5 — now picks up user-config defaults transparently. Live env-vars set at invocation time still win (promotion only fires when the var is unset). New e2e test `user_config_promotes_vibe_registry_cache_into_runtime` proves the wiring: a user-config-pointed cache directory gets a real per-package clone after a `vibe install` against a fixture git registry, with no `VIBE_REGISTRY_CACHE` in the live env.

Workspace state: ~266 tests across the workspace (1 new e2e). `cargo clippy --workspace --all-targets -- -D warnings` clean. Reference docs at [`docs/commands/show.md`](../docs/commands/show.md) — the operator caveat removed; runtime-injection section added.

The earlier slices stay in force.

**M1.4 `vibe show` user-config layer — LANDED (2026-05-04).** Closes the remaining gap in the §9.5 precedence chain that `vibe show` v0 left open. New `vibe-core::user_config::UserConfig` reads `~/.config/vibe/config.toml` (with `XDG_CONFIG_HOME` / `%APPDATA%` / `VIBEVM_USER_CONFIG` resolution) into a strictly-typed `[env]` `BTreeMap<String, String>`. `vibe show config` consumes it as the fourth provenance layer: live env-var > user-config > built-in default; sensitive vars (`VIBEVM_PUBLISH_TOKEN`) stay `redacted` regardless of source. `vibe show config --json` gains a `user_config { path, loaded, error? }` block that surfaces the resolved path and parse-failure mode so an operator with a malformed file sees that the layer is silently inert. v0 scope deliberately stops at inspection — runtime consumers (cache root, tracing init) still read live env-vars only; runtime injection is a follow-up.

5 new unit tests in `vibe-core::user_config::tests` (default-empty, missing-file-is-default, parses [env], rejects unknown top-level section, rejects malformed TOML); 3 new e2e tests in `cli_e2e.rs` — `show_config_user_layer_provides_default_for_unset_env`, `show_config_live_env_overrides_user_config`, `show_config_user_token_default_redacts_value` (the token-bytes-never-leak gate against a deliberate misuse where the operator drops a token into the user-config). `cargo clippy --workspace --all-targets -- -D warnings` clean. Reference docs at [`docs/commands/show.md`](../docs/commands/show.md).

The earlier slices stay in force.

**M1.4 `vibe show` v0 — SHIPPED (2026-05-04).** Inspection commands online. Two subcommands ship in v0; the runner-aware ones (`graph` / `node` / `plan`) defer to M1.5 alongside the LLM-build pipeline.

`vibe show effective` materialises the project's full spec corpus as a single deterministic stream — `spec/boot/*.md` sorted by `NN-` prefix first, then `spec/WAL.md`, then per-package `files_written` in lockfile order (with `spec/boot/*` paths skipped to avoid duplicating step 1). Each section is preceded by a `--- spec://… (origin)` provenance header where the origin is `user`, `wal`, or `package:<kind>:<name>@<version>`. The boot snippet attribution comes from each `LockedPackage::boot_snippet` field; user-foundation files (`00-core.md` / `90-user.md`) and any unclaimed boot file fall through to `user`. `--json` emits a structured envelope with `command = "show:effective"` and a `sections[]` array carrying `spec_uri` / `path` / `origin` / `body`.

`vibe show config` dumps the effective configuration — every `[[registry]]` / `[[mirror]]` / `[[override]]` from `vibe.toml`, plus runtime knobs read from environment variables (`VIBE_REGISTRY_CACHE`, `VIBE_LOG`, `VIBEVM_PUBLISH_TOKEN`). Each entry carries a `provenance` tag: `vibe.toml` for manifest-sourced values; `env` for an env-var-set non-sensitive value; `redacted` for an env-var-set token-shaped value (the raw bytes are NEVER printed — the entry surfaces as `(redacted; set in environment)` per [PROP-000 §20](common/PROP-000.md#token-secrecy)); `default` for unset env vars. User-level `~/.config/vibe/config.toml` is not yet a layer in the precedence chain — that ships when the file format lands.

Workspace state: ~261 tests across the workspace (3 new in `cli_e2e.rs` — `show_effective_emits_boot_files_and_wal_with_provenance`, `show_effective_attributes_installed_package_files` (full install + JSON envelope walk verifies the `package:flow:wal@…` attribution and the spec/flows/wal/ entries land), `show_config_emits_registry_block_with_provenance` (verifies the registry block has `provenance = "vibe.toml"` and `VIBEVM_PUBLISH_TOKEN` always surfaces as `default` or `redacted`, never the raw value)). `every_subcommand_renders_help` smoke covers `show`, `show effective`, `show config`. `cargo clippy --workspace --all-targets -- -D warnings` clean. Reference docs at [`docs/commands/show.md`](../docs/commands/show.md).

The earlier slices stay in force.

**M1.3 `vibe check` v0 — SHIPPED (2026-05-04).** Spec-consistency linter activated. `vibe-check` crate fleshed out from its M0 stub with six of the ten checks listed in `VIBEVM-SPEC.md` §12: `manifest_validity` (vibe.toml + vibe.lock parse against the v2 schema), `wal_freshness` (WAL mtime under `--wal-max-age-hours`, default 24), `wal_wellformed` (canonical `## current phase` / `## constraints` / `## done` / `## next` / `## known issues` sections present, parenthetical-suffix-tolerant matching), `boot_directory` (every `spec/boot/<file>` matches `NN-name.md`, no two files share an `NN` prefix), `lockfile_files` (every locked entry's `files_written` exists on disk; orphan files in `spec/flows|feats|stacks` warn), `review_aging` (`<!-- REVIEW: YYYY-MM-DD ... -->` markers older than `--review-max-age-days`, default 14; placeholder / prose forms silently skipped). Four checks deferred to v1+: dead `spec://` references, orphan `{#anchor}`s, anchor-uniqueness, implementation coverage. `--fix` queued for the same v1+ slot since fixable findings only emerge once the deferred checks come online. Exit code per spec: 0 if no errors, 1 if errors, 0 with warnings only. Reference docs at [`docs/commands/check.md`](../docs/commands/check.md).

Workspace state: ~258 tests across the workspace (15 new in `vibe-check::tests` covering each check + the `parse_iso_date` / `looks_like_date` helpers + the placeholder-skip path; 3 new in `cli_e2e.rs` — `check_clean_project_exits_zero_with_no_findings`, `check_boot_prefix_collision_exits_nonzero`, `check_emits_json_envelope`). `every_subcommand_renders_help` smoke covers `check`. `cargo clippy --workspace --all-targets -- -D warnings` clean.

The earlier slices stay in force.

**M1.2 `vibe update` v0 — SHIPPED (2026-05-04).** Phase B v0 of the registry refactor closed the multi-source surface; today's slice opens M1.2 by landing the lock-aware version-bump pipeline. `vibe update <pkgref>...` and `vibe update --all` re-resolve installed packages against their original root constraints (carried under `[meta].root_dependencies`), fetch new content via the same `MultiRegistryResolver` (mirror dispatch + cross-source `content_hash` gate inherited transparently from install), and emit a per-file diff — Added / Removed / Modified / Identical — before applying. User-edit detection is byte-for-byte against the install-time cache (`.vibe/cache/<kind>/<name>/v<old-version>/`); a divergent on-disk file refuses the update with `UserEditedFile` and a 3-way-diff hint. Dep-graph evolution is refused at this layer (`DependencyShapeChanged` when `[requires]` shape changes); narrow v0 holds the line for the version-bump-only contract. Lockfile entry rewritten in place: `version`, `content_hash`, `source_url`, `source_ref`, `resolved_commit`, `boot_snippet`, `files_written`. `dependencies` and `overridden` preserved.

Workspace state: ~239 tests across the workspace (6 new in `vibe-install::tests` covering classify-Added/Removed/Modified/Identical, refuse-on-UserEdit, refuse-on-OldCacheMissing, refuse-on-DependencyShapeChanged, refuse-on-NotInstalled, full apply_update + register_updated round-trip; 3 new in `cli_e2e.rs` — `update_bumps_to_new_version_and_diffs_files` (per-package git registry with both v0.1.0 and v0.2.0 tags, install at `^0.1`, rewrite root constraint to `*`, run `vibe update`, verify on-disk diff applied + lockfile bumped), `update_refuses_when_user_edited_file` (CLI-level UserEditedFile gate; user's edit survives), `update_when_constraint_pins_old_version_reports_up_to_date` (constraint `^0.1` keeps install pinned at v0.1.0 even when v0.2.0 is upstream)). `every_subcommand_renders_help` smoke now covers `update`. `cargo clippy --workspace --all-targets -- -D warnings` clean.

Reference docs at [`docs/commands/update.md`](../docs/commands/update.md). Index in `docs/README.md` updated. ROADMAP §M1.2 flipped from queued to shipped (v0).

The earlier M1.6 surface stays in force:

**M1.6 Phase B v0 — SHIPPED (2026-05-03).** Phase A is closed; the registry-management CLI surface, the read-only mirror-dispatch runtime, and now the cache-mutating mirror dispatch with cross-source `content_hash` verification are in. Active commits since the Phase A checkpoint (`9646de9`):

- `1089417 fix(vibe-install): drop uninstalled package from root_dependencies` — regression surfaced by walking `manual-tests/M1.5-gate-v2-per-package-smoke.md` top-to-bottom against the live GitHub host. `unregister_installed` now retains roots whose `(kind, name)` doesn't match the uninstalled package, symmetric with the install merge.
- `152c607 test(manual): record M1.5-gate-v2 smoke pass on GitHub host` — first formal walk of the smoke filled in. Date 2026-05-01, vibevm `1089417`, peeled SHAs `1c3a1355` / `a620157d` / `d76512034`, Windows 11 / git 2.52.
- `8260f83 feat(cli): vibe registry list` + `7c26faf docs(commands): vibe registry list reference` — read-only inspector for `[[registry]]` / `[[mirror]]` / `[[override]]` blocks; reports the host adapter `vibe registry publish` would dispatch to per PROP-002 §2.10.
- `001f364 feat(cli): vibe registry add` + `2c13276 docs(commands): vibe registry add reference` — mutating sibling: append a new `[[registry]]` (or insert as `--position primary`); validates name uniqueness, URL shape via `extract_*_segment`, naming convention, and position. Manifest-only — no host probe, no lockfile mutation.
- `3fa8c01 feat(cli): vibe registry set-mirror` + `54e64f5 docs(commands): vibe registry set-mirror reference` — append a `[[mirror]]` block; named `<OF>` requires the registry to exist, wildcard `*` is accepted even before any registry is configured (forward-compatible).
- `2e9ebf8 feat(vibe-registry): mirror-aware lookups (Phase B v0)` — read-only mirror dispatch landed. `GitPackageRegistry` carries `mirror_urls` (org-level, populated by `MultiRegistryResolver::from_manifest` from `mirrors_for(reg.name)` priority-sorted output). `list_versions` and `fetch_dep_manifest` archive path try primary first, then each mirror; the cache-mutating `fetch` and `refresh_package` paths stay primary-only until cross-source `content_hash` verification lands. The `try_lookup<T, F>` helper centralises the dispatch and returns the **primary's** error on full failure (most informative diagnostic). `tracing::info!` on mirror-served lookups, `tracing::debug!` on per-mirror failures.
- `5d7e751 feat(cli): vibe registry remove` + `1c9adf8 docs(commands): vibe registry remove reference` — closes the registry-management CRUD: drop `[[registry]]` (refuses to orphan named mirrors; wildcard `*` mirrors are unaffected) or `[[mirror]]` (exact `(of, url)` match; warns on hand-edited duplicates).
- `feat(vibe-registry): mirror dispatch on cache-mutating paths` (this slice) — `GitPackageRegistry::fetch` and `refresh_package` walk primary then each `[[mirror]]` URL in priority order, with `bootstrap_or_update_at` handling per-source bootstrap-or-update-then-wipe-on-failure mechanics. The clone-fallback path in `fetch_dep_manifest` (used when the host disables `git archive` — GitHub case) inherits the same primary-then-mirror walk via `refresh_package`. `tracing::info!` on mirror-served fetches, `tracing::debug!` on per-source failure with full URL context. `cached.source_uri` is **always** the canonical primary URL — mirrors are an availability detail, never a lockfile-recorded identity (PROP-002 §2.3 step 3).
- `feat(vibe-registry): cross-source content_hash verification` (same slice) — new `GitPackageRegistry::fetch_with_expected_hash(resolved, cache, Option<&str>)` and `MultiRegistryResolver::fetch_with_expected_hash` walk primary-then-mirrors and, when an expected hash is supplied (typically the lockfile pin), gate each source: a source serving disagreeing bytes triggers a `tracing::warn!` ("source served content with unexpected content_hash; falling through to next source"), the local clone is wiped between attempts so a poisoned source cannot leave residue, and the walk continues. If every source disagrees, the **last** successful fetch's `CachedPackage` is returned (with the disagreeing hash); `vibe-install`'s `plan_install` then renders the `ContentDrift` user-actionable error against the lockfile pin — registry-layer concerns (sources, fallback) stay separated from install-layer concerns (lockfile-aware error rendering). `expected_hash = None` (no pin yet — fresh `(kind, name)`) is the equivalent of the existing single-source fetch.
- `feat(install): forward lockfile pin into mirror-aware fetch` (same slice) — `vibe-cli/install.rs` looks up the lockfile pin (`lockfile.find(node.kind, &node.name).map(|p| p.content_hash.clone())`) and threads it through `InstallResolver::resolve_and_fetch(pkgref, cache, expected_hash)` into `MultiRegistryResolver::fetch_with_expected_hash`. Local-directory registry path ignores the hint — there's only one source there, and `plan_install`'s integrity check still applies. Architecture diagram in `docs/architecture.md` updated.
- `feat(cli): vibe registry vendor` (this slice) — offline mirror generator per [PROP-002 §6](modules/vibe-registry/PROP-002-decentralized-registry.md#phase-b). New `RegistryVendorArgs` (`--out`, `--force`, `--path`) + `RegistrySubcommand::Vendor` dispatch. `run_vendor` walks the lockfile, calls the mirror-aware `refresh_package` to ensure each per-package clone is on disk and at the lockfile-pinned `source_ref`, then copies the clone's `.git/` into `<out>/<naming>(<kind>,<name>).git/` to produce a self-contained bare repo per package. `[[override]]`-served entries and unattributed entries (LocalRegistry / legacy v1) are reported as skipped with a clear reason. Operator content safety: a non-empty `--out` is a hard error without `--force`. The vendor dir gets a generated `README.md` explaining how to wire it as `[[mirror]] url = "file://..."`; the suggested URL is also surfaced in `--json` output (`suggested_mirror_url`). `bare_clone_from_clone` is a Rust-native copy of the `.git/` tree — no `git` invocation at vendor time, only at install time when the consumer reads from the mirror. `walkdir` promoted to `[dependencies]` in `vibe-cli/Cargo.toml`. Docs at `docs/commands/registry-vendor.md`; index in `docs/README.md` lists it alongside the other `registry` subcommands.

Workspace state: ~232 tests across the workspace (8 new in `vibe-registry::git_package_registry::tests` for mirror dispatch + cross-source verification; 5 new in `vibe-cli::commands::registry::tests` covering `bare_clone_from_clone` + `file_url_for_dir`; 2 new in `crates/vibe-cli/tests/cli_e2e.rs` — `vendor_produces_bare_repo_per_lockfile_entry` (full e2e: install from per-package git registry → vendor → `git ls-remote` against vendored bare repo confirms tag preserved → `git clone --branch v0.1.0` from the vendored repo produces the expected payload) and `vendor_refuses_non_empty_out_dir_without_force`). The `every_subcommand_renders_help` smoke now also covers `registry list` / `add` / `set-mirror` / `remove` / `vendor` (previously only `sync` / `publish`). `cargo clippy --workspace --all-targets -- -D warnings` clean.

Phase B v0 effective surface is now: mirror dispatch on read paths (Phase B v0 prior slice), mirror dispatch on cache-mutating paths + cross-source `content_hash` verification (this WAL's earlier slice), and the offline vendor generator (this WAL slice). Mirrors are useful for actual installs, fault-tolerant against primary outages, and integrity-checked across sources; the vendor command produces drop-in `file://`-mirror dirs that close the air-gapped story without touching the resolver.

- `test(manual): M1.6-mirror-vendor-smoke.md` — runnable end-to-end protocol covering Phase B v0's new surface. Two scenarios in one file (≈ 200 lines, well under the 300-line manual-test cap): Scenario A walks `vibe registry vendor → wire as file:// [[mirror]] → break the network primary → re-install (mirror takes over)` against the live GitHub `vibespecs` org plus a local vendor mirror, and asserts (a) mirror dispatch actually fires (`tracing::info!` "fetch served by mirror" capture), (b) lockfile records the **canonical** primary URL as `source_url` even when a mirror served the bytes (PROP-002 §2.3 step 3), and (c) `content_hash` is byte-identical across the two installs (cross-source identity). Scenario B exercises the multi-`[[registry]]` priority walk: an empty `primary-empty` and a `fallback` carrying `flow-wal` at `v0.1.0`, both built from the in-tree `fixtures/registry/flow/wal/v0.1.0` via `git init` + `git tag` + `git clone --bare`; resolver walks them in order; lockfile attributes the install to `fallback`. Scenario B step B4 also pins the discriminator: a hostile-DNS / 4xx primary still translates to `UnknownPackage` and falls through, not a hard error. Index in `manual-tests/README.md` updated.

The smoke is the M1.6 acceptance gate that automated `cargo test` can't reach — it needs a live registry, a real `~/.vibe/`-style cache directory in a tempdir, and human judgement on the `tracing` log shape. First-walk pass-line is TBD until someone runs it top-to-bottom; that's the next blocker on tagging M1.6.

Beyond that: M1.2 (`vibe update`), M1.3 (`vibe check`), M1.4 (`vibe show`) — all open in their original roadmap positions.

---

**M1.1-revision Phase A — DONE (2026-04-29).** Decentralized per-package registry shipped end-to-end on its production host. All three v0.1.0 demo flows (`flow:wal`, `flow:sync-from-code`, `flow:atomic-commits`) live at `https://github.com/vibespecs/flow-<name>` with `v0.1.0` tags; a fresh `vibe init` → `vibe install flow:wal` / `flow:sync-from-code` / `flow:atomic-commits` resolves all three, populates lockfile v2, refreshes per-package clones via `vibe registry sync`. Registry org migrated from GitVerse to GitHub on 2026-04-29 because GitVerse's public REST API does not expose org-scoped repo creation; `GitHubCreator` adapter behind the existing `RepoCreator` trait drives the publish flow against `POST /orgs/{org}/repos`. The vibevm tool source itself stays on GitVerse — only the registry org moves.

**Phase A close-out summary:**

- 6 commits since the prior checkpoint: `docs(spec,guides,manual-tests)` migration policy → `feat(vibe-publish,cli)` GitHub adapter + per-host token loader → `feat(core,cli)` `DEFAULT_REGISTRY_URL` rotation → `fix(vibe-publish)` credential redaction in error messages → `fix(vibe-registry)` clone-fallback + tag-aware update → this WAL checkpoint.
- 3 live publishes performed (`https://github.com/vibespecs/flow-wal`, `flow-sync-from-code`, `flow-atomic-commits`), each tagged `v0.1.0`. Token never displayed in any output, log line, error message, or commit body during the run.
- Cargo workspace stays green: `cargo test --workspace` (~210 tests across the workspace, 30 in `vibe-publish` alone covering host adapter selection, token redaction, scope-violation guards), `cargo clippy --workspace --all-targets -- -D warnings` clean.

**Next milestone:** M1.6 (multi-registry polish — Phase B of the decentralized-registry refactor). M1.5-gate docs landed; M1.2 / M1.3 / M1.4 still open.

The M1.1 monorepo-shaped registry (one `anarchic/vibespecs` repo, `<kind>/<name>/v<ver>/` directories, `[registry]` singleton in `vibe.toml`) was replaced — at the design level — with a decentralized per-package model before any downstream consumer is at risk of being locked into it. Full design lock lives in [PROP-002](modules/vibe-registry/PROP-002-decentralized-registry.md).

What this means architecturally:

- **Packages become standalone repos** under a hosting organization (`git@gitverse.ru:vibespecs`). Default repo naming `<kind>-<name>`. Versions are git tags (`v0.1.0`, `v0.2.0`). No monorepo.
- **`vibe.toml` gains `[[registry]]` array** + `[[mirror]]` + `[[override]]`. Priority-ordered resolve; mirrors are transparent; overrides bypass the resolver for pins. Schema supports the full shape; Phase A runtime exercises one registry, Phase B (M1.6) exercises several live.
- **Identity is `(kind, name, version, content_hash)`** — URL is informational. Mirror-switching and host-migration never churn the lockfile. Integrity check enforced on every fetch.
- **Lockfile schema v2** — `registry`, `source_url`, `source_ref`, `resolved_commit`, `content_hash`, `dependencies`, `overridden` per package; `schema_version`, `solver`, `root_dependencies` in `[meta]`. v1 lockfiles auto-migrate on next write.
- **Transitive depsolver** — `resolvo` crate (BSD-3-Clause, Rust-native, used by Pixi / Rattler at conda scale). `DepSolver` trait leaves a `libsolv` fallback slot. Capability-based deps: `[provides]` / `[requires]` / `[[requires_any]]` / `[obsoletes]` / `[conflicts]` — all semantic, not advisory.
- **Maintainer utility** `vibe registry publish <path>` — creates a package repo through a host adapter (GitVerse in v1), pushes content, tags version. Non-admin error surface tuned (401/403/push-denied/tag-collision all render actionably).
- **JTD + codegen** for wire contracts — GitVerse API client, `vibe --json` events, future LLM provider wrappers. Toolchain project-local under `tools/jtd-codegen/`.
- **Local fixtures relocate** from `packages/` to `fixtures/registry/` — keeps `packages/` free for the future dogfooding path (vibevm using vibevm).

The three live v0.1.0 flows (`flow:wal`, `flow:sync-from-code`, `flow:atomic-commits`) stay at `anarchic/vibespecs` for now — read-only, pointer README forthcoming. Phase A migrates them into per-package repos under `vibespecs/<kind>-<name>` via the new publish utility.

**Standing owner directives** that landed this slice (see [PROP-000](common/PROP-000.md) §15–§19 and [`CLAUDE.md`](../CLAUDE.md)):

- Dependency weight is not a decision factor — pick best-in-class.
- JTD + codegen is the default for wire contracts.
- Production architecture in the prototype phase ("Google-principal lens").
- Complexity expectation ≥ RPM for the dep model.
- Load-bearing setup docs at repo root: [`DEV-GUIDE.md`](../DEV-GUIDE.md), [`RUNTIME-GUIDE.md`](../RUNTIME-GUIDE.md).
- Project facts stay in the project; no project-level state in tool-specific global user-memory.

**Immediate next work (after this checkpoint).** Phase A code adjustments for the host migration land first: new `GitHubCreator` behind `RepoCreator`, host-aware adapter selection in the CLI, per-host token loader (`~/.vibevm/<host>.publish.token` precedence), `DEFAULT_REGISTRY_URL` rotated to `https://github.com/vibespecs`, manual-test rewritten for the GitHub-shape flow. After the workspace stays green (`cargo test --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`), the live publish of `flow:wal@0.1.0` / `flow:sync-from-code@0.1.0` / `flow:atomic-commits@0.1.0` runs against `github.com/vibespecs`. **Non-routine** per CLAUDE.md Rule 4 (creates real public artefacts in the new org), so it requires explicit owner sign-off before push.

**Host migration to GitHub (2026-04-29).** GitVerse's public REST API does not expose an org-scoped repo creation endpoint — `POST /orgs/{org}/repos` returns 404 / WAF 403 against `https://api.gitverse.ru` even with correct auth and Accept headers; only `POST /user/repos` is documented, and there is no documented user-to-org transfer endpoint. Without org-scoped creation `vibe registry publish` cannot drive the publish loop end-to-end on GitVerse without manual web-UI pre-creation per release, which defeats the point of a publish utility. The owner's decision (2026-04-29): keep the **vibevm project repository** on GitVerse (`anarchic/vibevm` — unaffected) and migrate the **package registry organization** to GitHub — `https://github.com/vibespecs`. Identity remains content-hashed per [PROP-002 §2.1](modules/vibe-registry/PROP-002-decentralized-registry.md#identity); no `content_hash` is invalidated by the host change. Full architectural rationale: [PROP-000 §7](common/PROP-000.md#registry) and [PROP-002 §2.10](modules/vibe-registry/PROP-002-decentralized-registry.md#publish).

**GitHub API surface (assumed; live-verified during this slice).** Base URL `https://api.github.com`. Auth: `Authorization: Bearer <T>`. Accept: `application/vnd.github+json`. Versioning header: `X-GitHub-Api-Version: 2022-11-28`. Endpoints used: `GET /repos/{owner}/{repo}` (presence check); `POST /orgs/{org}/repos` (repo creation — works natively, returns 201 with full repo metadata). Push auth: HTTPS via the publish token, embedded into the push URL as `https://x-access-token:<TOKEN>@github.com/vibespecs/<repo>.git` for the duration of `git remote add` / `git push`; modern git ≥ 2.31 redacts URL passwords in its own log output. Adapter source: `crates/vibe-publish/src/github.rs`.

**GitVerse API surface (live-verified 2026-04-26, retained).** Base URL `https://api.gitverse.ru`. Auth: `Authorization: Bearer <T>`. Accept header MUST carry the version: `application/vnd.gitverse.object+json;version=1`. `GET /repos/{owner}/{repo}` works; `POST /orgs/{org}/repos` does not. Findings baked into `crates/vibe-publish/src/gitverse.rs` (commit `36cbf08`); the GitVerse adapter remains in tree for any future Gitea-shape host that fully supports the org-scoped POST.

**Token convention (per PROP-000 §20).** Publish-token loader walks: `VIBEVM_PUBLISH_TOKEN` env → `~/.vibevm/<host-prefix>.publish.token` (`github.publish.token`, `gitverse.publish.token`) → legacy `~/.vibevm/git.publish.token`. CLI prints token *source* only; value never appears in any vibevm-produced output. Adapter scope: each `RepoCreator` impl refuses operations outside the org named in the project's `[[registry]].url`.

**JTD toolchain.** Scaffolding is in place (`tools/jtd-codegen/`, `xtask`, `schemas/`, `crates/vibe-wire/`); the `jtd-codegen` binary itself needs a one-time install per `tools/jtd-codegen/README.md` before the first `cargo xtask codegen` run. Migration of existing hand-rolled `Serialize` structs to JTD-driven types is incremental and lands as the consumers are touched.

## Constraints (do not violate without discussion)

- **Language:** Rust only for the CLI. See [spec://vibevm/common/PROP-000#language](common/PROP-000.md#language).
- **License:** proprietary EULA placeholder (see [`LICENSE.md`](../LICENSE.md)); eventual target is UPL 1.0 — owner's decision. See [spec://vibevm/common/PROP-000#license](common/PROP-000.md#license). Third-party deps: permissive only (MIT / Apache-2.0 / BSD / Unlicense; MPL-2.0 case-by-case; GPL / AGPL / LGPL forbidden).
- **Manifest format:** TOML for human-edited configs (`vibe.toml`, `vibe.lock`, `vibe-package.toml`); JTD+codegen for wire contracts ([PROP-000 §16](common/PROP-000.md#jtd)).
- **Vocabulary lock:** only `flow`, `feat`, `stack`, `tool`. Never `lifecycle`, `phase`, `goal`, `plugin` (except as passing synonym for `package`).
- **User-owned files** (`vibe install`/`uninstall` never modifies): `spec/boot/00-core.md`, `spec/boot/90-user.md`, `spec/WAL.md`, `VIBEVM-SPEC.md`, `refs/book/**`, any 00-09 or 90-99 boot file.
- **Four project rules** authoritative in [spec://vibevm/common/PROP-000#commits](common/PROP-000.md#commits), copied into `CLAUDE.md` / `AGENTS.md` / `GEMINI.md`: (1) attribution — human-authored; (2) Conventional Commits; (3) group by meaning; (4) autonomy on routine changes only.
- **Memory discipline** pinned in `CLAUDE.md` (and copies): project facts go into the repo (`CLAUDE.md`, `MEMORY.md`, `TASKS.md`, `spec/**`); tool-specific global user-memory holds only machine-local facts.
- **Setup doc obligation** ([PROP-000 §19](common/PROP-000.md#setup-docs)): any change to toolchain / prereqs / env / paths updates `DEV-GUIDE.md` or `RUNTIME-GUIDE.md` in the same commit.
- **Dependency weight** not a decision factor ([PROP-000 §15](common/PROP-000.md#dep-weight)) — pick best library, reject only on license / abandonment / security / bad API.
- **Architect with production lens** ([PROP-000 §17](common/PROP-000.md#prod-arch)): load-bearing surfaces (lockfile, registry protocol, dep-resolver, wire formats) ship production-quality even in prototype phase.
- **Complexity expectation ≥ RPM** ([PROP-000 §18](common/PROP-000.md#complexity)): capability-based, virtual-package-aware, disjunction-supporting dep model from day one.
- **Git backend:** shell-out to system `git`, behind `GitBackend` trait (PROP-001 §2.1 — size argument pruned per PROP-000 §15; Windows SSH-auth and diagnostic clarity still carry the call).
- **Cache root:** `~/.vibe/registries/<canonical-url-hash>/packages/<kind>-<name>/` per PROP-002 §2.6. `VIBE_REGISTRY_CACHE` env-var overrides.
- **Registry default in `vibe init`.** New projects scaffold `[[registry]] name = "vibespecs" url = "https://github.com/vibespecs"` — ORG root on GitHub (not a package repo). Single source of truth: `vibe_core::manifest::DEFAULT_REGISTRY_URL`. Override with `--registry-url <URL>` / `--registry-ref <REF>`; opt out with `--no-registry`.
- **Manual-test protocol:** runnable smoke-tests in [`manual-tests/`](../manual-tests/), one file per scenario, clean-slate setup + teardown. Policy in [PROP-000 §14](common/PROP-000.md#manual-tests).
- **REVIEW marker discipline:** when the spec is silent, pick the conservative interpretation, mark with `<!-- REVIEW: … -->`, surface in the session report.
- **`refs/` not committed.** Upstream reference material (book + cloned study repos).

## Remotes

- **vibevm source (this repo):** `git@gitverse.ru:anarchic/vibevm.git` (SSH) / `https://gitverse.ru/anarchic/vibevm` (web). **Stays on GitVerse.**
- **Package registry (target as of 2026-04-29):** organization `vibespecs` on **GitHub** — `https://github.com/vibespecs/<kind>-<name>` per package. Phase A populates it via `vibe registry publish` driving the new `GitHubCreator` adapter.
- **Legacy package registry (read-only transition):** `git@gitverse.ru:anarchic/vibespecs.git`. Holds three v0.1.0 flows in monorepo form (HEAD `2203239`, 2026-04-23). No new publishes here; superseded by the GitHub-hosted per-package repos during Phase A; kept readable for existing projects with schema-v1 lockfiles until they migrate.
- **Publish tokens (local).** Per-host file precedence: `~/.vibevm/<host>.publish.token` (e.g. `github.publish.token` for github.com, `gitverse.publish.token` for gitverse.ru) → legacy `~/.vibevm/git.publish.token`. Env-var `VIBEVM_PUBLISH_TOKEN` overrides everything. Token secrecy invariant per [PROP-000 §20](common/PROP-000.md#token-secrecy) — never displayed, never persisted outside `~/.vibevm/`, never committed. Verified by the owner as having `repo:create` (GitHub) / equivalent rights in the `vibespecs` organization.

## Done

### M0 — walking skeleton (complete, published)

- [x] `VIBEVM-SPEC.md` received (v1.0), book and reference sources read.
- [x] Project rules landed in `CLAUDE.md` / `AGENTS.md` / `GEMINI.md` and [PROP-000 §12](common/PROP-000.md#commits).
- [x] `git init`, `.gitignore`, `LICENSE.md`.
- [x] Boot snippets, PROP-000 foundation.
- [x] Cargo workspace with 7 crates.
- [x] Full plan / apply / register / uninstall loop against a local-directory registry. 64 tests green at M0 tag.

### M1.1 — monorepo git-backed registry (shipped 2026-04-22, now partially superseded by M1.1-revision)

- [x] [PROP-001](modules/vibe-registry/PROP-001-git-backend.md), `GitBackend` trait + `ShellGit`, `Registry` trait, `LocalRegistry` + `GitRegistry`, normalized-URL hash cache at `~/.vibe/registries/<hash>/`, 1-hour freshness TTL, `git+<transport>://…` lockfile source URIs.
- [x] End-to-end test `install_from_git_registry`; live smoke [`M1.1-git-registry-smoke.md`](../manual-tests/M1.1-git-registry-smoke.md).
- [x] `vibe init` writes `[registry]` pointing at the default registry.
- **Partially superseded:** cache layout (§2.4), Registry trait shape (§2.3), lockfile `source_uri` format (§2.6) replaced by PROP-002. GitBackend / ShellGit / freshness / Windows UX remain authoritative.

### M1.5-gate content — three v0.1.0 demo flows (published 2026-04-22 / 2026-04-23 on the legacy monorepo)

- [x] `flow:wal@0.1.0` at vibespecs `98e51fc` — canonical flow, boot-snippet prefix `10-`.
- [x] `flow:sync-from-code@0.1.0` at vibespecs `47582af` — prefix `20-`.
- [x] `flow:atomic-commits@0.1.0` at vibespecs `2203239` — prefix `30-`.
- [x] Live multi-package smoke [`M1.5-gate-multi-package-smoke.md`](../manual-tests/M1.5-gate-multi-package-smoke.md) passed 2026-04-23 against monorepo registry.
- **Now:** these three flows are the live-migration target of M1.1-revision Phase A — they move into per-package repos `vibespecs/flow-wal`, `vibespecs/flow-sync-from-code`, `vibespecs/flow-atomic-commits` via the new publish utility.

### M1.1-revision documentation slice (landed 2026-04-24, this session)

- [x] [PROP-000](common/PROP-000.md) §15–§19 — dep-weight, JTD, production-architecture lens, complexity ≥ RPM, load-bearing setup docs.
- [x] [`CLAUDE.md`](../CLAUDE.md) / [`AGENTS.md`](../AGENTS.md) / [`GEMINI.md`](../GEMINI.md) — "Memory discipline: project facts stay in the project" section.
- [x] [`DEV-GUIDE.md`](../DEV-GUIDE.md) and [`RUNTIME-GUIDE.md`](../RUNTIME-GUIDE.md) at repo root, minimal skeletons.
- [x] `VIBEVM-SPEC.md` §7.3 (capability-based deps), §7.4 (lockfile v2), §7.5 (`[[registry]]` / `[[mirror]]` / `[[override]]`), §8.1 (decentralized registry frame), §8.2 (per-package layout), §8.3 (canonical-URL-rooted cache + `ls-remote` / `git archive` optimisations), §8.4 (maintainer publish utility), new §8.6 (depsolver), §11.2 revision note, §16 M1 acceptance expanded.
- [x] [PROP-001](modules/vibe-registry/PROP-001-git-backend.md) — "Superseded parts" block identifying §2.3 / §2.4 / §2.6 as revised by PROP-002; size-based argument in §2.1 pruned per PROP-000 §15.
- [x] [PROP-002](modules/vibe-registry/PROP-002-decentralized-registry.md) — full design lock for the decentralized registry refactor.
- [x] [`ROADMAP.md`](../ROADMAP.md) — M1.1-revision active section, M1.6 (multi-registry polish) queued.
- [x] [`TASKS.md`](../TASKS.md) at repo root — live checklist for the current slice.

## Code slice landed (2026-04-24 → 2026-04-25)

The full Phase A code slice is in. Each item below is one or more
shipped commits on `origin/main`; cross-reference the commit log for
specifics. Total workspace state: 169+ tests green, clippy clean
with `-D warnings` across the workspace, six new crates / modules
since the documentation checkpoint:

- **`chore(git): pin line endings to LF`** — `.gitattributes` everywhere; content_hash is OS-stable.
- **`feat(core): capability-based package dependencies`** — `CapabilityRef`, `[provides]`/`[requires]`/`[[requires_any]]`/`[obsoletes]`/`[conflicts]` typed and serde-wired; legacy `[dependencies]` migrates transparently.
- **`feat(core): vibe.toml schema v2`** — `[[registry]]` array + `[[mirror]]` + `[[override]]`; singleton legacy form auto-migrates on read; `NamingConvention` enum with three forms.
- **`feat(core): vibe.lock schema v2`** — `schema_version`, `solver`, `root_dependencies` in `[meta]`; `registry`/`source_url`/`source_ref`/`resolved_commit`/`dependencies`/`overridden` per package; serde alias on `source` reads v1 transparently.
- **`feat(registry): shallow ShellGit primitives`** — `list_tags` (via `git ls-remote --tags`, peeled-form deduped) + `fetch_file_at_ref` (via `git archive`, in-process tar extraction).
- **`feat(registry): GitPackageRegistry`** — per-package repo addressing through `NamingConvention`, tag-based versions, lazy clones, `fetch_dep_manifest` reads manifest without cloning.
- **`feat(registry): MultiRegistryResolver`** — priority + override + mirror schema; identity verification on overrides; `mirrors_for(name)` accessor for Phase B; `refresh_lockfile_clones` for `vibe registry sync`.
- **`refactor(registry): provenance through CachedPackage`** — `registry_name`/`source_ref`/`resolved_commit`/`overridden` flow from registry into lockfile.
- **`feat(install): switch CLI to MultiRegistryResolver`** — `git+` prefix stripping at backend boundary; e2e test rewritten for per-package fixture.
- **`feat(registry): per-package vibe registry sync`** — walks lockfile, refreshes per-package clones; legacy / override / unattributed entries reported correctly.
- **`feat(vibe-resolver): DepSolver trait + NaiveDepSolver`** — DFS solver with capability/obsoletes/conflicts/disjunction handling; `MultiRegistryProvider` and `LocalRegistryProvider` adapters; resolvo / libsolv slots reserved.
- **`feat(install): transitive install via NaiveDepSolver`** — `vibe install` now drives the solver end-to-end; lockfile `dependencies` populated with exact pins; `[meta].root_dependencies` carries user-typed roots.
- **`feat(vibe-publish): RepoCreator + GitVerseCreator + vibe registry publish`** — Gitea-compatible HTTP client (reqwest+rustls); `Token` redaction; `Publisher` orchestrator; CLI subcommand with `--dry-run`. Live API verification deferred to first real publish.
- **`build(tools): JTD codegen scaffolding`** — `xtask` crate, `tools/jtd-codegen/` README + gitignore, first JTD schema, `crates/vibe-wire/` placeholder, `.cargo/config.toml` alias.
- **`chore(fixtures): relocate packages/ → fixtures/registry/`** — `git mv`, history preserved; `packages/` reserved for future dogfooding.
- **`test(manual): M1.5-gate-v2-per-package-smoke.md`** — protocol for the live three-package smoke against the new `vibespecs` org. Fill in "Last known pass" on first successful run.
- **`feat(vibe-publish): correct GitVerse API surface from live probing`** (commit `36cbf08`, 2026-04-26) — base URL `api.gitverse.ru`, Bearer auth, versioned Accept header, dry-run UX fix on the publisher. Live API discovery findings documented inline in `gitverse.rs` doc-comment so future readers don't re-walk the rabbit hole.
- **`docs(claude,agents,gemini): session-end checkpoint command spec`** (2026-04-26) — `ЗАВЕРШИ СЕССИЮ` / `END SESSION` and variants now drive a defined wind-down: overwrite `CONTINUE.md`, update this WAL, commit + push, emit TL;DR. Section lives at the bottom of all three boot files (kept byte-identical).
- **`docs(continue): cold-resume checkpoint at root`** (2026-04-26) — comprehensive `CONTINUE.md` written so any next session can pick up Phase A from cold without re-deriving GitVerse API findings, repo map, or decision history.

### Phase A close-out — live migration to GitHub (2026-04-29)

- **`docs(spec,guides,manual-tests): migrate registry org to GitHub`** (`72dae08`) — PROP-000 §7 split-host posture (vibevm source on GitVerse, registry org on GitHub), PROP-000 §20 token-secrecy invariant, PROP-002 §2.10 host-adapter selection + `RepoCreator::push_url` + per-host token loader, WAL/boot 90-user/ROADMAP/RUNTIME-GUIDE/DEV-GUIDE/docs/commands updates, manual-test rewritten for the GitHub host.
- **`feat(vibe-publish,cli): GitHub host adapter and per-host token loader`** (`ab0a3d4`) — `GitHubCreator` against `https://api.github.com` with the canonical `Accept: application/vnd.github+json` and `X-GitHub-Api-Version: 2022-11-28` headers, scope-guarded `RepoCreator::expected_org` / `validate_scope`, `creator_for_url(...)` factory, per-host token-file precedence (`~/.vibevm/github.publish.token` first, legacy `git.publish.token` last), CLI host-aware adapter selection.
- **`feat(core,cli): rotate DEFAULT_REGISTRY_URL to GitHub vibespecs`** (`39a2152`) — single-source-of-truth constant moves to `https://github.com/vibespecs`; default registry name from `default` to `vibespecs`.
- **`fix(vibe-publish): redact credentials from git error messages`** (`6e1bb3a`) — `redact_credentials(s)` helper closes a leak vector where `args.join(" ")` and `clone_url.to_string()` baked credentialed push URLs into `PublishError::Git` / `PushDenied` / `HostUnreachable` / `TagCollision` variants. Six unit tests pin the redaction.
- **`fix(vibe-registry): clone fallback and tag-aware update for GitHub`** (`86dfae3`) — two latent M1.1-revision bugs surfaced by GitHub: `git archive --remote` is not exposed by GitHub (returns HTTP 422 + flush-packet), so `fetch_dep_manifest` now falls back to a per-package shallow clone on `ArchiveUnsupported`; `update()` couldn't reset to a tag because `origin/<tag>` doesn't exist as a remote-tracking branch, so it now fetches with `--tags` and tries `refs/tags/<ref>` before `origin/<ref>`.
- **Live migration applied (3 publishes):** `https://github.com/vibespecs/flow-wal`, `flow-sync-from-code`, `flow-atomic-commits` each tagged `v0.1.0`. Token loaded from `~/.vibevm/github.publish.token`, never displayed. End-to-end smoke verified: anonymous `vibe init` → install all three → lockfile v2 with `registry = "vibespecs"` / GitHub `source_url`s / `content_hash`s populated; `vibe registry sync` refreshes 3, skips 0; `vibe list` shows three packages.

## Next

**Forward queue (2026-05-05 session-end snapshot).** Sorted by smallness × payoff. Detailed write-up in [`CONTINUE.md`](../CONTINUE.md).

1. **M1.8 — `vibe review` static quality scoring.** New `vibe-eval` crate, three-axis rubric (validation / implementation / activation), no LLM dependency at this level. ~1 weekend.
2. **M2.10 — `vibe search` registry inspector.** Walks every configured `[[registry]]` URL. Naive at first; indexing later. ~1 weekend.
3. **`vibe update` feature-awareness.** Mirror `plan_install_with_options` into `plan_update_with_options`. ~1 weekend; closes a known gap.
4. **vibe-mcp follow-ups.** Gemini / Codex / Copilot agent writers, `list_capabilities` / `query_capabilities` discovery tool, user-level config (`~/.config/claude/...`).
5. **Documentation files.** `docs/commands/{features,subskills,purls,outdated,mcp-serve,mcp-install,mcp-status}.md`. Mechanical translation of `--help` text.
6. **M1.5 — LLM provider abstraction + `vibe build`.** Big, non-routine — needs explicit owner sign-off per CLAUDE.md Rule 4 before starting. 3-6 weekends. Once `vibe-llm` is real, M2.7 (`--optimize` + multi-model A/B) and M2.9 (scenario gen from real commits) light up.
7. **libsolv FFI / `SatDepSolver`** (PROP-003 §2.1, Phase A). 2-3 weekends; standalone slice.

**Historical Phase A close-out follow-ups (still open).**

- Smoke-test Last-known-pass line in [`manual-tests/M1.5-gate-v2-per-package-smoke.md`](../manual-tests/M1.5-gate-v2-per-package-smoke.md) — the manual protocol still says "TBD" since the in-session smoke ran an automated bash equivalent, not the full markdown protocol.
- Schedule a recurring agent to verify the `vibespecs` org on GitHub stays reachable and `v0.1.0` tags don't drift (peeled SHAs as of 2026-04-29: `flow-wal` `1c3a1355`, `flow-sync-from-code` `a620157d`, `flow-atomic-commits` `d76512034`).

Comprehensive cold-resume document (long form, with repo map, decision history, exact recipes) lives at [`CONTINUE.md`](../CONTINUE.md). It is written by the session-end checkpoint command (`ЗАВЕРШИ СЕССИЮ` / `END SESSION`) and supersedes itself wholesale on each invocation; if it disagrees with this WAL, trust the WAL.

**Beyond Phase A.** M1.6 polishes multi-registry / mirror dispatch / `vibe vendor` per [PROP-002](modules/vibe-registry/PROP-002-decentralized-registry.md#phase-b). M1.5-gate docs (`docs/commands/*.md`, `docs/authoring-{flow,feat,stack}.md`) all landed.

## Known issues

- **Legacy lockfile v1 auto-migration UX.** Every project with an existing `vibe.lock` from M1.1 will see a migration notice on next `vibe install`. Behaviour benign (resolution unchanged); message must be actionable, not noisy.
- **Line-ending warnings** on every commit — `.gitattributes` with `* text=auto eol=lf` side-quest still open.
- **Registry cache locking** — two concurrent `vibe` invocations can race on the same per-package clone directory. Noted in PROP-001 §6 as M2 hardening; behaviour today: if a clone fails, delete the cache dir and retry.
- **Path display on Windows** strips `\\?\` UNC prefixes; lockfile stores forward-slash relative paths (portable).

## Session context

- **Entry point for next session:** read `CLAUDE.md`, then this WAL, then [PROP-000](common/PROP-000.md) and [PROP-002](modules/vibe-registry/PROP-002-decentralized-registry.md); consult [`TASKS.md`](../TASKS.md) for the current queue. The remaining Phase A item is the live migration — see "Next" above for the procedure.
- **Do NOT touch:** `VIBEVM-SPEC.md` (owner-frozen — the approved PROP-002-driven amendments landed in the documentation slice; any further edit needs a new owner sign-off), `refs/book/**`, `spec/boot/00-core.md`, `spec/boot/90-user.md`, any `fixtures/registry/flow/<name>/v0.1.0/` snapshot (canonical test payloads — changes must be a new version).
- **Key commands to know:**
  - `cargo test --workspace` — 169+ tests green on `main` at checkpoint.
  - `cargo clippy --workspace --all-targets -- -D warnings` — clean.
  - `cargo xtask codegen` — regen JTD-derived Rust types (requires `tools/jtd-codegen/` install per its README).
  - `cargo xtask check-codegen` — drift check; CI uses this once a schema is wired into a real consumer.
  - `cargo run -p vibe-cli -- init --path <dir>` — scaffold a project.
  - `cargo run -p vibe-cli -- install flow:wal --path <project>` — transitive resolve via `NaiveDepSolver`, populated lockfile v2 entry.
  - `cargo run -p vibe-cli -- registry publish <path> [--registry <name>] [--dry-run]` — publish a package (maintainers; reads token from `~/.vibevm/<host>.publish.token`, value never echoed).
  - `cargo run -p vibe-cli -- registry sync --path <project>` — refresh per-package clones referenced by the lockfile.
