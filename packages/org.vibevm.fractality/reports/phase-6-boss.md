# Phase 6 report — boss integration and the scoreboard v0

_Campaign: FRACTALITY-IGNITION v0.1 · Phase 6 · executed 2026-07-10.
The plan's §14 ledger stays the canonical commit map._

## What the phase delivered

- **Boot snippet 75** (`spec/boot/75-tool-fractality.md` in the
  fractality package, wired via `vibe.toml [boot_snippet]`): the
  boss-side trigger table — when a task smells delegable, consult the
  matrix; one command per shape; the packet as the work order; the
  Never lines. Slot 75 sits outside the redbook grid (no collision;
  delegation-rules holds 77).
- **The `fractality-delegate` skill**
  (`spec/skills/fractality-delegate/`, declared as `[[skill]]`): the
  guided loop — route via the matrix, author the packet, fire sync or
  swarm, observe (never blind), review as a contributor PR, record
  field data. `[[binary]]` declarations for all three binaries landed
  alongside (PROP-025 dispatch readiness).
- **`fractality stats`** — the D16 scoreboard verb over
  `GET /v0/metrics`: outcomes/tokens/cost/wall totals plus by-profile,
  by-model, and by-day tables (web-tool quota counter included). No
  shadow accounting anywhere else: the CLI renders exactly what the
  bus aggregates.
- **The dogfood (MT-05, RP1's task):** the host repo's seven remaining
  `license = "EULA"` package manifests (+ their EULA-placeholder
  `LICENSE.md` files) relicensed to UPL-1.0 **through the fabric
  itself** — two GLM-turbo workers in host-repo worktrees, disjoint
  batches (rust/core family, typescript family), acceptance commands
  in the packets, boss diff review + grep-to-zero + host self-check
  before the merges. Outcome and numbers: MT-05's recorded run.

## The P6 baseline — measured honestly

Eligible grunt tasks in the dogfood exercise: 2 (the two relicensing
batches). Delegated through the fabric: 2 → **P6 = 100% ≥ 50%,
PASS** — with the honest caveat the plan itself demanded: this session
was already delegation-primed (it built the fabric), so the baseline
measures the *tooling's* capability to carry real work end to end more
than a cold boss's *propensity* to reach for it. The propensity
question is exactly Campaign 2's mandate (the initiative system), and
this number is the floor it must beat in a cold session.

## Strange things / surprises (this phase's harvest)

- **F19 (Windows MAX_PATH × worktrees).** The dogfood's first firing
  failed before any worker ran: `git worktree add` of the host repo
  into a run dir overflows MAX_PATH (`Filename too long`) — the
  worktree inherits the repo's full depth (deep `vibedeps/` paths) on
  top of the runs-root prefix. Product fix: provisioning and removal
  now pass `-c core.longpaths=true`. Found only because the dogfood
  used a REAL deep repository — the hello-world fixtures never could.
- **The cwd-poisoning class bit the boss.** A `vibe install` step left
  the session shell parked in the delegation-rules package; the next
  "floor" invocation silently gated **the host tree** (its own
  adoption, different baseline) and reported green — while the
  fractality floor had five unseen findings. Caught by baseline
  arithmetic (0 ≠ 3). The workspace law («pin the cwd in the launch
  command») now demonstrably applies to the boss's own gates, not just
  delegates.
- The five findings the real floor then surfaced were all legitimate:
  the broker's env read (recorded as the second worker-context
  env-root), two cell-budget overruns (split: CLI `boss.rs`; pod
  mcp-write → `worker_env.rs`), and the F17 FFI needing its
  `#[spec(deviates)]` testimony — specmark rejects non-`spec://` URIs
  in `deviates`, so the testimony points at the governed project
  anchor rather than the discipline REQ string.

## Deliberately deferred / unfinished (named)

- **Quota windows in `stats`** — by-day is rendered; a "N of 4000 MCP
  calls this month" rollup is a trivial consumer sum left for when a
  real month of data exists.
- **`vibe skill install` projection** of `fractality-delegate` into
  agent homes — the declaration ships; running the projection on this
  box is a one-liner the owner can do at will.
- **vibedeps/.vibe-cache mirrors of the relicensed manifests** —
  refreshed via `vibe install` in the three consumers after the merge;
  stale copies elsewhere regenerate on their next install (noted in
  MT-05).

## Delegation scoreboard (the law's ledger)

- **Delegated this phase: 2 of 2 eligible** (the dogfood batches, both
  green first landing — turbo-class mechanical work per the matrix).
- **Kept, with cause:** packet authoring and the RP1 acceptance
  (review of delegated output — the never-delegate set), the boot
  snippet/skill/stats authoring (spec + seam design), the F19 fix.
