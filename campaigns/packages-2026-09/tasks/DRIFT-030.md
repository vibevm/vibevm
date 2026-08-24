# DRIFT-030 — the hoist counter learns that the root is a consumer too {#root}

```
<status stage="impl" state="plan" ref="DRIFT-030"/>
```

**Status:** **returned** — §8 first bullet fired, 2026-07-26. The §4 step 1 gate
answers *yes*: the counter fix alone double-adds at the root, measured on a
fixture of vibevm's exact shape rather than deduced. The counter is **necessary
and not sufficient**, and who owns the dedup is a design question in the
reviewer's lane. See §9 for the three candidate owners.
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
> — spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-038#MODE-STATIC-SOFT
```

```
> When the same package is compiled into several units unhoisted, the model
> sees the same prompt several times and can be confused about which copy is
> authoritative — a correctness hazard the owner weighs above the
> "explicit-over-implicit" cost of a smart default.
> — spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-038#duplication-hazard
```

```
> **Within one static zone** … → hoist into `Z`'s `STATIC.md`. Dedup achieved
> **and** the package still loads only when `Z` loads … Within-zone hoisting is
> free and always done.
> — spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-038#HOIST-WITHIN-ZONE
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
- Observable today: `spec/boot/STATIC.xml` carries each of the four git-\*
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
- **Do not hand-edit** `spec/boot/STATIC.xml` or `spec/boot/INDEX.md`; they are
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
grep -c "^# Flow: Atomic Commits" spec/boot/STATIC.xml
grep -c "vibe:static org.vibevm.world/git-practices" spec/boot/STATIC.xml
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

### 2026-07-26 — executor: RETURNED under §8, first bullet

**§4 step 1 — the answer is YES: the counter fix alone double-adds at the
root.** Not by an accident of ordering — by two independent write paths into
one location.

The two paths, with file:line:

1. `crates/vibe-workspace/src/install/bootgen.rs:80-86` — the root's
   `node_dependency_boot` closure feeds `boot::compute_effective_boot`, which
   emits a `static` `BootEntry` for every closure member whose link resolves
   static (`crates/vibe-workspace/src/boot.rs:246-288`). The entry's `path` is
   built at `crates/vibe-workspace/src/install/bootgen.rs:305-310`. For a
   git-\* member the link resolves static twice over: `declared_link` is forced
   `Some(Static)` by the root's `static-transitive` edge on `redbook`
   (`bootgen.rs:321-325` via `static_transitive_closure`, `bootgen.rs:350-374`;
   the edge is declared at `vibe.toml:28`), and the member's own
   `[boot_snippet].link = "static"` would resolve it static anyway.
2. `crates/vibe-workspace/src/install/bootgen.rs:87-91` — `append_hoisted`
   (`crates/vibe-workspace/src/install/bootgen/hybrid_emit.rs:310-336`) then
   pushes each `shared` unit's `own_boot_path` unconditionally. That string is
   built by `build_unit_table` at `hybrid_emit.rs:50-51` with the identical
   `{slot}/{source}` shape `bootgen.rs:309` uses, so it is the **same path**.

Nothing dedups between them. `compute_effective_boot` dedups only *within* its
own BFS closure (`bootgen.rs:266-283`, the `visited` set) and never sees the
hoist append — the append happens after it returns, on the same `EffectiveBoot`.
`render_static` then concatenates entry by entry
(`crates/vibe-workspace/src/boot_artifacts.rs:224-261`) with no dedup on `path`,
so the file content is emitted once per entry.

**Measured, not deduced.** A throwaway fixture reproducing vibevm's exact shape
— root `--static-transitive-->` content-minimal aggregator (no `[boot_snippet]`)
`--static-->` member (`[boot_snippet].link = "static"`) — driven through
`apply_resolution`:

- *Baseline (today's code):* root copies of the member text = **2**, aggregator
  copies = **1**. Root markers: the member's own snippet, the aggregator's
  compiled `STATIC.md`, and the member again inside it. This reproduces the live
  `spec/boot/STATIC.xml` defect in a fixture.
- *With §4 step 2's counter applied* (entry-point nodes counted as pullers,
  mirroring `node_dependency_boot`'s link resolution): root copies = **2**,
  aggregator copies = **0**. Root markers:

  ```
  <!-- vibe:static org.vibevm/member — vibedeps/flow-member/1.0.0/boot/member.md -->
  <!-- vibe:static org.vibevm/agg — vibedeps/flow-agg/1.0.0/spec/boot/STATIC.md -->
  <!-- vibe:hoisted org.vibevm/member — text in the root STATIC.md -->
  #use spec://org.vibevm/member
  <!-- vibe:static org.vibevm/member [shared by node.entrypoint/., org.vibevm/agg] — vibedeps/flow-member/1.0.0/boot/member.md -->
  ```

So the counter fix does exactly half the job: the aggregator's local zone
correctly degrades to the `#use` marker `##HOIST-MARKERS` / `##MARKER-USE`
design, and `emit_package_units` / `zone_to_effective` need no change. But the
duplicate simply migrates from *inside the aggregator's unit* to *the root's own
lane*, where it now reads as two `vibe:static` blocks for the same file. §6's
`grep -c "^# Flow: Atomic Commits" spec/boot/STATIC.xml` would still be **2**.

**Why the collision is structural, not incidental.** PROP-038 `##HOIST-LCA`
(`spec/modules/vibe-workspace/PROP-038-hybrid-boot-linking.xml:72`) puts the hoist
target at the least common ancestor *within a continuous static zone*, and
`##HOIST-WITHIN-ZONE` (line 74) calls within-zone hoisting free and always done.
vibevm's chain root → `redbook` → `git-practices` → member is unbroken static, so
the LCA of `{root node, git-practices}` **is the root** — the hoist destination
and the root's own compile site are the same file. The implementation reaches
that location by two routes and only one of them knows about hoisting. The code
comment at `bootgen.rs:87-88` ("The absolute root is the hoist point") is right
about *where*; what is missing is that the root can already hold the text.

This is also why §4 step 2 is *necessary* — `##RES-USE-COUNTER` (line 167) says a
package "forced static by a `static-transitive` ancestor" increments its
static-use count, and today the root's forced subtree increments nothing
(`hoist::soft_static_pulls`, `crates/vibe-workspace/src/boot/hybrid/hoist.rs:58-81`,
walks only `table`, built from materialised packages). The counter is correct and
still wanted. It is just not sufficient.

**§8's first bullet fires verbatim**, so no counter change was landed and no
compensating de-duplication was added at the append site. For the reviewer, the
three candidate owners of the dedup, all consistent with §2's contract and none
implemented here:

1. **The append yields** — `append_hoisted` skips a package whose path the
   root's `EffectiveBoot` already carries. §8 names this the "second mechanism"
   shape and forbids the executor from picking it unilaterally.
2. **The closure entry yields** — the root's dependency-closure entry for a
   `shared` package is suppressed (or flipped to `use_ref`), making
   `append_hoisted` the single owner of every hoisted package's text at the
   root. Keeps the shared-by hint `##MARKER-SHARED-BY` requires on the surviving
   copy.
3. **The root is the hoist destination, never a puller** — a package the root
   already compiles is by construction already at the LCA, so it needs no
   hoisting at all; what needs fixing is the *aggregator's* local zone, which
   should emit the `#use` marker for any member the hoist point already carries,
   `shared` or not. This one satisfies §6 as written without `append_hoisted`
   firing: first grep → 1 (the root's closure copy), second grep → 1 (the
   aggregator's zone entry survives, now containing markers). It also implements
   `##HOIST-LCA` more literally than the current unconditional-root hoist point
   does — and that literal reading is a spec-shaped question, i.e. the
   reviewer's lane per §5.

**Landed:** the `<!-- REVIEW: … -->` marker at the convergence point
(`crates/vibe-workspace/src/install/bootgen.rs:87-107`, house form
`// <!-- REVIEW: … -->` per `crates/progress-core/src/rollup.rs:162`), and this
entry. No behavioural change; §6's `vibe update --all` block was not run (see
below). Budget spent: 2 files, well inside §8's 6-file / 200-line signal.

**Not verified — open items, not assumptions.** The reviewer's instruction for
this run replaced §6's execution with `cargo fmt --all` + `cargo test -p
vibe-workspace` only, because a concurrent markup task holds
`packages/org.vibevm.ai-native/core-ai-native/v0.8.0/` dirty. So
`bash tools/self-check.sh` and the whole `vibe update --all` verification block
were **not** executed and no claim is made about them. (`tools/self-check.sh`
also read as locally modified at the start of this run, by neither this task nor
the markup one.) The header status line still reads `queued — not dispatched`;
the executor's boundary for this run was §9 only, so flipping it to `returned` is
left to the reviewer.
