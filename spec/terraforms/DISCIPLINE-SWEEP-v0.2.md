# vibevm Discipline Sweep v0.2 — the project instance {#root}
**status: STANDING · recurring (daily / weekly) · vibevm-specific · supersedes v0.1**

*The METHOD is no longer here. v0.1 of this manual carried three layers in
one file — the portable sweep template, the Rust idioms, and vibevm's own
numbers — and the first two now ship with the Discipline itself
(SELF-SUFFICIENCY-PLAN Phase 5): the template is
[`spec://org.vibevm.ai-native/core-ai-native/04-SWEEP-PLAYBOOK`](../../vibedeps/flow-core-ai-native/0.7.0/spec/04-SWEEP-PLAYBOOK.md)
(tiers, cadence, collector contract, the WISH→census→Rule ladder), the Rust
idioms are GUIDE §14 in the rust-ai-native stack, and the runnable surface
is the shipped `rust-ai-native` tool driven by the `/rust-ai-native-sweep`
agent skill. This file is only what remains genuinely vibevm's: the local
wrapper surface, the project's standing numbers, and this machine's
quirks.*

## 1. Running the sweep here {#running}

The skill (`/rust-ai-native-sweep`) walks the shipped playbook. On this repo
the flag-compatible wrappers remain first-class:

```sh
bash tools/self-check.sh                 # the repo floor (adds the package gates, steps 6-9)
cargo xtask health                        # = rust-ai-native health (+ optional --mirrors probe)
cargo xtask conform check                 # = rust-ai-native-conform over vibevm's conform.toml
cargo xtask specmap --check               # = rust-ai-native-specmap over vibevm's specmap.toml
cargo xtask test-gate                     # xfail-strict vs discipline/registry/tests-baseline.json
cargo xtask tripwire                      # debt tripwires vs discipline/registry/debt.json
cargo xtask fast-loop --enforce-budget    # per-cell 60s first-signal budget
```

Tier 0 here means `self-check.sh` (it wraps the playbook's floor AND the
package's own fmt/test/clippy/self-trace steps 6–9). The living registries
sit at `discipline/registry/`, goldens at `discipline/golden/`
(re-capture: `discipline/golden/capture.sh`), the health snapshot at
`discipline/health/latest.json` — committed, so its git diff is the trend.

## 2. Standing project facts (verify against the collector, never trust this list) {#facts}

- Gating: read `conform.toml` — `gated_crates` (10 today) / `[[exempt]]`
  (4, each with its reason) / `gated_pub_doctest` (`vibe-core`,
  `vibe-mcp`). The count comes from the file via the collector, never from
  memory (the "count the list, not the record" lesson, SHRINK v0.1 §0).
- conform baseline: 0 frozen (empty since PUBDOC-DRAIN v0.1).
- The canonical deviates target is
  `spec://org.vibevm.ai-native/core-ai-native/mechanisms/ENGINE-CONFORM-v0.1#rules`.
- Known standing landmines live in the collector's `danger_band_files` —
  re-read `discipline/health/latest.json`, not a stale list here.
- Owner-frozen surfaces (never swept, drift is FILED): `spec/boot/00-core.md`,
  `spec/boot/90-user.md`, `VIBEVM-SPEC.md`, `refs/book/`.

## 3. Machine quirks (THIS box; machine-scoped, not project fact) {#quirks}

Boot-resident since the deferrals-closeout campaign: the canonical copy
lives in [`spec/boot/90-user.md`](../boot/90-user.md) (owner-sanctioned)
and loads at every session boot; this list stays as the sweep's local
reference:

- Edits through editor tools only — PowerShell 5.1 corrupts UTF-8-no-BOM
  round-trips; recover with `git restore`.
- `self-check.sh` through Git Bash, not WSL; check the REAL exit code
  (`; echo "EXIT=$?"`), never a `| tail`'d pipe.
- Commits via `git commit -F - <<'MSG'` heredoc only.
- Windows UAC blocks test executables named `*install*` (os-740).
- `bash … > "$VAR/file" 2>&1` with an unset `$VAR` writes to `/file` and
  silently never runs the command — inline the path or set the var on the
  same line.

## 4. History {#history}

v0.1 (2026-06-14, tree `91bc763`) was authored as the synthesis of the
five terraform campaigns and carried the full method inline; its worked
example and tier text are preserved in git history and superseded by the
shipped playbook. Sweep output contract, cadence, and the
what-this-does-NOT-do list: see the playbook §§3–5 — they apply here
verbatim, with the WAL branch active (this project keeps
`spec/WAL.md` per `06-WAL-CONVENTION`).
