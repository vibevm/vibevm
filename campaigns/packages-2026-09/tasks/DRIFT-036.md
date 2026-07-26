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

Every live Rust workspace under `packages/org.vibevm.ai-native/**` is built by
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
      packages/org.vibevm.ai-native/** holding a vendored engine copy
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
