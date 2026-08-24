# D6e — the git family + `sync-from-code`: six claimed absences, re-verified before any demotion

_Worked 2026-07-29. Subjects: `vibevm/vibepacks/org.vibevm.world/git-attribution-policy/v0.1.0/`,
`vibevm/vibepacks/org.vibevm.world/git-conventional-commits/v0.1.0/`,
`vibevm/vibepacks/org.vibevm.world/git-atomic-commits/v0.1.0/`,
`vibevm/vibepacks/org.vibevm.world/sync-from-code/v0.1.0/`. Six obligations, all
`build-or-demote`, 8 drift verdicts. Every one asserts that some checker,
mechanism or record **does not exist**._

_Worked under [§6.1 `##ABSENCE-NAMES-ITS-PERIMETER`](../PHASE-D-BATCH-PLAN.md#delegation-lessons)
and [§3.7](../PHASE-D-BATCH-PLAN.md#compliance-blindness): a demotion is the
**last** step, not the first, and a `not-found` is a fact about the search
perimeter until the perimeter has been checked. No code was written; no `git`
command that writes was run; nothing under `run/` was touched._

Obligations: F-230 · F-306 · F-234 · F-303 · F-338 · F-339.

**The §3.6 split is the whole of this batch.** These four packages are
*normative flows* — they say «this is the rule», not «a mechanism exists». The
question every sentence had to answer before it could be touched was
[§3.6](../PHASE-D-BATCH-PLAN.md#which-side)'s: is the package claiming
something is BUILT (an absence → demote), or prescribing a norm the consumer
does not keep (route (b) → the package does not move)? Getting that wrong in
the easy direction is how Phase D would quietly rewrite a discipline to
describe a lax consumer.

**The standing perimeter** (referred to below as *the standing perimeter*), run
from the repository root:

```
packages/**  vibedeps/**  crates/**  xtask/**  tools/**  spec/**
discipline/**  terraform/**  research/**  campaigns/**  legacy-spec/**
fixtures/**  schemas/**  docs/**  manual-tests/**
and the repository root's own *.md / *.toml / *.json / *.sh / *.ps1
minus  **/target/**  .git/**  **/node_modules/**  campaigns/*/run/**
```

`refs/**` is searched but reported **separately**: it is a third-party study
corpus, not our shipped surface, and a hit there is not an implementation of
ours. Where an absence is about a *file that should exist* (a hook, a CI file),
the perimeter is widened by name — `.git/hooks/`, `.githooks/`, `.github/`, and
`git config` — because an absent file cannot be grepped for.

**And the four layers, because they are why the perimeter keeps biting:** a
mechanism's SPEC lives in the package, its ENGINE in that package's library
crates, its DRIVER in a CLI, and its DEPLOYMENT in the consuming project. A
fact can be true at any one and invisible at the other three.

---

## F-230 — the posture is chosen and recorded and nothing enforces it; both anchors are norms, and the consumer is the side that is wrong

**Outcome:** ROUTE-B CANDIDATE (2 of 2)
**Anchors:** 0 of 2 moved. Not touched, and deliberately:
`##A-POSTURE-IS-CHOSEN-ONCE-RECORDED-AND-ENFORCED`,
`##one-place-to-read-one-place-to-change`.
**Files touched:** none
**Perimeter searched:** the standing perimeter, for `co-authored-by` ·
`signed-off-by` · `pre-push` · `attribution` · `human-authored` ·
`machine-authored` · `AI-authored` · `generated (with|by) …ai|llm|claude|gpt|copilot`
over `*.rs` · `*.sh` · `*.ps1` · `*.py` · `*.toml` · `*.json` · `*.yml` ·
`*.bat` · `*.md`, **widened by name** — because an absent file cannot be
grepped for — to `.git/hooks/`, `.githooks/`, `.github/`, `git config --get
core.hooksPath`, `git config --local --list`, a tree-wide `find` for CI
manifests, a full listing of `tools/`, and the ten declared steps of
`tools/self-check.sh`. Real git history was read (`git log`, read-only) for the
outcome the policy is about.

**What the search found — the absence is real, and it is the consumer's:**

```console
$ ls -1 .git/hooks/ | grep -v '\.sample$'          # (no output)
$ ls -1 .githooks                                   # No such file or directory
$ ls -R .github                                     # No such file or directory
$ git config --get core.hooksPath ; echo rc=$?      # rc=1
$ git config --local --list | grep -iE 'trailer|sign|coauthor'   # (no output)
```

No CI manifest exists in our shipped surface. A tree-wide `find` for
`*.yml`/`*.yaml` under CI-shaped paths returns **41 files and every one is
under `refs/**`** — `refs/src/bazel/.github/workflows/`,
`refs/src/cargo/.github/workflows/`, `refs/src/spec-kit/.github/workflows/`,
and so on. **Reported separately per the perimeter rule: `refs/` is a
third-party study corpus and a hit there is not an implementation of ours.**

`tools/` holds `first-run.ps1`, `first-run.sh`, `jtd-codegen`,
`progress-dashboard`, `self-check.sh`, `user-home-tripwire.sh`. The floor's own
header enumerates its ten steps (`tools/self-check.sh:6-36`) — fmt, test,
clippy, `vibe check`, `conform check`, `sync-engines --check`, the
`core-ai-native` package gate, the language-stack gates, the specmap self-trace,
the MCP gates — and attribution is not among them.

**The one near-miss, checked rather than assumed.** A grep for `attribution`
across `crates/` and `xtask/` hits `xtask/src/batch_review/mod.rs:179`:

```console
$ sed -n '176,186p' xtask/src/batch_review/mod.rs
    let known_dead: std::collections::BTreeSet<String> = [
        "atomic-commits",
        "attribution-policy",
        "conventional-commits",
        "autonomy",
    ]
```

That is F-097's dead-package-name allowlist feeding `c12_package_refs` — a check
that package *references* resolve, not a check that commits carry no attribution
mark. It is not a checker for this policy.

**What the policy's own product says.** `flow:health-audit` is installed
(`vibedeps/flow-health-audit/`) and `AUDIT.md` is 458 lines; a case-insensitive
grep for `attribution|co-authored|machine-authored|human-authored` over it
returns **nothing**, so the checklist's own audit line is absent too. *(That
line is `##THE-AUDIT-CHECKLIST-LINE` in `enforcement-checklist.xml`, not one of
my anchors — recorded, not touched.)*

**And the outcome the policy exists to produce is nonetheless achieved:**

```console
$ git log -400 --format='%H %B' | grep -ci 'co-authored-by'
2
$ git log -400 --format='%an <%ae>' | sort -u
Oleg Chirukhin <oleg@anarchic.pro>
```

Both hits are **commit bodies quoting this campaign's own measurement** —
`e8438cda` («…the single `co-authored-by` grep hit is a commit body quoting…»)
and `89c90aed` — not trailers. **Zero real trailers, one author, 400 commits.**
The policy is kept by configuration and not by a checker, which is exactly the
state the verdict names.

**Now the second anchor, measured rather than carried.**
`##one-place-to-read-one-place-to-change` promises «one place to read it, one
place to change it, zero copies to drift», under the single-place law two lines
above it. The consumer keeps neither half:

```console
$ grep -n "git-attribution-policy" vibevm/vibespecs/boot/STATIC.xml
421:<!-- vibe:static org.vibevm.world/git-attribution-policy — …/55-flow-attribution-policy.xml -->
615:<!-- vibe:static org.vibevm.world/git-attribution-policy — …/55-flow-attribution-policy.xml -->

$ diff <(sed -n '421,470p' vibevm/vibespecs/boot/STATIC.xml) <(sed -n '615,664p' vibevm/vibespecs/boot/STATIC.xml)
   (no output — two verbatim copies in one generated always-loaded file)
```

The whole `git-practices` block repeats, so this is **compiler output, not a
hand restatement**: the flow is pinned `=0.1.0` by the family aggregator
(`vibedeps/flow-git-practices/0.1.0/vibe.toml:31`), which `redbook` in turn pins
(`vibedeps/flow-redbook/0.2.0/vibe.toml:29`) and the host installs
`link = "static-transitive"` (`vibe.toml:28`). On top of the two compiled
copies the policy is restated at `CLAUDE.md:5`, `AGENTS.md:5`, `GEMINI.md:5`,
`vibevm/vibespecs/boot/00-core.xml:21`, `vibevm/vibespecs/common/PROP-000.xml:161`
(`##GP-ATTRIBUTION`), and `.claude/agents/opus5.md:15`.

The drift the verdict predicted is present:

```console
$ grep -n "12\.1" vibevm/vibespecs/common/PROP-000.xml          # (no output)
$ grep -nE "^## 12" vibevm/vibespecs/common/PROP-000.xml
157:## 12. Commit and push discipline {#commits}
$ grep -n "PROP-000 §12.1" vibevm/vibespecs/boot/00-core.xml
21:… The rule itself (and its copy in PROP-000 §12.1) is the only place …
```

**PROP-000 has no §12.1** — it has an unnumbered `## 12. Commit and push
discipline`. The host restatement cites a section that does not exist, which is
the copy-drift the anchor says the single-place law prevents.

**Which layer has it, if any:** **nowhere** for a checker at any of the four
layers — the package ships no executable (`vibe.toml` declares one
`[boot_snippet]` and nothing else), no engine crate exists for it, no CLI drives
it, and the consumer deployed no hook. **Host deployment** for the
configuration half: the snippet is compiled into `vibevm/vibespecs/boot/STATIC.xml` and read
at every session start, and the surface it governs is clean.

**Why nothing moved.** Both sentences are the package **prescribing**, not
describing. «Whatever posture a project wants, it **must** be chosen once,
recorded, and enforced» is a norm in the imperative; «one place to read it, one
place to change it, zero copies to drift» is the single-place law's stated
payoff, owed to a project that keeps the law. This consumer chose and recorded
and did not enforce, and it restates the policy in eight further places. That is
[§3.6](../PHASE-D-BATCH-PLAN.md#which-side) route (b) in its purest form: the
rule is sound, the host does not keep it, and softening the package to match a
lax consumer is the *профанация* the mandate exists to prevent. **The compliance
work is the host's — and, for the duplicated snippet, arguably the boot
compiler's, since no amount of host discipline can stop a generated file
carrying the block twice.**

**Verdict recommendation, per anchor:**
`##A-POSTURE-IS-CHOSEN-ONCE-RECORDED-AND-ENFORCED` → **drift stands, route (b)** —
the absence is real and total, and the sentence is a norm; the enforcement is
owed by the host, not by the package. `##one-place-to-read-one-place-to-change`
→ **drift stands, route (b)** — two compiled copies plus six restatements plus
one already-dangling `PROP-000 §12.1`, all of them host-side; the law is sound
and the consumer breaks it.

---

## F-306 — the README claims the package makes the choice «explicit and enforced»; explicit ships, enforced is a document

**Outcome:** PARTIAL → DEMOTED (1 of 1)
**Anchors:** 1 of 1 moved: `##PACKAGE-MAKES-THE-CHOICE-EXPLICIT-AND-ENFORCED`
(defined at `README.md:14`, checked against the file rather than assumed).
**Files touched:**
`vibevm/vibepacks/org.vibevm.world/git-attribution-policy/v0.1.0/README.md`
**Perimeter searched:** F-230's perimeter (shared — same absence, same day, not
re-run), **plus** the package's own tree listed in full rather than grepped,
because the claim's subject is the package: `find
vibevm/vibepacks/org.vibevm.world/git-attribution-policy -type f`, and its `vibe.toml`
read for what an install actually writes.

**What the search found:**

```console
$ find vibevm/vibepacks/org.vibevm.world/git-attribution-policy -type f
  …/v0.1.0/LICENSE.md
  …/v0.1.0/README.md
  …/v0.1.0/spec/boot/55-flow-attribution-policy.md
  …/v0.1.0/spec/flows/attribution-policy/ATTRIBUTION-POLICY.md
  …/v0.1.0/spec/flows/attribution-policy/disclosure-alternative.md
  …/v0.1.0/spec/flows/attribution-policy/enforcement-checklist.md
  …/v0.1.0/vibe.toml

$ grep -A4 '^\[boot_snippet\]' …/v0.1.0/vibe.toml
source = "spec/boot/55-flow-attribution-policy.md"
category = "flow"
link = "static"
```

Seven files, six of them prose. `vibe.toml` declares **one `[boot_snippet]` and
no executable** — no hook template, no script, no `[[skill]]`, nothing an
install could wire. The enforcement the sentence claims lives at
`enforcement-checklist.xml:34-42`: a lead line reading *«Run before every push
(or wire as a `pre-push` hook)»*, and under it a fenced `sh` block whose body is
one `git log --format='%H %B' @{u}..HEAD` piped into a `grep -inE
'co-authored-by|…'`. It is text a project must copy out for itself. **«or wire
as a `pre-push` hook» is the package handing the wiring to the reader**, and
F-230's perimeter established that this reader did not do it.

**Why this one is route (a) where F-230's two are route (b).** The subject of
the sentence is **the package**: *«This package makes the choice explicit and
enforced»*, in the indicative, in the README's what-this-is block, carrying
`@impl/done`. It is not a norm addressed to a consumer — it is a claim about the
package's own tree, and the package's own tree falsifies half of it. That makes
`falsifier` decisively `self` on the deciding fact, and
[§3.6(a)](../PHASE-D-BATCH-PLAN.md#which-side) applies without a judgement call.
The registry types it `mixed` because two of its four evidence refs are host
files; the deciding one is not.

**Why PARTIAL and not a flat demotion.** *Explicit* is true and verifiable:
both postures are documented as first-class, the single-place law is stated, and
the boot snippet lands compiled into this consumer's always-loaded lane
(`vibevm/vibespecs/boot/STATIC.xml:421`). And the package's **own** doctrine ranks that above
scanning — `enforcement-checklist.xml:57` («Scanning catches slips; configuration
prevents them») and `:63-65` («Put the policy there — this package's boot
snippet is exactly that — and the agent stops *producing* the marks»). Measured,
that half works: zero attribution trailers and one author across 400 commits. A
flat «not built» would have been false in the half that ships and is doing the
job. So the clause names the halves and only the enforcement half is convicted —
by the flow's own law at `enforcement-checklist.xml:5`, that a policy with no
checker is a wish.

**Which layer has it, if any:** **spec** for *explicit* (three documents plus
the snippet, all present); **host deployment** for the configuration effect (the
snippet compiled into `vibevm/vibespecs/boot/STATIC.xml`); **nowhere** for a checker — not in
the package, not as an engine crate, not as a CLI driver, not deployed by this
consumer.

**What changed and why:** the sentence is kept **word for word** and gains an
italic clause naming what is missing (an installed checker; no hook, no CI file,
no trailer setting), what was searched (the standing perimeter plus the four
by-name checks), which half nonetheless ships, and the flow's own law that
convicts the other half. Marker `@impl/done` → `@spec/done`. No rule text was
weakened and no prescription was rewritten.

**New obligations noticed:** (1) `##THE-AUDIT-CHECKLIST-LINE`
(`enforcement-checklist.xml:78`, `@impl/done`, **not in my six**) requires one
attribution line in the periodic audit checklist of a project running
`flow:health-audit`; this project runs it and `AUDIT.md` carries no such line —
same route-(b) shape as F-230. (2) `##MESSAGES-AND-TRAILERS-ARE-MECHANICALLY-CHECKABLE`
(`enforcement-checklist.xml:31`, `@impl/done`) is *true as stated* — they are
checkable — but sits directly above the scan nobody wired, so a later wave may
read it as the same claim I just demoted. Recorded, untouched.

**Verdict recommendation, per anchor:**
`##PACKAGE-MAKES-THE-CHOICE-EXPLICIT-AND-ENFORCED` → **demoted, now confirmed** —
the sentence now says which half ships and which is a wish, and the marker
matches.

---

## F-234 — re-measured: the imperative-mood breach is half again as large as recorded, the lowercase breach a quarter the size, and both anchors are still norms

**Outcome:** ROUTE-B CANDIDATE (2 of 2), with **both recorded figures
superseded**
**Anchors:** 0 of 2 moved. Not touched: `##HEADER-IMPERATIVE-MOOD`,
`##HEADER-LOWERCASE`.
**Files touched:** none
**Perimeter searched:** two perimeters, because this obligation has two halves.
For the **measurement**: this repository's own history, read-only
(`git log -400 --format=%s`, `--format=%h\t%s`), at HEAD `e118b76f`. For the
**checker**: the standing perimeter over `*.rs` · `*.sh` · `*.ps1` · `*.py` ·
`*.toml` · `*.json` · `*.yml`, for `commitlint` · `conventional.?commit` ·
`commit-msg` · `commit_msg` · `imperative` · `subject line` ·
`git log --format=%s`, plus a by-name check for `package.json`,
`.commitlintrc*`, `commitlint.config.*` and a full listing of
`crates/vibe-check/src/checks/`.

**The measurement, re-run rather than carried.** The verdict's figures do not
reproduce, and neither number is wrong — **the window moved.**

```console
$ git log -400 --format=%s | wc -l
400
$ git log -1 --format='%h %ad %s' --date=short
e118b76f 2026-07-29 docs(continue,wal): the checkpoint for a phase a third through, …
```

*Header shape.* **400 of 400 carry a `type(scope):` prefix** — the shape rule
itself is kept without exception in this window.

*Imperative mood.* **213 of 400 — 53.2 % — open with `the`, `a` or `an`**:
`the` 154, `a` 56, `an` 3. The verdict recorded 144 of 400 (36 %). Add the
numeral and quantifier openers, which are noun phrases by the same test —
`three` 16, `four` 8, `one` 6, `two` 6, `five` 5 — and the majority of the
window does not complete *«If applied, this commit will …»*. The three
commonest first words after the prefix are now `the` (154), `a` (56) and
`three` (16); the verdict's third was `Phase` (42), which is 7 in this window.

*The failure the rule literally names is still nearly absent.* A scan for
past-tense / third-person openers (`^[a-z]+(ed|es|s)$`, minus plural nouns)
returns **two candidates, and both are false positives** on inspection:
`666fe2c6 feat(campaign): sources 2 and 3 become one command, over the boot lane`
(plural noun, present-tense verb) and
`12e12d4c docs(typescript-ai-native-lang): marked from its twins` (elliptical
participle). The rule's own examples — «added», «fixes», «refactored» — describe
a failure this project does not commit.

*Lowercase.* The verdict's methodological point survives and its number does
not:

```console
naive count (first char isupper): 62 of 400 = 15.5 %
  … of which identifiers (W2, W6, C3, B-001, F-128, B7, NOTOUCH, F …): 48+
  … of which plain capitalised words:                                    9
```

The nine, in full: `Phase` seven times (`33bd5b1e`, `fac57627`, `ef40a1ce`,
`0acc448f`, `053f1671`, `56172a8f`, `3ec0424c`), `PHASE` once
(`7c674c18 docs(campaign): PHASE C CLOSES — qualified-naming lands the last 190
anchors`), and `I` once
(`f7028e72 fix(campaign): I moved two files under a finished table, and the
checker found it`). That is **9 of 400 = 2.3 %**, against the verdict's 42 of
400 = 10.5 %. The distinction the verdict identified — that a naive count
over-counts because most capitalised openers are identifiers — is exactly right
and is why the naive 62 must not be reported as the breach.

**Why the two runs disagree, and it matters for whoever inherits this.** The
400-commit window at HEAD spans **four days, 2026-07-25 to 2026-07-29**, and
**314 of its 400 commits (78.5 %) are scoped `campaign`, `wal` or `continue`** —
`campaign` alone is 275. **The window no longer measures the project; it
measures this campaign.** A host obligation written as «144 of 400» will not
reproduce next week either. Any figure carried forward should name its HEAD and
its date range, or be stated as a rate over a fixed date window rather than a
commit count.

**The checker: absent, on a named perimeter.**

```console
$ grep -rniE "commitlint|conventional.?commit|commit-msg|commit_msg|imperative" \
    --include=*.rs --include=*.sh --include=*.ps1 --include=*.py \
    --include=*.toml --include=*.json --include=*.yml \
    crates xtask tools discipline terraform schemas fixtures manual-tests \
    vibevm/vibepacks/org.vibevm.world
  … every hit is a commit-message PRODUCER, not a checker:
  crates/vibe-publish/src/git_publish.rs:57   let commit_msg = format!("Release {package_name}@{version}");
  crates/vibe-cli/src/commands/registry/redirect/update.rs:450  build_redirect_update_commit_msg(…)
  xtask/src/batch_review/mod.rs:180           "conventional-commits",   ← the dead-name allowlist again
  vibevm/vibepacks/org.vibevm.world/*/vibe.toml       package descriptions

$ ls package.json .commitlintrc* commitlint.config.*   # rc=2, none exist
$ ls -1 crates/vibe-check/src/checks/
activation_conflict.rs  boot_directory.rs  features_graph.rs  i18n_coverage.rs
lockfile_files.rs  manifest_validity.rs  mod.rs  redirect_block.rs
review_aging.rs  subskill_structure.rs  wal_freshness.rs  wal_wellformed.rs
```

Twelve `vibe check` rules and not one inspects a commit. Combined with F-230's
finding that **`.git/hooks/` is empty, `core.hooksPath` is unset and no CI file
exists in our surface**, there is no layer at which a commit-message check could
be running: no spec for one, no engine, no CLI driver, no deployment.

**Why nothing moved.** Both anchors are bullets under `## Header` that state the
rule and its reason — *«**Imperative mood.** "add", not "added" … The subject
completes the sentence "If applied, this commit will …"»* and *«**Lowercase.**
Including the first word after the `type(scope):` prefix. The typed prefix is the
visual anchor…»*. Neither asserts that anything checks it; neither describes a
built thing. What the verdict measured is **the consumer breaking a rule the
package correctly states**, which is
[§3.6](../PHASE-D-BATCH-PLAN.md#which-side) route (b). A package whose rule is
broken 213 times in 400 does not get quieter about the rule.

**Verdict recommendation, per anchor:** `##HEADER-IMPERATIVE-MOOD` → **drift
stands, route (b)** — the breach is real and larger than recorded (213/400,
53.2 %), and the sentence is a norm the host owes. `##HEADER-LOWERCASE` →
**drift stands, route (b)** — the breach is real and smaller than recorded
(9/400 genuine, 2.3 %; 62 naive, 48+ of them identifiers), and the sentence is a
norm the host owes. **Both figures in the registry's reasons should be replaced
by the re-measured ones, with the HEAD and window named.**

**New obligations noticed:** `##ALL-COMMITS-FOLLOW-THE-CONVENTIONAL-COMMITS-SPECIFICATION`
(`conventional-commits.xml:5`, `@impl/done`, **not in my six**) is the one
sentence in this file that *is* a factual claim rather than a rule — and it
measures **true** on the header shape: 400 of 400 in this window carry a
`type(scope):` prefix. Recorded so a later wave does not demote it by
association with its neighbours.

---

## F-303 — the package never claims a checker, so there is no absent checker; the word that *is* false is «mechanically», and it is a different defect

**Outcome:** RE-JUDGE: confirmed (no absence), **with a re-type
recommendation** — see the verdict line
**Anchors:** 0 of 1 moved. Not touched: `##SUM-THE-NO-ALSO-TEST` (defined at
`ATOMIC-COMMITS-PROTOCOL.xml:218`; the two sibling anchors in this obligation's
`evidence_refs`, `##THE-TEST-IS-MECHANICAL-THE-WORD-ALSO`:95 and
`##THE-WORD-ALSO-IS-STILL-THE-TEST`:138, are evidence, not anchors of the
obligation).
**Files touched:** none
**Perimeter searched:** the whole subject document read in full rather than
grepped — the question «does the package claim a checker?» is answered by
reading, not by absence — plus a keyword sweep of that document for `checker` ·
`linter` · `hook` · `CI` · `script` · `automat*` · `tool will` · `command`;
plus F-234's commit-checker sweep over the standing perimeter, not re-run; plus
this repository's own history read-only for the test's real behaviour; plus the
compiled boot lane (`vibevm/vibespecs/boot/STATIC.xml`) and the installed copy under
`vibedeps/`, which is the layer a package-scoped read cannot see.

**What the search found — the document does not claim a checker, anywhere:**

```console
$ grep -niE "checker|linter|hook|CI|script|automat|tool will|command" \
    …/git-atomic-commits/v0.1.0/spec/flows/atomic-commits/ATOMIC-COMMITS-PROTOCOL.md
  7:  set must be split, and *why* this dis[ci]pline matters …
 39:  the dis[ci]pline ship anyway.
 71:  ### Commit log as de[ci]sion history
116:  ##sibling-document-pointers Me[ch]anical procedure for producing the split: …
124:  introdu[ci]ng a new type, …
207:  5. Do NOT run any git [command]s after the proposal until I approve.
```

**Every hit is a substring inside another word** — `discipline`, `decision`,
`introducing` — except line 207, which forbids an agent from running git
commands. In 224 lines the protocol names no checker, no linter, no hook and no
automation. **There is therefore no absent checker to demote.** The verdict's
premise — *«a rule whose checker would have to make a judgement is a rule with
no checker»* — presupposes a checker the document never promises.

**Where the test does live, which is the layer a package-scoped read misses.**
The atomicity flow is compiled into this consumer's always-loaded boot
(`vibevm/vibespecs/boot/STATIC.xml:363`, and again at `:557` — the same duplication F-230
records), and the compiled snippet carries the *procedure*:
`vibevm/vibespecs/boot/STATIC.xml:392` — «Group changes into atomic commits — one commit per
intent, not per file» — and points at the full protocol at `:411`. The pointed-at
document is installed and on disk at
`vibedeps/flow-git-atomic-commits/0.1.0/spec/flows/atomic-commits/ATOMIC-COMMITS-PROTOCOL.md`.
The carrier of this test is a session that reads it, and that session boots here
every time.

**The §3.7 corollary fired, and it is why this row was looked at twice.** The
verdict says **in its own words** that it was reasoned by analogy: *«the same
shape the harvest recorded for the capitalisation rule»*, citing
`campaigns/packages-2026-09/harvest/world-w1-git-family.md:130`. The batch plan's
rule is that when a verdict says it was restated to match a sibling, the whole
set is re-verified — and the sibling **did not survive**: F-234 above re-measured
the capitalisation breach at 9 of 400 (2.3 %) against the recorded 42 (10.5 %).
The premise this row inherited is the premise that moved.

**What *is* false here, measured.** The summary says the test *«catches
violations mechanically»*. Applied mechanically to this repository:

```console
$ git log -400 --format='%h%x00%s%x00%b%x1e' | (classify bodies containing \balso\b)
commits parsed: 400
commit bodies containing the word "also": 73    total occurrences: 78
```

**73 of 400 — 18.3 %.** Of the 29 printed with 70 characters of context on each
side, **every one is a narrative connective**, not a bundled second idea:
`4dba52ef` «Also records the recovery rule…», `ef40a1ce` «The LOG entry also
states…», `6072033a` «The registry also carries…», `a6436a80` «There is also
exactly one tag in 2117 commits…». *(I did not hand-classify all 73; the claim
rests on the shape of that sample, and the sample is uniform.)* So the test
applied mechanically catches **prose**, and separating prose from bundling
requires exactly the judgement this same file demands at
`##USE-JUDGEMENT` (line 136) — four lines above its own restatement of the test
at line 138. **The document's mechanicality claim is contradicted by the
document's own next section.**

**Which layer has it, if any:** **spec** — the test is a rule for a human, stated
in the protocol and installed under `vibedeps/`; **host deployment / boot lane**
for the procedure that invokes it (`vibevm/vibespecs/boot/STATIC.xml:392`). **Nowhere** for
an automated checker — and nowhere is where the package puts it, deliberately.

**Why nothing moved, stated plainly.** Demoting this would append *«Specified,
not built: nothing runs this test»* to a sentence that never said anything runs
it. That clause would **invent an absence** and tell a reader the flow intended
an automation it never intended — the mirror image of the error
[§3.7](../PHASE-D-BATCH-PLAN.md#compliance-blindness) exists to prevent, and
outside [§3.3](../PHASE-D-BATCH-PLAN.md#demote)'s own scope, which is *«where the
fact promises a mechanism nothing implements»*.

**Verdict recommendation, per anchor:** `##SUM-THE-NO-ALSO-TEST` → **the
`missing-support` reading does not survive; re-type rather than close.** Two
answers, and the boss's to pick. **(a)** Re-judge **confirmed** on the absence:
nothing is missing, because nothing was claimed built, and the verdict inherited
its premise from a sibling that has since been re-measured. **(b)** Keep it open
but re-typed as a **`contradiction`** — `##SUM-THE-NO-ALSO-TEST`'s
«mechanically» against the same file's `##USE-JUDGEMENT`, with the 73/400
measurement as evidence — which is `falsifier: self` (as the registry already
records), route (a), and closes by a **prose edit on one word**, not by a
demotion. Under either answer this row does **not** belong on `build-or-demote`,
and I have made no edit that would prejudge it.

---

## F-338 — the revisit trigger is genuinely absent from every sync this repository ran, and the step that requires it was loaded in the boot lane while they ran

**Outcome:** ROUTE-B CANDIDATE (1 of 1)
**Anchors:** 0 of 1 moved. Not touched:
`##STEP-DRAFT-A-DIFF-AGAINST-THE-SPEC-SECTION` (defined at
`spec/boot/20-flow-sync-from-code.md:37`).
**Files touched:** none
**Perimeter searched:** the standing perimeter over `*.md` · `*.rs` · `*.toml` ·
`*.json` · `*.sh` · `*.ps1` for `revisit when` · `when to revisit`, **and the
whole of it rather than `spec/` alone** — the verdict's figure of 13 is a
`spec/`-scoped number, and [§3.7](../PHASE-D-BATCH-PLAN.md#compliance-blindness)
says a package-scoped perimeter reads adoption as absence. Plus this
repository's own history, read-only: every `docs(spec)` commit (181 of them),
every commit whose body names sync-from-code, and the per-file diffs of the four
candidates. Plus the **compiled boot lane** (`vibevm/vibespecs/boot/STATIC.xml`) and the
installed copy under `vibedeps/`, which is the layer that decides whether the
rule was even in the room.

**The syncs, re-measured. The verdict's two are real, and there are two more.**

```console
$ git log -1 --format='%h %ad %s' --date=short 4ea09ad0 04d7e4ae
4ea09ad0 2026-07-25 docs(spec): PROP-042 §4 gains the four verbs the code already shipped
04d7e4ae 2026-07-26 docs(spec): the token loader's first source was never in the list
```

Both bodies invoke the protocol by name — `4ea09ad0` «Sync-from-code, applied on
the owner's approval», `04d7e4ae` «Corrected against
crates/vibe-publish/src/token.rs under sync-from-code, with the owner's
approval (F-063)». **A trap worth recording:** a whole-commit grep on `4ea09ad0`
returns **13** hits for `revisit`, which looks like a falsification —

```console
$ git show 4ea09ad0 --format="" | grep -c -i revisit
13
$ git show 4ea09ad0 --format="" -- spec/ | grep -c -i revisit
0
```

— and every one of the 13 is an `"id": "…-revisit"` key inside
`campaigns/progress-2026-08/run/cache.json`, the campaign's verdict cache, which
rides along in the same commit. **The spec diff itself
(`vibevm/vibespecs/modules/vibe-cli/PROP-042-aiui-observation.xml`, +17 lines) contains no
revisit trigger at all** — it adds four `##VERB-*` bullets and stops.
`04d7e4ae`'s spec diff (`PROP-002-decentralized-registry.xml`, 11 lines) likewise
returns 0. Two further code-driven spec corrections that do not name the
protocol — `812bfecc` (*«the command list was two verbs short of the tool»*) and
`5ad0aaf2` — also return **0**. **Four for four.**

**The practice across the whole perimeter, which is where the verdict's number
needs qualifying.** A raw sweep returns **581 hits**, but the count is almost
entirely an artefact:

```console
$ grep -rniE "revisit when|when to revisit" … | (by first path component)
   454 campaigns    85 packages    28 vibedeps    13 spec    1 ROOT
$ … | (campaigns, by file)
   162 campaigns/packages-2026-09/baseline.json
    76 campaigns/packages-2026-09/tasks/evidence/ev-W3b.json
    38 campaigns/packages-2026-09/tasks/evidence/batch-W3b-2.json
     …
     5 campaigns/packages-2026-09/PHASE-D-BATCH-PLAN.md
```

The `campaigns/` bulk is **the campaign's own JSON evidence and baseline files
quoting package anchors whose ids contain the word** — machine records, not
instances of the practice. Strip them and the genuine practice outside `spec/`
is **9 lines in two documents**: `PHASE-D-BATCH-PLAN.md` × 5 (§3.1, §3.2, §3.3,
§3.4, §3.6) and `vibevm/vibespecs/terraforms/PACKAGES-ACTUALIZATION-CAMPAIGN-v0.1.xml` × 4,
both in the sibling `decision-records` form. Likewise the `packages/` 85 are
`fractality` (24), `delegation-rules` (16) and `decision-records` (5) — other
packages using the form, not this consumer's spec tree.

**So the verdict's `spec/` measurement holds, and I can sharpen it.** The 13 in
`spec/` are **12 actual triggers plus the rule itself**, the latter compiled into
this repository's own boot at `vibevm/vibespecs/boot/STATIC.xml:255`: *«| **When to revisit**
| A measurable trigger: metric + threshold + where it is observed. |»* Against
that, `**Decision**`-labelled sections number **151 in `spec/`** (264 across the
perimeter) — so the trigger is present on roughly **8 %** of the decisions that
should carry one.

And the shape test fails as recorded. Reading all twelve: `PROP-000:23` «Never,
in the scope of v1»; `PROP-000:57` two event conditions; `PROP-036:95` «if the
artifacts stop being committed»; `PROP-043:98/141/255` «the XML storage frontend
lands» / «never expected» / «the post-campaign fold»; `PROP-001:25` «when a
concrete reason arises» — which is close to the flow's own *bad* example;
`PROP-001:113` «if and when we need one of:». The **closest to compliant** is
`vibevm/vibespecs/terraforms/PACKAGES-ACTUALIZATION-CAMPAIGN-v0.1.xml:116` — *«a third of
`world`'s units land `unverifiable`»* — which has a metric and a threshold and
leaves the observation point implicit. **None of the twelve carries all three.**

**The fact that decides the routing.** The step was not merely shipped, it was
**loaded**:

```console
$ grep -n "sync-from-code" vibevm/vibespecs/boot/STATIC.xml
1221:<!-- vibe:static org.vibevm.world/sync-from-code — vibedeps/flow-sync-from-code/0.1.0/boot/20-flow-sync-from-code.md -->
$ sed -n '1248,1251p' vibevm/vibespecs/boot/STATIC.xml
2. Draft a diff against the relevant spec section. Include: new value,
   reason, and the condition under which the decision should be
   revisited.
```

This anchor's sentence is verbatim in this repository's always-loaded boot file
— **once**, not twice, unlike the git-practices block — and was therefore read
at the start of the very sessions that ran both syncs. The rule was in the room
and was not followed. *(The registry's `installed: False` on this row is a path
artefact: the snippet compiles from `boot/20-flow-sync-from-code.md`, not
`vibevm/vibespecs/boot/…`, in the install slot.)*

**And the package passes its own test**, which is the mandate's acceptance
criterion and worth recording explicitly. `SYNC-PROTOCOL.xml:143-144` carries the
flow's own worked example — *«**When to revisit:** if p99 network latency drops
below 100 s based on mon/latency-p99»* — **metric, threshold, and observation
point, all three**. The discipline holds itself to its own rule; the consumer
does not.

**Which layer has it, if any:** **spec** — the step, the required part, the
reviewer check and a compliant worked example, all in the package; **host
deployment / boot lane** — the step compiled into `vibevm/vibespecs/boot/STATIC.xml:1248` and
the trigger's shape definition at `:255`. **Nowhere** in the consumer's four
code-driven spec edits, and on 8 % of its decision sections.

**Why nothing moved.** *«Draft a diff … Include: new value, reason, and the
condition under which the decision should be revisited»* is a numbered
procedure step addressed to an agent. It claims no checker, no artefact and no
record. What the verdict measured is the **consumer skipping step 2's third
clause four times out of four while the step sat in its boot file** — textbook
[§3.6](../PHASE-D-BATCH-PLAN.md#which-side) route (b). A flow whose rule is
skipped does not get quieter about the rule; the compliance work is the host's.

**Verdict recommendation, per anchor:**
`##STEP-DRAFT-A-DIFF-AGAINST-THE-SPEC-SECTION` → **drift stands, route (b)** —
the absence is real, larger than recorded (four syncs, not two), and the sentence
is a procedure step the host owes; the package's own example satisfies its own
rule.

**New obligations noticed:** the two syncs that *did* invoke the protocol
(`4ea09ad0`, `04d7e4ae`) also both bundle the campaign's `run/` state into the
same commit, which `##NEVER-BATCH-TWO-UNRELATED-CODE-CHANGES` and
`##APPROVAL-STEP-STOPS` («a sync is its own atomic step») both speak to. Same
route-(b) shape, different anchors, outside my six — recorded, not touched.

---

## F-339 — the reviewer side of F-338: the package requires a part its consumer never hands over, and the `falsifier: self` on this row is a heuristic artefact

**Outcome:** ROUTE-B CANDIDATE (1 of 1)
**Anchors:** 0 of 1 moved. Not touched: `##PART-A-REVISIT-TRIGGER` (defined at
`spec/flows/sync-from-code/review-workflow.md:18`; `##check-trigger` at `:61`,
which the registry lists as this row's `fact`, is a **section heading anchor**
`{#check-trigger}`, not the anchor under judgement — checked against the file
rather than assumed).
**Files touched:** none
**Perimeter searched:** identical to F-338's and **not re-run** — same
measurement, same day, same commits; the two anchors are the author side and the
reviewer side of one fact. Added here: `review-workflow.xml` read in full, and
the registry row's four `evidence_refs` opened one by one, because this row is
the only one in my six that the script types `falsifier: self` while its
substance is about the consumer.

**What the search found — the measurement is F-338's, and it is unchanged.**
Four code-driven spec edits (`4ea09ad0`, `04d7e4ae`, `812bfecc`, `5ad0aaf2`),
**zero revisit triggers in any of their spec diffs**; 12 triggers in `spec/`
against 151 `**Decision**`-labelled sections; **none of the twelve carrying
metric + threshold + observation point together**, which is the shape
`review-workflow.xml:67` sets as its good example (*«When p99 network latency
drops below 100 s, per mon/latency-p99»*) and `vibevm/vibespecs/boot/STATIC.xml:255` compiles
into this repository's boot as the field's requirement.

So the reviewer-side statement is falsified in the only way it can be: the third
of three parts is never handed over, so the checklist item at
`review-workflow.xml:61` has never had anything to check, and
`##IF-ANY-OF-THE-THREE-IS-MISSING-THE-PROPOSAL-IS-INCOMPLETE` (`:21`) and
`##DO-NOT-APPROVE-AN-INCOMPLETE-SYNC` (`:26`) were true of all four approvals.

**Why `falsifier: self` is wrong here, and it matters for the routing.** The
row's evidence refs are all inside the package —
`SYNC-PROTOCOL.xml:143`, `:144`, `:150`, and `review-workflow.xml:61` — so the
span heuristic concludes the package falsifies itself. Opened one by one, they
do the opposite:

- `SYNC-PROTOCOL.xml:143-144` is the flow's **own worked example** —
  *«**When to revisit:** if p99 network latency drops below 100 s based on
  mon/latency-p99»* — carrying metric, threshold **and** observation point. It
  satisfies the rule it illustrates.
- `SYNC-PROTOCOL.xml:150` is `##PART-THE-REVISIT-TRIGGER`, the author-side twin
  of this anchor, stating the same requirement and adding the reason: *«A
  decision without a revisit trigger becomes a sacred cow.»*
- `review-workflow.xml:61` is the reviewer's check on that part.

**Three statements of one rule plus a compliant example is internal consistency,
not self-contradiction.** Nothing in the package falsifies the package. What
falsifies the anchor sits entirely outside it — in four host commits and 139
host decision sections — so this row is `host`, not `self`, and the routing is
[§3.6](../PHASE-D-BATCH-PLAN.md#which-side) route (b) exactly as F-338's. The
`falsifier` field is mechanical and the batch plan says so at
[§3.6](../PHASE-D-BATCH-PLAN.md#which-side); this is one of the cases where the
mechanism lands wrong, and it is worth recording because a batch cut on
`falsifier == "self"` is precisely what §6.1's first lesson was bought with.

**Which layer has it, if any:** **spec** — the required part, the reviewer
check, the good and bad examples, and a compliant worked instance, all inside
the package; **host deployment / boot lane** — the sibling `decision-records`
field table at `vibevm/vibespecs/boot/STATIC.xml:255` defines the shape and is loaded every
session. **Nowhere** in what this consumer's agents actually handed its reviewer.

**Why nothing moved.** *«**A revisit trigger** — the condition under which the
decision should be re-examined»* is item 3 of a required-parts list in a human
review checklist. It claims no checker, no artefact and no record; it prescribes
what a proposal must contain. The package is right, its example is right, its
rule is loaded in the consumer's boot, and the consumer skipped the part four
times out of four. **Softening a review checklist because the reviews it governs
were incomplete is the *профанация* the mandate exists to prevent.**

**Verdict recommendation, per anchor:** `##PART-A-REVISIT-TRIGGER` → **drift
stands, route (b)** — the absence is real and is entirely the consumer's; the
row should additionally be re-read as `falsifier: host`, since all four of its
in-package evidence refs are the package keeping its own rule.

---

## Batch summary

| id | anchors | outcome | package moved? |
|---|---:|---|---|
| F-230 | 2 | ROUTE-B CANDIDATE (2) | no |
| F-306 | 1 | PARTIAL → DEMOTED (1) | **yes** — `README.md` |
| F-234 | 2 | ROUTE-B CANDIDATE (2), both figures re-measured | no |
| F-303 | 1 | RE-JUDGE: confirmed (no absence); re-type recommended | no |
| F-338 | 1 | ROUTE-B CANDIDATE (1) | no |
| F-339 | 1 | ROUTE-B CANDIDATE (1) | no |

**8 verdicts examined, 1 anchor moved, 7 recommended out of the package.** That
ratio matches the phase's central finding — 179 anchors examined and 25 moved
across waves 2–4 — and this batch sits at the low end of it because all four
subjects are **normative flows**: three git-practice rules and a review
protocol, whose sentences are prescriptions rather than build claims.

**Claimed absences that turned out false: one of six obligations, and not in the
usual direction.** No host artefact disproved any of these six the way
`discipline/` and `terraform/` disproved seventeen in wave 5 — the checkers
really are absent, at every layer, and I looked for them by name as well as by
string. The one that did not survive is **F-303**, and it failed on the *other*
axis: nothing was missing because nothing was ever claimed built, and the verdict
says in its own words that it inherited its premise from the capitalisation
verdict — whose number **also** did not survive re-measurement here (42/400
recorded, 9/400 measured). **The §3.7 corollary fired twice in one batch: once on
the row that cited its sibling, and once on the sibling itself.**

**What the host is owed, if these route out:** a checker or a marked exception
for the attribution policy (F-230), the single-place law against two compiled
copies and six restatements plus a dangling `PROP-000 §12.1` (F-230), the
imperative-mood and lowercase breaches at their **re-measured** sizes (F-234),
and the revisit trigger on both the author and reviewer sides of every future
sync (F-338, F-339). Every figure in this record names the HEAD it was taken at
— `e118b76f`, 2026-07-29 — because the 400-commit window it rests on spans four
days and is 78.5 % this campaign's own commits.
