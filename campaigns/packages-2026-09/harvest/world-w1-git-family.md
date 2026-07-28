# W1 — the git family, all three §3.1 sources captured

_Captured 2026-07-28 at the W1 opening, over `packages/org.vibevm.world/git-*` and
the last 400 commits of this repository. Every number below is the output of the
command printed above it._

The five `git-*` flows are the cheapest batch in `world` for one reason: **§3.1's
source 2 — «the host's observed conformance» — is this repository's own history**,
so no document is asked whether another document is right. Sources 1 and 3 are the
two mechanised joins the phase already built.

## Source 1 — the package agreeing with itself {#source-1}

```console
$ python campaigns/packages-2026-09/tasks/source1-join.py \
    packages/org.vibevm.world/git-atomic-commits \
    packages/org.vibevm.world/git-attribution-policy \
    packages/org.vibevm.world/git-autonomy \
    packages/org.vibevm.world/git-conventional-commits \
    packages/org.vibevm.world/git-practices
source-1 join over 17 file(s) under packages/org.vibevm.world/git-atomic-commits, …
  relative .md citations resolved: 11
  broken: 0
```

**Eleven relative citations, none broken.** Every protocol document a boot snippet
points at exists and carries the anchor it is cited by. The mechanical half of
source 1 is clean for this batch; the judgement half — whether the target *says
what the snippet says it says* — is per-anchor and stays the reviewer's.

## Source 3 — the installed reality, and where it diverges {#source-3}

```console
$ python campaigns/packages-2026-09/tasks/source23-boot-join.py
boot-lane join over 31 contribution(s) in spec/boot/STATIC.md
  org.vibevm.world/git-atomic-commits  [INSTALLED NO-SOURCE]
    installed: vibedeps/flow-git-atomic-commits/0.1.0/boot/30-flow-atomic-commits.md
  org.vibevm.world/git-autonomy  [INSTALLED NO-SOURCE]
    installed: vibedeps/flow-git-autonomy/0.1.0/boot/32-flow-autonomy.md
  org.vibevm.world/git-conventional-commits  [INSTALLED NO-SOURCE]
    installed: vibedeps/flow-git-conventional-commits/0.1.0/boot/31-flow-conventional-commits.md
  org.vibevm.world/git-practices  [INSTALLED NO-SOURCE]
    installed: vibedeps/flow-git-practices/0.1.0/spec/boot/STATIC.md
  … the same three again, through the git-practices umbrella
```

Two facts, both load-bearing for this batch's verdicts.

**INSTALLED holds; NO-SOURCE is a path, not an absence.** Each snippet is present
under `vibedeps/` exactly where the host's provenance marker names it — so a
consumer does receive the artifact, which is what source 3 is for. What does not
resolve is the *package-side* path: the installed copies were written from
`boot/…`, and the packages now ship the same snippets at `spec/boot/…` (DRIFT-039
moved them). The join cannot pair them by path, so it reports NO-SOURCE and
declines to compare words.

**Each of the four git flows appears TWICE in the host's boot lane** — once
directly and once compiled in through the `git-practices` umbrella, which ships its
own `spec/boot/STATIC.md` containing the other three. That is **F-078**,
reproduced mechanically here rather than read: `atomic-commits`,
`conventional-commits` and `attribution-policy`/`autonomy` are each read twice at
every session boot.

## Source 2 — the host's observed conformance {#source-2}

The consuming project is this repository, and its history is the observation.

### Header grammar

```console
$ git log -400 --format=%s | grep -cE '^[a-z]+\([a-z0-9._/-]+\)!?: '
397
$ git log -400 --format=%s | grep -cE '^[a-z]+!?: '
0
$ git log -400 --format=%s | sed -nE 's/^([a-z]+)(\([^)]*\))?!?: .*/\1/p' | sort | uniq -c | sort -rn
    273 docs · 45 chore · 42 feat · 21 fix · 7 test · 3 refactor · 3 perf
      2 tools · 2 style · 1 spec · 1 build
```

**397 of 400 carry a `type(scope):` header and not one omits the scope.** Three of
the remaining three use a comma-separated multi-scope — `docs(wal,continue)`,
`feat(core,registry,install,tree)`, `refactor(resolver,cli)` — which the grammar's
`[a-z0-9._/-]+` scope does not admit and which the «narrowest accurate subsystem»
rule reads as a commit that should have been several.

**Three commits carry a type outside the allowed table**: `tools` twice
(`e538edb5`, `b64e7085`) and `spec` once (`302b37c1`). The table is
feat · fix · chore · docs · build · test · refactor · perf · style · ci · revert.

### Subject length

```console
$ git log -400 --format=%s | awk 'length>72' | wc -l
82
$ git log -400 --format=%s | awk 'length>60' | wc -l
297
$ git log -400 --format=%s | awk '{print length}' | sort -rn | head -1
89
```

**82 of 400 break the hard limit of 72 — 20.5 %, longest at 89.** And **297 of 400
break the soft limit of 60 — 74.3 %**, which is the number nobody had measured: the
flow states 60 as the limit and 72 as the hard ceiling, and this repository treats
60 as advisory to the point of ignoring it in three subjects out of four. That is
**F-123**, now with its second half.

### Capitalisation after the prefix

```console
$ git log -400 --format=%s | sed -nE 's/^[a-z]+(\([^)]*\))?!?: ([A-Za-z-]+).*/\2/p' \
    | grep -E '^[A-Z][a-z]{2,}$' | sort | uniq -c
     42 Phase
```

The flow says «never capitalise the first word after the `type(scope):` prefix».
**It is broken 42 times in 400 — 10.5 % — and every one of them is the same word,
`Phase`.** A naive count of «capital letter after the prefix» returns 111 and is
wrong: the other 69 are identifiers — `C4`, `C7`, `F-122`, `R-001` — which are
names, not capitalised words. *The check that would enforce this rule has to know
the difference, which is why the rule has no checker.*

### Body — where the *why* lives

```console
$ git log -400 --format='%H%x1f%b%x1e'   # counted by record, not by line
records: 400  empty-body: 1
```

**399 of 400 carry a body.** The one that does not is `fcc1cff9`.

### Atomicity, as far as history can show it

```console
$ git log -400 --format='%x1e%H' --name-only   # files per commit
n=400  mean=5.29  median=3  max=130
commits touching exactly 1 file: 102        commits touching >10 files: 38
```

A file count is not an idea count, and the atomic rule is explicitly *one idea, not
one file* — so this measures the shape of the history, not conformance. What it
shows: a median of three files, a quarter of commits touching exactly one, and a
long tail of 38 commits over ten files, the largest at 130.

```console
$ git log -400 --merges --format=%h | wc -l
4
$ git log -400 --format='%at %ct' | awk '$1!=$2' | wc -l
3
```

Four merge commits and three commits whose author and committer dates differ. The
autonomy flow's red lines — force-push, history rewrite — leave no positive trace
in a repository's own log, so **history can corroborate the attribution and
conventional-commits flows and cannot, on its own, confirm or falsify the autonomy
flow's red-line discipline.** A verdict resting on «no force-push is visible» would
be an absence asserted rather than checked.

### Attribution

```console
$ git log -400 --format=%B | grep -ci 'co-authored-by'
1
$ git log -400 --grep='[Cc]o-[Aa]uthored-[Bb]y' --format='%h %s'
89c90aed docs(campaign): F-123 — we break a rule we ship, at a fifth of commits
$ git log -400 --format='%an' | sort -u
Oleg Chirukhin
```

**Zero `Co-Authored-By` trailers.** The single grep hit is a commit *body quoting
the finding that there are none* — the measurement's own prose, not a trailer. One
author across four hundred commits. The attribution posture holds on the surface it
is written to protect; its measured breach is elsewhere and is **F-087** — four
commit bodies name a model, two as a colour-theme name and two as configuration
data, and **none states or implies machine authorship**.

**Scope:** §3.1 sources 1, 2 and 3 for the five `git-*` flows of
`packages/org.vibevm.world/`, batch W1.
