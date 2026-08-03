# DRIFT-035 — the boot lane carries each statically-linked package once {#root}

```
<status stage="impl" state="plan" ref="DRIFT-035"/>
```

**Status:** queued
**Executor:** Opus. **Reviewer:** Fable, against §6 verbatim.
**Cluster:** workspace
**Finding:** F-078, third state (campaign LOG, 2026-07-26).
**Owner ruling, 2026-07-26:** «сделай что хочешь с вариантом (c), посчитай.»
**Supersedes:** DRIFT-030, returned on its §8 stop after measuring that the
counter fix alone moves the duplicate rather than removing it.

## 1. Goal {#goal}

`spec/boot/STATIC.md` carries each statically-linked package exactly once, with
every other consumer referencing it — what `##MODE-STATIC-SOFT` promises and
what the tree does not do.

## 2. Contract {#contract}

```
> **`static-soft`** — **the default** … a package statically linked by more than
> one consumer is **hoisted** to a shared location (§2.4) and linked **once**;
> each consumer references it.
> — spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-038#MODE-STATIC-SOFT
```

```
> **Within one static zone** … → hoist into `Z`'s `STATIC.md`. Dedup achieved
> **and** the package still loads only when `Z` loads … Within-zone hoisting is
> free and always done.
> — spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-038#HOIST-WITHIN-ZONE
```

```
> Across dynamic zones … → the only shared always-loaded location is the
> **global root** `STATIC.md`, and hoisting there makes the package **eager**
> — spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-038#HOIST-CROSS-ZONE
```

`##HOIST-CROSS-ZONE` is quoted because it is the case the fix must **not**
break: there, the root genuinely does not carry the package and the append must
still fire.

## 3. Current state {#current}

Established by DRIFT-030, **measured on a fixture, not deduced.** Do not
re-derive; do re-run the measurement (§4 step 1).

- Two write paths reach the root's static lane and neither knows the other:
  **(1)** `compute_effective_boot` (`crates/vibe-workspace/src/boot.rs:246-288`),
  fed by the root's closure at `install/bootgen.rs:80-86`, path built at
  `bootgen.rs:305-310`; **(2)** `append_hoisted`
  (`install/bootgen/hybrid_emit.rs:310-336`), called at `bootgen.rs:87-91`,
  pushing `own_boot_path` built at `hybrid_emit.rs:50-51` with the **identical**
  `{slot}/{source}` shape. `render_static` (`boot_artifacts.rs:224-261`)
  concatenates entry by entry with **no dedup on `path`**.
- `hoist::soft_static_pulls` (`boot/hybrid/hoist.rs:58-81`) walks only `table`,
  built from materialised packages, so an entry-point node's own static pulls
  are never counted. Each git-\* member scores **one** puller, misses the
  `pullers.len() >= 2` threshold (`bootgen.rs:56-61`), and stays unhoisted.
- Measured on a fixture of this repo's exact shape (root `static-transitive` →
  content-minimal aggregator `static` → member with its own `link = "static"`):

  | | root copies | aggregator copies |
  |---|---|---|
  | today | **2** | 1 |
  | counter fix alone | **2** | 0 |

  The aggregator's zone correctly degrades to the `#use` marker §2.5 designs;
  the duplicate **migrates** into the root's own lane.
- Why: `##HOIST-LCA` puts the hoist target at the LCA of a *continuous static
  zone*. Here the chain root → redbook → git-practices → member is unbroken
  static, so the LCA **is the root** — the hoist destination and the root's own
  compile site are the same file.
- Live symptom: each of the four git-\* snippets appears twice in
  `spec/boot/STATIC.md`.

## 4. Required behavior {#behavior}

**Step 1 is a measurement and it gates the rest.** Reasoning about this
subsystem has been wrong twice; a fixture settled it in one run each time.

```
1. Rebuild DRIFT-030's fixture (§9 of that task describes it) and
   record the baseline: root copies 2, aggregator 1. If it does NOT
   reproduce, STOP — the tree has moved and the diagnosis needs redoing.
2. Apply the counter fix: count an entry-point node's own static pulls
   in `soft_static_pulls`, so a member pulled by both the root and an
   aggregator scores two. Re-measure. Expect root 2, aggregator 0.
3. Apply the rule below and re-measure. Expect root 1, aggregator 0
   with a `#use` reference.

THE RULE, stated precisely so it does not over-reach:

    `append_hoisted` skips a package that the root's own effective-boot
    closure ALREADY emits. It still fires for a hoist the root does not
    otherwise carry — which is exactly the ##HOIST-CROSS-ZONE case,
    where the consumers are separated by a dynamic edge and the root
    holds no copy of its own.

4. Both mechanisms must agree on ONE notion of "the root already emits
   this". Derive the skip from the closure the root actually compiled,
   not from a second computation of what it should have compiled — two
   computations that agree today are the next thing nothing keeps
   honest, and this campaign has found four of those.
5. Materialisation stays byte-idempotent: a second `vibe update --all`
   on an already-updated tree changes nothing.
```

Edge cases: a package with exactly one static consumer must **not** start
hoisting (threshold stays two); `static-hard` keeps its unhoisted local
compilation (`##MODE-STATIC-HARD`); a cross-zone hoist still lands in the root
and stays eager; a root that does **not** statically pull the package at all is
unaffected.

Error paths: none new. Suppressing a duplicate append removes no diagnostic.

## 5. Boundaries {#boundaries}

- **Do not edit `spec/**`.** PROP-038 wants a sentence saying the root is a
  hoist destination and not a puller — **the reviewer writes it.** A spec doubt
  is a §8 stop.
- **Do not suppress the write of a slot's own boot artifacts.** PROP-038
  `##UNIT-PER-PACKAGE` mandates them; DRIFT-029 returned for proposing exactly
  that, and doing it would turn a duplicated boot lane into a **failed install**
  (`render_static` hard-errors on a missing contribution, pinned by
  `boot_artifacts::tests::render_static_errors_on_a_missing_contribution`).
- **Do not hand-edit** `spec/boot/STATIC.md` or `INDEX.md`; they are generated
  and the fix is proven by regenerating them.
- **Do not touch** `packages/**` or any manifest's `link` to dodge the problem.
- **Do not touch** `campaigns/**` except §9 of this file.

## 6. Acceptance {#acceptance}

```bash
cargo fmt --all
cargo test -p vibe-workspace
bash tools/self-check.sh ; echo "EXIT=$?"
```

Read the floor's **real** exit code.

Then, from a clean tree:

```bash
cargo run -q -p vibe-cli --bin vibe -- update --all --assume-yes
grep -c "^# Flow: Atomic Commits" spec/boot/STATIC.md
grep -c "vibe:static org.vibevm.world/git-practices" spec/boot/STATIC.md
git status --short
```

- first grep → **1** (it is 2 today);
- second grep → **1**, *not* 0 — the aggregator still contributes its unit; that
  unit now *references* the members instead of containing them;
- `spec/boot/STATIC.md` shrinks by roughly the 192 lines the duplicate occupies;
- `vibedeps/flow-git-practices/0.1.0/spec/boot/STATIC.md` still **exists**;
- a second identical `update --all` leaves `git status` unchanged.

**Report the three fixture measurements from §4 as a table in §9** — baseline,
counter-only, counter+rule. That table is the evidence this task exists to
produce, and it is worth more than the diff.

New tests: one asserting a package pulled statically by both an entry-point node
and a materialised package lands in the root's lane exactly once; one asserting
a cross-zone hoist still lands. Name them for the behaviour.

Discipline: `#[spec(implements = "spec://…#anchor")]` citing the §2 anchors,
`cargo fmt --all`, clippy clean, **two commits** (the counter, then the rule —
they are two logical changes and the fixture table shows why), **no AI
attribution anywhere**.

## 7. Analogies {#analogies}

`bootgen.rs:56-61` — the `pullers.len() >= 2` threshold is the shape to
preserve; step 2 changes what feeds `pullers`, never the rule itself.

## 8. Stop rule {#stop}

- If §4 step 1's baseline does not reproduce: **STOP.**
- If step 3's measurement does not give root 1: **STOP and report the table.**
  Do not add a second compensating dedup to reach the number — that is the
  failure mode this task inherits from DRIFT-030, and which mechanism owns dedup
  would then be a reviewer question again.
- **Budget signal:** past **8 files / 250 lines**, stop and return. Boot
  composition is load-bearing and a large diff means the diagnosis is incomplete.

## 9. Log {#log}

*(appended by executor / reviewer)*
