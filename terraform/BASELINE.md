# BASELINE — Phase −1 inventory snapshot

_PLAYBOOK-TERRAFORM-VIBEVM v0.2, Phase −1 ("Inventory: freeze
reality"). Captured 2026-06-10 on branch `new`; the audited tree is
commit `ccbc3d9` (code-identical to `99c63de` on `main` — the only
delta is the WAL branch-policy note). This file is the one-time
snapshot that records the measurements; the regenerable artifacts and
their determinism contract are listed in §Determinism._

## The absolute gate

`cargo build --workspace` — **exit 0**. Measured 6.9 s as an
incremental no-op build on a warm cache (the full gate had been run
the same morning); a cold-build time was deliberately not measured —
recorded as such rather than invented.

## Tests (record-only run)

- Runner: **cargo-nextest 0.9.137** (installed this session, as the
  playbook authorizes; fallback would have been
  `cargo test --no-fail-fast`).
- `cargo nextest run --workspace --no-fail-fast`:
  **998 run / 998 passed / 0 failed / 3 skipped**, 11.8 s.
- A second identical run reproduced the same totals (determinism leg).
- The 3 skipped are the `#[ignore]`d live tests in
  `crates/vibe-cli/tests/cli_live_e2e.rs` — known-red when run live
  against the partially-migrated test orgs. They are the only entries
  in [`registry/tests-baseline.json`](registry/tests-baseline.json)
  (status `failing-known`, debt DBT-0002). **No failing, no flaky, no
  obsolete hermetic tests** — the xfail-strict exception list starts
  near-empty.
- The repo's own four-step gate (`bash tools/self-check.sh`) was also
  green today: fmt clean, tests green, clippy `-D warnings` clean,
  `vibe check` 0 errors / 1 warning (`wal_freshness` — transient,
  cleared by the next WAL update).

## Debt registry

[`registry/debt.json`](registry/debt.json) /
[`registry/DEBT.md`](registry/DEBT.md): **18 entries** —
**1 P1 · 7 P2 · 10 P3**; dispositions 1 filed · 1 accepted · 16 open.

By kind: 5 disputed-spec · 4 stale-doc · 3 unimplemented-req ·
3 external-drift · 1 coverage-gap · 1 failing-test · 1 orphan-code.

Sources: the 11 non-fixed AUDIT.md findings (2026-05-23 seed run,
imported 1:1 with their dispositions), 5 conflict-scan findings, and
2 found during this inventory (ROADMAP staleness DBT-0017; the
`vibe init` hint DBT-0018, surfaced by characterization).

## Aspiration inventory

[`registry/intent.json`](registry/intent.json) /
[`registry/INTENT.md`](registry/INTENT.md): **31 entries** — 30 open ·
1 already-done at harvest (CHANGELOG side quest, recorded rather than
dropped, per the carry-over guarantee). Sources: WAL/CONTINUE 4 ·
ROADMAP M1-era 9 · M2 9 · M3 2 · side quests 7. `TASKS.md` is absent
on disk; ROADMAP's dangling pointer to it is DBT-0017 evidence.

## Conflict scan over `spec/**`

**5 disputed pairs** (DBT-0012 … DBT-0016), each recorded with
evidence quotes from both units; **nothing resolved** (BROWNFIELD §5 —
adjudication is the owner's act).

Method and yield:

- *Heuristic: duplicate anchors.* All `{#anchor}` occurrences swept;
  same-file duplicates flagged. **1 hit:** PROP-003 `{#phases}` ×2
  (DBT-0015). Cross-file repeats (`{#open}` ×14 etc.) are not
  conflicts — anchors are file-scoped and `spec://` URIs are
  path-qualified. The `{#req-conditional-fixpoint}` repeat across two
  `spec/neworder` documents is the same worked example quoted twice —
  excluded by judgment (illustrative sample, not a live unit).
- *Heuristic: MUST/MUST-NOT collisions on shared subject windows.*
  9 substantive normative lines across `spec/common`, `spec/modules`,
  `spec/boot`; **0 collisions** among them (token secrecy, adapter
  scope, license policy, frozen files — mutually consistent).
  Precision data for BROWNFIELD §11 OQ-1: on this corpus the keyword
  heuristic produced no false positives, but also found nothing — the
  real conflicts were caught by the duplicate-anchor scan (1) and the
  semantic pass (4).
- *LLM-proposed semantic conflicts* (proposals only): PROP-002 vs
  PROP-008 naming default (DBT-0012); boot 00-core vs 90-user registry
  host (DBT-0013); 90-user repo-shape line vs PROP-008/live org
  (DBT-0014); PLAYBOOK vs BROWNFIELD marker homing (DBT-0016 —
  package-internal; acted on per §0.2 precedence, recorded unresolved).

## Characterization of record

[`golden/`](golden/): **5 hermetic flows, 12 CLI steps, all exit 0**,
captured by the re-runnable [`golden/capture.sh`](golden/capture.sh)
(normalization contract in its header: backslashes→`/`, sandbox→
`<SANDBOX>`, repo root→`<REPO>`, lockfile `generated_at`→
`<TIMESTAMP>`, fixed `golden-proj` basename).

| flow | what it pins |
|---|---|
| `init` | the 10-file scaffold a fresh project gets |
| `install-qualified` | qualified-pkgref install from the fixture registry (LocalRegistry path); lockfile v5 with `group` + `content_hash` |
| `install-short-name` | PROP-008 Phase 5 short-name resolution at the CLI boundary |
| `check-installed` | `vibe check` clean output, text + `--quiet` |
| `uninstall` | slot removal + manifest/lockfile cleanup symmetry |

Deliberate exclusions (recorded): the `manual-tests/` live recipes
(network, non-deterministic — health tracked by DBT-0002/DBT-0005);
`vibe search` (writes a machine-global `~/.vibe/search-cache/` —
side-effectful; covered by the hermetic `cli_search` suite).
Transcripts are machine-class-specific (captured on Windows); the
LocalRegistry path they drive is exactly the proxy-coverage situation
DBT-0001 records — the golden set should grow a `GitPackageRegistry`
flow when INT-0002 lands the harness.

Known pinned bug: none observed — but the `init` flow pins the stale
kind-qualified hint (DBT-0018), which is the point of characterization:
the pin is visible debt, and changing that hint later must come with a
debt/intent reference in the PR (BROWNFIELD §6).

## Workspace shape

13 workspace members: `vibe-cli`, `vibe-core`, `vibe-graph`,
`vibe-index`, `vibe-registry`, `vibe-resolver`, `vibe-llm`, `vibe-mcp`,
`vibe-check`, `vibe-publish`, `vibe-wire`, `vibe-workspace`, `xtask`.
Rust edition 2024, rust-version 1.93, version 0.1.0-dev across the
workspace.

## Determinism contract

"A second inventory run is a no-op diff" is interpreted as: **at the
same tree state, re-running each extraction reproduces the committed
artifact byte-for-byte.** Verified this run:

- `golden/capture.sh` run twice → `diff -r` empty (after normalizing
  the two volatile fields it documents);
- `cargo nextest run --workspace --no-fail-fast` run twice → identical
  totals (998/998/3) and an unchanged exception set;
- the registries (`debt.json`, `intent.json`, `tests-baseline.json`)
  embed no volatile fields (no timestamps, no hashes of moving
  targets); their regeneration inputs are the greps and document reads
  recorded above.

This BASELINE itself is a one-time snapshot *recording* measured
values (build seconds, run seconds); those measurements are stated
once here, not embedded in regenerable artifacts.

## Package feedback from this phase (for the discipline's v0.2)

1. PLAYBOOK §Phase−1 and BROWNFIELD §3 contradict each other on
   REVIEW/TODO homing (debt vs intent) — DBT-0016; this run followed
   the playbook per §0.2 precedence.
2. BROWNFIELD §3's `kind` enum has no bucket for *untested production
   path* or *external-state drift*; this registry extends it with
   `coverage-gap` and `external-drift` rather than mis-bucketing into
   the nearest neighbour.
3. Heuristic-precision data point (OQ-1): duplicate-anchor scan is
   high-precision (1/1 true positive); the MUST/MUST-NOT keyword
   window produced zero candidates on this corpus — the semantic pass
   carried the load (4/5 findings).
4. The acceptance line "the owner has reviewed and dispositioned …"
   makes Phase −1 completion an owner act by construction — the
   inventory below is therefore delivered as *pending disposition*:
   **P1: DBT-0001** (existence already owner-acknowledged via AUDIT
   seed, re-confirm under terraform framing); **disputed-spec
   existence: DBT-0012, DBT-0013, DBT-0014, DBT-0015, DBT-0016**.
