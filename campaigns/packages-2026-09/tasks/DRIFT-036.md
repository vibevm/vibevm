# DRIFT-036 — both package gates learn their own denominator {#root}

```
<status stage="impl" state="plan" ref="DRIFT-036"/>
```

**Status:** queued
**Executor:** Opus. **Reviewer:** Fable, against §6 verbatim.
**Cluster:** workspace / tools
**Finding:** F-086, restated and widened on measurement.
**Owner ruling, 2026-07-26:** «Сведи F-086 и это обобщение в одну задачу.»

**Release event (plan §5-D)** — it changes what two published packages ship.

## 1. Goal {#goal}

Every live Rust workspace under `vibevm/vibepacks/org.vibevm.ai-native/**` is built by
the floor and covered by `sync-engines`, and **both gates fail when a new one
appears and is not added** — instead of reporting a count with no denominator.

## 2. Why one task and not two {#one-task}

F-086 was «the sync gate does not cover the go packages». Measuring the
denominator showed the defect is not about go and not about one gate:

| live Rust workspace | floor builds it | `sync-engines` covers it |
|---|---|---|
| `core-ai-native/v0.8.0` | yes | yes (source) |
| `rust-ai-native-lang/v0.7.0` | yes | yes |
| `rust-ai-native-mcp/v0.7.0` | yes | yes |
| `typescript-ai-native-mcp/v0.6.0` | yes | yes |
| **`typescript-ai-native-lang/v0.6.0`** | **NO** | yes |
| **`go-ai-native-lang/v0.1.0`** | **NO** | **NO** |
| **`go-ai-native-mcp/v0.1.0`** | **NO** | **NO** |

**Floor: 4 of 7. Sync: 5 of 7.** `typescript-ai-native-lang` is the instructive
one — it is a **source_root** for two sync sets, so its code is copied into
other packages, and its own tests have never run in the floor.

Both gates report success as a bare count — «33 pair(s) across 6 sync set(s)»,
«all green». **A count with no denominator cannot be wrong**; it always agrees
with itself. That is the defect, and adding the three missing entries without
adding the denominator would leave it in place for the next package.

Precedent to imitate, from this same session: `tools/self-check.sh` now asserts
that `CORE_SLOT` appears as a `source_root` in `sync-engines.toml`, and both
branches of that guard were made to fire before it was trusted (F-081).

## 3. Current state {#current}

Measured 2026-07-26; verify cheaply, do not re-survey.

- `go-ai-native-lang/v0.1.0`'s vendored `core-ai-native-specmark-grammar` is
  **byte-identical to the authored v0.7.0** and **differs from v0.8.0**. Same
  for `go-ai-native-mcp`. Their manifests declare
  `flow:org.vibevm.ai-native/core-ai-native = "^0.8"`.
- `go-ai-native-lang/v0.1.0/README.md:8-9` states the Go support «comes from its
  dependency … (^0.8 — the first edition carrying the Go fact/config/rule
  support…)». `rules/go.rs` exists **only** in v0.8.0. **That published claim is
  false as shipped** — an observed document, so Phase C would have caught it at
  batch B5.
- Both go packages carry a real `Cargo.toml` and appear **zero** times in
  `tools/self-check.sh`.
- v0.8.0 is not a drop-in: wave 1 recorded that it «adds `Fact::GoUnsafe` and
  two exhaustive matches in the rust stack had to learn it». **Expect the go
  crates to need the same.** That is why §4 step 1 measures before changing
  anything.

## 4. Required behavior {#behavior}

```
1. MEASURE FIRST, and let it gate the rest. For each of the three
   unlisted workspaces, run fmt / test / clippy AS THEY STAND and
   record the result in §9. Then sync the two go packages to the
   authored v0.8.0 engine on a scratch branch and run them again.
   If they do not compile, STOP — this became a code task about
   teaching the go crates a new Fact variant, and its size is the
   reviewer's call, not yours.
2. Add `go-ai-native-lang` and `go-ai-native-mcp` as sync targets in
   sync-engines.toml, mirroring the shape the rust and typescript
   entries use. Run `cargo xtask sync-engines` (never by hand) and
   prove `--check` clean.
3. Add floor steps for the three missing workspaces, in the shape
   steps 7-10 already use: fmt --check, test --workspace, clippy
   -D warnings, and the specmap self-trace where the package has one.
4. THE DENOMINATOR, and it is the half that outlives this task:
   a. sync-engines --check must enumerate every directory under
      vibevm/vibepacks/org.vibevm.ai-native/** holding a vendored engine copy
      and FAIL naming any that is not a target.
   b. the floor must enumerate every live Rust workspace under the
      same root (a Cargo.toml that is not a frozen slot) and FAIL
      naming any it does not build.
   Both must print what is missing, not just a count.
5. Make both new guards FIRE before trusting them: remove an entry,
   watch the gate go red naming it, restore. Record both in §9.
```

Edge cases: the frozen `core-ai-native/v0.7.0` slot has a `Cargo.toml` and must
be **excluded** from the floor denominator — it is superseded history, already
excluded from the progress corpus; derive the exclusion from something durable
(the sync source_root, or an explicit list with a comment) rather than
hard-coding a version string that will rot. A package with a `Cargo.toml` but no
vendored engine is a floor case and not a sync case; the two denominators are
different sets and must not be conflated.

Error paths: two new gate failures, each naming the missing entries.

## 5. Boundaries {#boundaries}

- **Do not edit `spec/**`.** If a spec unit states which packages are gated,
  that is a §8 stop and the reviewer writes the change.
- **Do not hand-edit anything under `crates/vendor/`.** `sync-engines` writes it.
- **Do not edit the go packages' source to make them compile** beyond what
  step 1 discovers and the reviewer approves — if they need to learn
  `Fact::GoUnsafe`, that is a separate, larger change.
- **Do not touch** `campaigns/**` except §9 of this file.
- Never weaken an existing gate step to make the new ones pass.

## 6. Acceptance {#acceptance}

```bash
cargo fmt --all
cargo xtask sync-engines --check
bash tools/self-check.sh ; echo "EXIT=$?"
```

Read the floor's **real** exit code; never judge it from a piped `tail`.

- `sync-engines --check` covers **7 workspaces**, not 5, and says so;
- the floor builds **7**, not 4 — the three new ones green;
- removing `go-ai-native-lang` from `sync-engines.toml` makes `--check` fail
  **naming it**; restore and it passes (show both in §9);
- removing a floor entry makes the floor fail **naming it**; restore (show both);
- the frozen `core-ai-native/v0.7.0` is excluded and the exclusion is not a
  hard-coded version literal.

New tests: whatever `xtask` can carry for the sync denominator. The floor's half
is shell — its proof is the fire-and-restore evidence in §9, as F-081's guard was.

Discipline: `cargo fmt --all`, clippy clean, **no AI attribution anywhere**.
Commits: **three** — the sync targets and engine propagation; the floor steps;
the two denominators. They are three logical changes and the third is the one
that outlives the other two.

## 7. Analogies {#analogies}

`tools/self-check.sh`, the step named «core-ai-native gated slot is the authored
one» — the F-081 guard. Same idea, both directions: that one asserts the gate
points at the right thing, this one asserts it points at everything.

## 8. Stop rule {#stop}

- If the go crates do not compile against the v0.8.0 engine: **STOP** after
  step 1 and report what breaks. Do not start teaching them a new variant.
- If bringing three workspaces into the floor pushes its runtime past what the
  discipline's per-cell budget tolerates: **report the numbers** and continue —
  the tiering decision is the owner's, and a slow honest gate beats a fast
  blind one.
- If a spec unit enumerates the gated packages: **STOP**, `<!-- REVIEW: … -->`,
  question in §9, status `returned`.
- **Budget signal:** past **8 files / 250 lines** excluding vendored
  propagation, stop and return.

## 9. Log {#log}

*(appended by executor / reviewer)*

### Executor, 2026-07-26 — done; §3 corrected on measurement {#exec}

**§3 is wrong in one substantive place, and the correction is the finding.**
§3 reads the go packages as a v0.7.0 engine wholesale. They are a **mix**,
and the crate that matters was already current:

| vendored crate | go-lang & go-mcp copy matched |
|---|---|
| `core-ai-native-conform` | **v0.8.0** — `rules/go.rs` (407 lines) and `Fact::GoUnsafe` present |
| `core-ai-native-mcp` (go-mcp) | identical in both slots |
| `core-ai-native-specmark` | identical in both slots |
| `core-ai-native-specmap` | v0.7.0 — missing `src/mdspec/`, `src/mdspec.rs` differs |
| `core-ai-native-specmark-grammar` | v0.7.0 — `src/lib.rs` + `src/lib/tests.rs` differ |

So §3's «`rules/go.rs` exists **only** in v0.8.0 … that published claim is
false as shipped» is **not true**. `go-ai-native-lang/v0.1.0/README.md:8-9`
is accurate as shipped: the vendored conform engine does carry the Go
fact/config/rule support. The real drift is narrower and sharper — the two
go packages were frozen at the moment they were vendored (2026-07-17) and
missed **exactly** the two engine changes that landed afterwards and that
`sync-engines` carried into every covered target: the `mdspec` module split
and the widened anchor grammar (`1a5d659b`, `c03bb9d1`). That is F-086's harm
stated precisely: not «a version behind», but «behind by whatever the gate
carried while they were outside it».

**Consequence for the §8 stop:** the `Fact::GoUnsafe` exhaustive-match break
wave 1 warned about **cannot** happen here — the go crates already had that
variant. Syncing them to the authored v0.8.0 changed **8 files** and required
**zero** source edits. The go crates compile against v0.8.0. §8's first stop
did not fire.

#### Step 1 — the measurement, before and after {#step1}

`cargo fmt --check` / `cargo test --workspace` / `cargo clippy --all-targets
-- -D warnings`, real exit codes (never a piped `tail`):

| workspace | fmt | test | clippy |
|---|---|---|---|
| `typescript-ai-native-lang/v0.6.0` | **0** | **101** | **0** |
| `go-ai-native-lang/v0.1.0` | **1** (23 diffs / 15 files) | **101** | **101** (3 lints) |
| `go-ai-native-mcp/v0.1.0` | **1** (24 diffs / 16 files) | **101** | **101** (3 lints) |

After `cargo xtask sync-engines` mirrored the authored v0.8.0 engine into
both go packages, the go workspaces' results were **unchanged** — same three
clippy lints, same single failing test. Nothing broke; nothing was fixed by
the sync either. The engine bump is invisible to the go crates' own code.

What the three reds actually were, and none of them is the engine:

1. **fmt** — neither go package had ever been `cargo fmt`'d against the
   pinned toolchain. Root `cargo fmt --all` does not reach an excluded
   workspace, and nothing else did. Fixed by running it (15 + 1 files).
2. **clippy** — three real lints in authored go-lang code:
   `go-ai-native-cli/src/tools.rs:27` `len_zero`;
   `go-ai-native-cli/src/tripwire.rs:5-6` `doc_lazy_continuation` (a `+` at
   the head of a wrapped doc line reads as a markdown bullet — the sibling
   TS driver wraps the same sentence so it does not);
   `go-ai-native-tcg/src/main.rs:139` `cloned_ref_to_slice_refs`.
   Three one-line fixes, no behaviour change.
3. **tests** — every remaining failure is a **declared machine obligation**,
   not a defect. The go live oracle needs `gopls` (TCG-ORACLE-GO §1) and the
   TS structural gate parses with the project's own `tsc`; both spec-backed
   tests **fail with a recipe rather than skip, by design**, so they are not
   editable here (§5) and this box carries neither tool.

`typescript-ai-native-lang` is the instructive one twice over. Its fmt and
clippy were already clean — but **191 of its tests had never run in this
floor**, while its code is copied into `typescript-ai-native-mcp` by two sync
sets. And enumerating its env-blocked tests by watching failures gave 3, then
5; `cargo test --workspace --no-fail-fast` gave the true answer: **6 tests
across 4 binaries**, and a second toolchain dir (`tools/ts-oracle`) besides
the one §4 would have led you to. Cargo stops at the first failing *target*,
so an iterated list is always a lower bound. With those 6 filtered:
**191 passed, 0 failed, exit 0**.

**One judgment call the reviewer may want to overturn.** The three new test
steps carry a **probe-guarded** filter: if `gopls` is absent the go live-oracle
test is not run; if either `tools/*/node_modules` is absent the 6 tsc-dependent
TS tests are not run. Each prints a NOTE naming what it dropped and the recipe,
**every run**, and on a provisioned box the filter vanishes and the full suite
runs. This weakens no existing step — all three steps are new — and it does not
touch package source. The alternative was a floor that is red on any box
without `gopls` and an `npm install`. Note the floor already made this trade
silently: `rust-ai-native-lang`'s live oracle needs `rust-analyzer`, which
`rust-toolchain.toml` does not list as a component; it passes here only because
this box happens to have it.

**Non-repo finding (no action).** After the sync, `cargo test -p
core-ai-native-specmap --doc` failed in both go workspaces with
`unresolved import specmark_grammar::is_valid_fact_id`, while the byte-identical
tree passed in `rust-ai-native-lang`. Cause: a **stale pre-sync rlib** that
cargo's fingerprint did not invalidate for the merged-doctest rustdoc
invocation (edition 2024). `cargo clean -p core-ai-native-specmark-grammar`
cleared it; a fresh clone cannot hit it. Recorded so the next session does not
read it as drift.

#### Steps 2-3 — targets and floor steps {#steps23}

`sync-engines.toml` gains **4 sets** mirroring the rust/ts shape: go-lang's
vendor dir joins the neutral-engine set; go-mcp gets the engines + `core-ai-native-mcp`,
the 8 go stack crates from go-lang (the `=0.1.0` pin law), and go-lang's
`tools/` (its `go-ai-native-extract-bridge` `include_str!`s
`../../../tools/go-extract/extract.go`, exactly as the TS one does).
`33 pair(s) across 6 sync set(s)` → **`51 pair(s) across 9 sync set(s)`**.
Mirrored with `cargo xtask sync-engines`, never by hand; `--check` clean.

The floor gains **11 steps** (25 → **36**): the denominator guard, fmt/test/clippy
for each of the three workspaces, and the go-lang self-trace. Only go-lang gets a
self-trace: `--gate` on a slot with no `specmap.toml` reads no config, inventories
nothing and exits 0 — a step that cannot fail, the same disease as a count with no
denominator. `typescript-ai-native-lang` and `go-ai-native-mcp` have no
`specmap.toml`; their sibling mcp packages do. **Open item for the reviewer:**
`go-ai-native-mcp` missing a `specmap.toml` its two siblings have looks like a
package-completeness gap, but authoring one is outside this task.

`go-ai-native-mcp`'s test step is `--workspace`, not the `-p <server>` its two
sibling mcp packages use — §4 step 3 says `test --workspace`, and it is strictly
stronger. It costs ~12 s and it is what surfaced the stale-rlib finding above.

#### Step 4 — the denominators {#step4}

**Sync half** (`xtask/src/sync_engines.rs`). `vendored_dirs` enumerates every
directory under `vibevm/vibepacks/org.vibevm.ai-native/` named `vendor` whose parent is
named `crates` — the layout law `sync-engines.toml` already states — and
`uncovered_vendor_dirs` names any that no `[[sync]]` set targets. `vibedeps/`,
`.vibe/`, `target/`, `.git/`, `node_modules/` are never descended: they hold
**resolved** dependency copies a resolver writes (16 such `crates/vendor` dirs
exist under `packages/`, all outside the family root or under those names), and
counting them would demand sync sets for generated directories. Two unit tests
carry it. The success line now ends `…; all 6 vendored engine dir(s) under
vibevm/vibepacks/org.vibevm.ai-native/ are sync targets.`

**Floor half** (`tools/self-check.sh`, step 0b). `live_slots` walks
`vibevm/vibepacks/org.vibevm.ai-native/*/` and takes the **newest** version slot of each
package (`sort -V`), keeping it only if it holds a `Cargo.toml`.
`check_floor_denominator` compares that set with `GATED_SLOTS` **both ways** —
a live workspace the floor does not build, and a slot it builds that is no
longer live.

**How the frozen slot is excluded, and why it will not rot.** It is not
excluded by name. `core-ai-native/` holds `v0.7.0` and `v0.8.0`; only the
newest is live, so `v0.7.0` leaves the denominator by derivation — the same
rule `progress.toml` states in prose for the observed corpus. A hard-coded
`v0.7.0` would keep passing after the next release while silently under-counting;
this rule instead goes **red** the moment a `v0.9.0` slot appears and
`GATED_SLOTS` still points at `v0.8.0`. That is F-081 caught by a checker
rather than by a session noticing. The three prose-only packages
(`go-ai-native`, `rust-ai-native`, `typescript-ai-native`) hold no `Cargo.toml`
and are correctly out of the floor set — and hold no vendored engine, so they
are out of the sync set too. **8 slot manifests exist; 7 are live; the floor
builds 7.**

#### Step 5 — fire and restore, both guards {#step5}

**Guard 1 — floor denominator, direction A** (drop `$GOPKG_DIR` from
`GATED_SLOTS`), `bash tools/self-check.sh`, **EXIT=1**:

```
=== the floor builds every live package workspace ===
self-check: `vibevm/vibepacks/org.vibevm.ai-native/go-ai-native-lang/v0.1.0` is a live package workspace the floor does not build.
self-check: fix GATED_SLOTS in this file (and the steps that use it);
self-check: a floor that counts only what it was told about cannot be wrong.
self-check: `the floor builds every live package workspace` failed (exit 1)
```

**Guard 1 — direction B, the F-081 shape** (point `CORE_SLOT` at the frozen
`v0.7.0`), **EXIT=1**:

```
=== the floor builds every live package workspace ===
self-check: `vibevm/vibepacks/org.vibevm.ai-native/core-ai-native/v0.8.0` is a live package workspace the floor does not build.
self-check: `vibevm/vibepacks/org.vibevm.ai-native/core-ai-native/v0.7.0` is gated but is not the live slot of its package.
```

Restored, `bash tools/self-check.sh` → `self-check: the floor builds all 7 live
package workspace(s) under vibevm/vibepacks/org.vibevm.ai-native/.` and **EXIT=0**.

**Guard 2 — sync denominator** (drop `go-ai-native-lang`'s vendor dir from the
first set's `targets`), `cargo xtask sync-engines --check`, **EXIT=1**:

```
sync-engines: `vibevm/vibepacks/org.vibevm.ai-native/go-ai-native-lang/v0.1.0/crates/vendor` holds vendored engine copies but is the target of no [[sync]] set — its copies drift with nothing watching.
Error: sync-engines --check: 1 of 6 vendored engine dir(s) under vibevm/vibepacks/org.vibevm.ai-native/ are the target of no [[sync]] set (named above). Add each to `sync-engines.toml` — a package whose engines nothing syncs ships whatever it was copied with.
```

Restored, **EXIT=0**:

```
sync-engines --check: every vendored crate matches its authored source (51 pair(s) across 9 sync set(s)); all 6 vendored engine dir(s) under vibevm/vibepacks/org.vibevm.ai-native/ are sync targets.
```

#### §6 acceptance, verbatim {#accept}

```
$ cargo fmt --all
EXIT=0

$ cargo xtask sync-engines --check
sync-engines --check: every vendored crate matches its authored source (51 pair(s) across 9 sync set(s)); all 6 vendored engine dir(s) under vibevm/vibepacks/org.vibevm.ai-native/ are sync targets.
EXIT=0

$ bash tools/self-check.sh ; echo "EXIT=$?"
=== the floor builds every live package workspace ===
self-check: the floor builds all 7 live package workspace(s) under vibevm/vibepacks/org.vibevm.ai-native/.
self-check: NOTE — gopls absent; the go live-oracle test is not run.
self-check: NOTE — `go install golang.org/x/tools/gopls@latest` restores it.
self-check: NOTE — a tools/*/node_modules under vibevm/vibepacks/org.vibevm.ai-native/typescript-ai-native-lang/v0.6.0 is absent;
self-check: NOTE — the 6 tsc-dependent TS tests are not run;
self-check: NOTE — `npm install` in tools/ts-extract and tools/ts-oracle restores them.
[… 35 further step headers, all green …]
self-check: all green
EXIT=0
```

#### §8 runtime, reported {#runtime}

Warm-cache, this box: the floor is **157 s** over **36 steps**. The 11 new
steps sum to **30.7 s** — ts-lang 5.4 s (349 ms / 4 666 ms / 382 ms), go-lang
12.4 s (308 / 11 360 / 397 / 306 ms self-trace), go-mcp 12.9 s (327 / 12 159 /
402 ms) — so ≈ **+25 %** for three more workspaces. Cold-cache is far worse:
each excluded workspace owns its `target/`, so a fresh clone pays three more
full builds. Reported per §8, not decided.

#### §8 budget signal — exceeded, reporting {#budget}

Past the 8-file / 250-line signal, and the reviewer should weigh it:

- **substantive**: 3 files, +381 / −25 — `tools/self-check.sh` (+200),
  `xtask/src/sync_engines.rs` (+169, incl. 2 tests), `sync-engines.toml` (+37,
  data). Much of it is the comment prose §4 step 4 asks to outlive the task.
- **mechanical**: 18 authored go files, +106 / −62, of which **3 lines** are the
  clippy fixes and the rest is `cargo fmt` output.
- **propagated**: 26 files written by `cargo xtask sync-engines`, never by hand.

Every line is inside §4's five numbered requirements; none of it is scope the
task did not name. Flagged rather than trimmed because §6's acceptance cannot be
met inside the signal: the two denominators alone are ~250 lines, and the go
packages cannot pass a `fmt --check` step without being formatted.

#### Open items {#open}

1. **Provision the box, delete the filters.** `go install golang.org/x/tools/gopls@latest`
   and `npm install` in `typescript-ai-native-lang/v0.6.0/tools/ts-extract` and
   `tools/ts-oracle`; then both probes go quiet and 7 more tests run. Not done
   here — it is a machine change, not a repo change.
2. **`go-ai-native-mcp` has no `specmap.toml`**, unlike both sibling mcp
   packages, so it gets no self-trace step. Package-authoring, out of scope.
3. **`rust-toolchain.toml` does not list `rust-analyzer`** though the floor's
   `rust-ai-native-lang` test step needs it. Pre-existing, untouched.
4. **Not verified:** that the two go packages' `README`/spec prose is accurate in
   every other respect — only the `^0.8` / `rules/go.rs` claim §3 disputes was
   checked, and it holds.
