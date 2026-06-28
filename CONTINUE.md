# CONTINUE.md — cold-resume checkpoint

_Written 2026-06-28. **The conform half of the code-bearing-packages refactor
(PROP-024 Ф1–Ф7) is COMPLETE, green, and the relocated checker is PROVEN to
catch violations in its shipped form. The binary is now `conform-rust`. The
NEXT campaign — relocating the traceability toolchain (specmap/specmark) into
the same package — is fully PLANNED and ready for a fresh session.** Local on
`main` at `c240fc7`, **2 commits ahead of the mirrored tip `c3fcf63`** (the
`conform-rust` rename + the plan doc) plus this checkpoint, all NOT mirrored.
Floor green: `self-check.sh` 8 steps exit 0, specmap 614/583/596/0/0/0, vibe
check 0/0/0._

> **`spec/WAL.md` is the canonical living state**; if this snapshot and the WAL
> disagree, the WAL wins. The **git log is the authoritative per-item record**.
> Boot first (`CLAUDE.md` → `spec/boot/INDEX.md` → its files → `spec/WAL.md`),
> then read this.

---

## TL;DR

PROP-024 makes a package a project that ships runnable code, so installing a
discipline stack yields *working checkers*. **Ф1–Ф7 relocated the conform
(structural-gate) half** into `stack:org.vibevm/rust-ai-native` and **proved it
works**: the standalone `conform-rust` binary — and the binary built from the
materialised consumer slot — runs against a real project, catches a planted
violation, exits non-zero, frozen into a permanent integration test.

That move was scoped **conform-first**: it stripped the conform crates of
`specmark` so they could move without it (which stayed in vibevm). That left a
debt — the relocated code is now *less* disciplined than the vibevm code it came
from (no `scope!` tags, can't carry `#[spec(deviates)]`), and the package ships
only *half* the verification stack. **The owner confirmed the fix (Option B):
relocate specmap/specmark into the same package too.** That campaign is written
up cold-executable and is the next session's work.

## Where work stands

- **Branch `main`**, tip `c240fc7`. **2 commits ahead of the mirrored tip
  `c3fcf63`** (`2a2932a` conform-rust rename, `c240fc7` the plan doc) + this
  checkpoint. Origin/the mirrors are at `c3fcf63` (the Ф4–Ф7 batch was mirrored
  to GitVerse + GitHub mid-session on the owner's explicit word).
- Floor **green** at the tip: `bash tools/self-check.sh` exit 0 (8 steps: 5
  vibevm + 3 package gate); `cargo xtask specmap --check` 614/583/596/0/0/0;
  `vibe check` 0/0/0.
- Working tree clean (except this checkpoint commit).

## Two open items

1. **Mirror (held, outward-facing).** The 2 ahead commits + this checkpoint are
   local. The mirror is gated for the owner's explicit word (PROP-016
   hub-and-spoke). Unblock: owner says "mirror" → `cargo xtask mirror` (after
   `cargo xtask mirror --check`).
2. **The next campaign — traceability relocation (PLANNED).** Move `specmap-core`
   + `specmark` + `specmark-grammar` into `rust-ai-native` (Option B,
   owner-confirmed). The full cold recipe is in
   **`spec/terraforms/TRACEABILITY-RELOCATION-PLAN-v0.1.md`** — read it whole
   before starting.

## What landed this session (Ф4–Ф7 + verification + rename)

- **Ф4a** (`26858dc`+`f2e2ab7`) decouple conform from specmark. Finding: the
  `#[spec(deviates)]` on `sarif::render` was load-bearing (conform reads it to
  excuse `.expect`) — render made total (`.unwrap_or_default()`).
- **Ф4b** (`2b0e6f6`) relocate conform-core/conform-frontend-rust/env-audit +
  new `conform-cli` into the package (own Cargo workspace, external path-dep,
  xtask shim, conform.toml 16→13, **self-check grew a package gate, steps 6-8**).
- **Ф4c** (`12c8592`) the first code-bearing install surfaced that copy + both
  `compute_content_hash` ports hashed/copied build output → fixed with a
  shippable-tree `filter_entry` + tests; `vibe.lock` reproducible.
- **Verification** (`302454b`) the shipped checker proven to catch violations
  (standalone + slot-built), frozen into `conform-cli/tests/catches_violations.rs`.
- **Ф5** (`b0d1830`+`56c08a0`+`ac50f72`) spec tails repointed off the Ф3-defunct
  const policy to `conform.toml`.
- **Ф6** (`6c5ee9e`) specced the future `conform-frontend-typescript`.
- **Ф7** (`d725b71`+`c3fcf63`) checkpoint; then **mirrored to both replicas on
  the owner's word**.
- **Rename** (`2a2932a`) the package binary `conform` → **`conform-rust`** (it
  embeds only the Rust frontend; a TS stack will ship `conform-typescript`;
  per-language suffix avoids PATH collisions). `cargo xtask conform` unaffected
  (drives the library, not the binary name).

## The next campaign — traceability relocation (read the plan)

**`spec/terraforms/TRACEABILITY-RELOCATION-PLAN-v0.1.md`** (committed `c240fc7`).
Option B: specmap-core + specmark + specmark-grammar → rust-ai-native, so the
package ships the *whole* Rust verifier and the conform crates re-acquire their
tags (the discipline disciplines itself again). Phase shape mirrors the conform
move: **spike (proc-macro path-dep, gating) → sever the specmap-core→vibe-wire
JTD edge → productise (specmap.toml) → relocate + a `specmap-rust` binary →
re-tag the conform crates + package self-trace → checkpoint.** Verified facts
already baked in: 11 specmark dogfooders rewire via ~3 root Cargo.toml lines
(they use `.workspace = true`); specmap-core is consumed only by xtask;
`schemas/specmap.jtd.json` → `vibe-wire/src/generated/specmap/` is the one edge
out (no other consumer); specmap-core keeps its specmark dep throughout (no
decouple needed — specmark travels with it). Why NOT discipline-core now: a
neutral package depending on a Rust proc-macro is a backwards edge — deferred
until a second language needs the shared core.

## Non-obvious findings (still in force)

- **A discipline tag can be load-bearing for a GATE, not just for specmap.**
  conform reads `#[spec(deviates)]` text (frontend `is_spec_deviates`) to excuse
  unwrap/unsafe/env in domain code. Stripping such a tag without handling the
  excused site turns the gate red — but self-check's `conform check` catches it
  (self-guarded). Resolved for render (now total).
- **specmark-free relocated crates are DEBT, not cleanliness.** The conform-first
  scoping left the relocated code un-traced + unable to carry deviation
  testimony. This is *the* reason the traceability relocation campaign exists —
  the discipline must discipline itself, and the package must ship the whole
  verifier.
- **The package content_hash must be over the SHIPPABLE tree** (Ф4c). Build
  output is volatile; hashing it makes `vibe.lock` non-reproducible. Fixed; the
  two duplicated hashers' exclude lists are currently identical (verified) and a
  drift would surface as a loud install-time integrity error, not silently.
  `.vibeignore` glob support is the one spec-incomplete bit (PROP-024 §2.2,
  "optional", no consumer needs it) — a noted follow-up.
- **The Cargo nested-workspace topology works on Windows** for library
  path-deps; the traceability campaign must spike the **proc-macro** path-dep
  (specmark) before moving — that is Phase 0 of the plan.
- **Machine quirks (unchanged):** Edit/Write only (PS `Set-Content` corrupts
  UTF-8-no-BOM); `git commit -F - <<'MSG'`; `self-check.sh` via Git Bash; **check
  the real exit code, never a `| tail`'d pipe** (it masks the script's exit);
  don't `2>&1`-redirect native cargo in PowerShell.

## Repository map (conform relocated; specmap/specmark still in `crates/` pending the campaign)

```
vibevm/                      Rust workspace; binary = `vibe`; tooling = cargo xtask
├─ Cargo.toml                root: exclude=[packages,vibedeps]; conform deps → package; conform-cli
├─ conform.toml              vibevm's conform policy — 13 gated / 4 exempt
├─ spec/
│   ├─ WAL.md                CANONICAL living state
│   ├─ common/               PROP-000.. + PROP-024-code-bearing-packages.md
│   ├─ discipline/           ENGINE-CONFORM, PROP-014, README (mechanism table → package)
│   └─ terraforms/           DISCIPLINE-SWEEP, *-PLAN history, **TRACEABILITY-RELOCATION-PLAN-v0.1.md (the next campaign)**
├─ schemas/specmap.jtd.json  the specmap type schema — moves with specmap-core in the campaign
├─ packages/org.vibevm/rust-ai-native/v0.2.0/
│   ├─ Cargo.toml + LICENSE.md      the package's own Cargo workspace
│   └─ crates/{conform-core, conform-frontend-rust, conform-cli (bin conform-rust), env-audit}
│                                   ← campaign ADDS specmap-core, specmark, specmark-grammar, specmap-cli (bin specmap-rust)
├─ vibedeps/                 materialised install (git-TRACKED); rust-ai-native slot ships crates/ (no target/)
├─ crates/
│   ├─ specmap-core          → vibe-wire JTD edge; hardcoded crates/ scan — MOVES in the campaign
│   ├─ specmark / specmark-grammar   proc-macro + grammar — MOVE in the campaign
│   ├─ vibe-registry/lib.rs  shippable-tree filter (599 lines, at the budget edge — split is a follow-up)
│   ├─ vibe-index/content_hash.rs    same filter (parity-duplicated)
│   └─ vibe-* (core/cli/install/registry/resolver/workspace/mcp/check/publish/index/wire/graph/llm)
├─ xtask/                    conform.rs + specmap.rs (shims/drivers), health.rs, mirror, codegen
├─ tools/self-check.sh       8-step floor gate (5 vibevm + 3 package)
└─ specmap.json / specmap-ratchet.json   the traceability index + orphan ratchet
```

## Recent commit chain (newest first)

```
c240fc7 docs(plan): traceability relocation campaign + resume pointers   (this session)
2a2932a refactor(conform): name the binary conform-rust                   (this session)
c3fcf63 docs(wal): checkpoint — Ф4-Ф6 landed green, verification proven   (← mirrored tip)
d725b71 docs(continue): cold-resume checkpoint — code-bearing packages Ф4-Ф6
6c5ee9e docs(typescript): spec the future conform-frontend-typescript     (Ф6)
ac50f72 docs(conform): drop a stale CONFORM_GATED reference               (Ф5)
56c08a0 docs(discipline): reflect the conform.toml policy in the sweep     (Ф5)
302454b test(conform): prove the shipped gate catches violations e2e       (verification)
b0d1830 docs(discipline): point the mechanism table at the relocated conform (Ф5)
12c8592 fix(registry): exclude build output from the content hash and copy (Ф4c)
2b0e6f6 refactor(conform): relocate the conform toolchain into the package (Ф4b)
f2e2ab7 chore(specmap): regen + exempt the conform crates from the ratchet (Ф4a)
26858dc refactor(conform): drop the specmark tags from the conform crates  (Ф4a)
```

## Quick-start

```sh
bash tools/self-check.sh; echo "EXIT=$?"          # 8-step floor gate; must be 0 (real exit code)
cargo xtask specmap --check                        # clean (614 / 583 / 596 / 0 / 0)
cargo xtask conform check                          # 0 findings, 13 gated / 4 exempt (via the conform_cli shim)
cargo run -q -p vibe-cli -- check --path .         # vibe check 0/0/0
cargo run -p vibe-cli -- install --registry packages --assume-yes   # re-materialise vibedeps

# The SHIPPED conform engine directly (consumer surface) — binary is conform-rust:
cargo run --manifest-path packages/org.vibevm/rust-ai-native/v0.2.0/Cargo.toml \
  -p conform-cli --bin conform-rust -- check --path .

cargo xtask mirror --check                         # confirm GitVerse + GitHub sync (HELD; do not mirror without owner word)
```

## Next-steps recipe (whoever picks up)

1. **Mirror the 2 ahead + this checkpoint on the owner's word** (`cargo xtask
   mirror`). The Ф4–Ф7 batch is already mirrored; only the rename + plan +
   checkpoint are pending.
2. **Execute the traceability relocation campaign** —
   `spec/terraforms/TRACEABILITY-RELOCATION-PLAN-v0.1.md`, fresh session, phase
   by phase (start with the gating proc-macro spike, Phase 0).
3. **Smaller follow-ups, none blocking:** split `crates/vibe-registry/src/lib.rs`
   (599 lines); `.vibeignore` glob support; eventually consolidate the neutral
   engines into `discipline-core` (only when a second language needs it).

The WAL supersedes this snapshot wherever they diverge. Session-resume phrase:
`восстанови сессию` (boots into a status report and waits — the candidate next
work above is the owner's call, not a standing mandate).
