# D6c — `source-mirrors` + `licensing` v0.1.0: nine claimed absences, re-verified before demotion

_Worked 2026-07-29. Subjects:
`packages/org.vibevm.world/source-mirrors/v0.1.0/` and
`packages/org.vibevm.world/licensing/v0.1.0/`. Five obligations, all
`build-or-demote`, 9 drift verdicts. Every one asserts that some mechanism,
gate, record or audit line **does not exist**._

_This batch is worked under
[§3.7 `#compliance-blindness`](../PHASE-D-BATCH-PLAN.md#compliance-blindness) and
[§6.1 `##ABSENCE-NAMES-ITS-PERIMETER`](../PHASE-D-BATCH-PLAN.md#delegation-lessons):
a demotion is the **last** step, not the first, and a `not-found` is a fact
about the search perimeter until the perimeter has been checked. Measured over
the previous wave's 76 `build-or-demote` verdicts, 18 claimed absences were
false and 17 of those were disproved by HOST artefacts. **Every entry below
names the perimeter it searched.** No code was written; no `git` command that
writes was run; `cargo xtask mirror` was **not** run (it pushes to real
remotes — its source was read instead); nothing under
`campaigns/packages-2026-09/run/` was touched._

Obligations: F-204 · F-333 · F-237 · F-238 · F-315.
(F-332, the sixth obligation on `source-mirrors`, is the dangling `../flows/…`
pointer family and is deliberately out of this batch — its repair is an address
repair the owner has already ruled on. `spec/boot/62-flow-source-mirrors.md`
was not touched.)

**The standing perimeter** (referred to below as *the standing perimeter*), run
from the repository root:

```
packages/**  vibedeps/**  crates/**  xtask/**  tools/**  spec/**
discipline/**  terraform/**  research/**  campaigns/**  legacy-spec/**
fixtures/**  schemas/**  docs/**  manual-tests/**
and the repository root's own *.md / *.toml / *.json / *.sh / *.ps1
minus  **/target/**  .git/**  **/node_modules/**  campaigns/*/run/**
```

`refs/**` is searched but reported **separately** — it is a third-party study
corpus, not our shipped surface, and a hit there is not an implementation of
ours. Its whole yield for this batch is at the foot of this file.

**Why that perimeter and not the package.** Both packages are *tool-neutral*
flows: they specify a discipline, and this host repository is the project that
adopted it — `spec/common/PROP-016-source-mirrors.md:8` names the
`source-mirrors` flow as its own general model and thins itself to «what is
specific to vibevm», and `spec/boot/STATIC.md:802-852` carries the licensing
snippet verbatim into every session's boot. A mechanism in this family has four
layers — SPEC in the package, ENGINE in a host crate, DRIVER in a CLI, and
DEPLOYMENT in the consuming project. A fact can be true at any one and invisible
at the other three.

**A fifth place a `world` flow's mechanism can live, and it decided two of this
batch's nine verdicts: the package's own reference implementation.**
`fanout-mechanics.md` ships fifteen lines of `sh` (lines 166-193 as this pass
found them; 178-205 after the two demotions below lengthened the file). When a
prescription is falsified only by the host's port and the package's own script
keeps it, the absence is the consumer's and §3.6 route (b) applies; when the
package's own script breaks its own rule, §3.6 route (a) applies and the package
yields. That distinction splits F-204 down the middle.

---

## F-204 — the fan-out's fail-loud contract: the commit list is built by nothing, but the ancestry gate is built by the package's own script and only the host omits it

**Outcome:** MIXED — 2 DEMOTED (both PARTIAL), 1 ROUTE-B CANDIDATE
**Anchors:** 2 touched of 3. Demoted: `##RESPONSE-ABORT-THAT-TARGET`,
`##SUM-A-NON-FAST-FORWARD-ABORTS-THAT-TARGET-LOUD`. **Not touched, recommended
route (b):** `##INVARIANT-THE-ANCESTRY-GATE`.
**Files touched:**
`packages/org.vibevm.world/source-mirrors/v0.1.0/spec/flows/source-mirrors/fanout-mechanics.md`
**Perimeter searched:** the standing perimeter, for the *thing* rather than the
verdict's string — `merge-base` · `is-ancestor` · `ls-remote` · `rev-list`
(a commit range has to be computed by something), plus a **filename** sweep for
any second port of the fan-out (`find -iname '*fanout*' -o -iname '*mirror*'`),
because an absent implementation cannot be grepped for; plus `tools/*.sh`,
`tools/*.ps1` and `mirrors.toml`, because the flow's own reference
implementation is a shell script and a port could be one too; plus the whole of
`xtask/src/mirror.rs` read line by line rather than grepped. `cargo xtask
mirror` was **not executed** — it pushes to real remotes.

**What the search found:**

```console
$ rg -n 'merge-base|is-ancestor' <standing perimeter>
packages/…/source-mirrors/v0.1.0/spec/flows/source-mirrors/fanout-mechanics.md:181
vibedeps/flow-source-mirrors/0.1.0/…/fanout-mechanics.md:151
packages/org.vibevm.fractality/…/vibedeps/flow-source-mirrors/…:151      (vendored)
packages/org.vibevm.fractality/…/.vibe/cache/…/source-mirrors/…:151      (cached)
spec/terraforms/PACKAGES-ACTUALIZATION-CAMPAIGN-v0.1.md:3143             (a citation)
campaigns/packages-2026-09/…/ev-W6b.json, batch-W6b-3.json, baseline.json (this campaign's own evidence)
```

Every hit is this document, a vendored or cached copy of it, a citation of it,
or the campaign's own verdict text. **No implementation of the ancestry gate
exists outside the flow's own reference script.**

```console
$ rg -n 'ls-remote|rev-list' --glob '*.rs' crates xtask
xtask/src/mirror.rs:157    let out = git(root, &["ls-remote", url, &format!("refs/heads/{MAINLINE}")])?;
xtask/src/mirror.rs:160            "git ls-remote {url}: {}",
…  (crates/vibe-publish, crates/vibe-registry — the package registry, a different
    concern per PROP-016 §3)

$ rg -n 'ls-remote|rev-list|merge-base' tools/ mirrors.toml
(no output)

$ find . \( -iname '*fanout*' -o -iname '*mirror*' \) -not -path './.git/*' \
       -not -path '*/target/*' -not -path './refs/*'
mirrors.toml · xtask/src/mirror.rs · crates/vibe-cli/src/commands/registry/config/mirror.rs
(a registry mirror, a different concern) · manual-tests/M1.6-mirror-vendor-smoke.md ·
docs/commands/registry-set-mirror.md · the package + its vendored/cached copies ·
this campaign's own run/mirror/ shards
```

`rev-list` appears nowhere in the workspace at all, and `xtask/src/mirror.rs`
is the **only** port of the fan-out in the repository — there is no `fanout.sh`.
Reading it settles all three anchors:

- `remote_main` (`xtask/src/mirror.rs:156-168`) is the sole `ls-remote` caller
  and is reached only from `probe` (`:331`) and the `Mode::SelfPull` arm
  (`:306`). The `Mode::Push` arm (`:284-304`) goes straight from `push_args` to
  `git push` with nothing between. So the client-side ancestry gate is genuinely
  absent from the host, and the protective *outcome* survives only because git
  refuses a non-fast-forward server-side and `push_args` has no force spelling
  to override it with.
- On a rejected push the operator gets `FAIL   {name} {ref} -- {git stderr}`
  (`xtask/src/mirror.rs:296-303`) and then the bail *"mirror: N push(es) failed
  -- a non-fast-forward means a target diverged (someone wrote it directly);
  reconcile by hand, never --force: {pairs}"* (`:313-320`). The **target** is
  named; **no commit is**.
- The never-force half is not merely kept, it is hardened past this document's
  ask: `push_args` is a pure function (`xtask/src/mirror.rs:262-268`) and
  `push_args_never_force` (`:426-440`) asserts no `--force`, `-f` or
  `+`-prefixed refspec for four ref shapes.
  `spec/common/PROP-016-source-mirrors.md:64` books that as «runnable capital,
  not prose» — this document's own §never-force-test wording.

**The find that splits the obligation, and it is the package's own script.**
The reference implementation **does** perform the ancestry gate —
`git ls-remote` then `git merge-base --is-ancestor` — and it does **not**
enumerate the divergent commits: the next line prints
`"$name: DRIFT — host has commits mainline lacks; reconcile by hand"`,
a statement that a divergence exists, not a list of it. So the two failing
clauses have opposite falsifiers:

| clause | flow's own script | host's port | §3.6 route |
|---|---|---|---|
| ancestry gate before every push | **implements it** (`:192-193`) | absent | **(b)** |
| «the commits it has that mainline lacks» | **does not implement it** (`:194`) | absent | **(a)** |

*Line-number convention for this entry: every `fanout-mechanics.md` number in
the console transcripts above is **pre-edit**, because the searches preceded
the demotions. Numbers quoted in prose and in the table are **post-edit** — the
script now sits at `:178-205`, its `ls-remote` at `:192`, its
`merge-base --is-ancestor` at `:193`, its DRIFT line at `:194`, and
`##INVARIANT-THE-ANCESTRY-GATE` at `:212`. `xtask/src/mirror.rs` was not
edited, so its numbers are stable.*

**Which layer has it, if any:** **package spec + the package's own reference
implementation** for the ancestry gate; **host crate** (`xtask/src/mirror.rs`)
for the loud per-target abort, the non-zero exit, the never-force invariant and
its unit test, and the reconcile-by-hand instruction; **nowhere** for the
divergent-commit list.

**What changed and why:** two facts keep their prescription word for word and
gain a *Half built* clause naming exactly which half fails and what was
searched; both markers move `@impl/done` → `@spec/done`. Neither is flattened
to «not built», because a flat demotion of
`##SUM-A-NON-FAST-FORWARD-ABORTS-THAT-TARGET-LOUD` would deny three clauses
that ship — one of them the marquee never-force invariant this repository has
under unit test and cites in its own PROP. The marker census on the file
confirms exactly two moved: lines carrying `@impl/done` 52 → 50, lines carrying
`@spec/done` 5 → 7.

**Why `##INVARIANT-THE-ANCESTRY-GATE` was NOT touched — this is route (b), and
the routing is the boss's, not mine.** The sentence is *«The two invariants to
preserve when you port it: the **ancestry gate** before every push, and the
**absence of any force path**»* — a norm addressed to a porter, and §3.3's
demotion premise («the fact promises a mechanism nothing implements») is
**false here**: the mechanism is implemented, nineteen lines above the anchor,
in the very script the anchor says to port. Demoting it would print «specified,
not built» over a gate this package ships in `sh`. The absence is the host's
port, which preserved one invariant of two — and the port is the *older*
artefact: the Phase C verdict records `xtask/src/mirror.rs` as commit
`5a1d3139` of 2026-06-14 against the package's `ff4ccdf7` of 2026-07-07, so the
flow generalised an origin that never had the gate and then prescribed it.
(I did not re-derive that with `git`; it is cited as the verdict's own record.)
The repair is a host obligation — «add the pre-push `ls-remote` +
`merge-base --is-ancestor` gate to `fan_out`, which would also make the
commit-list clause implementable» — or an owner ruling that the host
deliberately relies on git's server-side refusal instead, which is §3.6(c) and
would need the exception written down host-side. **Recommendation only. I
recorded no routing and wrote no verdict.**

**New obligations noticed — the consistency corollary fires here.** §3.7's
corollary says that where a verdict was restated to match its siblings, the
whole set must be re-verified. Four sibling anchors in this file rest on the
same two facts I have just split, and none is in F-204's anchor list:

1. `##STEP-VERIFY-ANCESTRY` (l. 72-74, `@impl/done`) and
   `##SUM-FAN-OUT-PER-TARGET-IS-FETCH-VERIFY-PUSH-REPORT` (l. 230-231) assert
   the four-step shape whose step 2 the host does not run — but whose step 2
   the package's own script *does* run. They are the same route-(b) shape as
   `##INVARIANT-THE-ANCESTRY-GATE` and must not be demoted on a host-only
   search.
2. `##A-NON-FAST-FORWARD-MEANS-THE-HOST-CARRIES-A-MAIN-YOU-LACK` (l. 88-90) is
   a statement about git's behaviour, not about a port, and is true as written.
3. A real host defect, outside my anchor list and not fixed: `probe`
   (`xtask/src/mirror.rs:327-342`) tests **equality**, not ancestry —
   `Some(sha) if sha == head => InSync`, everything else `Drift` — so a target
   legitimately *behind* mainline is reported as drifted by `mirror --check`
   and by `health --mirrors`. That bears on
   `##CHECK-FETCHES-EACH-TARGET-AND-COMPARES-TO-MAINLINE` and
   `##SYNC-MEANS-LEVEL-DRIFT-NAMES-A-HOST-THAT-MOVED`.

**Verdict recommendation, per anchor:**
`##RESPONSE-ABORT-THAT-TARGET` → **demoted, now confirmed** — the host half was
always built, the commit half is built by nothing including this document's own
script, and the clause now says so.
`##SUM-A-NON-FAST-FORWARD-ABORTS-THAT-TARGET-LOUD` → **demoted, now confirmed** —
three clauses ship (one under unit test); the fourth carries the body row's
failure per the summary-restatement precedent, and the clause names all four.
`##INVARIANT-THE-ANCESTRY-GATE` → **drift stands, route (b)** — the gate is
implemented in this package's own reference script at `:192-193`; only the
consumer's port omits it, so the package must not move.

---

## F-333 — the revisit trigger the host was said never to have recorded, recorded in the flow's own prescribed form; and the marker was already at the honest state

**Outcome:** RE-JUDGE: confirmed (false absence)
**Anchors:** 0 touched of 1. Not touched: `##SUM-WHAT-IT-COSTS`.
**Files touched:** `none`
**Perimeter searched:** the standing perimeter, plus a deliberate **narrowing
back onto the verdict's own file** to reproduce its command, then a widening
from the string to the practice — `revisit` · `revisit when` · `when to
revisit` · `revisit trigger` · `deferred until` · `until needed` ·
`worth opening if` · `when a host must` · `integrator` · `parallel` — and the
body section of the same protocol document that this summary summarises, which
is where the flow defines what «a revisit trigger» means in its own words.

**What the search found — two things, and the second one first, because it
disposes of the obligation on its own.**

**The marker is already at the honest state.** §3.3's closure moves
`@impl/done` → `@spec/done`. `##SUM-WHAT-IT-COSTS` is **already `@spec/done`**:

```console
$ sed -n '189,191p' packages/…/source-mirrors/v0.1.0/spec/flows/source-mirrors/SOURCE-MIRRORS-PROTOCOL.md
- ##SUM-WHAT-IT-COSTS What it costs: one human serializes merges. Acceptable — and cheaper
  than the alternative — for small-team projects; record a revisit
  trigger for the day it is not. @spec/done
```

Every other bullet in that summary list carries `@impl/done`; this one does
not. There is no demotion available here and none was made.

**And the trigger is recorded.** The verdict rests on one command, and I ran it
unchanged — it reproduces:

```console
$ rg -n -i "revisit|parallel|integrator" spec/common/PROP-016-source-mirrors.md
(exit 1 — no match)
```

Then the same file, searched for the **thing** instead of the spelling:

```console
$ rg -n -i "deferred until|until needed|worth opening if|when a host must" \
      spec/common/PROP-016-source-mirrors.md
72:1. ##open-server-side **Server-side mirroring.** When a host must originate writes
   outside `cargo xtask mirror` (e.g. heavy web-UI merging on one host), add
   one-directional server-side mirroring (a GitHub Action mirroring GitHub→GitVerse,
   or GitVerse's own pull-mirror for the reverse). It touches CI secrets (an owner
   act), so it is deferred until needed. @spec/work
74:3. ##open-vibe-mirror-surface … Whether the two should share code is a FEAT worth
   opening if the target set grows large. @spec/work
```

**Why that is the trigger and not merely an adjacent open question — the flow
says so itself, in the body this summary summarises.** `SOURCE-MIRRORS-PROTOCOL.md`
§costs (l. 126-157) defines both the condition and the remedy:

```console
$ sed -n '153,157p' packages/…/spec/flows/source-mirrors/SOURCE-MIRRORS-PROTOCOL.md
##when-a-project-outgrows-one-integrator-this-is-the-wrong-tool When a project outgrows one
integrator — several full-time committers merging in parallel all day —
this model is the wrong tool, and the honest answer is to add
one-directional server-side mirroring or move to a shared-forge
workflow. @spec/done

##RECORD-THAT-AS-A-REVISIT-TRIGGER Record that as a revisit trigger, not a someday-maybe. @impl/done
```

The host's `PROP-016:72` names the flow's own remedy — **«one-directional
server-side mirroring»**, word for word — under a stated condition («when a
host must originate writes outside `cargo xtask mirror`»), with the reason it
is not done yet («it touches CI secrets, an owner act») and a state marker
(`@spec/work`, under a section header carrying
`<status stage="spec" state="work" comment="B1 2026-07-24: three questions still open, no owner ruling yet"/>`).
That is a recorded trigger with a condition, a remedy, and a reason — not a
someday-maybe, which is exactly the distinction `##RECORD-THAT-AS-A-REVISIT-TRIGGER`
draws. The Phase C verdict in fact **found this artefact and named it** — «the
escape hatch is recorded in the prescribed one-directional server-side form at
`PROP-016` §5 open question 1» — and then declared the revisit trigger absent
anyway, on a grep for the word. The escape hatch *is* the revisit trigger; the
flow's own remedy sentence and its trigger sentence are one bullet apart.

**The practice, for the record, is the host's default and its form is compiled
into the boot lane.** The `decision-records` flow's four-field record reaches
every session at `spec/boot/STATIC.md:255` — *«**When to revisit** | A
measurable trigger: metric + threshold + where it is observed»* — and
`:299-300` forbids *«a decision with a missing reason or a missing revisit
trigger — that is a fact with decoration, not a record»*. The host writes them
under several spellings, none of which a single-word grep of one file could
reach: `spec/common/PROP-000.md:23` (`##LANG-REVISIT`), `:57`
(`##LICENSE-REVISIT`, whose previous trigger is recorded as **fired** on
2026-07-12 and spent), `spec/modules/vibe-registry/PROP-001-git-backend.md:113-121`,
`spec/modules/vibe-cli/PROP-036-package-tree.md:95`,
`spec/modules/vibe-progress/PROP-043-progress-markup.md:98,141,255`.

**On the verdict's second clause — «the *acceptable and cheaper than the
alternative* evaluation is absent from every host document searched, so the
summary asserts a judgement the host never made».** Two corrections. The
judgement is the **flow's**, made for a genre («for small-team projects»), not a
report of a host judgement — its body carries it at
`SOURCE-MIRRORS-PROTOCOL.md:145-148`, `##for-a-small-team-the-trade-is-strongly-positive`,
already `@spec/done`. And the host did record the comparison, in the place a
decision record lives: `spec/common/PROP-016-source-mirrors.md:78`
(`##HIST-AUTHORED`, «2026-06-14 — authored, in force»), which closes
*«Supersedes the interim multi-push-remote and the abandoned
bidirectional-multi-master sketch»* — naming the two alternatives this model
was chosen over.

**Which layer has it, if any:** **host deployment** — the trigger is at
`spec/common/PROP-016-source-mirrors.md:72`, in the consuming project, because
recording it is what complying with this fact means. That is §3.7's structure
exactly: a search confined to `packages/` cannot see a trigger whose whole
purpose is to be written down by the adopter.

**What changed and why:** nothing. Demoting `##SUM-WHAT-IT-COSTS` would have
been impossible in form (already `@spec/done`) and false in substance (the
adopter kept the norm, in the flow's own prescribed words).

**New obligations noticed:** `##RECORD-THAT-AS-A-REVISIT-TRIGGER`
(`SOURCE-MIRRORS-PROTOCOL.md:157`, `@impl/done`, **not in my anchor list**) is
the body rule this summary restates, and it is confirmed by the same host
artefact — whatever was concluded about it needs `spec/common/PROP-016-source-mirrors.md:72`
in the perimeter before it moves. Same for
`##when-a-project-outgrows-one-integrator-this-is-the-wrong-tool` (l. 153-156),
already `@spec/done`.

**Verdict recommendation, per anchor:**
`##SUM-WHAT-IT-COSTS` → **confirmed** — the cost is factually this host's
situation, the flow makes the acceptability judgement for its own genre, and
the revisit trigger the verdict called absent is recorded at
`spec/common/PROP-016-source-mirrors.md:72` in the flow's own prescribed
«one-directional server-side mirroring» form; the marker is already `@spec/done`.

---

## F-237 — the `draft-eula` skill exists, ships, and is installed; and the «never claim without checking» rule is a prohibition the consumer broke, not a mechanism nobody built

**Outcome:** MIXED — 1 RE-JUDGE: confirmed (false absence), 1 ROUTE-B CANDIDATE
**Anchors:** 0 touched of 2. Not touched, false absence:
`##THE-DRAFT-EULA-SKILL-DRAFTS-OR-REVIEWS-THE-POSTURE`. **Not touched,
recommended route (b):** `##NEVER-CLAIM-A-LICENCE-IS-PERMISSIVE-WITHOUT-CHECKING`.
**Files touched:** `none`
**Perimeter searched:** the standing perimeter **plus** the three harness skill
homes and the whole of `.vibe/` and `.zcode/`, which the standing perimeter does
not name — a materialised skill is a dotfile artefact and lives nowhere else —
for `draft-eula` · `draft_eula`; **plus** a *directory listing* of
`.claude/skills/`, `.agents/skills/` and `.opencode/skills/` rather than a grep,
because an absent file cannot be grepped for; **plus** an enumeration of every
`[[skill]]` declaration under `vibedeps/`, to find out whether the driver is
dead or merely unused for this package. For the second anchor, the perimeter was
the whole tree for a **dependency licence listing** by name and by content —
`deny.toml` · `about.toml` · `cargo deny` · `cargo about` · `cargo license` ·
SBOM · SPDX · CycloneDX · `reuse.toml` — plus `Cargo.lock`, plus `.github`,
plus `tools/self-check.sh`'s gate chain.

**What the search found — first anchor.**

The skill is real, it is declared, and it is installed into the consumer:

```console
$ ls packages/org.vibevm.world/licensing/v0.1.0/spec/skills/draft-eula/ ;
  ls vibedeps/flow-licensing/0.1.0/spec/skills/draft-eula/
SKILL.md
SKILL.md

$ sed -n '1,4p' packages/org.vibevm.world/licensing/v0.1.0/spec/skills/draft-eula/SKILL.md
---
name: draft-eula
description: Draft or review a project's license posture — the placeholder EULA with
  relicense intent, the permissive-only dependency check, and the third-party carve-out.
  Use when setting up a new project's LICENSE.md or auditing an existing one.
  Guidance, not legal advice.
---

$ sed -n '19,22p' packages/org.vibevm.world/licensing/v0.1.0/vibe.toml
[[skill]]
name = "draft-eula"
path = "spec/skills/draft-eula"
description = "Draft or review a project's license posture: …"
```

The anchor's whole claim is *«The `draft-eula` skill drafts or reviews the
posture»*. The skill's own front matter says «Draft or review a project's
license posture», and its body (`##DRAFTING-OR-REVIEWING-A-LICENSING-POSTURE`,
`##DETERMINE-THE-POSTURE`, `##DRAFT-THE-LICENSE-FILE`) does exactly that. **The
sentence is true.**

The **driver** exists too, and it is in live use in this repository:

```console
$ sed -n '1,10p' crates/vibe-cli/src/commands/skill/mod.rs
//! `vibe skill` — project package-declared skills into coding agents
//! (PROP-018 §2.6). …
//! Skills are enumerated from two sources: the project's own workspace
//! nodes and every installed package's `vibedeps/` slot manifest. Each
//! declared `[[skill]]` is projected into the target agents' skill
//! directories via the `vibe-mcp` writer …
```

`collect_skills` (`crates/vibe-cli/src/commands/skill/mod.rs:51-100`) reads the
lockfile and each package's `vibedeps/` slot manifest — which is exactly where
`draft-eula` is declared. So this is one `vibe skill install` away, not a
missing mechanism.

**And the gap is not about `licensing` at all — it is uniform across the whole
`world` flow family.** Enumerating every installed declaration against the three
harness homes:

```console
$ rg -n -A2 '^\[\[skill\]\]' vibedeps/*/*/vibe.toml
vibedeps/flow-health-audit/0.1.0/vibe.toml:19  name = "health-audit"
vibedeps/flow-wal/0.2.0/vibe.toml:19          name = "wal-status"
vibedeps/flow-licensing/0.1.0/vibe.toml:19    name = "draft-eula"
vibedeps/stack-rust-ai-native-lang/0.7.0/vibe.toml:49,54
                                              name = "rust-ai-native-sweep",
                                                     "rust-ai-native-terraform"
vibedeps/stack-typescript-ai-native-lang/0.6.0/vibe.toml:49,54
                                              name = "typescript-ai-native-sweep",
                                                     "typescript-ai-native-terraform"

$ ls -1 .claude/skills .agents/skills .opencode/skills
.claude/skills:    rust-ai-native-sweep  rust-ai-native-terraform
                   typescript-ai-native-sweep  typescript-ai-native-terraform  vibevm
.agents/skills:    rust-ai-native-sweep  rust-ai-native-terraform
                   typescript-ai-native-sweep  typescript-ai-native-terraform
.opencode/skills:  rust-ai-native-sweep  rust-ai-native-terraform
                   typescript-ai-native-sweep  typescript-ai-native-terraform
```

**Seven** package-declared skills are installed; **four** are materialised, into
all three harnesses. The three that are not are `health-audit`, `wal-status` and
`draft-eula` — every `world` flow skill and only those. So the host materialises
its `ai-native` stack skills and has never materialised a `world` flow skill.
That is a host deployment posture, uniform and visible, and it says nothing
about whether the `licensing` package's sentence is true.

`draft-eula` appears nowhere outside the package, its vendored and cached
copies, the compiled boot lane (`spec/boot/STATIC.md:837`) and this campaign's
own records — the verdict's negative reproduces exactly; it is the *reading* of
that negative that does not survive.

**Which layer has it, if any:** **spec** (the SKILL.md, in the package),
**installed reality** (`vibedeps/flow-licensing/0.1.0/spec/skills/draft-eula/SKILL.md`,
the consumer received it), and **driver** (`vibe skill`, `crates/vibe-cli`).
**Nowhere** for the last step only — materialisation into a harness skill home,
which no `world` flow has had here.

**Why nothing was demoted.** The sentence claims the skill *does* a thing; the
skill exists, ships, is installed, and does that thing. Writing «specified, not
built» over it would tell a reader that a skill sitting on disk in three places
does not exist — the §3.7 error in its purest form. The residual defect is
reachability from the boot line, and the verdict names its own cause: *«DRIFT
for the same structural reason as the pointer three lines above it»* — that
pointer is the `../flows/…` address family (F-332), which is **not** in this
batch and which the owner has already ruled closes by an address repair, not a
demotion. A **CORRECTION** was considered and rejected: the anchor names no path
and no wrong name, so there is nothing imprecise to repair; adding «materialise
it with `vibe skill install`» would be a *new* sentence in a boot snippet, which
is a published-surface change and a release event, not a demotion pass's work.
The sibling README claim `##CONTENT-THE-DRAFT-EULA-SKILL`
(`packages/org.vibevm.world/licensing/v0.1.0/README.md:32-33`) was already ruled
confirmed by Phase C on the same facts.

**What the search found — second anchor
(`##NEVER-CLAIM-A-LICENCE-IS-PERMISSIVE-WITHOUT-CHECKING`).**

The verdict's evidence reproduces in full, and I extended it. The host makes two
blanket permissiveness claims with no listing behind either:

```console
$ sed -n '106p' Cargo.toml
# Third-party (all permissive licenses — PROP-000 §3)

$ sed -n '42,44p' LICENSE.md
vibevm links against third-party Rust crates distributed under permissive
licenses (MIT, Apache-2.0, BSD, or equivalent); their terms are unaffected by
this license and continue to govern their respective code (see `cargo metadata` …
```

and nothing in the perimeter re-derives them:

```console
$ ls -d deny.toml about.toml .github
ls: cannot access 'deny.toml': No such file or directory
ls: cannot access 'about.toml': No such file or directory
ls: cannot access '.github': No such file or directory

$ grep -c 'license' Cargo.lock
0

$ rg -n 'cargo[- ]deny|deny\.toml|cargo[- ]about|about\.toml|cargo[- ]license|SBOM|spdx|cyclonedx|reuse\.toml' <standing perimeter>
ROADMAP.md:1018                       (a wish)
discipline/registry/INTENT.md:63      (INT-0030, the same wish)
discipline/registry/intent.json:424,432 (INT-0030's rescoping)
… everything else is this campaign's own evidence files

$ rg -n -i 'licen' tools/*.sh tools/*.ps1
(no output — the self-check gate chain has no licence step)
```

The two careful checks the verdict credits are real and I confirmed them:
`spec/modules/vibe-resolver/PROP-003-dep-evolution.md:92` classifies libsolv
only after naming its actual licence files, and
`spec/modules/vibe-registry/PROP-001-git-backend.md:95-98` reasons about GPL-v2
`git` and libgit2's Linking Exception rather than asserting. So the practice is
kept where it is load-bearing and broken where it is blanket.

**I also checked the one artefact the brief flagged as a possible false
negative, and it is not one.** `CLAUDE.md:127-137` carries a live «License
state» ledger — but it records **our own shipped surface**'s relicensing to
UPL-1.0 (MT-05 firings, the host `LICENSE.md` on 2026-07-12, which `"EULA"`
strings are off-limits). It is a record about the product's licence, not about
third-party dependency licences, and the dogfood spec it cites
(`…/fractality/v0.1.0/spec/manual-tests/MT-05-dogfood-relicense.md`) relicenses
vibevm's own package manifests. It cannot satisfy a rule about checking
*dependencies*.

**Which layer has it, if any:** **nowhere** for a dependency licence listing, at
any of the four layers. The absence is real — and it is an absence *in the host*,
of a check the host was told to make.

**Why `##NEVER-CLAIM-A-LICENCE-IS-PERMISSIVE-WITHOUT-CHECKING` was NOT touched —
route (b), and the routing is the boss's.** The sentence is a prohibition, in a
section headed `## Never`, addressed to a session: *«Never claim a licence is
permissive without checking; when unsure, treat it as non-permissive and ask.»*
There is no mechanism it promises and nothing that could be «built» to satisfy
it — the missing thing is the host's check, not the package's machinery, and the
verdict's own evidence is a list of host statements. §3.6: *«a package does not
yield to a consumer that simply does not comply»*; softening a `Never` rule
because the consumer broke it twice is precisely the *профанация* the mandate
exists to prevent, and it would be the campaign endorsing an unchecked blanket
claim about sixty crates. The host obligation is stated rather than hidden:
either produce a listing behind `Cargo.toml:106` and `LICENSE.md:42-43`, or
narrow both to what was actually checked. **Recommendation only. I recorded no
routing and wrote no verdict.**

**New obligations noticed:** the two host claims are load-bearing beyond this
package — `LICENSE.md:42-43` is a statement in the product's licence file, read
by anyone who receives the product, and `LICENSE.md:44` points readers at
`cargo metadata` as «the authoritative list» while `cargo metadata --offline`
cannot run here and `Cargo.lock` carries no licence field at all
(`grep -c 'license' Cargo.lock` = 0). That is a host-side accuracy question
above the campaign's pay grade and outside my edit scope; recorded, not acted on.

**Verdict recommendation, per anchor:**
`##THE-DRAFT-EULA-SKILL-DRAFTS-OR-REVIEWS-THE-POSTURE` → **confirmed** — the
skill exists in the package, ships to the consumer under `vibedeps/`, and does
what the sentence says; the driver `vibe skill` exists and materialised four
other package skills here. The residual is reachability, which is the address
family's shape and the owner's ruling, not a demotion.
`##NEVER-CLAIM-A-LICENCE-IS-PERMISSIVE-WITHOUT-CHECKING` → **drift stands,
route (b)** — a prohibition the consumer broke at `Cargo.toml:106` and
`LICENSE.md:42-43`; no mechanism is missing from the package, so the package
must not move.

---

## F-238 — the scheduled licence re-audit: the sibling flow it delegates to carries no licence line at all, and the CI half was declined on the record

**Outcome:** DEMOTED (2 of 2, both PARTIAL)
**Anchors:** 2 touched of 2: `##RE-AUDIT-ON-A-SCHEDULE`,
`##SUM-AUTOMATE-AND-RE-AUDIT`.
**Files touched:**
`packages/org.vibevm.world/licensing/v0.1.0/spec/flows/licensing/dependency-licenses.md`
**Perimeter searched:** the standing perimeter, and deliberately **widened past
the verdict's own** in three directions it did not go. The verdict searched
`vibedeps/flow-health-audit/`; I searched the **canonical package** at
`packages/org.vibevm.world/health-audit/v0.1.0/` as well, because a vendored
copy can lag its source. It searched `AUDIT.md`; I also searched the host's
audit *contract* (`spec/common/PROP-013-periodic-health-audit.md`), the
`manual-tests/` recipes, `tools/self-check.sh`'s gate chain, and
`cargo xtask health`'s section list, because «a periodic audit line» could be a
recipe or a collector section rather than a markdown bullet. And it searched
for the tooling by two names; I searched by seven — `cargo deny` · `deny.toml`
· `cargo about` · `about.toml` · `cargo license` · SBOM / SPDX / CycloneDX ·
`reuse.toml` — plus `Cargo.lock` and `.github`. Terms swept for the *thing*:
`licen` · `copyleft` · `GPL` · `permissive` · `SPDX`.

**What the search found.**

The sibling flow this fact delegates to carries no licence line, and that holds
in the canonical package, not only in the vendored copy:

```console
$ rg -n -i 'licen|copyleft|GPL|permissive|SPDX' \
      packages/org.vibevm.world/health-audit/v0.1.0/spec/
(no output)

$ sed -n '187,196p' packages/org.vibevm.world/health-audit/v0.1.0/spec/flows/health-audit/audit-checklist.md
### D4 · Dependency staleness {#d4}

- ##D4-LOOK-FOR **Look for.** Outdated dependencies and open security advisories.
  Pinned versions drifting behind; a transitive advisory nobody saw. @impl/done
- ##D4-AID **Aid.** The dependency manager's outdated/audit command
  (`npm outdated` / `npm audit`, `cargo outdated` / `cargo audit`,
  `pip list --outdated`, `go list -u -m all`, etc.). @impl/done
```

One dependency category, and it is versions and CVEs. The host's own audit
contract says the same thing in one line:

```console
$ sed -n '51p' spec/common/PROP-013-periodic-health-audit.md
- ##D4-DEP-STALENESS **D4 · Dependency staleness** — `cargo update --dry-run`;
  `cargo audit` / `cargo outdated`. @spec/done
```

And the host's audit *record* has never carried one:

```console
$ grep -n -i 'licen|copyleft|GPL|permissive' AUDIT.md ; echo "exit=$?"
exit=1
$ wc -l AUDIT.md ; grep -n '^## ' AUDIT.md
458 AUDIT.md
20:## Audit run — 2026-05-23 (seed)
154:## Audit run — 2026-06-10 (terraform close-out, instrumented category C)
191:## Audit run — 2026-06-12 (discipline depth — the full AI-Native sweep)
```

Three dated runs, 458 lines, zero licence hits. Nor is there anything to
re-run: no `deny.toml`, no `about.toml`, no `.github`, no SBOM/SPDX manifest,
`grep -c 'license' Cargo.lock` = 0, `rg -n -i 'licen' tools/*.sh tools/*.ps1`
empty, and `cargo xtask health`'s only extra section is the PROP-016 `mirrors`
probe (`xtask/src/main.rs:374-376`). **The absence is real on the widest
perimeter I ran for this batch, and — the decisive point — it is real inside
the shipped collection itself, not only in the host.** This is source 1 against
source 1: the `licensing` package points at `flow:health-audit`, and
`flow:health-audit` does not carry the line. §3.6 route (a).

**The half that is NOT drift, and the clause now says so.** The CI listing was
not overlooked — it was wanted, filed, and declined on the record:

```console
$ sed -n '1018,1020p' ROADMAP.md
- **`cargo deny` in CI.** Licence-check automated: fail the build if a
  dep with a non-permissive licence sneaks in. Matches PROP-000 §3's
  "permissive only" rule.

$ sed -n '421,436p' discipline/registry/intent.json
  "id": "INT-0030",
  "source": "ROADMAP.md side quests",
  "text": "cargo deny in CI (automated license check per PROP-000 §3)",
  "state": "rescoped",
  "resolution": { "decided": "2026-06-10",
    "by": "terraform Phase 6 reconciliation (owner session mandate)",
    "note": "cargo deny — couples to the CI decision (INT-0017); the license
             policy itself is enforced by review per PROP-000 §3." }

$ sed -n '73,74p' discipline/registry/INTENT.md
(INT-0028 CHANGELOG), **1 rejected** (INT-0017 CI matrix — the no-CI
posture is a standing Rule-4 owner decision), **27 rescoped** …
```

That is a **marked exception** on the consumer side, which Phase C ruled is not
drift. A flat «not built» over `##SUM-AUTOMATE-AND-RE-AUDIT` would have read as
a failure where the record shows a decision, so the clause names all three
states — built, declined-on-the-record, absent — rather than one.

**Which layer has it, if any:** **nowhere**, at all four layers, for the
scheduled licence re-audit and for the listing it would re-run. The one clause
that is built — «point the carve-out at the generated list» — is **host
deployment**: `LICENSE.md:44` sends readers to `cargo metadata` rather than to
a hand-maintained copy.

**I checked the artefact the brief flagged, and it does not close this.**
`CLAUDE.md:127-137`'s «License state» ledger and the MT-05 dogfood
(`…/fractality/v0.1.0/spec/manual-tests/MT-05-dogfood-relicense.md`) record the
2026-07 relicensing of **vibevm's own** surface to UPL-1.0 — the product's
licence, a one-off owner act, not a periodic listing of third-party dependency
licences. `##RE-AUDIT-ON-A-SCHEDULE` is about *a dependency* relicensing
between versions. Different subject; it cannot satisfy the fact.

**What changed and why:** two facts keep their prescription word for word and
gain a clause; both markers move `@impl/done` → `@spec/done`. Neither is
flattened. `##RE-AUDIT-ON-A-SCHEDULE`'s clause names the sibling's actual
content (D4, staleness and advisories), says there is no listing to re-run, and
points the adopter at where the line would go — `audit-checklist.md` is
explicitly «a starting set, not a closed one», so this is a gap the adopter
fills rather than a contradiction. `##SUM-AUTOMATE-AND-RE-AUDIT`'s clause
separates the three clauses it bundles. Marker census on the file confirms
exactly two moved: lines carrying `@impl/done` 24 → 22, lines carrying
`@spec/done` 6 → 8.

**New obligations noticed.** (1) `##AUTOMATE-THE-LISTING`
(`dependency-licenses.md:57-60`, `@impl/done`, **not in my anchor list**) is the
body rule whose summary half I have just annotated, and it closes on *«A rule
with no checker is a wish»* — self-descriptive here, since no checker exists;
whatever was concluded about it needs `discipline/registry/intent.json:421-436`
in the perimeter, which is where the host's reasoning lives, before it moves.
(2) A live host-registry inconsistency found on the way, outside this campaign:
`discipline/registry/INTENT.md:63` shows INT-0030 as `open` while
`intent.json:428` marks it `rescoped` — the two views of one registry disagree.
Recorded, not fixed; it is a host artefact and outside my edit scope.

**Verdict recommendation, per anchor:**
`##RE-AUDIT-ON-A-SCHEDULE` → **demoted, now confirmed** — the delegation is to a
sibling in the same shipped collection whose checklist has no licence line at
any grain, verified in the canonical package as well as the vendored copy; the
prescription stands and the clause says who does not yet carry it.
`##SUM-AUTOMATE-AND-RE-AUDIT` → **demoted, now confirmed** — one clause built
(`LICENSE.md:44`), one declined on the record (INT-0030 rescoped under a
standing no-CI owner decision, which is a marked exception and not drift), one
built by nothing; the clause names all three so no reader reads a decision as a
failure.

---

## F-315 — the README asserts a composition its own sibling does not implement, and the sibling entry three lines above it is already marked honestly

**Outcome:** DEMOTED (1 of 1)
**Anchors:** 1 touched of 1: `##COMPOSES-HEALTH-AUDIT`.
**Files touched:** `packages/org.vibevm.world/licensing/v0.1.0/README.md`
**Perimeter searched:** the same widened perimeter as F-238 — the standing
perimeter, plus the **canonical** `packages/org.vibevm.world/health-audit/v0.1.0/`
rather than only its `vibedeps/` copy, the host's audit contract
`spec/common/PROP-013-periodic-health-audit.md`, the host's audit *record*
`AUDIT.md`, `manual-tests/`, `tools/self-check.sh`, `cargo xtask health`'s
section list, and seven names of dependency-licence tooling. Terms: `licen` ·
`copyleft` · `GPL` · `permissive` · `SPDX`. The evidence is the same body of
searches as F-238 and is not repeated here; what follows is only what bears on
*this* sentence.

**What the search found.**

The claim is narrow and checkable: *«`flow:health-audit` — a periodic audit line
re-runs the dependency licence listing, catching a dependency that relicensed
between versions.»* It asserts something about a **named sibling in the same
shipped collection**, and the sibling does not have it — at any grain, in the
canonical package:

```console
$ rg -n -i 'licen|copyleft|GPL|permissive|SPDX' \
      packages/org.vibevm.world/health-audit/v0.1.0/spec/
(no output)

$ find packages/org.vibevm.world/health-audit -name '*.md'
…/spec/boot/42-flow-health-audit.md
…/spec/flows/health-audit/audit-checklist.md
…/spec/flows/health-audit/HEALTH-AUDIT-PROTOCOL.md
…/spec/flows/health-audit/running-an-audit.md
…/spec/skills/health-audit/SKILL.md
```

Five documents, none of which mentions a licence. Its single dependency
category is `##D4-LOOK-FOR` / `##D4-AID` at `audit-checklist.md:187-196` —
outdated versions and open security advisories. The host that adopted both
flows has never run such a line either (`AUDIT.md`, 458 lines, three dated
runs, zero licence hits), and there is no listing for a line to re-run (no
`deny.toml`, no `about.toml`, no SBOM, no licence field in `Cargo.lock`).

**Which layer has it, if any:** **nowhere**. Not in the `licensing` package,
not in the `health-audit` package it names, not in the host's audit contract,
not in the host's audit record, not in any tooling. This is the one anchor in
the batch where the absence is total across all four layers **and** the
falsifier sits inside the shipped collection — §3.6 route (a), the package is
wrong about its own sibling, and the routing question does not arise.

**A structural corroboration, from the anchor's own neighbours.** The
`## Composition` section mixes two genres and already marks them differently.
`##COMPOSES-SECRETS-HYGIENE` (`README.md:77-78`) states a *relationship* — «a
sibling one-place policy; both reward a mechanical check in CI over a prose
promise» — and carries `@spec/done`. `##COMPOSES-ATTRIBUTION-POLICY`
(`:82-84`) is the same genre and is also `@spec/done`. Only
`##COMPOSES-HEALTH-AUDIT` asserted a *running mechanism* in a sibling while
carrying `@impl/done`. Demoting it brings it into line with the two entries
around it rather than inventing a new state for it.

**What changed and why:** one fact demoted `@impl/done` → `@spec/done`, keeping
its sentence word for word and gaining a clause that names what the sibling
actually carries (D4 · Dependency staleness), what was searched, that there is
no listing to re-run, and that the composition is *intended* rather than
running. It also points at the body rule demoted on the same evidence,
`spec/flows/licensing/dependency-licenses.md#RE-AUDIT-ON-A-SCHEDULE` (F-238), so
the two do not drift apart later. Marker census on the file confirms exactly one
moved: lines carrying `@impl/done` 18 → 17, lines carrying `@spec/done` 6 → 7.
**Considered and rejected: a CORRECTION.** A correction is the right repair when
the prose is imprecise about where a real thing lives; here the thing does not
live anywhere, so redirecting the pointer would have had nowhere to point.

**New obligations noticed:** the honest repair on the *other* side is a
`flow:health-audit` change — a licence row under D4, or a D-category sibling —
which would let both this entry and `##RE-AUDIT-ON-A-SCHEDULE` be restored to
`@impl/done` at a stroke. That is a second package's edit and a cross-package
publication, so it is a release-route item and not this pass's; recorded for the
owner rather than attempted.

**Verdict recommendation, per anchor:**
`##COMPOSES-HEALTH-AUDIT` → **demoted, now confirmed** — the sibling package it
names carries no licence line anywhere in its five documents, verified in the
canonical package and not merely in the vendored copy; the composition is sound
as a design and unbuilt as a fact, and the clause now says exactly that.

---

## `refs/**`, reported separately

`refs/` is a third-party study corpus, not our shipped surface. Its entire
yield for this batch's search terms:

```console
$ rg -c 'merge-base --is-ancestor' refs
refs/src/warp/warp-master/specs/APP-4218/PRODUCT.md:1

$ rg -l 'draft-eula' refs
(no output)

$ rg -l 'deny\.toml|cargo-about|about\.toml' refs
refs/study/cargo/triagebot.toml          refs/src/cargo/triagebot.toml
refs/src/warp/warp-master/script/windows/prepare_bundled_resources.ps1
refs/src/warp/warp-master/script/prepare_bundled_resources
refs/src/warp/warp-master/script/install_cargo_release_deps
```

Not one is an implementation of ours: a Warp product spec that happens to
mention git's ancestry check, and cargo's / Warp's own build and triage
scripts. **No verdict in this batch turns on a `refs/` hit.**
