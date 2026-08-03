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
> — `spec://org.vibevm.core/vibevm/modules/vibe-resolver/PROP-017#root`

> `NaiveDepSolver` stays in tree as the "small graphs / no features / no
> disjunctions" fast path. **The default clause is superseded** … the
> production default became **resolvo**, not `sat`.
> — `spec://org.vibevm.core/vibevm/modules/vibe-resolver/PROP-003#solver-upgrade`

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
- impl 2026-07-25 (Opus). §4.4 taken first: each of the three was read
  against the tree before a word was written. **All three reproduce**, but
  the third reproduces with a twist worth its own paragraph. Two files
  touched, documentation only; no behaviour, no signature, no `#[spec]`
  verb changed; no spec file opened for editing; §8 never fired.

  **Site 1 — `crates/vibe-resolver/src/lib.rs:3`. Reproduces.** Before,
  verbatim:

  ```
  //! Two traits and one implementation in this crate:
  ```

  After, verbatim (`lib.rs:3`):

  ```
  //! Two provider traits, one consumer trait, three solver cells:
  ```

  Verification. Three `impl DepSolver` in the crate: `naive.rs:57`
  (`NaiveDepSolver`), `sat.rs:183` (`Sat`), `resolvo_engine/mod.rs:80`
  (`ResolvoDepSolver`). The trait count was also wrong, which the task did
  not claim: `lib.rs` declares **three** `pub trait`s, not two — `DepProvider`
  (`:195`), `VersionEnumerator: DepProvider` (`:246`), `DepSolver` (`:301`).
  Hence the split phrasing rather than "three traits and three
  implementations": two of them face the registry world and one faces the
  consumer, which is the distinction PROP-017 §5 `SWAP-BOUNDARIES` draws and
  the sentence now carries it. The bullet list under the line was extended to
  match — `ResolvoDepSolver` and `Sat` had no entry at all, and
  `VersionEnumerator` is folded into the `DepProvider` bullet since it is a
  supertrait of it. Naive's three pinned-limitation bullets are unchanged:
  each was re-checked and each still holds, and `naive.rs:3` points here for
  them.

  **Site 2 — `crates/vibe-resolver/src/lib.rs:27–33`. Reproduces.** Before,
  verbatim:

  ```
  //! When any of these limits start hurting real users — capability
  //! routing across packages-not-yet-seen, optimal-version-after-merging
  //! constraints, disjunction backtracking — that is the trigger for
  //! adding a `ResolvoSolver` (PROP-002 §2.8 primary). The traits are
  //! shaped so the swap is one new `impl DepSolver`, no consumer-side
  //! changes. Same `GitBackend`-style indirection PROP-001 §2.2 uses to
  //! leave the door open for `libsolv`.
  ```

  After, verbatim (`lib.rs:33–47`):

  ```
  //! Those three limits were the trigger list for adding a `ResolvoSolver`
  //! (PROP-002 §2.8 primary), and **it fired**: decided 2026-06-14, port
  //! complete (PROP-017 §6). Constraint-merging and disjunction
  //! backtracking are resolvo's by construction (`[[requires_any]]` → a
  //! resolvo `Union`, PROP-017 §3); capability routing now reaches any
  //! provider in the transitive closure via [`resolvo_engine`]'s pre-scan,
  //! stronger than naive's already-seen match but still blind to a
  //! provider no package edge reaches — the one part of the trigger still
  //! live, and it now points at a registry reverse-index (PROP-017 §8),
  //! not at a solver. The swap cost what the seam promised consumer-side —
  //! one new `impl DepSolver`, no consumer, manifest, or lockfile change
  //! (PROP-017 §5) — plus one world-side enrichment,
  //! [`VersionEnumerator`]. The `GitBackend`-style indirection PROP-001
  //! §2.2 uses still holds the door open for `libsolv`, which PROP-002
  //! §2.8 keeps as the feature-gated fallback; that trigger has not fired.
  ```

  **§4.2 — the trigger inventory.** The block carries five claims that read
  as pending. Each was checked against the tree separately.

  *Fired (rewritten to say what happened):*

  1. **"optimal-version-after-merging constraints"** — fired. resolvo is CDCL
     SAT; `sort_candidates` (`resolvo_engine/provider.rs:247`) orders a name's
     solvables descending so the first solution is newest-feasible over the
     merged constraint set, which is PROP-017 §3 `ROW-PREFER-NEWEST`.
  2. **"disjunction backtracking"** — fired, and demonstrably.
     `provider.rs:285–312` maps `[[requires_any]]` onto a resolvo `Union`
     ("native OR + backtracking", PROP-017 §3 `ROW-REQUIRES-ANY`), and
     `resolvo_engine/tests.rs:276`
     `resolvo_disjunction_backtracks_past_conflicting_alternative` pins the
     exact case naive dies on.
  3. **"capability routing across packages-not-yet-seen"** — fired *in part*,
     and the surviving part changed target, which is why the new text spends
     three lines on it rather than one word.
     `resolvo_engine/capabilities.rs:43` `prescan` walks the transitive
     closure over `[requires.packages]` and `[[requires_any]]` edges across
     every available version, indexes every `[provides]`, and a
     `[requires.capabilities]` entry becomes a `Union` over the matching
     providers — so a provider the solve has not yet *processed* is now
     reachable, which naive cannot do. A provider that **no package edge
     reaches** is still invisible. PROP-017 §8 `FUTURE-CAPABILITY-INDEX`
     records precisely that remainder and re-uses the same words —
     "the trigger is capability routing across packages-not-yet-seen becoming
     load-bearing" — but now for a **registry reverse-index**, not for a
     solver. Saying "fired" flat would have been as wrong as leaving it.
  4. **"the swap is one new `impl DepSolver`, no consumer-side changes"** —
     fired, and half-held, so it is recorded as a cost rather than deleted.
     The consumer half held exactly (PROP-017 §5 `swap-recipe`: "No consumer,
     no manifest, no lockfile change"). The world-facing half did not: a
     candidate-choosing solver needs to see candidates, so the provider seam
     was enriched — PROP-017 §2.2 `ENUMERATION-NEEDED`, in tree as
     `VersionEnumerator: DepProvider` (`lib.rs:246`), which is what
     `ResolvoDepSolver` takes as its bound (`resolvo_engine/mod.rs:80`).

  *Not fired (left standing, deliberately):*

  5. **"Same `GitBackend`-style indirection PROP-001 §2.2 uses to leave the
     door open for `libsolv`"** — **accurate today**, and the only reason it
     was re-worded at all is that it shared a sentence with claim 4. PROP-002
     §2.8 `LIBSOLV-FALLBACK-SLOT` is live and current: "primary impl is
     `ResolvoSolver`; a future `LibsolvSolver` (FFI to C libsolv,
     BSD-3-Clause) drops in as a feature-gated alternative if resolvo ever
     hits a ceiling we can't raise." No `libsolv` exists anywhere in
     `crates/**` (the only hits are RPM/libsolv *vocabulary* references in
     `vibe-core`'s weak-deps docs). The new text says the trigger has not
     fired, in as many words, so a later reader is not left guessing whether
     it was checked.

  **Site 3 — `crates/vibe-workspace/src/freshness.rs:218`. Reproduces, but
  only half of it is false — recorded because §4.4 asked.** Before, verbatim:

  ```
  /// This *holds* the pins; it does not *skip* the registry walk. Skipping
  /// the walk for an unchanged subtree needs the depsolver's pin-preference
  /// machinery (PROP-003 §2.1), deferred with the SAT solver.
  ```

  After, verbatim (`freshness.rs:216–222`):

  ```
  /// This *holds* the pins; it does not *skip* the registry walk. Skipping
  /// the walk for an unchanged subtree needs the depsolver's pin-preference
  /// machinery — `pin_preferences` on `DepSolver` (PROP-003 §2.1), once
  /// deferred "with the SAT solver". Sat shipped, resolvo shipped as the
  /// production default, and neither brought the method; the walk stays,
  /// and preference inside the solve would be resolvo's `sort_candidates`
  /// (PROP-017 §3).
  ```

  Reasoning. The *predicate* is true and stays: the machinery really is
  absent — the shipped `DepSolver` declares `solve` and nothing else
  (`lib.rs:301–304`), and DRIFT-014 kept the `deviates` on that trait for
  exactly this gap. What is false is the **peg**: "deferred with the SAT
  solver" makes the absence contingent on an event that has since happened
  twice over — `Sat` shipped (`sat.rs:128`), then `ResolvoDepSolver` shipped
  as the default (`vibe-cli/src/registry.rs:117` `unwrap_or("resolvo")`,
  `:191`) — and neither brought `pin_preferences` with it. So the sentence
  was not deleted and not merely re-dated: it now says the deferral outlived
  its peg and names where the capability would actually land, resolvo's
  `sort_candidates` (`resolvo_engine/provider.rs:247`), which is the same
  landing site DRIFT-014's rewritten `deviates` reason names. §3's one-word
  summary ("'deferred with the SAT solver', which shipped") is right about
  the defect and understates which clause carries it.

  **Numbers (§6).** `wc -l crates/vibe-resolver/src/lib.rs`: **571 → 586**,
  14 under the 600-line budget; no split needed, so the edge case did not
  fire. Second budget worth flagging, because the task did not know it:
  `crates/vibe-workspace/src/freshness.rs` went **588 → 592**, only 8 under
  the same budget — the site-3 rewrite was deliberately compressed to seven
  lines for that reason, and the next edit to that file should expect to
  split it. `cargo doc -p vibe-resolver --no-deps`: builds, **5 warnings
  before and 5 after, none in the edited region** — `conditional.rs:4`
  (unresolved `Requires`), `resolvo_engine/mod.rs:4` and `:6` and `sat.rs:14`
  (public docs linking private items), `lib.rs:229` (redundant explicit link
  target, pre-existing on the `VersionEnumerator` doc, shifted down 15 lines
  by this diff). Every intra-doc link added here resolves, including
  `[`Sat`](sat::Sat)` and `[`resolvo_engine`]`.

  **Adjacent drift found, deliberately left standing** — same discipline
  DRIFT-014 applied to these three. `crates/vibe-resolver/src/naive.rs:3–4`
  reads "See [`crate`] module docs for the pinned limitations and **when to
  upgrade to a SAT-style solver**." That upgrade landed twice (`Sat`, then
  resolvo), so the pointer promises a future the same file already contradicts
  twenty-five lines lower, in the rustdoc DRIFT-014 wrote at `naive.rs:31–39`.
  It is F-059's shape exactly and it is not in `findings.json`; it is a
  two-line fix and it was **not** taken, because §4.1 names three sites and
  absorbing a fourth is what F-059 exists to prevent. Following the pointer
  still lands a reader on correct text, so nothing is broken today.

  **Gates run.** `cargo fmt --all` (clean).
  `cargo test -p vibe-resolver -p vibe-workspace` **all green**: vibe-resolver
  87 unit + `compile_fail` 1 + `differential_oracle` 1 + `embedded_provider` 1
  + `fixpoint_conformance` 3 + `recommends` 3 + `solver_properties` 7 + 11
  doctests; vibe-workspace 174 unit + 19 doctests. 0 failed on every target.

  `cargo xtask conform check`: **0 findings, 0 frozen in baseline, 0 new** —
  same as before this task. `bash tools/self-check.sh` (no flags, no
  override, against the developer's real `~/.vibe/`): **all green, exit 0** —
  all nine steps, including step 2 `cargo test --workspace` and step 3
  `cargo clippy --workspace --all-targets -- -D warnings`, the two that would
  catch a mistake here, plus `vibe check`, `sync-engines --check`, all four
  package gates (core-ai-native, rust-ai-native-lang, rust-ai-native-mcp,
  typescript-ai-native-mcp) and all four package self-traces (0 gated orphans
  each). The floor is left exactly as it was found.

  Two concurrency artefacts recorded so a reviewer reading the timestamps is
  not puzzled; neither is this task's and neither was "fixed" here:

  - An intermediate conform run reported **2 new findings**, both in another
    agent's then-uncommitted work —
    `crates/vibe-registry/src/git_backend/shell.rs:1` file-length 614 lines
    (**592 at HEAD**, under budget; that agent's +22 pushed it over) and
    `crates/vibe-cli/src/commands/progress/tests.rs:42` no-unwrap-in-domain
    (480 at HEAD → 536 in the tree; the flagged `.expect()` sits in a
    `payload_for` helper this task never saw). Neither edited file ever
    appeared in the finding set. Both cleared on their own once that agent
    landed its split (`git_backend/shell/query.rs`), and conform returned to
    0 without anything being done to them here.
  - One `cargo test --workspace` run died at
    `error: linking with `link.exe` failed: exit code: 1104` on
    `vibe-resolver` test `compile_fail` — a Windows output-file lock from a
    concurrent cargo, the same class DRIFT-014 hit on `vibe.exe`. Per the
    concurrency rule it was waited out, not diagnosed; the immediate retry
    compiled and passed.
