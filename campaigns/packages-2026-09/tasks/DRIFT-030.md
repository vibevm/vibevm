# DRIFT-030 — the hoist counter learns that the root is a consumer too {#root}

```
<status stage="impl" state="plan" ref="DRIFT-030"/>
```

**Status:** queued — **not dispatched.** Owner decides whether this runs during
Phase B; it rewrites boot composition for every consumer and does not block
markup.
**Executor:** Opus. **Reviewer:** Fable, against §6 verbatim.
**Cluster:** workspace
**Finding:** F-078, restated (campaign LOG, 2026-07-26).
**Supersedes:** DRIFT-029, returned on its §8 stop — its premise was wrong in
two independent ways and both corrections are folded in below.

## 1. Goal {#goal}

A package statically linked by both an aggregator and the root must be
**hoisted and compiled once**, as `static-soft` promises, instead of being
compiled into both and read twice.

## 2. Contract {#contract}

```
> **`static-soft`** — **the default**, the meaning of a bare `link = "static"`.
> Hoisting dedup at **compile time**: a package statically linked by more than
> one consumer is **hoisted** to a shared location (§2.4) and linked **once**;
> each consumer references it.
> — spec://vibevm/modules/vibe-workspace/PROP-038#MODE-STATIC-SOFT
```

```
> When the same package is compiled into several units unhoisted, the model
> sees the same prompt several times and can be confused about which copy is
> authoritative — a correctness hazard the owner weighs above the
> "explicit-over-implicit" cost of a smart default.
> — spec://vibevm/modules/vibe-workspace/PROP-038#duplication-hazard
```

```
> **Within one static zone** … → hoist into `Z`'s `STATIC.md`. Dedup achieved
> **and** the package still loads only when `Z` loads … Within-zone hoisting is
> free and always done.
> — spec://vibevm/modules/vibe-workspace/PROP-038#HOIST-WITHIN-ZONE
```

The observed duplication is the hazard `##duplication-hazard` names, in the
configuration `##MODE-STATIC-SOFT` exists to prevent. The spec is not in doubt;
the code does not implement it at the root boundary.

## 3. Current state {#current}

Verified 2026-07-26 by DRIFT-029 before it stopped. **Do not re-discover.**

- `hoist::soft_static_pulls` — `crates/vibe-workspace/src/boot/hybrid/hoist.rs:58-81`
  — walks only `table`, which is built from **materialised packages**. An
  entry-point node's own static pulls are never counted.
- So each `git-*` member scores **one** puller (`git-practices`), misses the
  `pullers.len() >= 2` threshold at
  `crates/vibe-workspace/src/install/bootgen.rs:56-61`, stays unhoisted, and is
  compiled a second time by the root's `static_transitive_closure`
  (`bootgen.rs:350-374`, reached via `vibedeps/flow-redbook/0.2.0/vibe.toml:29`).
- Why only this slot: all four members declare `link = "static"` in their **own**
  `[boot_snippet]`, and `build_unit_table` resolves undeclared edges off the
  target's suggestion (`hybrid_emit.rs:57-63`), making `git-practices` the
  tree's only intermediate static edge.
- Observable today: `spec/boot/STATIC.md` carries each of the four git-\*
  snippets twice (first copy at lines 363/421/486/514; second, via the
  aggregator's compiled unit, at 557/615/680/708).

**Two things DRIFT-029 disproved — do not reintroduce them:**

- The write at `hybrid_emit.rs:151-153` that puts `spec/boot/{INDEX,STATIC}.md`
  into a slot is **correct and mandated** by PROP-038 `##UNIT-PER-PACKAGE`. Do
  not suppress it.
- There is **no path-based fallback**. `bootgen.rs:305-307` reads a
  statically-linking dependency through its compiled `STATIC.md` by explicit
  design. Suppressing the write would leave that reference dangling, and
  `render_static` hard-errors on a missing contribution
  (`boot_artifacts.rs:256-257`), pinned by
  `boot_artifacts::tests::render_static_errors_on_a_missing_contribution`.

## 4. Required behavior {#behavior}

```
1. Resolve the open interaction FIRST, before changing the counter:
   does `append_hoisted` at the root double-add, given that it pushes
   hoisted members AFTER `compute_effective_boot` has already deduped?
   If counting the root would make the root hold each member from BOTH
   `static_transitive_closure` and the hoist append, the counter is not
   the whole fix. Write the answer in §9 with file:line either way.
2. Count an entry-point node's own static pulls in `soft_static_pulls`,
   so a member pulled by both the root and an aggregator scores two.
3. The member then hoists once. The aggregator's unit references it
   through the `#use` markers PROP-038 §2.5 designs, rather than
   containing a second copy.
4. Materialisation stays byte-idempotent: a second `vibe update --all`
   on an already-updated tree changes nothing.
```

Edge cases: a member pulled by exactly one consumer must **not** start hoisting
— the threshold is two, and `static-hard` must keep its unhoisted local
compilation (`##MODE-STATIC-HARD`). Cross-zone hoisting to the global root
(`##HOIST-CROSS-ZONE`) keeps its existing eager-loading cost; this task does not
change which location is chosen, only who is counted as a consumer.

## 5. Boundaries {#boundaries}

- **Do not touch `spec/**`.** If PROP-038 turns out to disagree with the fix,
  that is a §8 stop — the reviewer lands every spec change in this campaign.
- **Do not touch** `packages/**` or any package manifest. In particular, do not
  change `git-practices`' or any member's `link` to dodge the counter.
- **Do not hand-edit** `spec/boot/STATIC.md` or `spec/boot/INDEX.md`; they are
  generated and the fix is verified by regenerating them.
- **Do not touch** `campaigns/**` except §9 of this file.
- Never edit a golden test to make it pass. `render_static_errors_on_a_missing_contribution`
  in particular is load-bearing and must stay green.

## 6. Acceptance {#acceptance}

```bash
cargo fmt --all
cargo test -p vibe-workspace
bash tools/self-check.sh ; echo "EXIT=$?"
```

Read the floor's **real** exit code; never judge it from a piped `tail`.

Then, from a clean tree:

```bash
cargo run -q -p vibe-cli --bin vibe -- update --all --assume-yes
grep -c "^# Flow: Atomic Commits" spec/boot/STATIC.md
grep -c "vibe:static org.vibevm.world/git-practices" spec/boot/STATIC.md
git status --short
```

- first grep → **1** (it is 2 today);
- second grep → **1**, *not* 0. **DRIFT-029 §6 asserted 0 and that was wrong**:
  under the spec-conformant fix the aggregator still contributes its unit, which
  now *references* the hoisted members instead of containing them;
- `vibedeps/flow-git-practices/0.1.0/spec/boot/STATIC.md` still **exists** —
  §2.1 mandates it;
- `vibedeps/flow-delegation-rules/0.1.0/spec/boot/INDEX.md` still exists;
- a second identical `update --all` leaves `git status` unchanged.

New test: `vibe-workspace` gains a test asserting a package pulled statically by
both an entry-point node and a materialised package is hoisted once. Name it for
the behaviour.

Discipline: `#[spec(implements = "spec://…#anchor")]` citing the PROP-038
anchors in §2, `cargo fmt --all`, clippy clean, one atomic commit, Conventional
Commits, **no AI attribution anywhere**.

## 7. Analogies {#analogies}

`crates/vibe-workspace/src/install/bootgen.rs:56-61` — the existing
`pullers.len() >= 2` threshold is the shape to preserve; the change is to what
feeds `pullers`, not to the rule.

## 8. Stop rule {#stop}

- If §4 step 1 shows the counter fix alone produces a double-add at the root:
  **STOP and report.** Do not add a compensating de-duplication at the append
  site — that would be a second mechanism papering over the first, and which
  mechanism owns dedup is a design question for the reviewer.
- If any PROP-038 unit contradicts the fix: **STOP**, `<!-- REVIEW: … -->` at
  the code point, question in §9, status `returned`.
- **Budget signal:** past **6 files / 200 lines**, stop and return — boot
  composition is load-bearing and a large diff here means the diagnosis is
  incomplete.

## 9. Log {#log}

*(appended by executor / reviewer)*
