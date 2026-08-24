# the git family — §3.1 source 2, the host's observed conformance

_Captured 2026-07-28 over the last 400 commits of this repository._

§3.1's second source is «the host's observed conformance» — the consuming project
either behaves as the flow promises or it does not. For the five `git-*` flows that
source is **this repository's own history**, and it is the cheapest and most
independent evidence anywhere in `world`: no document is asked whether another
document is right.

```console
$ git log -400 --format=%s | grep -cE '^(feat|fix|chore|docs|build|test|refactor|perf|style|ci|revert)(\([a-z0-9._/-]+\))?!?: '
394

$ git log -400 --format=%s | awk 'length>72' | wc -l
82

$ git log -400 --format=%s | awk '{print length}' | sort -rn | head -1
89

$ git log -400 --format=%B | grep -ci 'co-authored-by'
0

$ git log -400 --format='%an' | sort -u
Oleg Chirukhin

$ git log -400 --format='%ad' --date=short $(: over-72 subjects by day) # see script in the LOG
     28 2026-07-25
     27 2026-07-26
     14 2026-07-24
      6 2026-07-28
      2 2026-07-27
      2 2026-07-23
      2 2026-07-22
      1 2026-07-21
```

**What conforms.** 394 of 400 subjects carry the `type(scope):` header the flow
requires; 399 of 400 carry a body, which is where the flow puts the *why*. Zero
`Co-Authored-By` trailers and one author across four hundred commits — the
attribution posture holds on the surface it is written to protect.

**What does not.** The flow sets a hard limit of 72 characters on the subject and
**82 of 400 exceed it — 20.5 %, the longest at 89.** The violation is spread across
the campaign's working days rather than concentrated in a slip, and **six of them
were written by this phase today**.

**Four commit bodies name a model, and none of them attributes authorship** — two
use `Anthropic` as the name of a colour theme, two describe model tiers as
configuration data. That is F-087, open on the owner, now measured rather than
reported: the policy's «never mention model names in commit messages» is broken
four times in four hundred, and its «never state or imply machine authorship» is
not broken at all.

**Scope:** §3.1 source 2 for the five `git-*` flows of `vibevm/vibepacks/org.vibevm.world/`.
