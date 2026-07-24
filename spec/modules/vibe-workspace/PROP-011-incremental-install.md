# PROP-011: Incremental install — skip resolution when fresh, materialise only the diff {#root}

<status stage="impl" state="done" comment="B0 2026-07-24: SHIPPED 2026-05-22"/>

##milestone-line **Milestone:** M1.21 ([`ROADMAP.md`](../../../ROADMAP.md)) — **shipped 2026-05-22.** Refines the install machinery of [PROP-009](PROP-009-loading-model.md) (M1.18); no dependency on PROP-008 or PROP-010. @impl/done

##status-line **Status:** SHIPPED 2026-05-22 — the three §5 design questions were resolved in an owner session, then implemented across the four §7 phases (see §8). @impl/done

##related **Related:** [PROP-009](PROP-009-loading-model.md) (the loading model — `apply_resolution`, `regenerate_boot`, `vibedeps::materialise`, the `vibe install` orchestration this PROP refines; §2.10 `vibe reinstall`); [PROP-007](PROP-007-workspace.md) (workspaces — unified resolution, the matryoshka); [PROP-010](../vibe-registry/PROP-010-local-package-cache.md) (the local cache — skip-when-fresh makes the common path offline-clean for free, §2.6 there). @spec/done

##OWNER-SANCTION **Owner sanction:** this PROP changes `vibe install`'s observable contract (it becomes lockfile-respecting — §2.2) and so edits `VIBEVM-SPEC.md` §9.1. The spec edit requires explicit owner sanction — **granted 2026-05-22**; it lands in Phase 4 (§7). @impl/done

---

## 1. Motivation {#motivation}

- ##correctness-first PROP-009 made `vibe install` **correct**: run anywhere in a workspace, it re-resolves the whole graph, re-materialises every `vibedeps/` slot, and regenerates every node's boot artifacts. Correctness-first — "regenerate everything deterministically" is obviously right and self-healing. @impl/done
- ##whole-tree-unconditional It is also **whole-tree, unconditionally**. Every `vibe install` — regardless of what changed, or whether anything dependency-relevant changed at all — re-runs the depsolver (a registry walk, network), and `vibedeps::materialise` does a `remove_dir_all` followed by a full recursive copy of *every* package tree. @impl/done
- ##heavy-cost For a large workspace that is a heavy operation paid on every invocation. @impl/done

- ##iteration-blocked A developer — or, increasingly, an agent — iterating fast inside a large project is blocked by this. @impl/done
- ##authoring-decoupled Most edits do not need `vibe install` at all: PROP-009's boot artifacts are *path manifests*, not content copies, so editing spec content never changes them — authoring is already decoupled from installing. @impl/done
- ##needed-must-be-cheap But when `vibe install` *is* needed (a dependency declaration changed), it must be cheap, and today it is not: it pays whole-tree cost for a one-subtree change. @impl/done

- ##fix-standard The fix is standard package-manager practice. @impl/done
- ##lockfile-oracle `cargo build` and `npm install` treat the lockfile as a freshness oracle: work the lockfile proves unchanged is skipped; only the diff is touched. @impl/done
- ##brings-discipline PROP-011 brings that discipline to `vibe install`. @impl/done

---

## 2. Decisions {#decisions}

### 2.1 Separate resolution from application {#two-phases}

##TWO-PHASES-SPLIT **Decision.** `vibe install` is understood as two phases, optimised independently — the current code conflates them. @impl/done

- ##PHASE-RESOLUTION **Resolution** — the depsolver: read every node's `[requires]`, pick one version per package. It **must stay unified** (one `vibe.lock`, one version per package across the workspace — the diamond problem; PROP-007 §2.4). It cannot be computed per-subtree. But it *can be skipped entirely* when its inputs are unchanged (§2.2). @impl/done
- ##PHASE-APPLICATION **Application** — materialise the resolution into `vibedeps/`, then regenerate boot artifacts. This does **not** have to be whole-tree. It can be a diff: materialise only the slots that changed (§2.3); boot regeneration is cheap and stays whole-tree (§2.4). @impl/done

##UNIFIED-NOT-WHOLE-TREE Resolution being unified does not force application to be whole-tree. PROP-011 keeps unified resolution and makes everything around it incremental. @impl/done

### 2.2 Skip resolution when the lockfile is fresh {#skip-resolution}

##FRESHNESS-CHECK **Decision.** Before running the depsolver, `vibe install` performs a **freshness check**: it compares the resolution inputs — the union of every workspace node's `[requires]` (registry, git, path, and resolved `var` packages) — against what the current `vibe.lock` was generated from. @impl/done

- ##SKIP-WHEN-FRESH If they are unchanged, the depsolver is **not run**: the resolution is exactly what `vibe.lock` already records, and the run proceeds straight to application (§2.3) against the locked versions. @impl/done
- ##FAST-PATH-COST This makes a `vibe install` where no dependency declaration changed cost only: discover the workspace, run the freshness check, apply. No network, no version re-selection — milliseconds even on a large workspace. @impl/done

- ##drift-wart It also fixes an observable wart. Today `vibe install` always re-resolves, so it silently bumps a package within its constraint on every run (a `^0.3` pin drifts to the newest `0.3.x` available). @impl/done
- ##LOCKFILE-RESPECTING With the freshness check, **`vibe install` becomes lockfile-respecting**: unchanged `[requires]` ⇒ the locked versions are honoured verbatim. @impl/done
- ##UPDATE-MOVES-LOCK `vibe update` remains the explicit "re-resolve and pick newer" command. @impl/done
- ##CARGO-CONTRACT-ALIGN This aligns `vibe install` with the `cargo build` / `npm install` contract — *install respects the lock; update moves it* — and makes a build reproducible. @impl/done

- ##HOLD-THE-LOCK When `[requires]` *has* changed, resolution runs, but **holds the lock for every dependency the change did not touch** (§5.3): each registry-resolved root the lock still satisfies is pinned to its locked version, so the re-resolve never drifts an untouched dependency — only the changed one and its subtree move. @impl/done
- ##HELD-PIN-CONFLICT A held pin that conflicts with the change is detected as a depsolver error and falls back to a full, free re-resolve. @impl/done
- ##NO-NEW-FIELD The freshness check itself adds no `vibe.lock` field: the lockfile *is* the baseline, and the check is a `cargo`-style **satisfiability test** of the locked versions against the current `[requires]` — see §5.1. @impl/done

### 2.3 Materialise only the diff {#materialise-diff}

##SLOT-SKIP **Decision.** The materialisation step skips any `vibedeps/` slot that already exists on disk for the resolved `(kind, name, version)`. @impl/done

- ##IMMUTABLE-PREMISE Versions are immutable (PROP-002), so a slot present for the exact resolved version is correct content; the `remove_dir_all` + full recursive copy is pure waste. @impl/done
- ##DIFF-ONLY Only **new** slots and **version-bumped** slots are materialised; slot pruning (PROP-009 FU4 — dropping orphaned slots) is unchanged. @impl/done
- ##one-subtree-win For a `vibe install` that changed one subtree, this turns a re-copy of the whole dependency corpus into a copy of just the handful of slots that actually moved. @impl/done

- ##TRUST-PRESENCE The skip trusts slot-presence-for-a-version as a proxy for correctness — by default it does not re-hash the slot. @impl/done
- ##trust-presence-why That is deliberate: hashing every slot on every install would defeat the optimisation, and the integrity escape hatch already exists (`vibe reinstall --force`, §2.5, re-fetches and re-copies unconditionally). @impl/done
- ##SLOT-INTEGRITY-SETTING Whether the fast path additionally verifies a slot's `content_hash` before trusting it is a **configurable strategy** — the `slot_integrity` setting, `trust-presence` by default (§5.2). @impl/done

### 2.4 Boot regeneration stays whole-tree — and why that is fine {#boot-regen}

##BOOT-WHOLE-TREE **Decision.** Boot-artifact regeneration (`regenerate_boot` over every node) is **kept whole-tree**. It is not the expensive part, and scoping it is low-value: @impl/done

- ##boot-cheap It is cheap: per node, an in-memory topological sort plus writing `INDEX.md` (a small TOML), the redirects (~1 KB each), and `STATIC.md` only when the node has static dependencies. The cost is `O(nodes)` small operations, not the corpus-sized I/O of materialisation. @impl/done
- ##boot-git-quiet It does not churn git: git tracks content, so regenerating a byte-identical `INDEX.md` produces no diff. @impl/done
- ##boot-self-healing Whole-tree regeneration is **self-healing**: a boot artifact left stale by an earlier bug is silently corrected on the next install. A scoped regeneration would preserve such staleness. @impl/done

- ##SCOPING-OUT-OF-SCOPE Scoping boot regeneration to the affected set — a changed node plus its ancestors, the shape PROP-009 §2.10 already specifies for `vibe reinstall` — is *possible* but **out of scope by owner decision**: it is the cheap phase, and effort belongs on §2.2 and §2.3. @impl/done
- ##scoping-option-recorded It is recorded here only so the option is not lost: should a workspace ever grow large enough that `O(nodes)` small writes genuinely matter, §2.10's node-plus-ancestors shape is the ready answer. It is not a deliverable of this PROP. @spec/done

### 2.5 Bypasses — no new flag {#force}

##NO-NEW-FLAG **Decision.** PROP-011 adds **no force flag to `vibe install`**. The two skips (§2.2, §2.3) each already have an explicit, named bypass: @impl/done

- ##BYPASS-UPDATE to re-resolve even though `[requires]` is unchanged — `vibe update` (re-resolves and may pick newer versions; PROP-009 §2.7 / FU3); @impl/done
- ##BYPASS-REINSTALL to re-materialise even though the slots are present — `vibe reinstall --force` (re-fetches from source and overwrites `vibedeps/`; PROP-009 §2.10). @impl/done

- ##BYPASSES-MAKE-SAFE The skips are safe precisely *because* these bypasses exist. @impl/done
- ##no-redundant-flag Keeping them as the bypass — rather than adding `vibe install --force` — avoids a redundant flag and keeps each command's job distinct. @impl/done

### 2.6 In-workspace `file://` sources are mutable {#local-mutable-source}

##IMMUTABILITY-PREMISE **Decision.** §2.2 and §2.3 both rest on **version immutability** — a locked version is correct content, so its resolution can be skipped (§2.2) and its present slot trusted (§2.3). @impl/done

- ##premise-holds That premise holds for a published registry version (PROP-002) and for a content-addressed git ref. @impl/done
- ##IN-WORKSPACE-MUTABLE It is **false for a package resolved from a `file://` source *inside the workspace*** — the in-repo self-hosting registry (`packages/`, `--registry packages`) the author edits in place while *authoring* a package: the source is a working tree whose content changes with no version or `[requires]` edit, exactly like a `path`- or `git`-source dependency (which §2.2 already excludes from the fast path). @impl/done
- ##EXTERNAL-FILE-IMMUTABLE An **external** local registry or mirror — a `file://` path *outside* the workspace — is a static dependency source, not an edited working tree, so it stays immutable and keeps the fast path. @impl/done

##mutable-treatment-lead So an *in-workspace* `file://`-sourced registry dependency is treated as **mutable**: @impl/done

- ##MUTABLE-FRESHNESS **Freshness (§2.2):** it can never be proven fresh cheaply, so the freshness check reports `Stale` for it and the depsolver re-runs — re-reading the source and re-hashing it. This mirrors the existing `path`/`git` handling: the check is "conservative by construction", so a mutable source yields `Stale` and `vibe install` falls back to a full resolution (always correct, and for a local source merely a no-network walk). @impl/done
- ##MUTABLE-MATERIALISATION **Materialisation (§2.3):** its `vibedeps/` slot is **never presence-trusted** — slot-present-for-a-version is not a proxy for correctness when the source is mutable, so the slot is re-materialised (re-copied) on every install regardless of `slot_integrity`. The §2.3 fast path keeps trusting presence for immutable sources (remote registries *and* external local registries). @impl/done

- ##SCOPE-IN-WORKSPACE-ONLY **Scope — in-workspace `file://` only, and not the giants.** Confined to `file://` sources *under the workspace root* (the self-hosting registry), which are **non-reproducible by nature** (a working directory, like a `path` dependency) and are actively edited — so honouring the edit concedes no reproducibility the source did not already concede. @impl/done
- ##DISCRIMINATOR The discriminator is `is_in_workspace_file_source`: the lockfile's `source_url` has a `file://` prefix **and** its decoded path lies under the (canonicalised, `\\?\`-free) workspace root (the path test is component-wise and case-insensitive on Windows; a `git+file://` local *git* repo is content-addressed and does not match the `file://` prefix). @impl/done
- ##IN-PLACE-EXCLUDED It explicitly **excludes `in-place` (PROP-022) packages** (`materialization.is_in_place()`): re-hashing or re-copying a giant working tree on every install is precisely the cost `in-place` materialisation exists to avoid, so an `in-place` package keeps the §2.2/§2.3 fast path and is refreshed only through its dedicated incremental path (`vibe update <pkg>`, the `git fetch` of PROP-022). @impl/done
- ##NOT-MTIME The choice is source-and-path-based, **not** mtime-based (§6 rejects mtime — file mtimes do not survive `git clone`). @impl/done

##AUTOMATIC-NO-FLAG **No new flag (consistent with §2.5).** This is automatic and source-aware — the author edits the in-repo source and runs `vibe install`; nothing to remember, and neither `vibe update` nor `reinstall --force` is needed for the local-authoring loop, while those bypasses remain for the immutable case. @impl/done

---

## 3. Command and crate surface {#surface}

- ##SURF-WORKSPACE `vibe-workspace` — the freshness check feeding `apply_resolution`; `vibedeps::materialise` (or its caller) gains the slot-present skip; the install orchestration learns the resolution-skip path. @impl/done
- ##SURF-CLI `vibe-cli` — `vibe install` wires the freshness check ahead of the depsolver; its report distinguishes "unchanged — nothing re-resolved" from a real apply. @impl/done
- ##SURF-LOCK-UNCHANGED `vibe.lock` — unchanged; the lockfile *is* the freshness baseline (§5.1), so no schema bump and no new field. @impl/done
- ##SURF-CONFIG vibevm user configuration — a new `[install] slot_integrity` key (`trust-presence` default, or `verify`) selects the §2.3 materialise-skip strategy (§5.2). Set once, persists across runs. @impl/done
- ##SURF-NO-OTHER-CHANGES No change to `vibe update` or `vibe reinstall` beyond their role as the §2.5 bypasses. @impl/done

---

## 4. Migration {#migration}

- ##NO-MIGRATION None. PROP-011 is purely an optimisation of an existing, correct operation — the output of `vibe install` is unchanged for any input where `[requires]` actually changed, and for an unchanged input the output is what `vibe.lock` already pinned. @impl/done
- ##INTENTIONAL-CHANGE The one observable change is intentional and improving: `vibe install` stops drifting versions within a constraint (§2.2). @impl/done
- ##LOCKFILES-AS-IS Existing lockfiles are read as-is; a freshness-input digest, if added, is an optional `meta` field absent lockfiles simply force one resolution. @impl/done

---

## 5. Resolved questions {#open}

##resolved-lead The three questions opened in draft 2 were resolved in an owner design session on 2026-05-22 (draft 3). @impl/done

1. ##RES-FRESHNESS-ORACLE **The freshness oracle — `cargo`'s model.** No digest field is added to `vibe.lock`; the lockfile *is* the baseline. @impl/done
   - ##ORACLE-SAT-TEST The freshness check is a **satisfiability test**, the shape `cargo` uses: re-read every node's `[requires]`, and the lock is fresh iff every declared dependency has a `[[package]]` entry whose pinned version satisfies the current constraint. @impl/done
   - ##ORACLE-ROOT-SET The declared root set must equal `meta.root_dependencies`; every locked package must have its `vibedeps/` slot materialised. @impl/done
   - ##ORACLE-TRANSITIVE-TRUST Transitive packages are trusted — they were resolved from roots, and an unchanged root set cannot have produced a different transitive closure (a transitive `[requires]` lives inside a `vibedeps/` slot, immutable once materialised). @impl/done
   - ##ORACLE-SCOPE *Implementation scope:* the check covers registry-resolved roots; a node carrying a git-/path-source dependency, a capability requirement, or an unresolved `version.var` is conservatively reported stale (never wrongly fresh), so the fast path serves the common case — a workspace of registry packages. @impl/done
   - ##ORACLE-CONTENT-BASED The check reads only the resolved versions the lock already records — no schema bump, no new field; the per-package `content_hash` keeps gating fetched content at fetch time, unchanged. It is content-based, never mtime-based (§6). @impl/done
2. ##RES-SLOT-INTEGRITY **Slot integrity on the fast path — a configurable strategy.** The §2.3 materialise-skip is governed by a **`slot_integrity` setting** in the vibevm user configuration, chosen once and persisted. @impl/done
   - ##INTEGRITY-VALUES Two values: `trust-presence` (the **default** — skip a slot already present for the resolved version) and `verify` (re-materialise every slot from source, so a hand-edited or corrupted one is silently overwritten). @impl/done
   - ##INTEGRITY-VERIFY `verify` is the always-re-copy discipline for an operator who wants a per-install guarantee; `vibe reinstall --force` runs it implicitly. @impl/done
   - ##INTEGRITY-SPOT-CHECK-DEFERRED *Implementation note:* a cheaper `content_hash` spot-check — re-hash a present slot, skip only on a match — was considered for `verify` but deferred: `compute_content_hash` lives in `vibe-registry`, a crate `vibe-workspace` does not depend on, so the spot-check waits until that helper is lowered into a shared crate. @spec/done
   - ##INTEGRITY-LATER-EXTENSION A project-level override is a possible later extension; neither is v1 scope. @spec/done
3. ##RES-HOLD-LOCK **Re-resolution holds the lock — minimum churn.** When `[requires]` has changed, `vibe install` re-resolves, but pins every registry-resolved root the lock still satisfies to its exact locked version, so only the changed dependency and its subtree move — an untouched dependency never drifts. @impl/done
   - ##HOLD-CONFLICT-FALLBACK A held pin that conflicts with the change is detected as a depsolver error and falls back to a full, free re-resolve. @impl/done
   - ##FU3-FINDING *Implementation finding:* the design assumed reuse of FU3's `vibe update <pkgref>` scoped resolution, but FU3 is **correctness-relaxed** — it splices a re-resolved subtree into the held lock without unifying them, which `vibe update` accepts as an operator-scoped action but `vibe install`'s unified contract (one version per package, §6) cannot. @impl/done
   - ##WALK-SKIP-DEFERRED Pinning via constraint-tightening holds the lock correctly with the current `NaiveDepSolver`; it does *not* skip the registry walk for an unchanged subtree. Walk-skipping needs the depsolver's pin-preference machinery (PROP-003 §2.1) and is deferred with the SAT solver. @spec/done

##draft2-closed Closed in draft 2: scoped boot regeneration — boot regeneration stays whole-tree (§2.4), the cheap phase is not optimised. @spec/done

---

## 6. Rejected / deferred alternatives {#rejected}

- ##REJ-SUBSET-RESOLUTION **Subset the resolution per member.** Rejected — resolution must be unified (one version per package; the diamond problem, PROP-007 §2.4). PROP-011 skips resolution when it is provably unneeded; it never resolves a subtree in isolation. @spec/done
- ##REJ-MTIME **mtime-based freshness.** Rejected — file mtimes are not preserved across `git clone` / `git checkout`, so an mtime oracle would mis-fire constantly. The freshness check is content-based (§5.1). @spec/done
- ##REJ-BOOT-SCOPE **Scope or otherwise optimise boot regeneration.** Out of scope by owner decision (§2.4) — boot regeneration is cheap, self-healing, and produces no git churn; effort goes to §2.2 and §2.3, the phases that are genuinely expensive. @spec/done
- ##REJ-FORCE-FLAG **A `vibe install --force` flag.** Rejected — `vibe update` and `vibe reinstall --force` are already the bypasses (§2.5); a third spelling would be redundant. @spec/done

---

## 7. Phase plan {#phases}

1. ##PHASE-1-FRESHNESS **Skip resolution when fresh** — the content-based freshness check; `vibe install` skips the depsolver on an unchanged `[requires]`, becoming lockfile-respecting. The largest win and the observable-contract change. @impl/done
2. ##PHASE-2-DIFF **Materialise only the diff** — the slot-present skip in the materialisation step, with the `slot_integrity` user-config setting selecting `trust-presence` (default) or `verify-hash` (§5.2). @impl/done
3. ##PHASE-3-MIN-CHURN **Minimum-churn re-resolution** — a changed `[requires]` re-resolves, but holds the locked version of every untouched registry root (§5.3); a held-pin conflict falls back to a full re-resolve. Skipping the registry walk for unchanged subtrees is deferred to the SAT solver (PROP-003). @impl/done
4. ##PHASE-4-DOCS **Docs + `VIBEVM-SPEC.md`** — the §9.1 edit (install respects the lock) under owner sanction; a `docs/` note. @impl/done

##phases-note Boot-regeneration scoping (§2.4) is out of scope by owner decision — not a phase. @spec/done

---

## 8. Version history {#history}

- ##HISTORY-DRAFT-1 **2026-05-21 — draft 1.** Requirements captured in an owner discussion on incremental install: the resolution / application split (§2.1), skipping the depsolver when `vibe.lock` is fresh — which also makes `vibe install` lockfile-respecting (§2.2), materialising only changed `vibedeps/` slots (§2.3), and the deliberate decision to leave boot regeneration whole-tree because it is the cheap phase (§2.4). @spec/done
- ##HISTORY-DRAFT-2 **2026-05-21 — draft 2.** Owner review: the §2.4 decision — boot regeneration stays whole-tree, the cheap phase is not optimised — confirmed and made firm; the corresponding draft-1 open question is closed. The PROP stands on its two substantive wins, §2.2 (skip resolution when fresh) and §2.3 (materialise only the diff). Three §5 open questions — the freshness oracle, slot integrity, incremental re-resolution — remain for a follow-up owner design session. Not yet implementation-ready. @spec/done
- ##HISTORY-DRAFT-3 **2026-05-22 — draft 3.** The three §5 open questions resolved in an owner design session. The freshness oracle is `cargo`'s satisfiability model — the lockfile is the baseline, no new field (§5.1). Slot integrity on the fast path is a `slot_integrity` vibevm user-config setting, `trust-presence` by default (§5.2). A changed `[requires]` re-resolves incrementally, full re-resolve as fallback (§5.3). Implementation-ready. @spec/done
- ##HISTORY-REFINEMENT **2026-06-27 — refinement (owner-directed).** In-workspace `file://` sources recognised as mutable (§2.6): a registry dependency resolved from the in-repo self-hosting registry (`--registry packages`, a `file://` path *under the workspace root*, the package-authoring shape) is a working tree, so it is excluded from both the §2.2 freshness fast path (always re-resolved, like `path`/`git`) and the §2.3 presence-trust (its slot is always re-materialised). Scoped to *in-workspace* `file://` — an external local registry or mirror (`file://` outside the workspace) keeps the fast path — and excluding `in-place` (PROP-022) giants, which refresh through `vibe update`. Closes the silent-staleness wart where editing the in-repo `packages/` source in place left `vibedeps/` stale with no signal. @spec/done
- ##HISTORY-SHIPPED **2026-05-22 — shipped.** Implemented across the four §7 phases: the freshness check (`vibe-workspace::freshness`), the materialise-diff skip and the `slot_integrity` user-config setting, and minimum-churn re-resolution via pin-holding. Two findings were reconciled back into this PROP (Sync-from-Code). FU3's scoped resolution is correctness-relaxed and cannot serve `vibe install`'s unified contract, so Phase 3 holds pins via constraint-tightening and the registry-walk skip is deferred to PROP-003's SAT solver (§5.3). `slot_integrity = verify` re-materialises rather than hash-comparing — the cheaper `content_hash` spot-check waits on `compute_content_hash` being lowered out of `vibe-registry` (§5.2). The `VIBEVM-SPEC.md` §9.1 edit landed under the owner sanction granted this session (§6). @spec/done
