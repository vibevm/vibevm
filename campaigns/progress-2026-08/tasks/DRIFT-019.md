# DRIFT-019 — three module docs stop describing the pre-port world {#root}

<status stage="impl" state="plan" ref="DRIFT-019"/>

**Status:** queued
**Executor:** Opus. **Reviewer:** Fable, against §6 verbatim.
**Cluster:** resolver (module documentation)
**Unit-stability check:** PROP-003 §2.1 and PROP-017 were corrected in Phase
D; the contracts these docs describe now state the shipped truth.

## 1. Goal {#goal}

The last three places in the tree that still say resolvo has not arrived
stop saying it.

## 2. Contract {#contract}

> **Status.** … **the port is COMPLETE** — `ResolvoDepSolver` is the shipped
> production default (`crates/vibe-cli/src/registry.rs:117`, `--solver`
> defaults to `resolvo`).
> — `spec://vibevm/modules/vibe-resolver/PROP-017#root`

> `NaiveDepSolver` stays in tree as the "small graphs / no features / no
> disjunctions" fast path. **The default clause is superseded** … the
> production default became **resolvo**, not `sat`.
> — `spec://vibevm/modules/vibe-resolver/PROP-003#solver-upgrade`

Finding realised: **F-059**.

## 3. Current state {#current}

Found by DRIFT-014 while fixing F-047's three `deviates`, and deliberately
left standing rather than absorbed into that diff — do not re-discover:

- `crates/vibe-resolver/src/lib.rs:3` — "Two traits and **one
  implementation** in this crate". There are three: `NaiveDepSolver`,
  `Sat`, `ResolvoDepSolver`.
- `crates/vibe-resolver/src/lib.rs:27–33` — names "adding a `ResolvoSolver`
  (PROP-002 §2.8 primary)" as a **future** trigger. It is three lines above
  a site DRIFT-014 just corrected, which is what makes it worth a task
  rather than a shrug: the file now contradicts itself.
- `crates/vibe-workspace/src/freshness.rs:218` — "deferred with the SAT
  solver", which shipped.

Ground truth, as in DRIFT-014: `crates/vibe-cli/src/registry.rs:117`
`unwrap_or("resolvo")` and `:191` constructing it.

## 4. Required behavior {#behavior}

1. Rewrite each of the three to describe the tree as it is. Where a line
   names a future trigger that has fired, say what happened instead of
   deleting the sentence — the trigger's history is why the code looks the
   way it does.
2. `lib.rs:27–33` is a trigger list. Read the whole list, not just the
   resolvo entry: any other trigger there that has already fired gets the
   same treatment, and any that has not stays untouched. Report both sets
   in §9.
3. `freshness.rs:218` sits next to `hold_pins`, which DRIFT-014 verified as
   the shipped mechanism (`vibe-workspace/src/freshness.rs:219`, called from
   `vibe-install/src/plan.rs:219`). Say what the code does now.
4. **Read before you write.** Each of these is a claim about the tree; check
   it against the tree rather than against this task. If any of the three
   turns out to be accurate after all, say so in §9 and leave it — a
   finding that does not reproduce is worth recording as such.

Edge cases: `vibe-resolver/src/lib.rs` is **571 lines against a 600-line
conform budget** (DRIFT-014 measured it). If your edits would cross 600,
stop and say so rather than splitting the file — that is a different task.

Error paths: none — documentation only.

## 5. Boundaries {#boundaries}

- **Documentation only.** No behaviour, no signatures, no `#[spec]` verb
  changes — DRIFT-014 already ruled on those three and they are settled.
- Do not touch `crates/vibe-cli/src/registry.rs` (conform construction site
  R-001).
- Never edit spec text.

## 6. Acceptance {#acceptance}

```bash
cargo test -p vibe-resolver -p vibe-workspace
cargo xtask conform check      # 0 findings, and lib.rs stays under 600 lines
bash tools/self-check.sh
```

- Each of the three quoted verbatim in §9, before and after.
- `wc -l crates/vibe-resolver/src/lib.rs` reported in §9.
- `cargo doc -p vibe-resolver --no-deps` builds without new warnings.
- Discipline: `cargo fmt --all`, clippy clean, atomic commits, no AI
  attribution.

## 7. Analogies {#analogies}

The rustdoc DRIFT-014 wrote on `NaiveDepSolver` and on `Sat`'s module
header — same crate, same day, the register to match: what is true, why it
is acceptable, and what it costs to choose it.

## 8. Stop rule {#stop}

If correcting a doc would require asserting something about behaviour you
cannot verify in the tree: STOP, record the question in §9, return.

Budget signal: past ~3 files or ~120 lines, stop and return.

## 9. Log {#log}

- queued 2026-07-25 (Fable). F-059 exists because DRIFT-014 surfaced these
  instead of quietly folding them into its own diff.
