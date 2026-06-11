# Adoption Log — Discipline v0.2 (TERRAFORM-PLAN-v0.3)

Raid-grained session log of the v0.3 adoption. Companion artifacts:
[`PREDICTIONS.md`](PREDICTIONS.md) (the pilot prediction ledger),
`REPORT.md` (written at close-out). The prior terraform's artifacts
live one level up in `terraform/` and are historical — they record
the v0.2 PLAYBOOK run and are not edited by this adoption.

---

## 2026-06-11 — Phase 0: Adopt & shim

**Scope & freeze.** Documents only (`spec/neworder/`, `packages/`,
`spec/discipline/`, boot artifacts, the two lock files); production
code frozen except the inert `spec://` URI strings and the ledger
epoch input — both metadata, no behavior. Discipline files moved
byte-verbatim (the owner's constraint: the product is not bent to
fit the pilot).

**What happened, in order:**

1. **Packaged the product.** The flat `spec/neworder/` drop became
   two packages under the in-repo local registry root `packages/`:
   - `flow:org.vibevm/discipline-core@0.2.0` — manifesto, card
     format, scaffold catalog, raid playbook, `cards/` (INDEX + the
     nine scaffold cards), `appendix/` (contradiction map, atlas),
     `legacy-projections/` (the eleven v0.1-era language guides),
     README (the drop's package README, verbatim).
   - `stack:org.vibevm/rust-ai-native@0.2.0` —
     `rust/GUIDE-AI-NATIVE-RUST.md`, `rust/tools/vibe-tcg.md`;
     `[requires]` on `flow:org.vibevm/discipline-core@^0.2`.
   New files per package: `vibe.toml`, a minimal boot snippet
   (minimal-sufficiency: boot says "cards load by trigger", it does
   not inline the corpus).
2. **Self-hosted install.** `vibe install flow:org.vibevm/discipline-core
   stack:org.vibevm/rust-ai-native --registry ./packages
   --assume-yes` — the Discipline's first carrier installed the
   Discipline through the Discipline's own tool. The stack→flow
   dependency resolved transitively. `[requires]` landed in
   `vibe.toml`, both packages in `vibe.lock` (schema 5, content
   hashes), slots materialised under `vibedeps/`, boot regenerated:
   `spec/boot/INDEX.md` now sequences 00-core → discipline-core →
   rust-ai-native → 90-user.
3. **Relocated the retained mechanisms.** PROP-014, BROWNFIELD,
   ENGINE-CONFORM, LEDGER-INTENT moved (byte-verbatim) from
   `spec/neworder/` to `spec/discipline/` — they stay inside
   `spec/**` because vibevm code carries their `implements` edges
   and mdspec scans only `spec/**`. The ~26 in-source
   `scope!`/`#[spec]` URIs were rewritten
   `spec://vibevm/neworder/…` → `spec://vibevm/discipline/…` in the
   same change set; `specmap.json` regenerated: **352 units / 170
   items / 177 edges / 0 suspects** — edge count and suspect-zero
   preserved through the relocate (prediction P0-2 holds so far).
4. **Shimmed `spec/neworder/`.** The directory now holds one
   README: the where-everything-went table, the reinstall recipe,
   and the carried-over v0.1 beta-gap notes. The duplicate
   `TERRAFORM-PLAN-v0.3.md` copy was removed — the plan's own text
   places vibevm-specific plans outside the product
   (`spec/terraforms/` is the canonical home).
5. **Pinned the pilot.** `vibevm.discipline.lock` records both
   pkgrefs + content hashes. The ledger epoch input changed from the
   old drop README to this pin file — the epoch's "discipline
   package in effect" component now tracks exactly what the pilot
   runs (cache invalidation only; the producer is deterministic).

**Honest findings (feed the REPORT):**

- `vibe.lock` `source_url` for a local-registry install is a
  machine-absolute `file:///C:/…` path. Committed, it is
  machine-specific noise; the slots being present means freshness
  holds and nothing re-fetches on a clean checkout, but the field
  should be repo-relative for in-repo registries. Logged as a debt
  candidate for the registry layer (not fixed here — Phase 0 is
  no-code-change).
- DBT-0016 (PLAYBOOK vs BROWNFIELD marker homing, tripwire
  `touch:spec/neworder/**`) fired on this change set, as designed.
  The v0.2 package dissolves the conflict's subject: the PLAYBOOK
  side is superseded by the generalized RAID playbook + this plan.
  Disposition updated accordingly.
- `[[registry]].url` accepts only git-cloneable URLs; a plain
  directory registry is CLI-flag-only (`--registry <path>`). Fine
  for the pilot (the recipe is in the shim README), but it means a
  bare `vibe install` after a `[requires]` edit cannot see
  `packages/` — re-resolve must repeat the `--registry` flag.
  Worth a PROP note when the cache (PROP-010) lands.
- **conform cached-vs-clean divergence (engine defect, found by
  this phase's gate).** The Phase-0 `conform check` flagged
  `unsafe-gate|crates/vibe-index/src/cli/stop.rs` as 1 NEW at line
  35 with the baseline entry (line 33) no longer firing. The file
  is untouched since `a9dc160` (the prior terraform's scope!
  backfill, which shifted the block +2 lines) — and that commit's
  own merge-time gate reported "0 new, 6 frozen". A clean-cache
  re-run of `cargo xtask conform check` in a worktree pinned to
  `a9dc160` reproduces **1 new** — so the merge-time green was an
  artifact of a stale `target/conform/` facts cache surviving a
  change to the very file it described. The store's
  `(file content-hash, producer)` key should have invalidated;
  it did not. Filed for the Phase-2 conform work (the engine is in
  scope there); the baseline line number is corrected 33→35 in this
  change set (same frozen finding, same count — not baseline
  growth). Discipline lesson for the REPORT: a checker whose cache
  can lie fails the scaffold-reality checklist's "cannot silently
  lie" clause — the determinism check must cover the cache path.

**Gate panel at phase close (all green):**

- `cargo xtask specmap --check` — clean: **352 spec units / 170
  tagged items / 177 edges / 0 suspects**, 6 known
  pin-into-unmarked warnings; orphan ratchet 0 gated, 6
  dispositioned (DBT-0019), 8 reasoned exemptions.
- `cargo xtask conform check` — **6 findings, 6 frozen, 0 new**
  (after the honest line correction 33→35; set and count
  unchanged).
- `cargo xtask test-gate` — **1075 results, 0 failed, 3 skipped**,
  xfail-strict green.
- `bash tools/self-check.sh` — all four steps green (`cargo fmt
  --all --check`; workspace tests; `clippy -D warnings`;
  `vibe check` 0/0/0). One fmt fix fell out of the URI rewrite
  (`specmark/tests/usage.rs`: the longer `discipline/` URI pushed
  an attribute over the line limit).

**Phase 0 exit criteria: met.** vibevm builds; the index
regenerates deterministically; 0 gated orphans; 177 edges / 0
suspects preserved; the Discipline is an installed package pinned
by `vibevm.discipline.lock`; `spec/neworder/` is a shim.
Predictions P0-1 (with the cache-defect caveat) and P0-2 recorded
with verdicts in `PREDICTIONS.md`.
