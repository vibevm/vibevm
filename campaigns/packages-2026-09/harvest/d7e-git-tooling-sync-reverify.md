# D7e — the git family + `tool-design-lessons` on `sync-from-code`: seventeen described facts, re-measured before any diff exists

_Worked 2026-07-29, wave 7. Subjects:
`packages/org.vibevm.world/git-attribution-policy/v0.1.0/` (8 anchors),
`packages/org.vibevm.world/tool-design-lessons/v0.1.0/` (7),
`packages/org.vibevm.world/git-conventional-commits/v0.1.0/` (1),
`packages/org.vibevm.world/git-practices/v0.1.0/` (1). Seven obligations, all
`sync-from-code`, all `reality-mismatch`, 17 drift verdicts._

**No package file was edited. Not one character.** This route's diffs are
approved by the owner one at a time
([§5](../PHASE-D-BATCH-PLAN.md#stop)), so a re-verdict that edits nothing
produces no diff and needs no approval — which is the only reason this pass
could run autonomously. Every correction below is written out as **proposed
text and left unapplied**. Nothing under `run/` was touched; no `git` command
that writes was run.

**Measured at HEAD `9f79acf1` (`9f79acf1d7ee927d28083f0fc0780d9d572f745b`),
2026-07-29** — *"fix(campaign): the last two boss-closable obligations, and
neither one moved a package"*. Wave 6's record was taken at `e118b76f`; the
window has moved since, and every figure below names the HEAD it was taken at
because [§7's wave-6 entry](../../../spec/terraforms/PACKAGES-ACTUALIZATION-CAMPAIGN-v0.1.md#log)
records that a 400-commit figure over this history decays within the week.

**And it decayed inside this run — the strongest available evidence for that
rule.** HEAD advanced **six commits** while this batch was being worked, from
`9f79acf1` (2026-07-29) to **`b61eb191`** (2026-07-31, *"docs(campaign): the
decision-record practice is adopted at 41 % next door, not unadopted"*). Over
those six commits the hard-limit breach in F-308's own window moved **131/400 →
136/400 (32.8 % → 34.0 %)**. A figure over this history does not decay within
the week; it decays within the session. All figures below are reported at
`9f79acf1` and, where they moved, at `b61eb191` as well. `git diff --name-only
9f79acf1..HEAD` touches only `campaigns/packages-2026-09/**` — **no `crates/`
and none of my four packages** — so every code-based and package-based finding
below is unaffected by the move.

**The standing perimeter**, run from the repository root:

```
packages/**  (INCLUDING packages/org.vibevm.fractality/**)
vibedeps/**  crates/**  xtask/**  tools/**  spec/**  discipline/**  terraform/**
research/**  campaigns/**  legacy-spec/**  fixtures/**  schemas/**  docs/**  manual-tests/**
and the repository root's own *.md / *.toml / *.json / *.sh / *.ps1
minus  **/target/**  .git/**  **/node_modules/**  campaigns/*/run/**
```

`refs/**` searched but reported **separately** — third-party clones, and their
CI files have already misled one search in this family.

**Two host facts taken as established, not re-derived.** (1) The boot compiler
emits four `git-*` snippets **twice** into `spec/boot/STATIC.md` — 31 static
contributions from 27 distinct sources, because the family is reached both
directly and through the `git-practices` umbrella; filed as `BACKLOG.md` B-006.
A fact falsified *by that duplication alone* is a host build defect under
[§3.6(b)](../PHASE-D-BATCH-PLAN.md#which-side), not a false package sentence.
(2) `packages/org.vibevm.fractality/fractality/v0.1.0/` is a **second complete
project that adopted this discipline** — own `vibe.toml`, own `vibedeps/`, own
Cargo workspace, **and its own git history and commit practice** — so for a
batch about commit discipline it is inside the perimeter, not outside it
([§3.7's wave-6 extension](../PHASE-D-BATCH-PLAN.md#compliance-blindness)).

Obligations: F-309 · F-308 · F-344 · F-157 · F-199 · F-087 · F-232.

---

## F-309 — the umbrella's README defers two members that shipped; its own manifest pins all four, and the installed copy carries the same defect

**Outcome:** SURVIVES
**Anchors:** 1 of 1 — `##THE-FAMILY-GROWS-TO-INCLUDE-ATTRIBUTION-AND-AUTONOMY`
(`packages/org.vibevm.world/git-practices/v0.1.0/README.md:23-24`) → **SURVIVES**
**Perimeter searched:** the standing perimeter narrowed to what can settle a
roster — the package's own `README.md` and `vibe.toml`, the install slot
`vibedeps/flow-git-practices/0.1.0/`, `vibe.lock`, the
`packages/org.vibevm.world/git-*` directory listing, and
`spec/common/PROP-000.md` §12. Widened to `packages/org.vibevm.fractality/**`:
the second project installs no `flow-git-practices`, so it cannot settle this
one either way.
**The verdict's own command, re-run:** the verdict quotes none; it cites
`PROP-000:159-164`, re-read below.
**Measured at:** `9f79acf1`, 2026-07-29 (no figure over the commit log).

**What the measurement shows.** The sentence under judgement, at
`README.md:23-24`:

> `##THE-FAMILY-GROWS-TO-INCLUDE-ATTRIBUTION-AND-AUTONOMY` The family grows to
> include **human-authored attribution** and **commit autonomy** as those
> members land. @status:spec/done

*"As those members land"* places both in the future. All four members exist, are
pinned, are installed and are locked:

```console
$ ls -d packages/org.vibevm.world/git-*
packages/org.vibevm.world/git-atomic-commits
packages/org.vibevm.world/git-attribution-policy
packages/org.vibevm.world/git-autonomy
packages/org.vibevm.world/git-conventional-commits
packages/org.vibevm.world/git-practices

$ ls -d vibedeps/flow-git-*
vibedeps/flow-git-atomic-commits       vibedeps/flow-git-attribution-policy
vibedeps/flow-git-autonomy             vibedeps/flow-git-conventional-commits
vibedeps/flow-git-practices
```

The package's **own manifest** pins four —
`packages/org.vibevm.world/git-practices/v0.1.0/vibe.toml:25-31`:

```toml
"flow:org.vibevm.world/git-conventional-commits" = "=0.1.0"
"flow:org.vibevm.world/git-atomic-commits" = "=0.1.0"
"flow:org.vibevm.world/git-autonomy" = "=0.1.0"
"flow:org.vibevm.world/git-attribution-policy" = "=0.1.0"
```

`vibe.lock:293-296` resolves all four under this package's `deps`, and
`spec/common/PROP-000.md:159-164` lists all four as current members with
`spec://` pointers (`##GP-ATTRIBUTION`, `##GP-CONVENTIONAL`, `##GP-ATOMICITY`,
`##GP-AUTONOMY`), none of them marked pending.

**The deciding fact is inside the package, so this is §3.6(a) without a
judgement call.** The registry types the row `mixed` because `PROP-000` and
`vibe.lock` are host files; but the sentence is falsified by
`git-practices/v0.1.0/vibe.toml` alone — the same package, four lines of its own
manifest. The README's bullet list at `:14-17` compounds it: it enumerates
`git-conventional-commits` and `git-atomic-commits` only, so a reader is handed
a two-member roster and told the other two are pending.

**The defect ships.** The installed copy carries it verbatim —
`vibedeps/flow-git-practices/0.1.0/README.md:17` ("The family grows to include
**human-authored attribution** and **commit** …") over a `vibe.toml` at `:25-31`
pinning the same four. A consumer reading the shipped umbrella is misinformed
about its own contents.

**Proposed correction (NOT APPLIED).** Replace `README.md:23-24` with:

```markdown
##THE-FAMILY-GROWS-TO-INCLUDE-ATTRIBUTION-AND-AUTONOMY The family is complete at
four members: **human-authored attribution** and **commit autonomy** landed
alongside the message format and the atomicity discipline, and all four are
pinned in this package's `vibe.toml`. @impl/done
```

Two notes for the boss, neither of them mine to decide. (1) The marker moves
`@spec/done` to `@impl/done`, because the sentence stops describing a plan and
starts describing the manifest. (2) The bullet list at `:14-17` names two of
four; completing it means **adding two new bullets with two new anchors**, which
changes this document's anchor set — [§3.1's own "Revisit when"](../PHASE-D-BATCH-PLAN.md#closure)
requires `vibe progress mirror` to run before `merge-verdicts.py` in that case.
Correcting the sentence alone leaves the list short but no longer contradicted;
correcting both is the fuller repair and the costlier one.

**Recommendation per anchor:** `##THE-FAMILY-GROWS-TO-INCLUDE-ATTRIBUTION-AND-AUTONOMY`
→ **drift stands, correction prepared** (route (a) — the package is wrong about
itself).

---

## F-308 — both length figures re-measured and both are worse; the sentence is a norm, and the breach is the consumer's

**Outcome:** SURVIVES — ROUTE (b), with **both recorded figures superseded**
**Anchors:** 1 of 1 — `##HEADER-TARGET-LENGTH-AND-HARD-LIMIT`
(`packages/org.vibevm.world/git-conventional-commits/v0.1.0/spec/flows/conventional-commits/conventional-commits.md:17-20`)
→ **SURVIVES — ROUTE (b)**
**Perimeter searched:** for the **measurement**, this repository's own history,
read-only. For the **enforcement** half, the standing perimeter **including
`packages/org.vibevm.fractality/**`** over `*.rs` · `*.sh` · `*.ps1` · `*.py` ·
`*.toml` · `*.json` · `*.yml` · `*.yaml` for `commitlint` ·
`conventional.?commit` · `commit-msg` · `commit_msg` · `husky`, plus a by-name
check of the second project for `.github/`, hook directories and a self-check
script.
**The verdict's own command, re-run:** the verdict quotes none; it states
figures, re-derived below.
**Measured at:** HEAD `9f79acf1`, 2026-07-29, window `git log -400`.

**What the measurement shows — both figures have moved, in the direction that
strengthens the verdict.**

```console
$ git log -400 --format=%s   | (length census)
commits: 400
len>72 (hard limit breach): 131 of 400 = 32.8%
len>60 (target breach):     316 of 400 = 79.0%
longest: 119 chars | fix(world): the second adopter of this discipline lives inside packages/, and half the wave's absences were blind to it
median=69.0 mean=68.6 min=38 max=119
```

| figure | verdict recorded | at `9f79acf1` | at `b61eb191` (6 commits later, same session) |
|---|---:|---:|---:|
| subjects over the hard limit of 72 | 82 / 400 — 20.5 % | **131 / 400 — 32.8 %** | **136 / 400 — 34.0 %** |
| subjects over the 60-character target | 297 / 400 — 74.3 % | **316 / 400 — 79.0 %** | 316 / 400 — 79.0 % |
| longest subject | 89 | **119** | 119 |

**The third column is the finding, not a footnote.** Six commits — all of them
`docs(campaign)`, none touching code — moved the hard-limit breach by 1.2
points. Any host obligation phrased as "N of 400" is stale before it is read.
The durable statement is the *rate and its direction*, with the HEAD named.

The hard-limit breach is **half again as large** as recorded and the longest
subject is **thirty characters longer**. The median subject is 69 characters —
above the target, below the hard limit — so the *typical* commit in this window
breaks the rule it targets. **The five longest subjects are all this campaign's
own**, and the single longest is the wave-6 commit recording the second-adopter
finding, at 119 characters: the campaign auditing the discipline is the heaviest
breaker of this particular rule.

**The enforcement half holds on the widened perimeter.** Searching the standing
perimeter **including the second adopter**, every hit is a message *builder* or
a package description, never a checker:

```console
$ grep -rniE "commitlint|conventional.?commit|commit-msg|commit_msg|husky" \
    --include=*.rs --include=*.sh --include=*.ps1 --include=*.py \
    --include=*.toml --include=*.json --include=*.yml --include=*.yaml \
    packages crates xtask tools discipline terraform schemas fixtures
  packages/org.vibevm.fractality/**/vibe.toml           -> descriptions / keywords
  packages/org.vibevm.world/git-*/vibe.toml             -> package descriptions
  crates/vibe-publish/src/git_publish.rs:57             -> let commit_msg = format!("Release …")
  crates/vibe-cli/…/registry/redirect/update.rs:450     -> build_redirect_update_commit_msg(…)
```

The second project has no enforcement of its own either — no `.github/`, no hook
directory, no self-check script; its only `*.sh` files are
`spec/manual-tests/trial/run-advise.sh`, `run-arm.sh`, `save-results.sh` and the
`fractality.sh` launcher. **Neither adopter checks a commit message at any
layer.**

**Why the package does not move.** The sentence is a **norm**, not a
description:

> `##HEADER-TARGET-LENGTH-AND-HARD-LIMIT` **Target length:** ≤ 60 characters.
> **Hard limit:** 72. Git web UIs truncate beyond that, and a truncated subject
> on the commit list is how decisions become invisible to readers who scan
> rather than scroll.

It is a bullet in the rules list under `## Header`, stating a limit and its
reason. It asserts no count, claims no checker, and describes nothing that
exists. What the verdict measured is **the consumer breaking a limit the package
correctly states** — [§3.6](../PHASE-D-BATCH-PLAN.md#which-side) route (b).
Relaxing 60/72 to match a repository whose median subject is 69 is the
*профанация* [§0](../../../spec/terraforms/PACKAGES-ACTUALIZATION-CAMPAIGN-v0.1.md#mandate)
exists to prevent — and the rule's stated reason (web UIs truncate) is a fact
about Git hosting, not about this repository's habits.

*(The neighbouring `##ALL-COMMITS-FOLLOW-THE-CONVENTIONAL-COMMITS-SPECIFICATION`
at `:5` **is** a factual claim and is not in my batch; wave 6 measured it true on
header shape, 400/400 carrying a `type(scope):` prefix. Recorded so this row's
route is not read across to it.)*

**Proposed correction (NOT APPLIED):** none — correct as written.

**Recommendation per anchor:** `##HEADER-TARGET-LENGTH-AND-HARD-LIMIT` →
**drift stands, route (b)**. The host obligation should carry the **re-measured**
figures with their HEAD: 131/400 over 72 (32.8 %) and 316/400 over 60 (79.0 %),
longest 119, at `9f79acf1`, 2026-07-29.

---

## F-344 — the denylist *names* the ignore file, and it does; the verdict read PROP-024's stronger clause into a sentence that does not make it

**Outcome:** FALSE
**Anchors:** 1 of 1 — `##P3-MECHANICS-A-SHORT-DENYLIST-NAMES-WHAT-WAS-NEVER-SOURCE`
(`packages/org.vibevm.world/tool-design-lessons/v0.1.0/spec/flows/tool-design-lessons/packaging-lessons.md:99-101`)
→ **FALSE**
**Perimeter searched:** the standing perimeter over `*.rs` for `vibeignore` ·
`SHIPPABLE_EXCLUDES` · `read_to_string.*ignore` · `ignore.*globs` ·
`parse_ignore` · `IgnoreFile`, plus the whole P3 section read rather than
grepped — the question "which claim does this sentence make?" is answered by
reading, not by absence — plus `spec/common/PROP-024-code-bearing-packages.md`
§2.2 and §5 for the sibling claim the verdict imported.
**The verdict's own command, re-run:** the verdict quotes none; its single
evidence line is `content_hash.rs:28`, re-read verbatim below.
**Measured at:** `9f79acf1`, 2026-07-29 (no figure over the commit log).

**What the measurement shows — the code matches the sentence five-for-five.**

```console
$ grep -rn "SHIPPABLE_EXCLUDES" crates/
crates/vibe-index/src/content_hash.rs:28:const SHIPPABLE_EXCLUDES: &[&str] = &[".git", ".vibe", "target", "node_modules", ".vibeignore"];
crates/vibe-registry/src/shippable.rs:18:const SHIPPABLE_EXCLUDES: &[&str] = &[".git", ".vibe", "target", "node_modules", ".vibeignore"];
```

The sentence under judgement, at `packaging-lessons.md:99-101`:

> `##P3-MECHANICS-A-SHORT-DENYLIST-NAMES-WHAT-WAS-NEVER-SOURCE` **Mechanics.** A
> short denylist **names** what was **never** source — VCS internals, caches,
> build output (for example `.git/`, `target/`, `node_modules/`) plus an
> optional package-level ignore file.

Map it onto the constant, term by term:

| the sentence says the denylist names… | the code's entry |
|---|---|
| VCS internals | `.git` |
| caches | `.vibe` |
| build output (for example `target/`, `node_modules/`) | `target`, `node_modules` |
| plus an optional package-level ignore file | **`.vibeignore`** |

**Five named categories, five entries, in order.** The verb is *names*, and the
denylist does name an optional package-level ignore file: `.vibeignore` is the
fifth element of the list. Under its plain reading the sentence is exact — which
the verdict itself concedes for the first four ("the main claim exact").

**Where the verdict went wrong: it imported a neighbouring document's stronger
clause.** The verdict reads the trailing phrase as *"plus whatever globs that
file lists"*, then correctly observes that nothing reads globs out of it. But
that is `PROP-024`'s wording, not this package's —
`spec/common/PROP-024-code-bearing-packages.md:123`:

> `##VIBEIGNORE-EXTENDS` plus **any glob listed in** an optional `.vibeignore` at
> the package root. **@status:spec/done**

The two sentences are deliberately different: PROP-024 says *any glob listed in*
the file, the package says *an optional … ignore file*. **And the host's own
spec already marks the glob feature `@spec/done` — specified, not built — at
both `:123` (`##VIBEIGNORE-EXTENDS`) and `:268` (`##SURF-VIBEIGNORE`).** So the
absence the verdict found is real, correctly recorded on the host side, and
attached to a host anchor. It is not a claim this package makes.

**The document's own next-but-one sentence rules the verdict's reading out.**
`##P3-MECHANICS-THE-DENYLIST-INTRODUCES-NO-CHOICE`, four lines later at
`packaging-lessons.md:113-114`:

> The denylist only formalises "what was never source"; it **introduces no
> choice**.

A per-package glob file is exactly a choice. Under the verdict's reading the P3
section contradicts itself within fifteen lines; under the plain reading it is
consistent, and consistent with the code. When two readings are available and
one makes the document incoherent, the other is the one the author wrote.

**Confirmed absent, and correctly so:** no code reads the file's contents.

```console
$ grep -rnE "vibeignore" --include=*.rs crates xtask
crates/vibe-index/src/content_hash.rs:28   (the exclusion by name)
crates/vibe-registry/src/shippable.rs:18   (the exclusion by name)
$ grep -rniE "read_to_string.*ignore|ignore.*globs|parse_ignore|IgnoreFile" --include=*.rs crates xtask
   … four hits, all `.gitignore` in tests; none for `.vibeignore`
```

Two references, both the name on the denylist. That is precisely what the
package's sentence describes.

**Proposed correction (NOT APPLIED):** none — correct as written.

**Two observations recorded, neither this anchor's defect.** (1) The
duplicated constant is **deliberate and gated**, not drift: the doc comment at
`content_hash.rs:23-27` says it is "duplicated verbatim in `vibe-registry`'s
hasher — the two MUST stay in lockstep (PROP-005 §3.2's
duplicate-rather-than-import port)", and `crates/vibe-index/tests/content_hash_parity.rs`
exists to gate divergence. (2) The verdict's recorded live consequence is real
and is **larger than it recorded**: `vibedeps` is not on `SHIPPABLE_EXCLUDES`,
and four directories on disk carry a nested one —
`vibedeps/flow-delegation-rules/0.1.0/vibedeps/` (26 flows),
`packages/org.vibevm.fractality/fractality/v0.1.0/vibedeps/` (27 flows),
`packages/org.vibevm.fractality/delegation-rules/v0.1.0/vibedeps/`, and the host
root's own. That is a host-code question for the reviewer, on no anchor of mine.

**Recommendation per anchor:** `##P3-MECHANICS-A-SHORT-DENYLIST-NAMES-WHAT-WAS-NEVER-SOURCE`
→ **re-judge confirmed**. No edit, no spec diff, no owner approval needed.

---

## F-157 — six anchors, four outcomes: the consent count is wrong by four, the toolchain file has no pin to read, and the lesson about prose drifting is confirmed by the evidence offered against it

**Outcome:** MIXED — 6/6 examined: **1 FALSE · 1 FALSE PREMISE, DIFFERENT
DEFECT · 1 SURVIVES · 3 SURVIVE — ROUTE (b)**
**Two of the six are recommended for re-judge with no edit**, which is the
outcome this route wants: no spec diff, no owner approval.

**Anchors:** 6 of 6, by name:

| anchor | line | outcome |
|---|---|---|
| `##S2-MECHANICS-NO-LOCKS-AND-NO-RELOAD` | `:77-79` | **FALSE PREMISE, DIFFERENT DEFECT** |
| `##S5-LAW-EDIT-DURABLE-STATE-IDEMPOTENTLY-AND-ADDITIVELY` | `:172-174` | **SURVIVES — ROUTE (b)** |
| `##S5-MECHANICS-FIVE-RULES-NONE-OPTIONAL` | `:176` | **SURVIVES — ROUTE (b)** |
| `##s6-context-a-prose-list-drifts` | `:211-213` | **FALSE** |
| `##S6-MECHANICS-RELATED-KNOWLEDGE-FOLLOWS-THE-SAME-RULE` | `:228-231` | **SURVIVES** |
| `##SUM-S5-DURABLE-ENV-EDITS` | `:291-292` | **SURVIVES — ROUTE (b)** |

All six in
`packages/org.vibevm.world/tool-design-lessons/v0.1.0/spec/flows/tool-design-lessons/self-updating-tools.md`.

**Perimeter searched:** the standing perimeter over `*.rs` for `rust-toolchain` ·
`toolchain.toml` · `SHIPPABLE` · `confirm(` · `assume_yes` · `IsTerminal` ·
`set_vibevm_home` · `ensure_on_path` · `is_transient_lock`, narrowed for the
consent question to `crates/vibe-cli/src/commands/vvm/**` (the module these
lessons were extracted from) and read in full rather than grepped. Widened to
`packages/org.vibevm.fractality/**` for `rust-toolchain`: its hits are the
vendored `rust-ai-native-*` doc comments, not a second implementation of this
tool. Plus `rust-toolchain.toml`, `Cargo.toml`, and `spec/common/PROP-019-version-manager.md`
§2.6 and §7.
**The verdict's own command, re-run:** one verdict quotes one, and **it
reproduces exactly**:

```console
$ grep -rn 'rust-toolchain\|toolchain.toml' crates/ xtask/ --include='*.rs'
crates/vibe-cli/src/commands/vvm/builder.rs:38:/// managed `--target-dir`, honouring the tree's `rust-toolchain.toml`
```

One hit, a doc comment, exactly as recorded.
**Measured at:** `9f79acf1`, 2026-07-29 (no figure over the commit log).

---

### `##S2-MECHANICS-NO-LOCKS-AND-NO-RELOAD` — FALSE PREMISE, DIFFERENT DEFECT

The sentence, at `:77-79`:

> **Because no in-use file is ever rewritten**, there are no file locks and no
> reload — the model is safe even for **a shared library the OS refuses to
> replace while it is mapped**.

The verdict answers it with `is_transient_lock`, which does exist —
`crates/vibe-cli/src/commands/vvm/placer.rs:175`, with `rename_into_place` at
`:182`, `remove_tree` at `:189` and a bounded backoff at `:197-198`. But **the
lock that machinery handles is not the lock this sentence is about**, and the
code's own doc comment says which one it is (`placer.rs:167-174`):

> Is `e` the transient **"a real-time scanner / indexer has a handle open on a
> file inside this directory"** lock? On Windows that is `ERROR_ACCESS_DENIED`
> (5); `fs::rename` / `fs::remove_dir_all` of a directory holding a **freshly
> written** `.exe` / `.dll` (the placer **stages** the full distribution …)
> trips it **moments after the write**.

That is a third-party indexer holding a handle on a **newly staged** file. The
sentence's subject — fixed by its own causal clause and its own example — is a
file **in use by the running process**, the case S2's context paragraph opens
with at `:63-65`: *"a real distribution is many files … and all of them are
**locked while the process runs**"*. The install model does eliminate that
class: nothing in use is ever rewritten, `store.write_current()` flips a
pointer, and the running process keeps its own directory. **The premise "the
design does not operate in a world without locks" does not falsify a sentence
scoped to in-use files.**

**And the document is not naive about locks elsewhere** — `##S7-MECHANICS-REMOVING-THE-ACTIVE-VERSION-REQUIRES-A-FORCE-FLAG`
at `:257-261` already says the running instance's files are handled
*"(best-effort — **skipped if locked**, collected on a later run …)"*. The
package acknowledges lock reality in the section where locks actually bite.

**The different defect, stated precisely.** What the sentence lacks is not a
correction but a caveat: a reader implementing this pattern on Windows will
still need bounded retry around the staging rename, because an external scanner
can hold a transient handle on files the tool has *just written* — a phenomenon
independent of whether anything is in use. The reference implementation pays
~5 s of backoff for it. That is an **omission in guidance**, not a false
statement, and it is one added clause if the boss wants it.

**Proposed correction (NOT APPLIED)** — *optional; the sentence is not false as
written*:

```markdown
##S2-MECHANICS-NO-LOCKS-AND-NO-RELOAD Because no in-use file is ever rewritten, there
are no file locks and no reload — the model is safe even for a shared
library the OS refuses to replace while it is mapped. (A freshly *written*
file is a separate matter: on some systems a real-time scanner may hold a
brief handle on what you have just staged, so the publish rename wants a
short bounded retry.) @impl/done
```

---

### `##S5-LAW-…-IDEMPOTENTLY-AND-ADDITIVELY` · `##S5-MECHANICS-FIVE-RULES-NONE-OPTIONAL` · `##SUM-S5-DURABLE-ENV-EDITS` — SURVIVE, ROUTE (b), and **the recorded count is wrong**

**The re-measurement first, because the verdict's figure does not hold.** The
verdict says consent *"is enforced on exactly ONE path, `self doctor --fix`
(`doctor.rs:103`)"*. It is enforced on **four**:

```console
$ grep -rn "confirm(" --include=*.rs crates/vibe-cli/src/commands/vvm/ | grep -v "fn confirm|use super|tests.rs|with_prompt"
crates/vibe-cli/src/commands/vvm/doctor.rs:103:    if args.fix && confirm(ctx, args.yes, "Write shims and put the shim dir on PATH?")? {
crates/vibe-cli/src/commands/vvm/relocate/mod.rs:325:        && !confirm(
crates/vibe-cli/src/commands/vvm/remove.rs:102:        if !confirm(
crates/vibe-cli/src/commands/vvm/remove.rs:208:            if !confirm(
```

| figure | verdict recorded | measured at `9f79acf1` |
|---|---:|---:|
| mutating paths gated by `confirm()` | **1** | **4** |
| durable-**environment** writers | (not stated) | **2** — one gated, one not |

`confirm()` itself (`mod.rs:439-455`) is a real consent gate with both halves the
law asks for — a `--yes`/unattended bypass at `:442`, and a **non-TTY hard
error** at `:445-449` (`"no TTY for confirmation; pass --yes to proceed
unattended"`) rather than a silent apply. A fifth mechanism, `require_tty()` at
`:461-468`, covers the remove/gc pickers that have no `--yes` bypass at all.

**So the verdict's characterisation — *"enforced on the rare path and skipped on
the common one"* — is the part that does not survive.** What does survive is
narrower and still real: of the **two** code paths that write durable
environment state, one is gated and one is not.

```console
$ grep -rn "set_vibevm_home\|ensure_on_path" --include=*.rs crates/vibe-cli/src | grep -v tests
crates/vibe-cli/src/commands/vvm/doctor.rs:106:  make_persister(env, shell)?.ensure_on_path(&shim_dir)?;   ← inside the confirm at :103
crates/vibe-cli/src/commands/vvm/mod.rs:311:    persister.set_vibevm_home(&home)?;                        ← run_use_cmd, NO confirm
crates/vibe-cli/src/commands/vvm/mod.rs:312:    persister.ensure_on_path(&store.shim_dir())?;             ← run_use_cmd, NO confirm
```

`run_use_cmd` (`mod.rs:291-330`) writes the user environment registry / rc-file
block at `:311-312` with no confirmation and no printed diff. It is honest
afterwards — `:324` prints *"switched live; the next `vibe` in this shell uses
it"* and `:325-328` prints the activation hint — and it offers an explicit
no-write escape at `:300-304` (`--eval`: *"Print only the line to eval in the
current shell; **persist nothing**"*). But consent-before-the-write is absent on
that path.

**Why the package does not move.** All three anchors are **norms**:

- `:172-174` is **the law**, italicised and imperative: *"Edit durable
  environment state idempotently and additively, **with consent and honesty** —
  and behind an injectable seam…"*
- `:176` is *"Five rules, none optional"* — a rule about the pattern, saying a
  designer does not get to drop one.
- `:291-292` is the Summary line, whose siblings are plainly imperative
  (`##SUM-S2-IMMUTABLE-INSTANCES` *"install and switch a whole immutable
  directory; flip a pointer, overwrite nothing in use"*;
  `##SUM-S7-SAFE-REMOVAL` *"a full wipe is flag-plus-reconfirm, never
  default"*).

None asserts that this tool complies. And the law is **the host's own**, twice
over: `spec/common/PROP-019-version-manager.md:215-226` lists the identical five
rules in the same order, `##RULE-CONSENT` among them. A package whose rule the
consumer breaks on one path does not get quieter about the rule —
[§3.6](../PHASE-D-BATCH-PLAN.md#which-side) route (b).

**Proposed correction (NOT APPLIED):** none for these three — correct as
written.

**Recorded, not touched (not my anchors):** `##S5-MECHANICS-CONSENT-AND-HONESTY`
(`:188-191`) asks a mutating edit to *"print the diff it will apply"*. Even the
gated path does not — `doctor.rs:103`'s prompt is the prose string *"Write shims
and put the shim dir on PATH?"*, and `confirm()` renders only that. So the
diff-printing clause is unmet on **both** durable-env paths, which is a sharper
finding than the one this obligation carries, on an anchor outside it.

---

### `##s6-context-a-prose-list-drifts` — FALSE, and the evidence offered against it confirms it

The sentence, at `:211-213`, `@spec/done`:

> If that list lives in prose, it **drifts** from what the code actually checks,
> and bumping the stack means editing several disconnected places.

This is a **conditional general observation** in a Context paragraph — *if* the
list lives in prose, *then* it drifts. It asserts no count, names no artefact,
and describes nothing in this or any codebase. It cannot be falsified by a
measurement of this repository.

The verdict's reason for it is an ironic one: that `PROP-019` §7's prose claim
`##MAINT-RUST-PIN` — *"the Rust pin is `rust-toolchain.toml` (read, not
hard-coded)"* — has itself drifted from the code. **That is the sentence being
right.** A prose list of the required stack drifted from what the code checks,
and bumping the pin would mean editing several disconnected places
(`tools.rs:32`, `tools.rs:38`, `Cargo.toml:53`). The evidence is a confirming
instance of the generalisation, offered as its refutation.

**Proposed correction (NOT APPLIED):** none — correct as written.

---

### `##S6-MECHANICS-RELATED-KNOWLEDGE-FOLLOWS-THE-SAME-RULE` — SURVIVES; and the toolchain file has **no pin in it to read**

The sentence, at `:228-231`:

> Related knowledge follows the same rule: **the default build profile is a
> single constant**, and **the language pin is *read* from the toolchain file,
> not hard-coded**.

**First clause: true.** `DEFAULT_PROFILE` is one `pub const`
(`crates/vibe-cli/src/commands/vvm/model.rs:129`), asserted by a test at
`model.rs:325`, with a single consumer at `mod.rs:275`.

**Second clause: false, and worse than the verdict recorded.** Nothing reads the
file — the verdict's own command reproduces at one doc-comment hit. And the pin
is hard-coded in **three** places:

```console
$ grep -n "1\.93" crates/vibe-cli/src/commands/vvm/tools.rs Cargo.toml
crates/vibe-cli/src/commands/vvm/tools.rs:32:        min_version: "1.93.0",   ← cargo
crates/vibe-cli/src/commands/vvm/tools.rs:38:        min_version: "1.93.0",   ← rustc
Cargo.toml:53:rust-version = "1.93"
```

*(The verdict cited `tools.rs:33`; the constant is at `:32` — `:33` is the
`help_url` line. One line off, and worth correcting in the record since the row
travels by `file:line`.)*

**The sharper finding: there is no pin in the toolchain file at all.**

```console
$ cat rust-toolchain.toml
[toolchain]
channel = "stable"
components = ["rustfmt", "clippy"]
```

`channel = "stable"` names a channel, not a version. So *"the language pin is
read from the toolchain file"* is not merely unimplemented — **the file contains
no pin to read**. A repair by code alone is impossible without first adding one.

**Why this one moves where its S5 neighbours do not.** It is a **Mechanics**
sentence in the indicative, describing the reference implementation in generic
dress — and its own section proves the genre: `##S6-MECHANICS-ONE-TABLE-WITH-FOUR-COLUMNS`
(`:219-220`) says *"One table, each row `(name, minimum version, check command,
help URL)`"*, which is `ToolSpec { name, check, min_version, help_url }` field
for field. The first half of the sentence under judgement is likewise a verified
description of `DEFAULT_PROFILE`. A sentence whose first clause describes this
codebase exactly is describing this codebase in its second clause too — and
there it is wrong.

**Proposed correction (NOT APPLIED)** — the honest version, which keeps the
lesson and drops the false claim:

```markdown
##S6-MECHANICS-RELATED-KNOWLEDGE-FOLLOWS-THE-SAME-RULE Related
knowledge follows the same rule: the default build profile is a single
constant, and the language pin belongs in the toolchain file, **read** rather
than hard-coded — the one place the reference implementation has not yet
closed, where the minimum version is still repeated in the tools table and the
workspace manifest. @spec/done
```

**A second reading the boss may prefer, and it is his call, not mine.** The rule
is sound and the consumer does not keep it, which is
[§3.6](../PHASE-D-BATCH-PLAN.md#which-side) route (b) — pin a version in
`rust-toolchain.toml`, read it in `tools.rs`, leave the package alone. That is a
host code change and therefore **Phase E's**, and it would close this anchor
without a spec diff. The correction above is the route-(a) alternative if the
package is to state today's truth instead. **I have made no edit that prejudges
either.**

**Recommendation per anchor:**
`##S2-MECHANICS-NO-LOCKS-AND-NO-RELOAD` → **re-judge confirmed** (optionally
with the caveat clause, which is an addition rather than a correction);
`##S5-LAW-EDIT-DURABLE-STATE-IDEMPOTENTLY-AND-ADDITIVELY` → **drift stands,
route (b)**;
`##S5-MECHANICS-FIVE-RULES-NONE-OPTIONAL` → **drift stands, route (b)**;
`##s6-context-a-prose-list-drifts` → **re-judge confirmed**;
`##S6-MECHANICS-RELATED-KNOWLEDGE-FOLLOWS-THE-SAME-RULE` → **drift stands,
correction prepared** — or route (b) if the boss prefers the code repair;
`##SUM-S5-DURABLE-ENV-EDITS` → **drift stands, route (b)**.

**The host obligation, at its corrected size:** consent is applied on **four**
mutating vvm paths, not one; of the **two** durable-environment writers, `self
doctor --fix` is gated and `vibe self use` (`mod.rs:311-312`) is not; and
neither prints the diff `##S5-MECHANICS-CONSENT-AND-HONESTY` asks for.

---

## F-199 — every cited restatement reproduces, the census does not; all three anchors are the single-place law, and the consumer is the side that breaks it

**Outcome:** SURVIVES — ROUTE (b) (3 of 3), with **the census figure superseded**
**Anchors:** 3 of 3, by name:

| anchor | file:line | outcome |
|---|---|---|
| `##CONTENT-THE-BOOT-SNIPPET` | `README.md:56-59` | **SURVIVES — ROUTE (b)** |
| `##SCOPE-THE-ONLY-PLACES-THE-TOPIC-IS-DISCUSSED` | `spec/boot/55-flow-attribution-policy.md:45-48` | **SURVIVES — ROUTE (b)** |
| `##HONEST-THE-POLICY-IS-RECORDED-OPENLY` | `spec/flows/attribution-policy/ATTRIBUTION-POLICY.md:79-82` | **SURVIVES — ROUTE (b)** |

**Perimeter searched:** the standing perimeter **including
`packages/org.vibevm.fractality/**`**, over `*.md` · `*.toml` · `*.rs` ·
`*.json` · `*.sh` · `*.ps1`, for `machine-authored` · `human-authored` ·
`AI-authored` · `co-authored-by` · `attribution policy` · `attribution-policy` ·
`machine authorship` · `human authorship`. Each of the verdict's eight cited
restatements opened at its cited line rather than trusted.
**The verdict's own command, re-run:** the verdict quotes no command; it states
"a repo-wide grep for the topic returns 88 lines across 50 files", re-derived
below on a stated basis.
**Measured at:** `9f79acf1`, 2026-07-29 (no figure over the commit log).

**All eight cited restatements reproduce, verbatim, at their cited lines.**

```console
spec/boot/00-core.md:21        ##RULE-ATTRIBUTION **Attribution — keep this repository human-authored.** Never mark commits…
spec/common/PROP-000.md:161    ##GP-ATTRIBUTION human-authored **attribution** — `spec://org.vibevm.world/git-attribution-policy/…`
spec/common/PROP-000.md:323    ##INV-HUMAN-AUTHORSHIP **Human authorship is the only attribution.** The posture is the…
CLAUDE.md:5                    The repository's commit-and-push discipline — human-authored **attribution** (never mark any part…
AGENTS.md:5                    (identical to CLAUDE.md:5)
GEMINI.md:5                    (identical to CLAUDE.md:5)
.claude/agents/opus5.md:15     reviews and commits. Never mark any artifact as machine-authored.
README.md:158                  … the four non-negotiable rules (attribution, Conventional Commits, …)
```

Read one by one they split two ways: **six restate the substance of the rule**
(`00-core.md:21`, `PROP-000:323`, `CLAUDE.md`/`AGENTS.md`/`GEMINI.md:5`,
`opus5.md:15`) and **two are pointers** naming the topic without stating the
rule (`PROP-000:161`, which is a bare `spec://` address; `README.md:158`, which
names "attribution" in a list of four rules and links to `CLAUDE.md`). The
sharpest of the six is `CLAUDE.md:5`, which **restates the rule parenthetically
in the same sentence that says it is not restated**: *"human-authored
**attribution** (never mark any part of this repository as AI-authored) … The
rules live in that inline lane, **not restated here**."*

**The census, re-measured — and it does not reproduce, because the basis
differs.** The verdict's "88 lines across 50 files" excluded `vibedeps/`,
`.vibe/`, `refs/` and the package itself, but **not `campaigns/`**. On my term
set the raw figure is **2 321 lines across 194 files**, of which **1 963 lines
are `campaigns/`** — this campaign's own evidence JSON and harvest prose quoting
the anchors under judgement. That is the same artefact wave 6 hit on the
revisit-trigger count, and it is why the raw number is meaningless.

Stripping the campaign's own records, **every install slot** (`vibedeps/` and
`.vibe/cache/` **anywhere**, including the second project's), and the package's
own tree:

| basis | lines | files |
|---|---:|---:|
| raw, my term set | 2 321 | 194 |
| minus `campaigns/` | 358 | 121 |
| **minus every install slot and the package's own tree** | **98** | **41** |

**98 lines across 41 files** is the honest restatement surface, against the
verdict's 88/50. The two are not comparable term-for-term and I do not claim the
verdict's number was wrong — I claim it is **unreproducible as stated**, because
it names no term set and no exclusion list. The heaviest single file is
`spec/boot/STATIC.md` at **24 lines**, which is the compiled lane carrying the
snippet twice.

**One large slice of the `packages/` count is a different word entirely, and
this is the "fact and evidence are about different things" trap.** In the second
project, "attribution" overwhelmingly means **delegation attribution** — which
session spawned which run: `crates/fractality-cli/src/swarm.rs:38` ("The
attribution default … a boss session's spawns"),
`crates/fractality-core/src/ids.rs:70` ("the attribution unit the scoreboard
…"), `crates/fractality-mission-control/src/sessions.rs:127`. A census that
counts those as policy restatements is counting a different population.

**And the decisive comparison: the second consumer very nearly keeps the law.**

```console
$ (attribution-sense restatements, second project, install slots and the
   delegation-attribution sense excluded)
packages/org.vibevm.fractality/CLAUDE.md:19            "human-authored surface, Conventional Commits, commits"
packages/org.vibevm.fractality/fractality/v0.1.0/spec/plans/FRACTALITY-RLM-PLAN-v0.1.md:588
                                                       "AI-attribution trailer (host Rule 1 — absolute)."
$ grep -niE "machine-authored|human-authored|attribution" \
    packages/org.vibevm.fractality/fractality/v0.1.0/CLAUDE.md
   (no output — the specspace's own boot contract restates nothing)
```

**Two mentions against the host's eight, and the specspace project's own
`CLAUDE.md` carries none at all** — it relies on the boot INDEX entry pointing
at the snippet. So the single-place law is not an impossible standard that no
consumer could meet: **this repository holds two consumers, and the one that
nearly keeps it is not the host.** That is the argument against softening the
package, made from inside the perimeter rather than from principle.

**Why the package does not move — anchor by anchor.**

- `##CONTENT-THE-BOOT-SNIPPET` (`README.md:56-59`) is a package-contents bullet.
  Its first half is verified true — the file ships, is declared
  `link = "static"` (`vibe.toml:16-18`), and lands in the consumer's
  always-loaded lane. Its last sentence is explicitly **conditional on the
  law**: *"**Under the single-place law**, this snippet is the one place in a
  consuming project where the topic lives."* A consumer that does not keep the
  law does not falsify a sentence that opens by naming it.
- `##SCOPE-THE-ONLY-PLACES-THE-TOPIC-IS-DISCUSSED` (`:45-48`) is a **scope
  rule** whose second sentence is an instruction (*"Everywhere else … assume
  human authorship only"*). Its qualifier is load-bearing — *"where AI tooling
  is discussed **in the attribution sense**"* — and the six host restatements
  are squarely in that sense. Breached, by the consumer.
- `##HONEST-THE-POLICY-IS-RECORDED-OPENLY` (`:79-82`) is item 2 of the argument
  that the posture is not sneaky. *"Recorded openly"* holds without
  qualification; *"in exactly one place"* is the single-place law again.

All three are the same law seen from three documents, and
[§3.6](../PHASE-D-BATCH-PLAN.md#which-side) route (b) governs all three:
the rule is sound, the host does not keep it, and rewriting a single-place law
to describe a consumer with eight copies is the *профанация* the mandate exists
to prevent.

**One half of the breach is not the host's discipline at all.** Two of the eight
"places" are `spec/boot/STATIC.md:421` and `:615` — **the same snippet, compiled
twice**, because the family is reached both directly and through the
`git-practices` umbrella. That is `BACKLOG.md` **B-006**, a boot-compiler defect;
no amount of host discipline can stop a generated file carrying the block twice.
It should be named as such in the host obligation rather than counted as a
restatement.

**Proposed correction (NOT APPLIED):** none — all three correct as written.

**Recommendation per anchor:** `##CONTENT-THE-BOOT-SNIPPET` → **drift stands,
route (b)**; `##SCOPE-THE-ONLY-PLACES-THE-TOPIC-IS-DISCUSSED` → **drift stands,
route (b)**; `##HONEST-THE-POLICY-IS-RECORDED-OPENLY` → **drift stands, route
(b)**. The host obligation should carry the **re-measured** surface — 98 lines /
41 files on a named basis, six substantive restatements plus two pointers — and
should separate B-006's two compiled copies from the six the host authored.

---

## F-087 — the escape hatch IS available to a consumer, and the tool-name mentions sit inside the policy's own carve-outs; only the never-restate rule survives

**Outcome:** MIXED — 3/3: **2 FALSE · 1 SURVIVES — ROUTE (b)**
**Anchors:** 3 of 3, all in
`packages/org.vibevm.world/git-attribution-policy/v0.1.0/spec/boot/55-flow-attribution-policy.md`:

| anchor | line | outcome |
|---|---|---|
| `##THE-ALTERNATIVE-IS-ADOPTED-BY-EDITING-THIS-SNIPPET` | `:8-11` | **FALSE** |
| `##NEVER-MENTION-TOOL-NAMES-IN-COMMITS-BRANCHES-OR-COMMENTS` | `:61-62` | **FALSE** |
| `##NEVER-RESTATE-THIS-POLICY-ANYWHERE-ELSE` | `:65-67` | **SURVIVES — ROUTE (b)** |

**Perimeter searched:** the standing perimeter **including
`packages/org.vibevm.fractality/**`** — and the widening is what decided two of
the three. For the escape hatch: both consumers' `spec/boot/` directories and
their `INDEX.md` manifests, read in full; the three on-disk copies of the
snippet compared by checksum. For the tool names: this repository's full history
read-only (`git log -i --grep`, `git branch -a`), and `crates/` + `xtask/` for
`claude|copilot|gpt-|gemini` in comments.
**The verdict's own command, re-run:** the verdict quotes none.
**Measured at:** `9f79acf1`, 2026-07-29; the branch and history figures
re-checked at `b61eb191`, unchanged.

---

### `##THE-ALTERNATIVE-IS-ADOPTED-BY-EDITING-THIS-SNIPPET` — FALSE. §3.7's mirror image, exactly as wave 6 predicted

The sentence, at `:8-11`:

> This is the project's chosen default posture; the alternative (open
> disclosure) is documented in this flow's `disclosure-alternative.md` and **a
> project may adopt it instead by editing this snippet**.

The verdict answers: *"the escape hatch the fact names is **not available to a
consumer**… in this host the only copy of the snippet is inside
`spec/boot/STATIC.md`, whose line 1 reads 'generated by vibe, do not edit'"*.
That is true **of the host** and the verdict generalises it to "a consumer".
**This repository has two consumers, and in the second one the snippet is a
plain editable file that boot reads directly.**

```console
$ ls packages/org.vibevm.fractality/fractality/v0.1.0/spec/boot/
75-tool-fractality.md   INDEX.md
                        ← no STATIC.md: there is no compiled lane at all

$ grep -n "^static" packages/org.vibevm.fractality/fractality/v0.1.0/spec/boot/INDEX.md
   (no output — no `static =` key)

$ sed -n '32,34p' packages/org.vibevm.fractality/fractality/v0.1.0/spec/boot/INDEX.md
[[entry]]
path = "vibedeps/flow-attribution-policy/0.1.0/spec/boot/55-flow-attribution-policy.md"
kind = "static"
```

Against the host, whose `INDEX.md:11` carries `static = "spec/boot/STATIC.md"`
and lists the attribution snippet **not at all** as an entry — it is inlined
into the generated file instead.

**The two consumers run different boot models, and the sentence is true in one
of them.** `kind = "static"` means, in the manifest's own words at
`INDEX.md:5`, *"read the file directly"*. In the second project the file the
session reads at boot **is** `55-flow-attribution-policy.md`, on disk, editable,
with no "do not edit" banner and no compile step between the edit and the next
session. Editing it is precisely the operation the sentence names.

The three on-disk copies, by checksum — the two install slots are byte-identical
to each other, and the package source differs only by Phase B's markup
([§3.5](../PHASE-D-BATCH-PLAN.md#vendored)):

```console
$ md5sum <package source> <host vibedeps> <second project's vibedeps>
3797915bef501f286b41e80ea6d19c0a  packages/org.vibevm.world/git-attribution-policy/v0.1.0/spec/boot/55-flow-attribution-policy.md
140701ca74459b9f16e2c431c4971e7e  vibedeps/flow-git-attribution-policy/0.1.0/spec/boot/55-flow-attribution-policy.md
140701ca74459b9f16e2c431c4971e7e  packages/org.vibevm.fractality/fractality/v0.1.0/vibedeps/flow-attribution-policy/0.1.0/spec/boot/55-flow-attribution-policy.md
```

**This is [§3.7's wave-6 extension](../PHASE-D-BATCH-PLAN.md#compliance-blindness)
firing again, on the same package family.** The verdict scoped its search to the
host's boot lane; the consumer for whom the fact is plainly true lives inside
`packages/`. The invariant the batch plan states — *the perimeter must contain
every project that adopted the discipline, wherever that project sits* — is what
separates this outcome from the recorded one.

*(What remains true, and is the host's: a re-install would clobber an edit to
either `vibedeps/` copy. That is a durability complaint about the install model,
not a refutation of "a project may adopt it by editing this snippet" — and it is
the same in both consumers.)*

**Proposed correction (NOT APPLIED):** none — correct as written.

---

### `##NEVER-MENTION-TOOL-NAMES-IN-COMMITS-BRANCHES-OR-COMMENTS` — FALSE. Every instance falls inside the policy's own two carve-outs

The sentence, at `:61-62`:

> Never mention model, agent, or AI-tool names in commit messages, branch names,
> or code comments.

The verdict's own words are *"every instance read is **configuration or product
data** and none states authorship"* — and it reads that as a breach. **It is the
policy's own scope section being satisfied.** Two carve-outs sit twenty lines
above the Never list, in the same snippet:

> `##SCOPE-PRODUCT-IS-CARVED-OUT` (`:49-51`) **Product scope is carved out.** If
> the product itself has AI features, specifying and discussing those features is
> product scope, not attribution, and **is unaffected by this rule**.
>
> `##SCOPE-WORKFLOW-DOCUMENTS-REMAIN-LEGAL` (`:52-55`) **Technical AI-workflow
> documents remain legal and unchanged** — checkpoint-file procedures, session
> protocols, agent instructions.

**`vibe`'s product is a package manager for AI-agent harnesses.** Its supported
agents are its domain vocabulary, and every code-comment hit is exactly that:

```console
$ grep -rniE "claude|copilot|gpt-|gemini" --include=*.rs crates xtask | grep -E "//|///"
crates/vibe-cli/src/cli/mcp.rs:27:   /// Five agents supported: Claude Code, Claude Desktop, Cursor,
crates/vibe-cli/src/cli/mcp.rs:70:   /// Restrict to a specific agent. One of `all`, `claude`,
crates/vibe-cli/src/cli/skill.rs:29: /// every skill-supporting agent (Claude Code, OpenCode, Codex).
crates/vibe-cli/src/cli.rs:61:       /// string; conventional values are `claude-code`, `claude-desktop`,
crates/vibe-check/src/lib.rs:102:    /// file (`CLAUDE.md` / `AGENTS.md` / `GEMINI.md`) at the project
```

Enum variants and CLI help text naming the harnesses the tool installs into —
product scope, verbatim. And `CLAUDE.md` / `AGENTS.md` / `GEMINI.md` are
**filenames of this project's own boot contract**, which
`##SCOPE-WORKFLOW-DOCUMENTS-REMAIN-LEGAL` covers by name ("agent instructions").

**Branch names are clean:**

```console
$ git branch -a --format='%(refname:short)'
cultural-backup   cultural-refactor   main
fractality/01KXAK9EEZFYFX9VG84NRS2TZF   … (11 worker branches, ULIDs)
refactor/qualified-address-restructure   github/main   origin/main
```

Not one names a model, agent or tool.

**Commit messages: the mentions are file paths and campaign evidence, not
authorship.** 132 of 2 202 commits match a tool name, and the matches are in
bodies, not subjects. Sampled at the top of that list, every one is a path or a
product fact — `.claude/skills/`, `.agents/skills/` and `.opencode/skills/`
(install destinations); `CLAUDE.md:191`, `CLAUDE.local.md`,
`spec/boot/STATIC.md` (filenames); *"what that record publishes per model —
Sonnet 4.6 and GPT-5.4-mini"* (a product feature). **None attributes authorship**
— which wave 6 established for the sibling `##NEVER-STATE-OR-IMPLY-MACHINE-AUTHORSHIP`
and which is the same evidence.

**The distinction the anchor needs, and the policy already draws it.**
`##SCOPE-THE-ONLY-PLACES-THE-TOPIC-IS-DISCUSSED` qualifies the whole topic with
*"in the attribution sense"*. Naming `claude-code` as an install target is not
the attribution sense; saying "never mark any artefact as machine-authored" is.
**That single qualifier is why this anchor is FALSE and F-199's second anchor
SURVIVES** — the same word, two senses, and the policy scopes itself to one of
them.

**Proposed correction (NOT APPLIED):** none — correct as written.

---

### `##NEVER-RESTATE-THIS-POLICY-ANYWHERE-ELSE` — SURVIVES, ROUTE (b)

The sentence, at `:65-67`:

> Never weaken, widen, or restate this policy anywhere else in the repository —
> one policy, one place. Changing it is one edit to this file, made by the owner.

This is the imperative twin of F-199's `##SCOPE-THE-ONLY-PLACES…`, and the host
breaks it six times over (§F-199 above). The wording has already drifted in one
of the copies, exactly as a single-place law predicts — verified at HEAD:

```console
$ grep -nE "^#{2,3} 12" spec/common/PROP-000.md
157:## 12. Commit and push discipline {#commits}
$ grep -n "12\.1" spec/common/PROP-000.md
   (no output — PROP-000 has no §12.1)
$ grep -n "PROP-000 §12" spec/boot/00-core.md
21:… The rule itself (and its copy in PROP-000 §12.1) is the only place …
```

`spec/boot/00-core.md:21` cites a section that does not exist — a copy citing a
copy that was renumbered. That is the drift the law exists to prevent, present
and unrepaired, and it is entirely host-side. Wave 6 recorded the same dangling
reference; **it is still dangling at `b61eb191`.**

An imperative "Never …" is a norm by construction; a package whose rule the
consumer breaks does not get quieter about the rule.
[§3.6](../PHASE-D-BATCH-PLAN.md#which-side) route (b).

**Proposed correction (NOT APPLIED):** none — correct as written.

**Recommendation per anchor:**
`##THE-ALTERNATIVE-IS-ADOPTED-BY-EDITING-THIS-SNIPPET` → **re-judge confirmed**;
`##NEVER-MENTION-TOOL-NAMES-IN-COMMITS-BRANCHES-OR-COMMENTS` → **re-judge
confirmed**; `##NEVER-RESTATE-THIS-POLICY-ANYWHERE-ELSE` → **drift stands, route
(b)**, with the dangling `PROP-000 §12.1` at `spec/boot/00-core.md:21` named in
the host obligation as a concrete, one-line repair.

---

## F-232 — the switching procedure has never fired, so the missing decision record falsifies nothing; the summary's "one edit in one place" is the law again

**Outcome:** MIXED — 2/2: **1 FALSE PREMISE, DIFFERENT DEFECT · 1 SURVIVES —
ROUTE (b)**
**Anchors:** 2 of 2, both in
`packages/org.vibevm.world/git-attribution-policy/v0.1.0/spec/flows/attribution-policy/disclosure-alternative.md`:

| anchor | line | outcome |
|---|---|---|
| `##SWITCH-EDIT-THE-SINGLE-POLICY-PLACE` | `:82-84` | **FALSE PREMISE, DIFFERENT DEFECT** |
| `##SUM-SWITCHING-IS-FORWARD-ONLY` | `:101-102` | **SURVIVES — ROUTE (b)** |

**Perimeter searched:** the standing perimeter **including
`packages/org.vibevm.fractality/**`** for `When to revisit` and for any file
named like a decision record; both consumers' `vibedeps/` checked for
`flow-decision-records`; this repository's history read-only for a
posture-change commit.
**The verdict's own command, re-run:** the verdict quotes none.
**Measured at:** `9f79acf1`, 2026-07-29.

**The absence is real, in both consumers.**

```console
$ ls -d vibedeps/flow-decision-records
vibedeps/flow-decision-records                                          ← installed, host
$ ls -d packages/org.vibevm.fractality/fractality/v0.1.0/vibedeps/flow-decision-records
…/vibedeps/flow-decision-records                                        ← installed, second project

$ grep -n "When to revisit" spec/common/PROP-000.md
23:  ##LANG-REVISIT   … Never, in the scope of v1 …          ← the LANGUAGE choice
57:  ##LICENSE-REVISIT … fired on 2026-07-12 and is spent …  ← the LICENCE choice

$ grep -rn "When to revisit" packages/org.vibevm.fractality/ | grep -v "vibedeps|\.vibe/" | wc -l
0
```

`flow:decision-records` is installed in both projects; **neither holds a decision
record for the attribution posture.** The host's two records are about the
implementation language and the licence.

### `##SWITCH-EDIT-THE-SINGLE-POLICY-PLACE` — FALSE PREMISE, DIFFERENT DEFECT

The sentence is **step 1 of a numbered procedure**, at `:82-84`, under the lead
`##a-posture-change-is-forward-only-lead` (*"A posture change is
**forward-only**:"*) and the section heading `## Switching postures {#switching}`:

> 1. The owner edits the single policy place (the boot snippet) to the **new
>    posture**, with a dated decision record and a revisit trigger (see
>    `flow:decision-records`).

**The step fires on a switch, and no switch has occurred.** The snippet at `:5`
says the human-authored posture is *"the project's chosen default"*; the
alternative has never been adopted; no commit in 2 202 records a posture change.
So the artefact the verdict finds missing is one that **step 1 has never been
invoked to produce**. Measuring a never-executed procedure's output against the
tree is the same category error as demoting a mechanism nobody claimed built.

**The different defect, named precisely.** There *is* a real gap here, and it
belongs to a different anchor: the obligation to record the posture **as
chosen**, not as switched, is `##A-POSTURE-IS-CHOSEN-ONCE-RECORDED-AND-ENFORCED`
— which **wave 6 examined as F-230 and routed (b)**
([`d6e-git-sync-absences.md`](d6e-git-sync-absences.md)). The verdict's evidence
supports that anchor and not this one. Recording the same absence twice, on an
anchor whose trigger has never fired, would double-count a single host
obligation.

**Proposed correction (NOT APPLIED):** none — correct as written.

### `##SUM-SWITCHING-IS-FORWARD-ONLY` — SURVIVES, ROUTE (b)

The Summary line, at `:101-102`:

> Switching postures is **one edit in one place**, forward-only, with a dated
> decision record; history is never rewritten to match.

*"Forward-only; history is never rewritten"* holds — no history rewrite exists,
and the sibling `##SUM-PUSHED-HISTORY-IS-FROZEN` in `git-atomic-commits` is
kept. *"One edit in one place"* does not hold **in the host**: the policy is
compiled twice into a generated do-not-edit file and restated in six further
host locations, so a switch would be roughly ten edits plus a recompile.

But that is the single-place law's promise, owed to a project that keeps the
law — and F-199 measured that **the second consumer restates it twice, its own
`CLAUDE.md` not at all**, so for that project the promise is close to literally
true. The package is right about what switching costs a compliant consumer; the
host is not a compliant consumer. [§3.6](../PHASE-D-BATCH-PLAN.md#which-side)
route (b).

**Proposed correction (NOT APPLIED):** none — correct as written.

**Recommendation per anchor:** `##SWITCH-EDIT-THE-SINGLE-POLICY-PLACE` →
**re-judge confirmed** (the step has never fired; its evidence belongs to F-230's
anchor, already routed out); `##SUM-SWITCHING-IS-FORWARD-ONLY` → **drift stands,
route (b)** — and it is the *same* host obligation as F-199's, not a new one.

---

## Batch summary

| id | anchors | outcome | package moved? |
|---|---:|---|---|
| F-309 | 1 | SURVIVES (1) | **no** — correction prepared, unapplied |
| F-308 | 1 | SURVIVES — ROUTE (b) (1), **both figures superseded** | no |
| F-344 | 1 | **FALSE** (1) | no |
| F-157 | 6 | MIXED — 1 FALSE · 1 FALSE PREMISE · 1 SURVIVES · 3 ROUTE (b) | **no** — correction prepared, unapplied |
| F-199 | 3 | SURVIVES — ROUTE (b) (3), **census superseded** | no |
| F-087 | 3 | MIXED — **2 FALSE** · 1 ROUTE (b) | no |
| F-232 | 2 | MIXED — 1 FALSE PREMISE · 1 ROUTE (b) | no |

**17 verdicts examined. Zero package files edited, zero characters.**

| outcome | anchors |
|---|---:|
| **FALSE** — the description is right | **4** |
| **FALSE PREMISE, DIFFERENT DEFECT** — the stated reason does not hold | **2** |
| **SURVIVES** — the package is wrong, correction prepared and unapplied | **2** |
| **SURVIVES — ROUTE (b)** — the rule is sound, the consumer does not keep it | **9** |

**Six of seventeen do not survive as recorded**, and every one of them can be
closed by a re-judge that edits nothing — **no spec diff, no owner approval**,
which is the whole reason this route could be worked autonomously. Only **two**
anchors need an owner-approved diff, and both are written out above and left
unapplied.

**The single most expensive finding: §3.7's mirror image fired again, on this
family.** `##THE-ALTERNATIVE-IS-ADOPTED-BY-EDITING-THIS-SNIPPET` was judged
false because *"the escape hatch is not available to a consumer"* — measured
against the host's compiled `spec/boot/STATIC.md` and its "do not edit" banner.
**The second consumer has no compiled lane at all**: its
`spec/boot/INDEX.md` carries no `static =` key and names
`vibedeps/flow-attribution-policy/0.1.0/spec/boot/55-flow-attribution-policy.md`
as a `kind = "static"` entry, which the manifest's own header defines as *"read
the file directly"*. In that project the snippet a session reads **is** a plain
editable file, and editing it is exactly the operation the sentence describes.
Two consumers, two boot models, and the verdict measured one of them.

**The second sharpest: a verdict can be falsified by the package's own scope
section.** `##NEVER-MENTION-TOOL-NAMES-IN-COMMITS-BRANCHES-OR-COMMENTS` was
convicted on evidence the verdict itself characterised as *"configuration or
product data"* — which is verbatim what `##SCOPE-PRODUCT-IS-CARVED-OUT` (`:49-51`)
exempts, twelve lines above the rule. The same qualifier — *"in the attribution
sense"* — is why F-199's `##SCOPE-THE-ONLY-PLACES-THE-TOPIC-IS-DISCUSSED`
**does** fall: one word, two senses, and the policy scopes itself to one.
Similarly, `##P3-MECHANICS-A-SHORT-DENYLIST-…` (F-344) is exact under its plain
reading, and the verdict's reading makes the P3 section contradict itself
fifteen lines later at `##P3-MECHANICS-THE-DENYLIST-INTRODUCES-NO-CHOICE`.
**Three of the four FALSE outcomes were settled by reading the subject document
further, not by searching wider.**

**Every figure re-measured, old value → new.**

| figure | recorded | re-measured | where |
|---|---:|---:|---|
| subjects over the 72-char hard limit | 82/400 — 20.5 % | **131/400 — 32.8 %** at `9f79acf1`; **136/400 — 34.0 %** at `b61eb191` | F-308 |
| subjects over the 60-char target | 297/400 — 74.3 % | **316/400 — 79.0 %** (both HEADs) | F-308 |
| longest subject | 89 chars | **119 chars** | F-308 |
| vvm paths gated by `confirm()` | "exactly ONE" | **four** call sites (`doctor.rs:103`, `relocate/mod.rs:325`, `remove.rs:102`, `remove.rs:208`) | F-157 |
| durable-environment writers | not stated | **two** — `doctor --fix` gated, `run_use_cmd` (`mod.rs:311-312`) not | F-157 |
| hard-coded Rust pin locations | `tools.rs:33`, `tools.rs:38`, `Cargo.toml:53` | **`tools.rs:32`**, `tools.rs:38`, `Cargo.toml:53` — one line off | F-157 |
| repo-wide attribution-topic surface | 88 lines / 50 files | **98 lines / 41 files** on a named basis (raw 2 321/194; minus `campaigns/` 358/121) | F-199 |
| host restatements of the policy | "at least eight" | **eight, all reproducing** — but six restate, two are pointers, and two of the eight are B-006's compiled duplicate | F-199 |
| second consumer's restatements | not measured | **two**, and its own `CLAUDE.md` none | F-199 |
| decision records for the posture | "none in the host" | **none in either consumer** — and `flow:decision-records` is installed in both | F-232 |

**One new fact the verdicts did not have.** `rust-toolchain.toml` contains
`channel = "stable"` and components — **no version at all**. So *"the language
pin is read from the toolchain file"* is not merely unimplemented: there is no
pin in the file to read, and a code-only repair is impossible without first
adding one. That is worth knowing before Phase E is asked to close it.

**What the host is owed, if these route out.** The single-place law against six
authored restatements plus B-006's compiled duplicate, and the dangling
`PROP-000 §12.1` at `spec/boot/00-core.md:21` — still dangling at `b61eb191`,
and a one-line fix (F-199, F-087, F-232). Consent on `vibe self use`'s durable
environment write, and the diff-printing clause unmet on **both** durable-env
paths (F-157). The subject-length breach at its **re-measured** size (F-308).
And a decision record for the attribution posture, which F-230 already carries —
**F-232 should not double-count it.**

**Two records that should be corrected rather than carried.** F-232's
`##SWITCH-EDIT-THE-SINGLE-POLICY-PLACE` measures a **procedure that has never
fired** — no posture change exists in 2 202 commits — so its evidence belongs to
F-230's `##A-POSTURE-IS-CHOSEN-ONCE-RECORDED-AND-ENFORCED`, not here. And
F-199's census figure names no term set and no exclusion list, which is why it
does not reproduce; **any figure over `campaigns/` must say whether the
campaign's own records are in or out**, because here they were 85 % of the raw
count.

**Finally, the decay is now measured rather than asserted.** HEAD moved six
commits *during this batch* — `9f79acf1` → `b61eb191` — and F-308's hard-limit
breach moved 1.2 points with it, on six commits that touched no code. Wave 6
recorded that a 400-commit figure decays within the week. It decays within the
session.
