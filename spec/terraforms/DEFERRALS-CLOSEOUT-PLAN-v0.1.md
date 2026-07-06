# DEFERRALS-CLOSEOUT-PLAN v0.1 — close every §10 deferral of the Self-Sufficiency campaign

_Status: PROPOSED — awaiting the owner's word. Written 2026-07-07 against
tree `57fa42e` (main, clean, 38 ahead of origin, mirror held). Cold-executable:
any phase is a safe stop; the floor must be green at every phase boundary._

Mandate (owner, 2026-07-07): take everything listed in
`SELF-SUFFICIENCY-PLAN-v0.1.md` §10 (plus the standing `vibe-registry`
line-budget note from CONTINUE.md) and plan its implementation. For
typescript-ai-native symmetry, do NOT wait for the full VibeVM TypeScript
pilot — build a small test demo project instead and implement the TS stack
symmetric to the Rust one.

## 0. The seven items this campaign closes

| # | §10 deferral | Closure shape |
|---|---|---|
| 1 | Engine-code consolidation into discipline-core | Phase 1 — authored home moves; vendor-sync mechanism |
| 2 | typescript-ai-native symmetry (conform/specmap/skills twins) | Phases 2–5 — engines, bins, skills, boot |
| 3 | (owner amendment) TS pilot → small demo project | Phase 6 — `research/ts-demo` + frozen test |
| 4 | DEBT.md / INTENT.md generated views | Phase 7 — `discipline-rust ledger render` |
| 5 | `vibe trace` product alias | Phase 8 — delegating subcommand |
| 6 | vibe-native binary delivery (future PROP) | Phase 9 — PROP-025 authored, spec-only |
| 7 | Owner-court: machine-quirks → `spec/boot/90-user.md` | Phase 10 — owner-sanctioned copy |
| + | `crates/vibe-registry/src/lib.rs` at 599/600 lines | Phase 11 — module-grain split |

Non-goals (named, stay deferred): implementing PROP-025 (spec lands, code
does not); `test-gate`/`tripwire`/`health`/`fast-loop`/`codemod` TS twins;
prettier/eslint floor steps as gated defaults; an AST-grade TS parser (swc /
TS Compiler API — upgrade path documented, not built); `vibe-tcg-ts`; the
full VibeVM TypeScript surface (the demo is the pilot-lite, not the pilot);
mirror/publish (owner-held, network-dead anyway).

## 1. Standing constraints

- **This box is offline** (gitverse:22, github:22 refused at plan time;
  crates.io unverified). Therefore: **no new external crate dependencies**
  anywhere in this campaign — every new crate uses the workspace's existing
  dependency set (serde, toml, anyhow, regex if already present, std). The
  npm cache DOES resolve `typescript` offline (probed 2026-07-07) and node
  is v24.18.0 (type-stripping stable → `node --test` runs `.ts` natively).
  Phase 0 re-probes both before relying on them.
- The four CLAUDE.md rules; floor green at every phase boundary
  (`bash tools/self-check.sh` exit 0, specmap `--check` clean 0 dangling,
  conform 0, package gates green).
- Machine quirks per DISCIPLINE-SWEEP-v0.2 §3 (Edit/Write only;
  `git commit -F - <<'MSG'`; Git Bash for self-check; real exit codes;
  no redirects into unset vars).
- Version moves: `discipline-core` 0.3.0 → **0.4.0** (gains a code-root),
  `rust-ai-native` 0.3.0 → **0.4.0** (engine authorship leaves, vendored
  copies arrive), `typescript-ai-native` 0.2.0 → **0.3.0** (gains a
  code-root + skills). vibevm's `specmap.toml` `[[external_specs]].root`
  hardcodes `vibedeps/flow-discipline-core/0.3.0/spec` — **must bump with
  the slot** or resolution silently dangles (checked by the 0-dangling
  floor). Registry publish of all three stays owner-held.

## 2. Decisions (D1–D8)

### D1 — consolidation mechanism: vendor-sync, not cross-slot path-deps

The blocker nobody wrote down: a stack crate cannot `path`-dep on a
discipline-core crate with ONE relative path, because the authored layout
(`packages/org.vibevm/<name>/v<ver>/`) and the materialised layout
(`vibedeps/<kind>-<name>/<ver>/`) differ in both directory naming and
version-prefix; a baked relative path satisfies one tree and breaks the
other, and each slot must stay a self-buildable workspace (PROP-024 §2.4 —
the property the fresh-project walk proved).

Options considered:
- (α) vibe rewrites cross-package path-deps at materialise time — real
  product surface, interacts with shippable-tree hashing (PROP-024 §2.2
  reproducibility), belongs in the PROP-025 family as future work. Rejected
  for this campaign, **named in PROP-025** (Phase 9).
- (β) align the two layouts — breaks PROP-011; rejected.
- (γ) **vendor-sync (chosen):** authored source of the language-neutral
  engines lives in `packages/org.vibevm/discipline-core/v0.4.0/crates/`
  (the Ф6 brief's option (a), "the principled end-state"); each stack ships
  a byte-identical **synced copy** under its own `crates/vendor/`, produced
  by a new `cargo xtask sync-engines` and gated by `sync-engines --check`
  as a new self-check step. Stacks stay self-contained workspaces; no vibe
  changes; divergence is mechanically impossible while the gate holds.

Crate disposition: `conform-core`, `specmap-core`, `specmark-grammar` →
discipline-core (authored) + vendored into both stacks. `specmark`
(proc-macro, Rust-tagging-specific), `conform-frontend-rust`, `env-audit`,
`conform-cli`, `specmap-cli`, `discipline-cli` stay rust-stack-authored.
vibevm root `Cargo.toml` repoints its `specmap-core` path-dep to the
discipline-core authored copy (`specmark` stays pointed at the Rust stack).
Vendored dirs are excluded from conform scanning (the `/vendor/` substring,
same mechanism as `/generated/`) and exempted in each stack's own
`specmap.toml` — authored copies carry the tags.

### D2 — TS fact source: hand-rolled lexical scanner (offline-safe)

The Ф6 brief names the TypeScript Compiler API / ts-morph as the parser.
Both need npm at runtime or a Node subprocess; swc needs new crates.io
deps. All three are network-gated on this box and add consumer surface.
**Chosen:** a small hand-rolled lexical scanner in Rust (comment/string/
template-literal aware line lexer) extracting exactly the facts the v1
rules need: import statements, the `unsafe`-set tokens (`any`, cross-type
`as`, non-null `!`, `@ts-ignore`, `@ts-expect-error -- reason`), file
metrics, JSDoc spec tags. Honest labelling: the frontend `id()` is
`"ts-lexical"`, its `version()` starts at 1, and the Ф6 brief gets a
status update naming the lexical MVP and the AST upgrade path (swc or
Compiler API sidecar) as a follow-up that only bumps `version()` and
retires cache slots — the exact mechanism the brief already specifies.
Unparseable constructs degrade to zero facts for that region, never an
error (the B5 rule).

### D3 — TS traceability markers: JSDoc tags, per GUIDE §9

Markers are JSDoc block tags — `/** @implements spec://… */`,
`@verifies`, `@documents`, `@deviates spec://… reason`, `@informs` — the
erasure-clean form GUIDE §9 already prefers over decorators. Module-level
edge: a file-top `/** @scope spec://… */` block mirrors `scope!`. The
scanner sniffs by extension: `.rs` → rscan (specmark), `.ts`/`.tsx`/`.mts`
→ tsscan (JSDoc). URI grammar validation reuses `specmark-grammar`
(now discipline-core-authored — one grammar, both languages). Edge budget
(≤3 per item), two-tier revisions, and asymmetric invalidation carry over
unchanged from PROP-014 — same index schema, same ratchet, so
`specmap.json` needs no format change.

### D4 — demo location and shape: `research/ts-demo`

New top-level `research/` (owner's suggestion) holding `ts-demo/` — a
minimal but REAL consumer: its own git-ignorable `vibedeps/` (bootstrap
documented, not committed — packages content is already in-repo twice),
its own `vibe.toml` (`[project]` + requires
`stack:org.vibevm/typescript-ai-native = "^0.3.0"`) resolved from the
in-repo `packages/` file:// registry (mutable per PROP-011 §2.6, so
package edits propagate on re-install), `tsconfig.json` at the GUIDE §1
floor (strict + noUncheckedIndexedAccess + exactOptionalPropertyTypes +
erasableSyntaxOnly), 2–3 cells (branded type at a seam, Result-shaped
error union, one runtime validator at an erasure boundary), `node:test`
tests runnable via bare `node --test` (no npm install needed),
`spec/PROP-001.md` with anchors, JSDoc markers citing them, and its own
`specmap.toml` (namespace `ts-demo`) + `conform.toml`. `research/` is
inert to vibevm's own gates (scan roots are explicit: `crates/*`, `xtask`)
— Phase 0 verifies.

### D5 — `vibe trace`: delegation, not embedding

`vibe trace <args…>` spawns `discipline-rust trace <args…>` from PATH and
passes the exit code through. On spawn failure it prints the recovery
recipe (`cargo install --path vibedeps/<slot>/crates/discipline-cli`, or
the in-place `cargo run` form) and exits non-zero. Embedding specmap-core
into vibe-cli is rejected: it couples the product binary to one engine
version while projects pin their own stack versions (skew), and delegation
keeps the product surface one function deep.

### D6 — binary delivery: PROP-025 is authored, not implemented

Closing the deferral as it was written ("a future PROP"). The PROP
specifies the whole design (see Phase 9 for the section map); the
documented `cargo install --path` answer remains the shipped mechanism.
Implementation is its own future campaign the owner can commission by
pointing at the PROP.

### D7 — the TS floor v1 composition

`discipline-typescript floor` runs, in order: **typecheck** (`npx tsc
--noEmit`, resolved from the project's node_modules or the npm cache) →
**tests** (`node --test`, the built-in runner — zero deps) → **conform**
(`conform-typescript`) → **specmap** (`specmap-typescript --check`). Four
steps, each with the per-policy origin line (the defaulted-policy
announcement carried over from the Rust floor). prettier/eslint steps and
a TS test-gate (junit/TAP parsing) are NAMED deferrals — absent tooling is
a hard step failure, never a silent skip, so a consumer without
`typescript` installed sees a red floor with the install recipe, not a
green lie.

### D8 — skills twins: adapt, not symlink

`typescript-ai-native` ships `[[skill]]` entries `discipline-sweep-typescript`
and `terraform-typescript` — the Rust skills' procedure skeleton with the
toolchain swapped (`discipline-typescript floor`, tsc/node-test loops, TS
cards, TS quirks). They cite the same discipline-core playbooks (the method
is the flow package's; the skill is the stack's instantiation — the
established split).

## 3. Phase 0 — probes and spikes (no commits; gate for everything after)

1. Re-probe network (ssh gitverse/github, crates.io HEAD) — informational.
2. `npm install --prefer-offline typescript` in a scratch dir — REAL
   install, not dry-run; record whether `npx tsc --version` runs. If the
   cache misses, Phase 6's tsc step is authored but recorded red-pending-
   network in the demo README and the WAL; everything else proceeds.
3. `node --test` on a scratch `.ts` with type annotations + an
   `erasableSyntaxOnly`-clean feature set — confirm v24 strip-types runs it.
4. Vendor-sync build spike: copy `conform-core` into a scratch stack-shaped
   workspace under `crates/vendor/conform-core`, repoint one consumer,
   `cargo build` — validates D1's topology on Windows paths.
5. Confirm `research/` inertness: create the empty dir, run
   `cargo xtask specmap --check` + `conform check` — byte-stable.
6. Acceptance: findings recorded in the WAL session section; any red probe
   downgrades its dependent step per the notes above (nothing else blocks).

## 4. Phase 1 — engine consolidation (discipline-core 0.4.0)

1. Version-bump dirs/manifests first (the 0.3.0→0.4.0 / 0.2.0→0.3.0 moves,
   `git mv`, requirements widened: stacks require core `^0.4`), so every
   later diff lands in final paths. vibevm `specmap.toml`
   `[[external_specs]].root` bumps with the slot on re-materialise.
2. `git mv` `conform-core`, `specmap-core`, `specmark-grammar` from
   `rust-ai-native/v0.4.0/crates/` → `discipline-core/v0.4.0/crates/`;
   discipline-core `Cargo.toml` becomes a workspace (its first code-root —
   PROP-024 applies as written; kind `flow` carries code the same way
   `stack` does).
3. New `cargo xtask sync-engines [--check]`: byte-copies the three crates
   into `rust-ai-native/v0.4.0/crates/vendor/` and
   `typescript-ai-native/v0.3.0/crates/vendor/` (`--check` = compare, exit
   1 on drift; a `.sync-manifest.toml` names source root + crate list so
   the tool is data-driven, not hardcoded).
4. Rust-stack crates repoint deps `../conform-core` → `../vendor/conform-core`
   etc.; vibevm root repoints `specmap-core` to the discipline-core
   authored path; conform/specmap configs gain the `/vendor/` exclusion +
   exempt entries (both stacks' own `specmap.toml`).
5. `self-check.sh` grows a step: `cargo xtask sync-engines --check`.
6. Re-materialise vibedeps (`vibe install`); regen specmap.
7. Acceptance: floor green end-to-end; vibevm index 0 dangling (the
   external_specs bump proven by the same 7 citations still resolving);
   rust-stack package tests + `specmap-rust --gate` green from the
   MATERIALISED slot; `sync-engines --check` green and RED when a vendored
   byte is touched (prove once, revert).
8. Commits: `build(packages): bump the discipline packages for the
   consolidation`, `refactor(discipline-core): take authorship of the
   neutral engine crates`, `feat(xtask): sync-engines vendoring gate`,
   `build(deps): re-materialise vibedeps at 0.4.0`.

## 5. Phase 2 — conform for TypeScript (engine rules + frontend + bin)

1. conform-core (authored home): add the TS fact shapes + rules-as-queries —
   file budget (reuse), `ts-unsafe-in-domain` (the §8 ban set as Class-F
   findings, `@ts-expect-error -- reason` honoured as a recorded deviation
   the way `#[spec(deviates)]` is), `ts-cell-isolation` (imports cross
   seams only — config names the seam filename, default `index.ts`).
   Rules live once in core; the Rust path is untouched (frontends feed
   facts; rule sets keyed by frontend id).
2. New TS-stack crate `conform-frontend-typescript` (lexical, D2) +
   `conform-cli-typescript` (bin **`conform-typescript`**, mirroring
   conform-cli's run/check surface, reading the consumer's `conform.toml`
   with a `[typescript]` section: roots, seam name, domain/exempt dirs).
3. Fixture-driven tests in the TS stack: a dirty fixture tree (an `any`, an
   unchecked `as`, a `@ts-ignore`, an over-budget file, a sibling-cell
   import) → exact findings + exit 1; a clean fixture → 0. The Ф6 brief
   (`tools/conform-frontend-typescript.md`) status flips
   specified → shipped-lexical, with the honest §5 note rewritten.
4. Acceptance: package tests green; vendored copies re-synced; floor green.
5. Commits: `feat(conform): typescript rule set in the neutral core`,
   `feat(typescript-ai-native): ship conform-typescript (lexical frontend)`.

## 6. Phase 3 — specmap for TypeScript (tsscan + bin)

1. specmap-core (authored home): extension-sniffing scanner dispatch;
   new `tsscan` module reading JSDoc tags (D3) into the SAME item/edge
   model; `specmark-grammar` validates URIs at scan time (grammar errors =
   findings, not panics). No index-schema change.
2. New TS-stack crate `specmap-cli-typescript` (bin **`specmap-typescript`**:
   mint/`--check`/`--gate`, mirroring specmap-cli).
3. Tests: fixture TS tree with tagged/untagged exports → index golden,
   orphan ratchet fires, `@deviates` without reason = finding; mixed-tree
   test (one .rs + one .ts root) proves both scanners coexist.
4. Acceptance: floor green; `specmap-typescript --check` reproduces its
   golden byte-for-byte.
5. Commit: `feat(typescript-ai-native): ship specmap-typescript (JSDoc scan)`.

## 7. Phase 4 — the `discipline-typescript` umbrella

1. New TS-stack crate `discipline-cli-typescript` (bin
   **`discipline-typescript`**): `init` (generates the six artifacts in
   their TS shape — conform.toml `[typescript]`, specmap.toml with
   discovered `[[external_specs]]`, both baselines, both registries — the
   Rust init generalised, shared helpers vendored or duplicated-with-test,
   NOT cross-linked to discipline-cli), `floor` (D7's four steps),
   `conform` / `specmap` passthroughs, `trace` (same specmap-core explain —
   works over the mixed index).
2. `test-gate`/`tripwire`/`health`/`fast-loop`/`codemod` subcommands print
   a named not-yet-shipped message citing this plan (visible deferral, not
   absence).
3. Acceptance: `discipline-typescript init && floor` green on a scratch TS
   fixture (tsc step per Phase 0 probe result); package tests green.
4. Commit: `feat(typescript-ai-native): ship the discipline-typescript
   umbrella`.

## 8. Phase 5 — skills twins, boot snippet, card statuses

1. `[[skill]]` × 2 in the TS manifest (D8) + skill trees under
   `spec/skills/`; dogfood `vibe skill install` (projections land,
   gitignored as before).
2. `20-stack-typescript-ai-native.md` boot snippet gains the shipped-
   toolchain block (mirroring the Rust one: bins, install recipe, wiring
   pointer) — GUIDE gets §15-equivalent wiring/sweep sections (the §13/§14
   analog the Rust guide got).
3. Cards INDEX: statuses flip ONLY where a runnable checker now exists
   (F — conform-typescript bans; the file-budget/cell rows; E stays
   specified until the fast-loop twin ships; I stays WISH).
4. Acceptance: `vibe check` clean; skill projection verified; floor green.
5. Commit: `docs(typescript-ai-native): consumer front door - skills,
   boot toolchain block, card statuses`.

## 9. Phase 6 — the demo project + frozen acceptance

1. Author `research/ts-demo` per D4; README states purpose (pilot-lite for
   the TS stack; NOT the VibeVM TS pilot) + bootstrap recipe.
2. Manual walk, recorded in the WAL: `vibe install` from the in-repo
   registry → `discipline-typescript init --namespace ts-demo` → tag →
   `specmap-typescript` mint/check (discipline-core citations RESOLVE
   through the demo's vibedeps — the cross-package resolution proof, TS
   edition) → `floor` (tsc step per probe) → `trace explain`.
3. Freeze the walk as a hermetic package test in the TS stack
   (`crates/discipline-cli-typescript/tests/fresh_ts_project.rs`, engine
   calls, no npm/node dependency — the node-side steps are the manual
   walk's job, same split as the Rust twin).
4. Acceptance: demo floor green (tsc modulo the probe); hermetic test
   green; vibevm floor untouched (`research/` inert).
5. Commits: `feat(research): ts-demo - the typescript discipline walking
   skeleton`, `test(typescript-ai-native): freeze the fresh-ts-project
   bootstrap`.

## 10. Phase 7 — DEBT.md / INTENT.md generated views

1. `discipline-rust ledger render [--check]`: reads
   `discipline/registry/{debt,intent}.json`, writes `discipline/DEBT.md` +
   `discipline/INTENT.md` — deterministic (stable ordering, a "generated
   by; do not edit" header naming the source + regen command); `--check` =
   regenerate-and-compare, exit 1 on drift. debt.json's own header already
   promises "Human view: DEBT.md, generated from this file" — this makes
   the promise true. Grouping: by disposition/state then severity then id;
   each entry renders id, kind, severity, one-line, spec refs.
2. Generate both views for vibevm, commit them; add `ledger render --check`
   to the sweep skill's item list (health-adjacent, not floor — the floor
   stays fast).
3. TS twin: NOT duplicated — the subcommand lives in discipline-cli (Rust
   umbrella) but operates on any project root; the TS umbrella's deferral
   message for `ledger` points at it. (Registries are language-neutral
   JSON; one renderer is enough until the TS umbrella grows its own.)
4. Acceptance: views committed + `--check` green; hand-edit → red (prove
   once, revert); floor green.
5. Commit: `feat(discipline-cli): ledger render - the DEBT/INTENT views`.

## 11. Phase 8 — `vibe trace` product alias

1. vibe-cli subcommand `trace` per D5 (args passed through verbatim;
   spawn-failure prints the install recipe; exit code propagated).
   RUNTIME-GUIDE gains the three-line note.
2. Acceptance: `vibe trace explain spec://vibevm/…` == `discipline-rust
   trace explain …` output on this tree; missing-binary path exercised in
   a test with a scrubbed PATH; `vibe check` clean; floor green.
3. Commit: `feat(cli): vibe trace - delegating alias over discipline-rust`.

## 12. Phase 9 — PROP-025: vibe-native binary delivery (spec only)

Author `spec/modules/vibe-workspace/PROP-025-binary-delivery.md`:

- §1 problem: code-bearing packages ship bins consumers must
  `cargo install --path` by hand (GUIDE §13); n stacks × m tools = manual
  PATH management vibe already knows how to do for itself (PROP-019).
- §2 manifest surface: `[[binary]]` (name, crate path, required toolchain)
  declared by code-bearing packages.
- §3 build step: post-materialise, consent-gated like hooks (PROP-020
  §2.1 consent precedent — an install-time build EXECUTES build.rs/
  proc-macros), `cargo build --release` in the slot, artifacts to a
  content-addressed store (PROP-019 diff-copy precedent; byte-identical
  rebuild = no new instance).
- §4 dispatch: a global shim dir (`~/.vibevm/bin`, prepended by first-run
  tooling; Windows `cmd /c` wrappers — the PROP-015 mcp lesson) whose
  shims resolve per-CWD: walk up to `vibe.lock`, exec THAT project's
  pinned version's artifact (the rustup model), fallback = newest.
- §5 staleness/offline: rebuild on slot-hash change; network-honest
  failure mode (cargo needs crates.io unless the cache is warm); the
  documented `cargo install --path` stays as the degraded manual path.
- §6 cross-package path-deps at materialise time — the D1 (α) mechanism
  specced as the companion feature (manifest path rewriting + its
  interaction with shippable-tree hashing), explicitly staged as v2.
- §7 uninstall/GC + `vibe vars` reporting; §8 security posture (consent
  recorded, scope discipline); §9 MVP cut-line for the implementing
  campaign.

Commit: `docs(spec): PROP-025 - vibe-native binary delivery (specified)`.
Acceptance: specmap ingests the new anchors clean; floor green.

## 13. Phase 10 — machine quirks into `spec/boot/90-user.md` (owner file)

`90-user.md` is owner-owned; this plan's approval is the explicit
instruction. Append a `## Machine quirks (this box)` section carrying the
five DISCIPLINE-SWEEP-v0.2 §3 items verbatim; the sweep manual keeps its
copy with a pointer note flipped to "boot-resident since this campaign".
Commit: `docs(boot): adopt the machine-quirks list into the user snippet`.
(If the owner prefers to hand-edit this file personally, say so on plan
review and this phase drops to a reminder in the checkpoint.)

## 14. Phase 11 — vibe-registry split + campaign checkpoint

1. `crates/vibe-registry/src/lib.rs` is at 599/600. Split by the module-
   grain precedent (the seven-file split of 2026-06-27): extract the
   largest coherent cell(s) into named modules (inspect at execution;
   candidates by shape: fetch/clone orchestration vs manifest parsing vs
   cache bookkeeping), each ≤600 with headroom, re-exports keep the seam
   stable, no behavior change (`cargo test -p vibe-registry` +
   characterization untouched).
2. Re-materialise vibedeps if any package content moved since Phase 5;
   regen specmap; full floor + the shipped `discipline-rust floor` +
   `discipline-typescript floor` on the demo.
3. WAL + CONTINUE.md checkpoint per the standing convention.
4. Commits: `refactor(registry): split lib.rs into module-grain cells`,
   `docs(wal)/docs(continue)` checkpoint pair.

## 15. Risks

- **Offline npm/crates drift** — mitigated by D2 (no new crate deps) and
  Phase 0's real-install probe; worst case the demo's tsc step is
  red-pending-network, recorded, everything else lands.
- **Vendor drift** — impossible while `sync-engines --check` is in
  self-check (Phase 1.5); the gate is proven red once before trust.
- **Index instability across the moves** — the external_specs root bump is
  called out (§1); Phase 1 acceptance pins 0 dangling; every phase regens
  specmap before its floor check.
- **Lexical-scanner false negatives** (a ban token inside exotic syntax) —
  bounded by the B5 zero-facts rule + fixtures; the AST upgrade path is
  named in the Ф6 brief update. False POSITIVES are the real cost — the
  string/template/comment-aware lexer plus the dirty-fixture suite is the
  control.
- **Scope creep in PROP-025** — spec-only is the cut; any "just implement
  the small part" urge goes through the owner.
- **Windows path depth/casing** in vendor copies and the demo walk —
  Phase 0 spike covers the build topology; the demo walk reuses the manual-
  walk script idioms (inline paths, real exit codes).

## 16. Campaign acceptance (what "done" looks like)

```sh
bash tools/self-check.sh; echo "EXIT=$?"        # exit 0, now incl. sync-engines --check
cargo xtask specmap --check                      # 0 suspects / 0 warnings / 0 dangling
cargo xtask conform check                        # 0 findings
discipline-rust floor --path .                   # green (vibevm, Rust floor)
discipline-rust ledger render --check            # DEBT.md / INTENT.md fresh
vibe trace explain "spec://vibevm/common/PROP-000#commits"   # delegates, renders
cd research/ts-demo && vibe install --assume-yes # from the in-repo registry
discipline-typescript floor                      # tsc → node --test → conform → specmap: green
discipline-typescript trace explain "spec://ts-demo/PROP-001#req-…"
# spec/modules/vibe-workspace/PROP-025-binary-delivery.md exists, anchors resolve
# spec/boot/90-user.md carries the quirks; wc -l crates/vibe-registry/src/lib.rs < 550
```

All commits local; mirror and registry publish stay held for the owner's
word (network permitting).
