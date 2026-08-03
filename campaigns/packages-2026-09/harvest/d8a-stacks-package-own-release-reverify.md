# D8a — re-verifying the package-own `release` verdicts across the three language stacks

_Phase D, wave 8, batch D8a. Six obligations over 19 drift verdicts, all in the
`org.vibevm.ai-native` language-stack packages, all closing through
[`release`](../PHASE-D-BATCH-PLAN.md#routes) — **the owner before publication**.
The `release` route has never been re-verified; waves 5–7 covered the other
three. A re-verdict that edits nothing produces no spec diff, which is the only
basis on which this batch may run without the owner in the room, so **no package
file was edited, no campaign state was written, no verdict JSON was touched, and
`merge-verdicts.py` was not run.** This file is evidence and a recommendation;
the verdict itself is the boss's._

**Measured at** `HEAD = f2b11b0a` (`fix(campaign): the registry snapshot on disk
was two waves stale, and it read as open work`, 2026-07-31) —
`git rev-parse --short HEAD`. Every count below names the command that produced
it, per wave 6's lesson that a recorded figure decays; any figure over `git log`
names this HEAD.

**HEAD advanced mid-batch, and the check is recorded rather than assumed** —
[§6.1 `##THE-CAMPAIGN-IS-INSIDE-ITS-OWN-CORPUS`](../PHASE-D-BATCH-PLAN.md#delegation-lessons)
says this campaign moves its own measurements inside a session, so it was
re-checked at the end. HEAD is now `45cd30b0` (the sibling **D8b** audience
batch, plus three unrelated host commits).
`git diff --name-only f2b11b0a 45cd30b0` touches
`campaigns/…/harvest/d8b-…md`, `run/cache.json`, four host crates
(`vibe-check`, `vibe-workspace`) and one `org.vibevm.world` protocol document —
**no file under `packages/org.vibevm.ai-native/**`, no stack crate,
no `crates/vibe-cli/src/cli.rs`, no `spec/common/PROP-031`.** Every quotation,
line number and count below therefore still holds at `45cd30b0`, and the eight
neighbouring cache verdicts cited in this file were re-read there unchanged.
D8b covers F-187 / F-189 / F-190 / F-213 — **no overlap with this batch's six.**

**Route check, run first per
[`##ROUTE-BEFORE-FALSIFIER`](../PHASE-D-BATCH-PLAN.md#delegation-lessons).**
All six report `closure_route: release` in
[`run/state/obligations.json`](../run/state/obligations.json), read as an
instrument and not as evidence: **6 obligations · 19 anchors** — F-153 (6),
F-115 (3), F-186 (3), F-188 (3), F-211 (2), F-212 (2). Falsifier `self` on
F-153 / F-115 / F-211 / F-212, `mixed` on F-186 / F-188. All six were selected
because their reasons claim to rest on **package-own** evidence; confirming that
claim per obligation is part of the job below.

**The standing perimeter.** Unless an entry narrows it, every search was run from
the repository root over: `packages/**` **including
`packages/org.vibevm.fractality/**`** (a second complete project that adopted
this discipline — [§3.7's wave-6 extension](../PHASE-D-BATCH-PLAN.md#compliance-blindness)),
`vibedeps/**`, `crates/**`, `xtask/**`, `tools/**`, `spec/**`, `discipline/**`,
`terraform/**`, `research/**` (including `research/rust-demo`, `research/ts-demo`,
`research/go-demo` with their own `vibedeps/`, `conform.toml`, `specmap.json`),
`campaigns/**` minus `campaigns/*/run/**`, `fixtures/**`, `schemas/**`, `docs/**`,
`manual-tests/**` and the repository root's own `*.md` / `*.toml` / `*.json` /
`*.sh` / `*.ps1`. **Excluded:** `legacy-spec/**` (owner ruling 2026-07-31 — not
evidence of practice in either direction), `**/target/**`, `.git/**`,
`**/node_modules/**`, `campaigns/*/run/**`. `refs/**` is third-party and is
searched but reported separately.

Before every absence search, §3.7's general form was applied — **a perimeter is
defined by where the mechanism can live, not by a directory name** — by writing
down what the mechanism would look like at each of the four layers: **SPEC**
(`core-ai-native`'s documents), **ENGINE** (that package's five library crates,
vendored under `crates/vendor/` in six siblings), **DRIVER** (each stack's CLI),
**DEPLOYMENT** (a consuming project). The searches below hunt the *thing*, not
the string the verdict used.

**§3.8 — the audience rule, and it governs this batch.** The `ai-native-lang`
packages are built for **external** consumers.
[`go-ai-native-lang` and `typescript-ai-native-lang` are not adopted by this host
and must not be](../PHASE-D-RELEASE-QUEUE.md#stacks-audience); they are checkable
only by their own artefacts and their **tests** (their `crates/`, `spec/`, cards,
`vibe.toml`, `tools/go-extract` and `tools/ts-extract` fixtures, in-crate test
modules). **Host evidence — what this repo installs, dispatches, runs — is VOID
against a Go or TypeScript sentence.** `rust-ai-native-lang` is the exception:
part of VibeVM is written in AI-Native Rust, so host evidence counts for the Rust
stack, and **Rust reasoning is never carried across to Go or TypeScript**. Where
an entry below uses a host observable, it says so and says which stack it is
allowed to bear on.

**The parallel-corpus rule this batch is cut by.** These stacks are one fact
projected per language, so every fact appearing in more than one stack is judged
in **all** its copies and reported per stack; a verdict that says it was restated
for family consistency triggers re-verification of the whole family, never the
row ([§3.7's corollary](../PHASE-D-BATCH-PLAN.md#compliance-blindness)).

---

## F-153 — the boot snippets' bare `<lang>/…` and `cards/…` resolve in no lane, in any of the three stacks

**Outcome:** SURVIVES — **6 of 6 anchors**, in every stack, on package-own
evidence alone. This is the batch's cleanest verdict and the one whose
characterisation in the queue also holds.
**Anchors:** 6 of 6 → all **STANDS**.

**Perimeter searched:** the standing perimeter for the *thing* rather than the
string — every directory named `cards` anywhere in the tree
(`find . -type d -name cards`, 30 hits), every `GUIDE-AI-NATIVE-*.md`
(`find packages/org.vibevm.ai-native -name "GUIDE-AI-NATIVE-*.md"`), plus the
**four boot lanes** a reader can stand in (the host, `research/rust-demo`,
`research/ts-demo`, the `fractality` specspace) and the three packages' own
`vibe.toml` manifests. §3.8 note: the decisive evidence is **entirely
package-own** — where each package puts its own files, and how its own manifest
addresses them. The lane evidence below is corroboration and is flagged as such;
none of it is load-bearing, so nothing here rests on a consumer the Go or
TypeScript stack does not have.

### The current text at HEAD, all six anchors

```
$ grep -n "GUIDE-AI-NATIVE\|cards/INDEX.md" \
    packages/org.vibevm.ai-native/{go-ai-native-lang/v0.1.0,rust-ai-native-lang/v0.7.0,typescript-ai-native-lang/v0.6.0}/spec/boot/20-stack-*.md
```

- go `20-stack-go-ai-native-lang.md:5-6` —
  «##GO-CODE-FOLLOWS-THE-GO-GUIDE Go code in this project follows the AI-Native Go guide
  (`go/GUIDE-AI-NATIVE-GO.md` **in this package**). @impl/done»
- go `:12-13` — «##CARD-REGISTRY-FOR-GO Card registry for Go: `cards/INDEX.md` **in this package** (trigger → card; …). @impl/done»
- rust `20-stack-rust-ai-native-lang.md:5-6` — «##RUST-CODE-FOLLOWS-THE-RUST-GUIDE … (`rust/GUIDE-AI-NATIVE-RUST.md` **in this package**). @impl/done»
- rust `:12-13` — «##CARD-REGISTRY-FOR-RUST Card registry for Rust: `cards/INDEX.md` **in this package** …»
- typescript `20-stack-typescript-ai-native-lang.md:5-7` — «##TYPESCRIPT-CODE-FOLLOWS-THE-TYPESCRIPT-GUIDE … (`typescript/GUIDE-AI-NATIVE-TYPESCRIPT.md` **in this package**). @impl/done»
- typescript `:13-14` — «##CARD-REGISTRY-FOR-TYPESCRIPT Card registry for TypeScript: `cards/INDEX.md` **in this package** …»

All six are unchanged from the text the verdict quotes. **The original reason is
accurate as written** and needs no restatement.

### The verdict's own measurement, re-run

The verdict states the targets «live under `spec/` in the package». Both halves
tested by existence, in the package lane and in the installed lane:

```
$ find packages/org.vibevm.ai-native -name "GUIDE-AI-NATIVE-*.md" -not -path "*/target/*"
packages/org.vibevm.ai-native/go-ai-native-lang/v0.1.0/spec/go/GUIDE-AI-NATIVE-GO.md
packages/org.vibevm.ai-native/rust-ai-native-lang/v0.7.0/spec/rust/GUIDE-AI-NATIVE-RUST.md
packages/org.vibevm.ai-native/typescript-ai-native-lang/v0.6.0/spec/typescript/GUIDE-AI-NATIVE-TYPESCRIPT.md

$ find packages/org.vibevm.ai-native -name "INDEX.md" -path "*cards*" -not -path "*/target/*"
packages/org.vibevm.ai-native/go-ai-native-lang/v0.1.0/spec/cards/INDEX.md
packages/org.vibevm.ai-native/rust-ai-native-lang/v0.7.0/spec/cards/INDEX.md
packages/org.vibevm.ai-native/typescript-ai-native-lang/v0.6.0/spec/cards/INDEX.md
```

The six bare addresses, tested as literal paths from each package's root:

```
MISSING packages/…/rust-ai-native-lang/v0.7.0/rust/GUIDE-AI-NATIVE-RUST.md
MISSING packages/…/rust-ai-native-lang/v0.7.0/cards/INDEX.md
MISSING packages/…/go-ai-native-lang/v0.1.0/go/GUIDE-AI-NATIVE-GO.md
MISSING packages/…/go-ai-native-lang/v0.1.0/cards/INDEX.md
MISSING packages/…/typescript-ai-native-lang/v0.6.0/typescript/GUIDE-AI-NATIVE-TYPESCRIPT.md
MISSING packages/…/typescript-ai-native-lang/v0.6.0/cards/INDEX.md
```

Each package root holds `Cargo.lock Cargo.toml LICENSE.md README.md crates spec
specmap.toml vibe.toml` (+`tools` for go and typescript, `target` for all three)
— **no `rust/`, no `go/`, no `typescript/`, no `cards/`.**

### The thing, not the string — every `cards/` in the tree is under `spec/`

```
$ find . -type d -name cards -not -path "./.git/*" -not -path "*/target/*" \
     -not -path "*/node_modules/*" -not -path "./legacy-spec/*"
```

**30 hits, and every single one ends `…/spec/cards`** — the three packages, the
installed slots (`vibedeps/stack-rust-ai-native-lang/0.7.0/spec/cards`), thirteen
superseded `.vibe/cache/` copies, the `fractality` specspace's own slot
(`packages/org.vibevm.fractality/fractality/v0.1.0/vibedeps/stack-rust-ai-native-lang/0.7.0/spec/cards`),
the `delegation-rules` package's slot, and the demo consumers'
(`research/rust-demo/vibedeps/…/spec/cards`, `research/ts-demo/.vibe/cache/…/spec/cards`).
**Zero `cards/` exists at any package root, in any lane, at any version.** There
is no lane in which the bare form resolves.

### From where the reader actually stands — both lanes, and the queue's characterisation

The verdict's rule is `r-path-does-not-resolve`, so *where the reader stands*
decides it. A boot snippet is not read in place: a consumer's `spec/boot/INDEX.md`
names it by full slot path, so the reader's cwd is the **consuming project root**
and the file being read is inside the slot.

```
$ grep -n "ai-native-lang" spec/boot/INDEX.md research/rust-demo/spec/boot/INDEX.md \
      research/ts-demo/spec/boot/INDEX.md packages/org.vibevm.fractality/fractality/v0.1.0/spec/boot/INDEX.md
spec/boot/INDEX.md:22:  path = "vibedeps/stack-rust-ai-native-lang/0.7.0/spec/boot/20-stack-rust-ai-native-lang.md"
spec/boot/INDEX.md:26:  path = "vibedeps/stack-typescript-ai-native-lang/0.6.0/spec/boot/20-stack-typescript-ai-native-lang.md"
research/rust-demo/…/INDEX.md:17:  path = "vibedeps/stack-rust-ai-native-lang/0.7.0/spec/boot/20-stack-rust-ai-native-lang.md"
research/ts-demo/…/INDEX.md:17:  path = "vibedeps/stack-typescript-ai-native-lang/0.6.0/spec/boot/20-stack-typescript-ai-native-lang.md"
fractality/…/INDEX.md:21:  path = "vibedeps/stack-rust-ai-native-lang/0.7.0/spec/boot/20-stack-rust-ai-native-lang.md"
```

(`research/go-demo` has no `spec/boot/INDEX.md` at all, which is exactly §3.8's
picture of the Go stack and is recorded, not used.) The snippet body is **not**
inlined into `spec/boot/STATIC.md` —
`grep -rn "GUIDE-AI-NATIVE\|cards/INDEX.md" spec/boot/` returns **no output** —
so the reader really is reading the file in the slot. From that slot root:

```
MISSING vibedeps/stack-rust-ai-native-lang/0.7.0/rust/GUIDE-AI-NATIVE-RUST.md
MISSING vibedeps/stack-rust-ai-native-lang/0.7.0/cards/INDEX.md
EXISTS  vibedeps/stack-rust-ai-native-lang/0.7.0/spec/rust/GUIDE-AI-NATIVE-RUST.md
EXISTS  vibedeps/stack-rust-ai-native-lang/0.7.0/spec/cards/INDEX.md
MISSING vibedeps/stack-typescript-ai-native-lang/0.6.0/typescript/GUIDE-AI-NATIVE-TYPESCRIPT.md
EXISTS  vibedeps/stack-typescript-ai-native-lang/0.6.0/spec/typescript/GUIDE-AI-NATIVE-TYPESCRIPT.md
```

**Does «in this package» rescue the path?** No — it makes the failure sharper.
The phrase fixes the origin at the package root, and the package root is exactly
where the bare path misses. It also fixes which convention applies, and **each
package's own machine-read manifest states that convention against itself**:

```
$ grep -n "boot_snippet" -A2 packages/…/typescript-ai-native-lang/v0.6.0/vibe.toml
18:[boot_snippet]
19:source = "spec/boot/20-stack-typescript-ai-native-lang.md"
$ grep -n "^path" packages/…/typescript-ai-native-lang/v0.6.0/vibe.toml
51:path = "spec/skills/typescript-ai-native-sweep"
56:path = "spec/skills/typescript-ai-native-terraform"
```

Same in the Go and Rust manifests (`source = "spec/boot/20-stack-go-…"`,
`path = "spec/skills/go-ai-native-sweep"`). **The package addresses its own files
package-root-relative *including* `spec/`; the snippet inside it does not.** That
is one document contradicting its own manifest, on package-own evidence, with no
consumer in the room — precisely the class
[§B.1 says survives the audience ruling](../PHASE-D-RELEASE-QUEUE.md#stacks-audience).

**The queue's characterisation — «needs no tag and no decision, only the correct
intra-package path» — is confirmed, and the phase has already ruled it twice.**
The reason names two earlier members of this family, and **both were repaired by
D4 and the repaired text is live at HEAD**:

```
$ grep -n "MAP-RUST-GUIDE\|READ-STACK-GUIDE" --include=*.md -r packages/ | grep -v "/target/"
packages/…/core-ai-native/v0.8.0/README.md:31:4. ##READ-STACK-GUIDE The active language stack's GUIDE (e.g. `spec/rust/GUIDE-AI-NATIVE-RUST.md` in the Rust stack). @impl/done
packages/…/core-ai-native/v0.8.0/spec/00-MANIFESTO.md:172:- ##MAP-RUST-GUIDE `spec/rust/GUIDE-AI-NATIVE-RUST.md` in `stack:org.vibevm.ai-native/rust-ai-native-lang` — … @impl/done
```

Both took a plain `spec/`-prefixed path, **no `@spec://` tag**, and both kept
`@impl/done` — the content was never in question, only the address, which is what
`relocation` means. So the third instance takes the same shape: six one-segment
prefixes, no tag, no ruling beyond publication. (Publication *is* required even
though the group-A ruling does not apply: the reader reads the **slot** copy, and
`vibedeps/stack-rust-ai-native-lang/0.7.0/spec/boot/20-stack-rust-ai-native-lang.md:4,8`
still carries `rust/GUIDE-AI-NATIVE-RUST.md` and `cards/INDEX.md` at HEAD. The
`release` route is correct.)

**Per stack:** go → STANDS ×2 · rust → STANDS ×2 · typescript → STANDS ×2. This
is the rare family where the parallel corpus propagated the **same** defect
faithfully into all three copies, so a family-wide edit is right here — and it is
right because the measurement above establishes it in each tree separately, not
because the sentences match.

**Proposed correction (NOT APPLIED)** — six one-word-group edits, mirroring the
D4 precedent exactly:

| file | line | `rust/…` → | 
|---|---:|---|
| go `spec/boot/20-stack-go-ai-native-lang.md` | 6 | `` `spec/go/GUIDE-AI-NATIVE-GO.md` `` |
| go same | 12 | `` `spec/cards/INDEX.md` `` |
| rust `spec/boot/20-stack-rust-ai-native-lang.md` | 6 | `` `spec/rust/GUIDE-AI-NATIVE-RUST.md` `` |
| rust same | 12 | `` `spec/cards/INDEX.md` `` |
| ts `spec/boot/20-stack-typescript-ai-native-lang.md` | 7 | `` `spec/typescript/GUIDE-AI-NATIVE-TYPESCRIPT.md` `` |
| ts same | 13 | `` `spec/cards/INDEX.md` `` |

Nothing else in the six sentences moves; every marker stays `@impl/done`.

**Two family siblings carry the identical defect and carry NO verdict — recorded
for the boss, not repaired here.** A query over
[`run/state/obligations.json`](../run/state/obligations.json) for each anchor
returns nothing:

1. **`##STACK-SHIPS-ITS-OWN-CARDS-PROJECTION`, in all three snippets** (go `:15-17`,
   rust `:15-17`, ts `:16-19`) — «This stack ships its own `cards/` projection».
   Bare `cards/`, same package, two lines below an anchor this obligation does
   convict. Already flagged once, in
   `harvest/d1-go-ai-native-lang-repairs.md:942`, and still unrecorded.
2. **`core-ai-native`'s own boot snippet** —
   `packages/…/core-ai-native/v0.8.0/spec/boot/10-flow-core-ai-native.md:9-18,38`
   says «The language-neutral corpus lives **in this package**: … (`00-MANIFESTO.md`,
   `01-PATTERN-CARD-FORMAT.md`, `02-EXECUTABLE-SCAFFOLDS.md`) … the mechanism specs
   under `mechanisms/` … and `appendix/`» and «Card registry: the active language
   stack's `cards/INDEX.md`». Every one of those lives under `spec/`
   (`ls packages/…/core-ai-native/v0.8.0/spec/` → `00-MANIFESTO.md …
   appendix boot legacy-projections mechanisms`). **Zero of that file's anchors
   appear in any obligation.** If F-153's six are repaired and these are not, the
   fix ships into a boot lane that still carries the same broken form one entry
   above it, in the package the three stacks depend on.

**Recommendation per anchor:** all six → **STANDS**. Reason accurate as written;
correction prepared; publication is the only owner gate.

---

## F-115 — one missing README convicted three sentences, and two of them point at a file that exists

**Outcome:** MIXED — **1 STANDS · 2 FALLS.** The defect is real and belongs to
**one** anchor. This is
[§6.1 `##A-REAL-DEFECT-CONVICTING-THE-WRONG-SENTENCE`](../PHASE-D-BATCH-PLAN.md#delegation-lessons)
in its purest form: **each falling anchor is falsified by its own verdict's
second evidence line**, at zero search cost.
**Anchors:** 3 of 3 — go → **FALLS** · rust → **FALLS** · typescript → **STANDS**.

**Perimeter searched:** the three aggregator packages' own trees, the three
`-lang` packages' own trees, the whole `packages/**` corpus for README coverage,
the installed slots, and `git log --all` for the file's entire history. **No host
observable is used**, so nothing here is void under §3.8 — this is exactly the
[«a roster against the package's own contents»](../PHASE-D-RELEASE-QUEUE.md#stacks-audience)
class §B.1 says survives.

**The verdict's own measurement, re-run.** The reason is a single sentence
applied to all three anchors: «the front door points at the `-lang` package's
README and **`typescript-ai-native-lang` ships no README.md at all**». Its second
half is true and I confirm it below. **Its first half is not one claim — it is
three different sentences**, and the note's instruction to check «what EXACTLY
each umbrella sentence points at» is what decides the obligation.

### The three sentences are not the same sentence

| stack | anchor line at HEAD | what it points at |
|---|---|---|
| go | `go-ai-native/v0.1.0/README.md:17-18` | «No code, no boot snippet, no policy lives here; **see the `-lang` stack's README for everything.**» |
| rust | `rust-ai-native/v0.7.0/README.md:21-22` | «The consumer front door — wiring, floor, sweep — is documented in **the `-lang` package's README and `spec/rust/GUIDE-AI-NATIVE-RUST.md`.**» |
| typescript | `typescript-ai-native/v0.6.0/README.md:22-24` | «The consumer front door — wiring, floor, sweep — is documented in **the `-lang` package's README and `spec/typescript/GUIDE-AI-NATIVE-TYPESCRIPT.md`.**» |

Each names **its own** family's `-lang` package, not the family next door. So the
sentence is true or false once per stack, and the measurement is a three-row
existence test:

```
$ for p in go-ai-native-lang/v0.1.0 rust-ai-native-lang/v0.7.0 typescript-ai-native-lang/v0.6.0; do
    f="packages/org.vibevm.ai-native/$p/README.md"
    [ -e "$f" ] && echo "EXISTS  $f  ($(wc -c < "$f") bytes)" || echo "MISSING $f"; done
EXISTS  packages/org.vibevm.ai-native/go-ai-native-lang/v0.1.0/README.md  (8060 bytes)
EXISTS  packages/org.vibevm.ai-native/rust-ai-native-lang/v0.7.0/README.md  (4925 bytes)
MISSING packages/org.vibevm.ai-native/typescript-ai-native-lang/v0.6.0/README.md
```

### The two falling anchors, each falsified by its own recorded evidence

Read from [`run/state/cache.json`](../run/state/cache.json) as an instrument, per
[§6.1's first cheap check](../PHASE-D-BATCH-PLAN.md#delegation-lessons):

```
go  ##AGG-FRONT-DOOR  "v": "drift", "ev": [
      ".../go-ai-native/v0.1.0/README.md:17  ##AGG-FRONT-DOOR No code, no boot snippet, no policy lives here; see the `-lang` stack's",
      ".../go-ai-native-lang/v0.1.0/README.md:1  # AI-Native Go (stack:org.vibevm.ai-native/go-ai-native-lang) {#root}",
      "the front door points at the `-lang` package's README and **`typescript-ai-native-lang` ships no README.md at all**…" ]

rust ##AGG-FRONT-DOOR  "v": "drift", "ev": [
      ".../rust-ai-native/v0.7.0/README.md:21  ##AGG-FRONT-DOOR The consumer front door — wiring, floor, sweep — is documented in",
      ".../rust-ai-native-lang/v0.7.0/README.md:1  # AI-Native Rust (stack:org.vibevm.ai-native/rust-ai-native-lang) {#root}",
      ".../rust-ai-native-lang/v0.7.0/spec/rust/GUIDE-AI-NATIVE-RUST.md:24  ## 0. The law, applied to Rust {#law}",
      "…`typescript-ai-native-lang` ships no README.md at all…" ]
```

**Both verdicts cite the target file, by path, quoting its line 1 — and then
convict the sentence for a different package's missing file.** The Rust verdict
cites *both* of its sentence's two targets and both resolve. The evidence list is
a confirmation; only the reason is a conviction.

**Go — the whole sentence is true, both halves.** The second half is the README
above. The first half («no code, no boot snippet, no policy lives here») is true
of the aggregator's own tree and its own manifest:

```
$ ls -a packages/org.vibevm.ai-native/go-ai-native/v0.1.0/
.  ..  LICENSE.md  README.md  vibe.toml
$ grep -c "boot_snippet\|\[\[binary\]\]" packages/org.vibevm.ai-native/go-ai-native/v0.1.0/vibe.toml
0
```

and the manifest's own `description` says the same words back: «Content-minimal by
design (PROP-028): no code, no boot snippet, no policy — it only names the family
members at one resolved version set.» The pointed-at README is 8060 bytes and
covers exactly «everything»: `## What ships`, `## External tooling — the complete
list`, `## Running the tools` (three forms), `## The lifecycle`,
`##wiring-and-sweep-pointers`, `##POLICIES-STAY-WITH-THE-CONSUMER-PROJECT`.

**Rust — the whole sentence is true, and it is true clause by clause.** The
sentence promises «wiring, floor, sweep» in the `-lang` README; the `-lang` README
delivers all three by name — `##wiring-and-sweep-pointers` (`:85-87`, «The wiring
recipe … is GUIDE §13; the sweep idioms are GUIDE §14»), `rust-ai-native floor`
in `## Running the tools` (`:55-63`) and in `## The lifecycle` (`:80`),
`/rust-ai-native-sweep` (`:81`). Its second target resolves too
(`spec/rust/GUIDE-AI-NATIVE-RUST.md`, 
[F-153 above](#f-153--the-boot-snippets-bare-lang-and-cards-resolve-in-no-lane-in-any-of-the-three-stacks)).
Aggregator tree identical: `LICENSE.md README.md vibe.toml`.

### The surviving anchor, and the absence measured properly

**TypeScript — STANDS.** Its verdict's evidence list is the tell: it cites the
sentence and the **guide**, and — alone of the three — **no `-lang/README.md`
line**, because there is none to cite. The sentence's first target does not exist.

The absence, with its perimeter named:

```
$ find packages/org.vibevm.ai-native -maxdepth 3 -name "README.md" -not -path "*/target/*"
…/core-ai-native/v0.7.0/README.md      …/core-ai-native/v0.8.0/README.md
…/go-ai-native-lang/v0.1.0/README.md   …/go-ai-native-mcp/v0.1.0/README.md
…/go-ai-native/v0.1.0/README.md        …/rust-ai-native-lang/v0.7.0/README.md
…/rust-ai-native-mcp/v0.7.0/README.md  …/rust-ai-native/v0.7.0/README.md
…/typescript-ai-native-mcp/v0.6.0/README.md   …/typescript-ai-native/v0.6.0/README.md
   (ten of the eleven ai-native package versions; the eleventh is the gap)

$ total=0; missing=0; for d in packages/*/*/v*/; do total=$((total+1));
    [ -e "$d/README.md" ] || { missing=$((missing+1)); echo "NO README: $d"; }; done
NO README: packages/org.vibevm.ai-native/typescript-ai-native-lang/v0.6.0/
packages scanned: 42 ; without README: 1
```

**42 shipped package versions in this repository; 41 carry a `README.md`; the one
that does not is the one the sentence points at.** And it is not a deletion — the
file has never existed:

```
$ git log --oneline --all -- "packages/org.vibevm.ai-native/typescript-ai-native-lang/*/README.md"
(no output)
```

Nor does it appear in the installed lane:
`ls vibedeps/stack-typescript-ai-native-lang/0.6.0/` → `Cargo.lock Cargo.toml
LICENSE.md crates spec tools vibe.toml`. The same absence was hit independently
by [D7d's F-279 entry](d7d-stacks-sync-reverify.md) from the other direction.

**And a pointer with no other reader.** The only live sentences naming a `-lang`
README anywhere in `packages/**` are these two aggregators plus vendored copies:

```
$ Grep "-lang`? (?:package|stack)'?s? README" packages/ --glob "*.md"
…/typescript-ai-native/v0.6.0/README.md:23
…/rust-ai-native/v0.7.0/README.md:22
…/fractality/v0.1.0/vibedeps/stack-rust-ai-native/0.7.0/README.md:18   (vendored copy of the rust one)
…/delegation-rules/v0.1.0/vibedeps/stack-rust-ai-native/0.7.0/README.md:18  (ditto)
```

**Which side moves is the owner's, and the corpus argues one way.** The sentence
is not wrong about what a consumer needs; the package is missing the file 41 of
its 42 siblings ship. **Writing `typescript-ai-native-lang/v0.6.0/README.md`
closes the anchor without editing a true sentence**, and it is the only closure
that leaves the three aggregators saying the same thing. The alternative — strike
«the `-lang` package's README and» from the TypeScript aggregator alone — leaves
the family with two shapes and a package with no front door. Recommending the
first, flagging it as a **build**, not an edit, and therefore a Phase E item the
`release` gate should carry rather than a diff to approve.

**Proposed correction (NOT APPLIED):**

- go `README.md:17-18` → **none. The sentence is true; do not edit it.**
- rust `README.md:21-22` → **none. The sentence is true; do not edit it.**
- typescript `README.md:22-24` → **no edit either**, if the README is written.
  If the owner instead rules the pointer wrong, the minimal honest text is
  «… is documented in `spec/typescript/GUIDE-AI-NATIVE-TYPESCRIPT.md`» with the
  README clause removed — but that ratifies a package with no README, so it is
  offered as the fallback, not the recommendation.

**Corrected reason, for whichever record survives** (the current one convicts two
true sentences):

> `typescript-ai-native-lang/v0.6.0` ships no `README.md` — the only one of the
> repository's 42 shipped package versions that does not, and the file has never
> existed in git history — so `typescript-ai-native/v0.6.0/README.md:22-24`
> points its consumer front door at a file that is not in the package. The Go and
> Rust aggregators name **their own** `-lang` READMEs, both of which exist
> (8060 and 4925 bytes), and both sentences are correct as written.

**Recommendation per anchor:** go → **FALLS** · rust → **FALLS** · typescript →
**STANDS** (reason needs the restatement above before any diff is shown).

---

## F-186 — `H4` **is** in a register: the ATLAS files four records under it. The obligation's third anchor is a different id and a different defect

**Outcome:** MIXED — **2 FALLS · 1 STANDS-RESTATED.** The obligation merges **two
unrelated defects** under one printed reason, and the printed reason is false.
**Anchors:** 3 of 3 — go `scaffold-g` → **FALLS** · rust `scaffold-g` →
**FALLS** · rust `scaffold-i` → **STANDS-RESTATED** (the reason on record does
not describe it).

**Perimeter searched:** the standing perimeter for the *thing* — **an H-numbered
evidence register under any name**, not the string `##FINDING-H4`. At the SPEC
layer: `core-ai-native/v0.8.0/spec/appendix/{ATLAS,CONTRADICTION-MAP}.md` read in
full, `spec/00-MANIFESTO.md`, `spec/legacy-projections/`. Corpus-wide: `\bH4\b`
over the whole tree (43 files), `\bH[1-6]\b` over `core-ai-native/**` and
`research/**`, and a roster-shaped pattern (`^\s*[-*|]?\s*\**H[1-6]\**\s*[—:–-]`)
over every `*.md`. `refs/**` reported separately below. No ENGINE/DRIVER/DEPLOYMENT
layer can hold a hypothesis roster, and none does
(`grep -rE "\bH[1-6]\b" --include=*.rs` over the six packages → no output).

### The verdict's own measurement, re-run — and the half that holds

```
$ grep -oE "##FINDING-[A-Z0-9]+-[0-9]+" …/core-ai-native/v0.8.0/spec/appendix/ATLAS.md \
    | sed 's/-[0-9]*$//' | sort | uniq -c
     16 ##FINDING-BLD    23 ##FINDING-DR1    24 ##FINDING-DR2
      9 ##FINDING-R2C    15 ##FINDING-R3
$ grep -c "##FINDING-" …/ATLAS.md
87
```

**16+23+24+9+15 = 87. The verdict's first clause is exactly right**, and the
ATLAS's own header agrees: «Total records: 98 · unique (non-duplicate): **87**»
(`ATLAS.md:7`). There is no `##FINDING-H4` anchor; the card never claimed there
was one.

### The half that is false, and it is the verdict's whole basis

The verdict concludes «**H4 is in no register in this repository** … no H-series
roster exists anywhere». **The ATLAS files four records under H4, in its own
record schema:**

```
$ grep -oE "(refines|contradicts):H[0-9]" …/ATLAS.md | sort | uniq -c
      1 contradicts:H1    1 contradicts:H3    3 contradicts:H5
      1 refines:H1        8 refines:H2        5 refines:H3
      4 refines:H4        5 refines:H5        4 refines:H6
   (32 references in total)

$ grep -nB1 "refines:H4" …/ATLAS.md | grep "##FINDING"
53:- ##FINDING-DR2-021 **DR2-021** — Misleading identifiers survive even deobfuscation; poison persists
61:- ##FINDING-DR1-017 **DR1-017** — Comments help LLM comprehension but indiscriminate comments add noise
67:- ##FINDING-DR2-002 **DR2-002** — Incorrect docs hurt; missing docs don't (Macke & Doyle)
75:- ##FINDING-R3-015 **R3-015** — Wrong prose is worse than no prose: machine-check or trust-label all code-adjacent text
```

**The card's gloss is «H4 (lying prose harms)». The fourth record filed under H4
is titled «Wrong prose is worse than no prose».** The id resolves, in the same
appendix in which the sentence's two sibling ids resolve, to four records that say
what the gloss says — one of them (`R3-015`, `_theory · high · refines:H4_`) reading
«Models condition on in-repo text with high trust; a lying comment is adversarial
input … harm of wrong prose exceeds harm of absence.» That is the card's `theory`
half of its own «Class: benchmark + theory» line.

**And the register reaches the consumer.** The stack `[requires]`
`flow:org.vibevm.ai-native/core-ai-native = "^0.8"`
(`rust-ai-native-lang/v0.7.0/vibe.toml:16`), and the installed core carries the
same four: `grep -c "refines:H4" vibedeps/flow-core-ai-native/0.8.0/spec/appendix/ATLAS.md` → **4**.

**This is §3.7's error in its purest form** — *search for the thing, not for the
string*. The verdict searched for an **anchor** (`##FINDING-H4`) and read its
absence as the absence of the register. The H-series does not live as an anchor;
it lives as the ledger's **axis field**, which is why it appears 32 times and
never once as a heading. The corpus's own prose uses exactly the phrasing this
measurement produces: `CONTRADICTION-MAP.md:46` says «**The ATLAS files four
records under H6**» — the same construction, for a sibling id, inside the package
the verdict searched.

**What the sentence asserts, per [§6.1's second cheap check](../PHASE-D-BATCH-PLAN.md#delegation-lessons).**
The card's line is «R2C-008 (…, benchmark), R3-006 (…, theory), H4 (lying prose
harms). Class: benchmark + theory. Tag: **[E-strong]**». Two **records** and one
**hypothesis**, each glossed in place; the `[E-strong]` tag is carried by the
benchmark + theory pair the Class line names. **The sentence never claims H4 is a
record id.** Convicting it of that is the neighbouring-defect shape §6.1 names.

**The neighbouring verdict says so too.** The **TypeScript twin of this card is
word-identical and was judged `confirmed`**:

```
typescript-ai-native-lang/v0.6.0/spec/cards/scaffold-g-doctests.md:33
  "v": "confirmed"     — text byte-identical to the go and rust copies
```

Three copies of one sentence; two drift, one confirmed; the confirmed one is
right. Per the parallel-corpus rule the whole family was re-judged and **all three
copies are correct as written.**

### The real defect, which belongs to a different document

**The H-roster is cited 32 times in the ATLAS, 5 times in the CONTRADICTION-MAP
and 12 times across the three stacks' live documents — and no document states
what H1–H6 each assert.** H1 and H5 are glossed only inside `CONTRADICTION-MAP.md`
C-1's heading and sides; H6 only inside `##C-7-OPEN-H6-UNIFORMITY`; H2, H3 and H4
have no gloss outside the sentences that cite them.

```
$ Grep "^\s*[-*|]?\s*\**H[1-6]\**\s*[—:–-]" **/*.md     (a roster-row shape)
(one hit, and it is a campaign harvest file — not a package document)
```

**Third-party corpus, reported separately per the perimeter rules.**
`refs/ts/talk.json` — a research-session transcript, not our shipped surface —
carries a «hypothesis scoreboard» naming H1–H6 and describing H4 as «подтверждена
и заострена … ложь в прозе бьёт (−23.2% CodeCrash)». So the roster is a genuine
research artefact whose *legend* never made it into the shipped corpus. Recorded
so the next pass neither "finds" it and calls the register shipped, nor re-derives
this. `refs/**` is third-party and proves nothing about the packages.

**Whose defect is it?** `core-ai-native`'s. The roster is referenced by the
**ATLAS's own record schema** and by the **CONTRADICTION-MAP's own headings**;
publishing a six-line legend in `spec/appendix/` resolves all 49 citations at
once, including the twelve in the stacks. Editing two stack cards resolves none of
it and leaves ten identical citations standing — [§4.5's «not a
closure»](../PHASE-D-RELEASE-QUEUE.md#addresses-scope) shape. A sibling note in
`harvest/d1-core-ai-native-repairs.md:205` reached the same conclusion from the
core side and filed it as a new fact rather than an edit; **that note survives
this re-verification and these two anchors do not.**

### Anchor 3 is a different id, a different defect, and the printed reason does not describe it

`rust-ai-native-lang/v0.7.0/spec/cards/scaffold-i-codemods.md:33` at HEAD:

```
##EVIDENCE-AND-TRANSFER-STRENGTH Evidence & Transfer-strength: first-principles
from R3-013 (ownership graph bounds throughput) + R2C-006 (edit size drives Rust
failure) + DL1-015 (constraints lift weak models). NOT in the follow-up.
Class: theory. Tag: **[E-hyp]**. @spec/done
```

**No `H4` in it.** The obligation's printed reason («**H4 resolves nowhere** …»)
is about the other two anchors; this anchor's own recorded reason is «**`DL1-015`
resolves nowhere** … the ATLAS carries `DR1-015` (:181)», which **is** accurate.
Anyone reading the obligation's headline reason would show the owner a diff
justified by a fact that is not about the line being changed.

**Re-measured, and there is now a control the original verdict did not have —
the Go copy of the same card gets it right:**

```
$ grep -n "EVIDENCE-AND-TRANSFER-STRENGTH" packages/org.vibevm.ai-native/*/v*/spec/cards/scaffold-i-codemods.md
go   :33  … + R2C-006 (edit size drives failure) + DR1-015 (constraints lift weak models). …
rust :33  … + R2C-006 (edit size drives Rust failure) + DL1-015 (constraints lift weak models). …
ts   :33  … + R2C-006 (edit size drives failure) + DL1-015 (constraints lift weak models). …

$ grep -c "DL1-" …/core-ai-native/v0.8.0/spec/appendix/ATLAS.md
0
$ sed -n '180,181p' …/ATLAS.md
- ##FINDING-DR1-015 **DR1-015** — Constrained decoding helps weak models most; can hurt strong ones
  _benchmark · med · refines:H3_ — … 'added value of constrained decoding increases as the model gets smaller.' …
```

The Go card writes **`DR1-015`**; Rust and TypeScript write **`DL1-015`**; the
ATLAS carries `DR1-015` and no `DL1-` prefix at all; and the gloss the three cards
share — «constraints lift weak models» — is `DR1-015`'s title almost verbatim.
**One letter, and the third stack is the proof of what was meant.** The Go card's
Evidence line is judged `confirmed`, its evidence list citing `ATLAS.md:181`
directly — so the family already contains a resolved copy of this exact citation.

**The TypeScript twin carries the same typo and is judged `confirmed`**
(`typescript-…/scaffold-i-codemods.md:33`, `"v": "confirmed"`, evidence «located
in the perimeter on the refs above» — with no ref above resolving `DL1-015`). It
carries **no obligation**, so it cannot be re-judged against a change to the Rust
card; the family would ship one repaired card and one identical broken one.

**Proposed correction (NOT APPLIED):**

- go `scaffold-g-doctests.md:33` → **none. Do not edit.**
- rust `scaffold-g-doctests.md:33` → **none. Do not edit.**
- rust `scaffold-i-codemods.md:33` → `DL1-015` → `DR1-015`, one letter, nothing
  else in the line moves, marker stays `@spec/done`. **The TypeScript copy takes
  the identical fix and needs a re-judgement first.**

**Corrected reason for the surviving anchor** (the obligation's headline reason
must not travel with this diff):

> `scaffold-i-codemods.md:33` cites `DL1-015`; the ATLAS carries no `DL1-` prefix
> at all and the record whose title the card glosses is `DR1-015`
> («Constrained decoding helps weak models most», `ATLAS.md:180-181`). The Go
> copy of the same card already writes `DR1-015` and is `confirmed` against
> `ATLAS.md:181`, so the intended referent is not in doubt. The TypeScript copy
> carries the same typo.

**New obligation the boss should open** (not this batch's to write): **the
hypothesis roster H1–H6 is cited 49 times across the shipped corpus and defined
nowhere in it.** Owner: `core-ai-native/v0.8.0/spec/appendix/`. Fixing it
retires the H-half of this obligation properly, instead of by editing two cards
that are right.

**Recommendation per anchor:** go `scaffold-g` → **FALLS** · rust `scaffold-g` →
**FALLS** · rust `scaffold-i` → **STANDS-RESTATED**.

---

## F-188 — all three MOTIVATIONS print a command that does not ship, and the Go one prints **the Rust verb's signature**

**Outcome:** **3 STANDS-RESTATED.** The drift is real in every stack — and the
obligation's single printed reason describes **one** of its three anchors and
mis-enumerates even that one. Every anchor needs its own reason before its diff
is shown.
**Anchors:** 3 of 3 → all **STANDS-RESTATED**.

**Perimeter searched:** per stack, that stack's own `crates/`, `spec/`, `tools/`
and manifests; plus, for the Rust anchor only (legitimate host evidence under
§3.8, since VibeVM is a consumer of the Rust stack), the host's `vibe` CLI command
enum and `spec/common/`. The `vibe` binary's own subcommand list is read as a
statement about **the product every consumer runs**, not about this repository's
adoption — but the TypeScript anchor below is decided **without it**, on the TS
package's own tree alone, so nothing there depends on that reading.

**The genre question, asked first per [§6.1](../PHASE-D-BATCH-PLAN.md#delegation-lessons).**
All three cards are `<status stage="spec" state="done"/>`, `##status-line` «BETA ·
[E-hyp] — **validate before relying on it**», with `##RISKS-AND-ASSUMPTIONS`
saying «**assumes weak agents can correctly parameterize the operation** —
UNVALIDATED». So the *scaffold* is openly a hypothesis. **But the three sentences
do not print a hypothesis — they print an executable command line with named flags
and say it «performs the change atomically and verifiably», in the present tense.**
That is an interface claim inside a Motivation section, and the Go card removes
any doubt by repeating it twice more outside Band 2 (below). A reader — and this
card's declared reader is the weakest agent tier — types it.

### Go — the printed signature is real, and it belongs to the **Rust** binary

Current text, `go-ai-native-lang/v0.1.0/spec/cards/scaffold-i-codemods.md:19`:

> ##MOTIVATION Motivation: A weak agent asked to "add a planner variant" must create the cell package, the conformance assertion, the directive tags, the registry arm, and the Example stub — **five files in lockstep**. `go-ai-native codemod add-cell <pkg> <cell> <seam> <variant> <spec-uri>` performs the change atomically and verifiably; the agent fills **five parameters** instead of coordinating five files. @spec/done

**The verdict's own citations, re-opened at HEAD.** `main.rs:147-165`:

```rust
enum CodemodCmd {
    /// Add a new cell package: doc.go with its `//spec:scope` marker,
    /// the cell source with a New constructor, and a smoke test with
    /// an executed Example — post-checked …
    AddCell {
        #[arg(long)] cell: String,
        #[arg(long)] spec_uri: String,
    },
}
```

`codemod.rs:108-114` writes exactly three files (`doc.go`, `{cell}.go`,
`{cell}_test.go`). **Two named flags against five positional parameters, three
files against five: the drift is real, and a reader typing the documented form
gets a clap argument error.** That much of the reason holds.

**But the reason's enumeration is wrong on one of its two named absences.** It
says «No registry arm and **no Example stub** is written.» The Example stub **is**
written — `smoke_test_source` (`codemod.rs:41-65`) emits

```go
func ExampleNew() {
    fmt.Println(New().Name())
    // Output: {cell}
}
```

which is precisely the `##RULE-EXAMPLE-PER-SEAM` artefact the boot snippet
requires («one `Example` function (compiled AND run; `// Output:` diffed)»). The
two that are genuinely missing are **the conformance assertion** — `cell_source`
(`codemod.rs:26-39`) emits `type X struct{}`, `New()`, `Name()` and **no
`var _ Seam = (*Impl)(nil)`**, which the same boot snippet's `##RULE-CELLS`
mandates for every cell — and **the registry arm**. And the *directive tags* are
written (`doc_source`, `codemod.rs:16-24`, emits `//spec:scope {spec_uri} r=1`).
So the sentence's five-item list is 3 written / 2 not, not 3 / 2 the way the
verdict names them.

**This is not merely a Motivation flourish — the card claims the verb ships,
twice more.** `##STRUCTURE-AND-PARTICIPANTS` (`:21`): «a `go/ast`+`go/format`
program **or the shipped CLI verb** for structural ones». Band-3 routine step 2
(`:42`): «(or use **the shipped** `go-ai-native codemod add-cell`)». Read further
before searching wider, and the further reading **strengthens** the verdict here
rather than dissolving it.

**And the new fact, which the original verdict did not have — the parallel-corpus
rule pointing at the cause.** The five parameters and five files the Go card
prints are **the Rust binary's, verbatim**:

```rust
// rust-ai-native-lang/v0.7.0/crates/rust-ai-native-cli/src/main.rs:177-197
enum CodemodCmd {
    /// Add a new cell to a crate: module + `#[cell]` manifest + REQ edge +
    /// smoke test + lib.rs registration, post-checked and rolled back on failure.
    AddCell {
        #[arg(long)] crate_dir: String,
        #[arg(long)] cell:      String,
        #[arg(long)] seam:      String,
        #[arg(long)] variant:   String,
        #[arg(long)] spec_uri:  String,
    },
}
```

**Five parameters — `crate_dir` ≈ `<pkg>`, `cell`, `seam`, `variant`, `spec_uri` —
and a doc comment naming five artefacts including `lib.rs` registration, the very
«registry arm» the Go card promises and the Go verb does not write.** The Go
card's Motivation is the Rust capability ported onto a Go verb that ships a
two-flag subset. This is [the release queue §B shape](../PHASE-D-RELEASE-QUEUE.md#stacks)
— one fact projected per language and true in only one of them — running from Rust
into Go. It also names the repair precisely: **make the Go sentence print the Go
verb**, not the family's.

### Rust — the command has no implementation, and the host's own PROP already wrote the correction

Current text, `rust-ai-native-lang/v0.7.0/spec/cards/scaffold-i-codemods.md:19`:

> ##MOTIVATION Motivation: A weak agent asked to "rename this seam across its 7 call-sites + the registry + the error enum" desynchronizes them. `vibe codemod rename-seam --from X --to Y` **performs** the change atomically and verifiably; the agent fills two parameters instead of coordinating seven edits. @spec/done

**The obligation's printed reason is the Go one and does not describe this
anchor.** The per-anchor reason on record does: «the command does not exist … no
`vibe codemod` subcommand: the shipped codemod enum carries one variant,
`add-cell`». Re-measured:

```
$ grep -nE "^\s{4}[A-Z][A-Za-z]+" crates/vibe-cli/src/cli.rs      # `pub enum Command` at :91
Init List Install Outdated Search Mcp Aiui Term Frame Skill Agentic Drain Uninstall
Update Reinstall Check Show Prefs Tree Registry Workspace Vvm Bin Trace Vars Progress Version
   — 27 verbs, and no `Codemod`.

$ Grep "rename-seam|rename_seam|RenameSeam"  (whole tree)
18 files: the rust + typescript cards and their five vendored/slot copies;
campaign records (cache/obligations/baseline/evidence/terraforms — not evidence);
legacy-spec (excluded); specmap.json and spec/common/PROP-031 — both of which
CITE the card rather than implement anything.
   — zero implementations, in any language, in any crate.
```

The Rust CLI's codemod enum carries exactly one variant, `AddCell`
(`main.rs:177-197`, quoted above). **The card's own Band 3 does not claim
otherwise** — step 2 reads «Implement a syn-based / cargo-integrated codemod
performing it atomically», i.e. the consumer builds it — so unlike the Go card,
the interface claim is confined to this one sentence. That makes the anchor
attribution *tighter*, not looser: `##MOTIVATION` is the only place in the Rust
card that says the command exists.

**The decisive corroboration is host-side and legitimate for Rust under §3.8 —
this repository's own spec had to disclaim the sentence.**
`spec/common/PROP-031-algorithmic-refactoring.md:21-22`:

```
- ##BEACHHEAD-SCAFFOLD-I The beachhead exists. The AI-Native discipline already ships
  **Scaffold I — codemods** (`scaffold-i-codemods`) … explicitly naming
  `codemod rename-seam --from X --to Y`. @spec/done
- ##BEACHHEAD-LIMITS But today it is (a) **one operation** (`add-cell` scaffolding only),
  (b) marked **`[E-hyp]`** (unvalidated hypothesis), and (c) **a single language stack's
  card**, not a cross-layer capability with a ratified model. @spec/done
```

A downstream PROP quoting the sentence and then having to write «but today it is
`add-cell` scaffolding only» is the strongest available evidence that the sentence
**reads as a fact claim, not as illustration** — the capability-vs-practice test
resolves to *practice*, because a real reader took it that way and had to correct
it one line later.

### TypeScript — decided inside the package, with no host observable at all

Current text, `typescript-ai-native-lang/v0.6.0/spec/cards/scaffold-i-codemods.md:19`:

> ##MOTIVATION Motivation: … `vibe codemod rename-seam --from X --to Y`, **built on `ts-morph`**, performs the change atomically and verifiably; the agent fills two parameters instead of coordinating seven edits. @spec/done

**Judged on the TS package's own tree only**, per §3.8 — nothing below is a host
observable, so the ruling that voids `F-187` and `F-189` does not reach it:

```
$ grep -rn "rename-seam\|rename_seam" packages/org.vibevm.ai-native/typescript-ai-native-lang/
…/spec/cards/scaffold-i-codemods.md:19    ← the sentence itself, and nothing else
   (one hit in the entire package: no crate, no tool, no fixture, no test)

$ sed -n '198,213p' …/crates/typescript-ai-native-cli/src/main.rs
enum CodemodCmd {
    /// Add a new cell: the seam module (`index.ts` with a file-level `@scope`
    /// marker) + a node:test smoke test, post-checked …
    AddCell { #[arg(long)] cell: String, #[arg(long)] spec_uri: String }
}
   — one variant, two flags, dispatched at main.rs:193-195. No rename verb.

$ grep -rn "ts-morph" packages/org.vibevm.ai-native/typescript-ai-native-lang/v0.6.0/ \
     --include=*.json --include=*.ts --include=*.rs --include=*.toml
(no output)
```

**Three absences, all inside the package:** the verb is not in the package's own
CLI; the operation name occurs only in the sentence that promises it; and
`ts-morph` — the library the sentence says the operation is «built on» — is not a
dependency, a tool, or a fixture anywhere in the package. The `##INTENT` line two
sections up (`:11`) says TypeScript's «mature codemod ecosystem (`ts-morph`,
`jscodeshift`, typed ESLint autofix) makes this **the most achievable scaffold
here**» — which is a true statement about the ecosystem and is **not** convicted;
what is convicted is the sentence that turns achievability into a shipped command.

### Proposed corrections (NOT APPLIED) — one per stack, and they are not the same edit

- **go `:19`** → print the verb that ships and the files it writes:
  «… `go-ai-native codemod add-cell --cell <cell> --spec-uri <uri>` creates the
  cell package atomically and verifiably — `doc.go` with its `//spec:scope`
  directive, the cell source with its `New` constructor, and a smoke test carrying
  an executed `Example` — post-checked by the new package's own `go test` and
  rolled back on failure. The seam-conformance assertion and the registry arm stay
  the author's, for now.» **Do not import the Rust five-parameter form**; it is
  the Rust binary's and this is the mechanism by which the error arrived.
- **rust `:19`** → keep the motivation, mark the command as the target rather than
  the present: «*a* `codemod rename-seam --from X --to Y` **would** perform the
  change atomically … The shipped codemod surface today is one verb,
  `rust-ai-native codemod add-cell` (`crates/rust-ai-native-cli`); the rename
  operation is specified and not built — see
  `spec://org.vibevm.core/vibevm/common/PROP-031#beachhead`.» That wording makes the card agree
  with the PROP that already cites it.
- **typescript `:19`** → the same demotion, naming the TS package's own shipped
  verb (`typescript-ai-native codemod add-cell --cell --spec-uri`) and dropping
  «built on `ts-morph`» or restating it as the intended substrate.

**Corrected reason, per anchor** (the obligation currently carries one Go-specific
reason for all three):

> **go** — the card prints `go-ai-native codemod add-cell <pkg> <cell> <seam>
> <variant> <spec-uri>` and a five-file change; the shipped verb takes two named
> flags (`--cell`, `--spec-uri`, `main.rs:153-164`) and writes three files
> (`codemod.rs:108-114`). Of the five artefacts promised, the package directory,
> the `//spec:scope` directive and the `Example` stub **are** written; the seam
> conformance assertion and the registry arm are not. The printed signature is
> `rust-ai-native-cli`'s `AddCell` (`main.rs:177-197`) transposed onto the Go
> binary. The card repeats the shipped-verb claim at `:21` and `:42`.
> **rust** — `vibe` has no `codemod` subcommand (27 verbs, `crates/vibe-cli/src/cli.rs:91`)
> and `rename-seam` has no implementation anywhere in the tree; the Rust codemod
> enum carries one variant, `add-cell`. The host's own
> `PROP-031 ##BEACHHEAD-LIMITS` already records the gap.
> **typescript** — judged inside the package: `rename-seam` occurs exactly once in
> the whole package, in this sentence; the package's CLI ships one codemod verb,
> `add-cell --cell --spec-uri`; and `ts-morph` appears nowhere in the package.

**Recommendation per anchor:** go → **STANDS-RESTATED** · rust →
**STANDS-RESTATED** · typescript → **STANDS-RESTATED**.

---

## F-211 — `init` documents three keys neither relay produces, and the third stack's copy ships all of them

**Outcome:** **1 STANDS · 1 STANDS-RESTATED.** The defect is real in both stacks
and identical in shape; the reason on record is written in **Go's key names** and
is wrong about the Rust anchor's strings.
**Anchors:** 2 of 2 — go → **STANDS** · rust → **STANDS-RESTATED**.

**Perimeter searched:** each stack's own `spec/*/mechanisms/TCG-PROTOCOL-*.md` read
in full around `##OP-INIT` (the whole §1 parity section and the whole §2 operation
list), each stack's own `crates/*-ai-native-tcg/src/serve.rs` at the cited lines
and at `run_serve`, and — as a **control, judged on its own tree only** — the
TypeScript twin's protocol document, its `tools/ts-oracle/oracle.ts`, its bridge
types and its own tests. No host observable anywhere in this entry.

### Go — the verdict re-measured verbatim, and it holds line for line

Current text, `go-ai-native-lang/v0.1.0/spec/go/mechanisms/TCG-PROTOCOL-GO-v0.1.md:57-62`:

> - ##OP-INIT **`init`** `{root}` → `{gopls_version, gopls_path, go_version,
>   root_files, ready}` — resolves and spawns gopls (ORACLE-GO §1), negotiates
>   capabilities (§2), applies §3 config, waits for readiness bounded by a
>   deadline. … @impl/done

```
$ sed -n '74,84p' …/crates/go-ai-native-tcg/src/serve.rs
fn init_result(oracle: &GoOracle<ChildTransport>) -> serde_json::Value {
    serde_json::json!({
        "gopls_version":     oracle.capabilities().server_version,
        "position_encoding": … Utf8 => "utf-8", Utf16 => "utf-16" …,
        "pull_diagnostics":  oracle.capabilities().pull_diagnostics,
        "ready":             oracle.ready(),
    })
}

$ sed -n '294,300p' …/serve.rs
if op == "init" {
    // Re-init: a fresh gopls session (overlays cleared).
    match GoOracle::spawn(&root, READINESS_BUDGET) {          ← the process root
```

**Two of five documented keys are produced** (`gopls_version`, `ready`); **three
are not** (`gopls_path`, `go_version`, `root_files` — `grep -n "gopls_path\|go_version\|root_files" …/go-ai-native-tcg/src/` returns no output);
**two undocumented keys are** (`position_encoding`, `pull_diagnostics`). And the
`{root}` parameter is never read: the `init` branch touches `frame.params`
nowhere, spawning against the `root` `run_serve` canonicalized at entry. **The
verdict is accurate in every particular for Go.**

### Rust — the same defect, different strings, and the reason on record names Go's

Current text, `rust-ai-native-lang/v0.7.0/spec/rust/mechanisms/TCG-PROTOCOL-RUST-v0.1.md:69-75`:

> - ##OP-INIT **`init`** `{root}` → `{ra_version, ra_path, toolchain, root_files,
>   quiescent}` — resolves and spawns the analyzer (ORACLE-RUST §1) … @impl/done

```
$ sed -n '76,86p' …/crates/rust-ai-native-tcg/src/serve.rs
fn init_result(oracle: &RustOracle<ChildTransport>) -> serde_json::Value {
    serde_json::json!({
        "ra_version":        oracle.capabilities().server_version,
        "position_encoding": … ,
        "pull_diagnostics":  oracle.capabilities().pull_diagnostics,
        "quiescent":         oracle.quiescent(),
    })
}

$ sed -n '215,218p' …/serve.rs
pub fn run_serve(root: &Path) -> Result<i32> {
    let root = rust_ai_native_tcg_bridge::verbatim_free(
        &root.canonicalize().unwrap_or_else(|_| root.to_path_buf()),
    );
$ sed -n '282,284p' …/serve.rs
if op == "init" {
    // Re-init: a fresh analyzer session (overlays cleared).
    match RustOracle::spawn(&root, QUIESCENCE_BUDGET) {        ← the process root
```

Identical arithmetic — **2 of 5 produced (`ra_version`, `quiescent`), 3 absent, 2
extra, `{root}` ignored** — but **the three missing keys are `ra_path`,
`toolchain`, `root_files`, not `gopls_path` / `go_version` / `root_files`**, and
the fifth documented key is `quiescent`, not `ready`. The reason on record names
Go's four strings and cites Go's `serve.rs:74-84`; shown to the owner unchanged it
would ask for approval of a Rust diff justified by Go's field names. That is the
same restatement debt F-212's note anticipates, one obligation early.

### Reading further — the document's own evolution rule makes the defect asymmetric

`##PARITY-ADDITIVE-ONLY-EVOLUTION` (`TCG-PROTOCOL-GO-v0.1.md:32-36`, and the same
clause in the Rust document's §1) states the contract's own change rule:

> additive-only evolution within a proto (new optional params, **new response
> fields — non-breaking**; renames/semantic changes bump the constant).

So `position_encoding` and `pull_diagnostics` are **sanctioned** additions that the
document has merely not caught up with — documentation lag on a permitted change,
not a contract breach. The three documented-and-never-produced keys are the other
direction entirely: a client written against `##OP-INIT` reads `root_files` and
gets `undefined`, and removing them is exactly the non-additive change the same
clause says must bump `ORACLE_PROTOCOL`. **The verdict treats the two halves as one
defect; they are opposite defects with opposite repairs**, and the correction below
splits them.

**No «not yet» qualifier exists to rescue either anchor.** Both documents'
`##status-line` assert the opposite: Go — «Status: v0.1 … **implemented by its
Phase 7**»; Rust — «Status: v0.1 … **implemented by its Phases 3–4**». Both
`##OP-INIT` anchors are `@impl/done`, and the §2 heading carries `req r1` (go) /
`req r2` (rust). The section around the anchor is a live operation list, not a
roadmap.

### The control — the TypeScript twin ships the shape, so this is a build gap, not an over-specified contract

Judged on the TypeScript package's own tree only (§3.8):

```
$ …/spec/typescript/mechanisms/TCG-PROTOCOL-v0.1.md:65-66
- ##OP-INIT **`init`** `{root, cells_dir?, seam?}` → `{ts_version, config_file, root_files}`

$ …/tools/ts-oracle/oracle.ts:1021-1023
      ts_version:  session.tsVersion(),
      config_file: norm(configFile),
      root_files:  session.rootFileCount(),

$ …/crates/typescript-ai-native-tcg-bridge/src/lib.rs:133-135
    pub ts_version: String,  pub config_file: String,  pub root_files: u64,

$ …/tools/ts-oracle/test/oracle.test.ts:112-113
  assert.match(String(result["ts_version"]), /^\d+\./);
  assert.ok((result["root_files"] as number) >= 2, "fixture has 2 cells");
```

**All three keys produced, typed, and asserted in the package's own tests**, and
the TS relay actually *honours* `{root}` — `serve.rs:130-134` defaults it when
absent («default the root to ours (the relay serves ONE project)») rather than
ignoring it. Its `##OP-INIT` is judged `confirmed` on exactly those refs. So the
documented shape is not aspirational family-wide: **it is shipped in the one stack
that implements it, and the go/rust relays return a subset.** `root_files` is a
count, `go_version` / `toolchain` a version string, `gopls_path` / `ra_path` a
resolved path the spawn code already has in hand — this is a small build, which is
why the correction below offers the owner both routes rather than assuming
demotion.

### Proposed correction (NOT APPLIED) — two routes, and the owner picks per stack

**Route (a), build:** add the three fields to `init_result` in each relay and
document `position_encoding` / `pull_diagnostics` — additive on both sides, no
proto bump, and it makes all three stacks answer `init` the same way, which
`##ONE-PRODUCT-CLIENT-DRIVES-ALL-THREE-RELAYS` (`TCG-PROTOCOL-GO-v0.1.md:37-39`)
already assumes.

**Route (b), demote the document to what ships** — go `:57-58`:

```
- ##OP-INIT **`init`** `{}` → `{gopls_version, position_encoding,
  pull_diagnostics, ready}` — resolves and spawns gopls (ORACLE-GO §1) …
  The relay serves ONE project and the root is its own process root, so
  `init` takes no parameters. @impl/done
```

and rust `:69-70` the same with `{ra_version, position_encoding,
pull_diagnostics, quiescent}`. **The two diffs are not interchangeable** — the key
names differ in every position but `root_files`/`position_encoding`/`pull_diagnostics`.

**Corrected reason for the Rust anchor:**

> `TCG-PROTOCOL-RUST-v0.1.md:69` prints `init {root}` →
> `{ra_version, ra_path, toolchain, root_files, quiescent}`. The shipped op
> ignores `params` entirely — the root is `run_serve`'s own canonicalized process
> root (`serve.rs:215-218`, used at `:284`) — and `init_result` (`serve.rs:76-86`)
> returns `ra_version`, `position_encoding`, `pull_diagnostics`, `quiescent`.
> `ra_path`, `toolchain` and `root_files` are never produced. The two extra fields
> are permitted by the document's own `##PARITY-ADDITIVE-ONLY-EVOLUTION` and are
> only undocumented; the three absent ones are the contract breach.

**Recommendation per anchor:** go `##OP-INIT` → **STANDS** · rust `##OP-INIT` →
**STANDS-RESTATED**.

---

## F-212 — the Rust skill already says `gated_crates` and names four shipped counters; the Go one names three strings that exist nowhere

**Outcome:** MIXED — **1 STANDS-RESTATED · 1 FALLS.** The note's warning is
confirmed exactly: «the reason is written against the GO skill», and applied to
the Rust anchor it convicts a sentence whose every name is the shipped one.
**Anchors:** 2 of 2 — go → **STANDS-RESTATED** · rust → **FALLS**.

**Perimeter searched:** each stack's own `crates/` (its CLI, its
`crates/vendor/core-ai-native-conform`) and its own `spec/`, judged separately. No
host observable; nothing here is void under §3.8.

### Rust — every name in the sentence is the shipped name

Current text, `rust-ai-native-lang/v0.7.0/spec/skills/rust-ai-native-sweep/SKILL.md:75-79`:

> 5. ##RATCHET-CENSUS-REGRESSIONS **Census regressions** (`unwrap_domain` /
>    `env_nonroot` / `unsafe_nonaudit` / `error_enums_missing_req` non-zero on a
>    gated crate) — drain immediately … **flip a crate into `gated_crates` only
>    after it drains to zero.** @impl/done

**It already says `gated_crates`.** The verdict's headline correction —
«`gated_packages` → `gated_crates`» — has nothing to correct here. And all four
counters ship, verbatim, in the Rust health collector:

```
$ grep -rn "unwrap_domain\|env_nonroot\|unsafe_nonaudit\|error_enums_missing_req" \
      …/rust-ai-native-lang/v0.7.0/crates/ | grep -v /target/
health.rs:59   error_enums_missing_req: BTreeSet<String>,
health.rs:62   unwrap_domain: u32,
health.rs:63   env_nonroot: u32,
health.rs:64   unsafe_nonaudit: u32,
health.rs:131  h.error_enums_missing_req.insert(enum_symbol.clone());
health.rs:140  h.unwrap_domain += 1;
health.rs:153  h.env_nonroot += 1;
health.rs:158  h.unsafe_nonaudit += 1;
health.rs:226  "error_enums_missing_req": …,   :229 "unwrap_domain": …,
health.rs:230  "env_nonroot": …,               :231 "unsafe_nonaudit": …,
```

…emitted **per crate, with a `gated` boolean beside them**
(`health.rs:218-233`: `{"crate": name, "gated": gated.contains(...), …}`), and
`config.gated_crates` is the summary's own key. So «non-zero on a gated crate» is
not merely correct terminology — **it is directly observable in the artefact the
sweep reads.** The config field is `pub gated_crates: Vec<String>`
(`crates/vendor/core-ai-native-conform/src/config.rs:44`), with a doctest at
`:25-31` asserting `cfg.gated_crates`.

**The verdict is falsified by its own evidence list**, exactly as F-115's two falls
were — its three refs are the sentence plus **the two shipped names the sentence
uses**:

```
rust ##RATCHET-CENSUS-REGRESSIONS  "v": "drift", "ev": [
  ".../rust-ai-native-sweep/SKILL.md:75  5. ##RATCHET-CENSUS-REGRESSIONS **Census regressions** (`unwrap_domain` / `env_nonroot` /",
  ".../crates/rust-ai-native-cli/src/health.rs:229          \"unwrap_domain\": h.unwrap_domain,",
  ".../crates/vendor/core-ai-native-conform/src/config.rs:44      pub gated_crates: Vec<String>,",
  "the config field is `gated_crates`, not `gated_packages`, and the three kind strings are wrong too — the shipped values are `init_decl`, `ambient_call`, `naked_go`…" ]
```

Ref 2 shows the counter the sentence names; ref 3 shows the config key the sentence
uses. **Nothing in the reason is about this document.**

### Go — the reason holds, and it understates the defect in a way that changes the diff

Current text, `go-ai-native-lang/v0.1.0/spec/skills/go-ai-native-sweep/SKILL.md:79-83`:

> 5. ##RATCHET-CENSUS-REGRESSIONS **census regressions** (`init_in_cell` /
>    `ambient_call_in_cell` / `naked_go_in_cell` / `error_string_match` /
>    `seam_error_missing_req` non-zero on a gated package) — drain immediately …
>    **flip a package into `gated_packages` only after it drains to zero.** @impl/done

Five kind strings measured against the Go stack's own crates and tools:

```
$ for n in init_in_cell ambient_call_in_cell naked_go_in_cell \
           init_decl ambient_call naked_go error_string_match seam_error_missing_req; do
    printf "%-24s hits=%s\n" "$n" "$(grep -rn "$n" …/go-ai-native-lang/v0.1.0/{crates,tools} | grep -v /target/ | wc -l)"; done
init_in_cell             hits=0
ambient_call_in_cell     hits=0
naked_go_in_cell         hits=0
init_decl                hits=8
ambient_call             hits=13
naked_go                 hits=7
error_string_match       hits=9
seam_error_missing_req   hits=7
```

The shipped vocabulary is `init_decl | blank_import | ambient_call | naked_go |
error_string_match | seam_error_missing_req`
(`crates/vendor/core-ai-native-conform/src/rules/go.rs:104-130`, and the same set
switched on at `crates/go-ai-native-tcg/src/lib.rs:100-121`). **Three of the five
named strings exist nowhere; two are correct** — the reason says «the three kind
strings are wrong» and does not claim otherwise, so it is accurate on this point.

`gated_packages` likewise exists only in prose:

```
$ grep -rn "gated_packages" …/go-ai-native-lang/v0.1.0/ | grep -v /target/
spec/go/GUIDE-AI-NATIVE-GO.md:626        (##SWEEP-FLIP-ONLY-AFTER-DRAIN)
spec/go/tools/conform-frontend-go.md:110
spec/skills/go-ai-native-sweep/SKILL.md:83   ← this anchor
   — three documents, zero code.
```

**And it is not a `[go]`-table alias.** The Go policy table exists — `GoConfig`
(`config.rs:106-140`: `roots`, `cells_dir`, `registry_pkg`) — and **carries no
gating key of its own**; gating is the shared top-level `gated_crates`
(`config.rs:44`). So the Go skill's `gated_packages` names nothing at either level.

**Reading further, and this is what changes the repair.** The ratchet item tells
the reader to watch these counters «non-zero **on a gated package**». The Go
health collector emits neither a per-kind breakdown nor a per-package split:

```
$ sed -n '131,150p' …/crates/go-ai-native-cli/src/health.rs
let snapshot = serde_json::json!({
    "schema": 1, "collector": "go-ai-native health",
    "files_in_scope": …, "file_length": { … },
    "ban_census": { "reasoned": census_reasoned, "unreasoned": census_unreasoned },
    "export_examples": { … }, "orphan_backlog": …,
});
$ sed -n '92,98p' …/health.rs
RawFact::GoUnsafe { reason, .. } => {
    if reason.is_some() { census_reasoned += 1; } else { census_unreasoned += 1; }
}
```

**Two aggregate integers over all `go_unsafe` facts, split only by whether a
reason is present.** Against Rust's per-crate objects with per-kind counts and a
`gated` flag, the Go collector cannot answer the question this ratchet item asks —
so **renaming `init_in_cell` → `init_decl` would leave the sentence still
unobservable**, and a diff written from the reason as it stands would produce a
sentence that is merely differently wrong. The repair is either «name the two
counters the collector emits» or «build the per-package census». That is why this
is STANDS-RESTATED rather than STANDS.

### Two more live sentences carry the same wrong name — one of them `confirmed`

`##SWEEP-FLIP-ONLY-AFTER-DRAIN` (`GUIDE-AI-NATIVE-GO.md:626`, «a package enters
`gated_packages` only at zero findings») is judged **`confirmed`**, and its
evidence list *cites this very anchor* and `conform-frontend-go.md:110` as
support:

```
GUIDE ##SWEEP-FLIP-ONLY-AFTER-DRAIN  "v": "confirmed", "ev": [
  ".../go-ai-native-sweep/SKILL.md:83  package into `gated_packages` only after it drains to zero.**",
  ".../crates/go-ai-native-cli/src/health.rs:161  eprintln!(",
  ".../spec/go/tools/conform-frontend-go.md:110  `registry_pkg` (default `internal/registry`), `gated_packages` / …" ]
```

**A `confirmed` verdict resting on the text a `drift` verdict convicts.** Per
[§6.1's first cheap check](../PHASE-D-BATCH-PLAN.md#delegation-lessons) one of the
two attributions is wrong, and the measurement above says it is the `confirmed`
one. If only the SKILL line is repaired, the GUIDE keeps telling Go adopters to
edit a key that does not exist, and the `confirmed` verdict cites a line that no
longer says what it cited.

### Proposed correction (NOT APPLIED)

- **rust `SKILL.md:75-79`** → **none. Every name is the shipped name; do not
  edit it.**
- **go `SKILL.md:79-83`** → the honest form names what the collector emits and
  what the config calls the list:

  ```
  5. ##RATCHET-CENSUS-REGRESSIONS **census regressions** — `go-ai-native health`'s
     `ban_census.unreasoned` non-zero (every `go_unsafe` fact without a
     `//spec:deviates` reason: `init_decl`, `blank_import`, `ambient_call`,
     `naked_go`, `error_string_match`, `seam_error_missing_req`) — drain
     immediately; restructure beats testify. On an ungated package they are the
     adoption backlog: **flip a package into `gated_crates` only after it drains
     to zero.** @impl/done
  ```

  and the same `gated_packages` → `gated_crates` fix is owed to
  `GUIDE-AI-NATIVE-GO.md:626` and `conform-frontend-go.md:110`, which are **not in
  this obligation** and need a re-judgement of the GUIDE's `confirmed` verdict
  first.

**Corrected reason for the Go anchor:**

> `SKILL.md:79-83` names five census kinds; three of them — `init_in_cell`,
> `ambient_call_in_cell`, `naked_go_in_cell` — exist nowhere in the package
> (0 hits over `crates/` and `tools/`), and the shipped vocabulary is
> `init_decl | blank_import | ambient_call | naked_go | error_string_match |
> seam_error_missing_req` (`crates/vendor/core-ai-native-conform/src/rules/go.rs:104-130`).
> The other two names are correct. The config list is `gated_crates`
> (`config.rs:44`); `gated_packages` appears in three of the package's documents
> and in none of its code, and the `[go]` table (`config.rs:106-140`) carries no
> gating key of its own. **Beyond the names:** the ratchet item asks for a
> per-kind, per-package reading that `go-ai-native health` does not produce — its
> snapshot carries `ban_census: {reasoned, unreasoned}` only
> (`health.rs:141-144`, fed at `:92-98`), against Rust's per-crate per-kind
> objects — so a rename alone does not make the sentence observable.

**Recommendation per anchor:** go → **STANDS-RESTATED** · rust → **FALLS**.

---

## Summary {#summary}

**19 anchors · 8 STANDS · 6 STANDS-RESTATED · 5 FALLS · 0 route-out candidates.**
Falls rate **5/19 = 26 %**, against wave 5's 18/76 (24 %), wave 6's 31/59 (53 %)
and wave 7's 47/171 (27 %). **Three of the six obligations carry at least one
false verdict, and eleven of the nineteen anchors need something changed before a
diff is shown.**

**The single recurring cause is not mis-measurement.** Four of the five falls
were settled by reading the failing verdict's **own evidence list** — F-115's go
and rust anchors each cite the file they are convicted for missing; F-212's rust
anchor cites the two shipped names its sentence uses. The fifth (F-186's `H4`)
was settled by [§3.7's rule](../PHASE-D-BATCH-PLAN.md#compliance-blindness):
the register exists, as an **axis field** (`refines:H4`, four records) rather than
as the `##FINDING-H4` anchor the verdict searched for.

**The second recurring cause is one reason serving anchors it does not describe.**
F-186 merges two unrelated ids (`H4` and `DL1-015`) under the `H4` reason;
F-188 applies a Go `add-cell` reason to two Rust/TypeScript `rename-seam`
sentences; F-211 applies Go's key names to Rust's; F-212 applies the Go skill's
`gated_packages` to a Rust skill that already says `gated_crates`. **Four of six
obligations.**

**Five word-identical or near-identical siblings are judged inconsistently**, and
every one of them is a case where the `confirmed` copy is right and the `drift`
copy is wrong, or the reverse:

| sibling pair | drift copy | confirmed copy | who is right |
|---|---|---|---|
| `scaffold-g` `##EVIDENCE-AND-TRANSFER-STRENGTH` (`H4`) | go, rust — **F-186** | typescript | **confirmed** — all three are correct |
| `scaffold-i` `##EVIDENCE-AND-TRANSFER-STRENGTH` (`DL1-015`) | rust — **F-186** | typescript | **drift** — go writes `DR1-015` and proves the typo |
| `##AGG-FRONT-DOOR` | go, rust, typescript — **F-115** | — | only **typescript** is wrong |
| `gated_packages` | go `SKILL.md:83` — **F-212** | go `GUIDE:626 ##SWEEP-FLIP-ONLY-AFTER-DRAIN` | **drift** — the GUIDE keeps the same wrong key |
| `##RATCHET-CENSUS-REGRESSIONS` | rust — **F-212** | — | **rust is correct**; only go is wrong |

### Recommendation per anchor

| obligation | anchor | recommendation |
|---|---|---|
| **F-153** relocation | `go-ai-native-lang/v0.1.0/spec/boot/20-stack-go-ai-native-lang.md#GO-CODE-FOLLOWS-THE-GO-GUIDE` | **STANDS** |
| | `…/go-ai-native-lang/…/20-stack-go-ai-native-lang.md#CARD-REGISTRY-FOR-GO` | **STANDS** |
| | `rust-ai-native-lang/v0.7.0/spec/boot/20-stack-rust-ai-native-lang.md#RUST-CODE-FOLLOWS-THE-RUST-GUIDE` | **STANDS** |
| | `…/rust-ai-native-lang/…/20-stack-rust-ai-native-lang.md#CARD-REGISTRY-FOR-RUST` | **STANDS** |
| | `typescript-ai-native-lang/v0.6.0/spec/boot/20-stack-typescript-ai-native-lang.md#TYPESCRIPT-CODE-FOLLOWS-THE-TYPESCRIPT-GUIDE` | **STANDS** |
| | `…/typescript-ai-native-lang/…/20-stack-typescript-ai-native-lang.md#CARD-REGISTRY-FOR-TYPESCRIPT` | **STANDS** |
| **F-115** reality-mismatch | `go-ai-native/v0.1.0/README.md#AGG-FRONT-DOOR` | **FALLS** — the Go `-lang` README exists (8060 B) and is cited by the verdict's own ev[1] |
| | `rust-ai-native/v0.7.0/README.md#AGG-FRONT-DOOR` | **FALLS** — both cited targets exist; verdict's own ev[1] and ev[2] |
| | `typescript-ai-native/v0.6.0/README.md#AGG-FRONT-DOOR` | **STANDS** — reason restated; the closure is a **build** (write the README), not an edit |
| **F-186** contradiction | `go-ai-native-lang/…/spec/cards/scaffold-g-doctests.md#EVIDENCE-AND-TRANSFER-STRENGTH` | **FALLS** — ATLAS files 4 records under `refines:H4` |
| | `rust-ai-native-lang/…/spec/cards/scaffold-g-doctests.md#EVIDENCE-AND-TRANSFER-STRENGTH` | **FALLS** — same |
| | `rust-ai-native-lang/…/spec/cards/scaffold-i-codemods.md#EVIDENCE-AND-TRANSFER-STRENGTH` | **STANDS-RESTATED** — `DL1-015` → `DR1-015`; the obligation's `H4` reason does not describe it |
| **F-188** missing-support | `go-ai-native-lang/…/spec/cards/scaffold-i-codemods.md#MOTIVATION` | **STANDS-RESTATED** — printed signature is `rust-ai-native-cli`'s; the Example stub *is* written |
| | `rust-ai-native-lang/…/spec/cards/scaffold-i-codemods.md#MOTIVATION` | **STANDS-RESTATED** — `vibe` has no `codemod` verb; reason on record is Go's |
| | `typescript-ai-native-lang/…/spec/cards/scaffold-i-codemods.md#MOTIVATION` | **STANDS-RESTATED** — judged package-own; `rename-seam` and `ts-morph` both absent from the package |
| **F-211** missing-support | `go-ai-native-lang/…/spec/go/mechanisms/TCG-PROTOCOL-GO-v0.1.md#OP-INIT` | **STANDS** |
| | `rust-ai-native-lang/…/spec/rust/mechanisms/TCG-PROTOCOL-RUST-v0.1.md#OP-INIT` | **STANDS-RESTATED** — missing keys are `ra_path` / `toolchain` / `root_files` |
| **F-212** reality-mismatch | `go-ai-native-lang/…/spec/skills/go-ai-native-sweep/SKILL.md#RATCHET-CENSUS-REGRESSIONS` | **STANDS-RESTATED** — the collector emits no per-kind, per-package census at all |
| | `rust-ai-native-lang/…/spec/skills/rust-ai-native-sweep/SKILL.md#RATCHET-CENSUS-REGRESSIONS` | **FALLS** — already says `gated_crates`; all four counters ship |

### Carried out of this batch for the boss — noticed, not acted on

1. **The hypothesis roster H1–H6 is cited 49 times in the shipped corpus and
   defined nowhere in it** (32 `refines:`/`contradicts:` fields in the ATLAS,
   5 uses in the CONTRADICTION-MAP, 12 live sentences across the three stacks).
   Owner: `core-ai-native/v0.8.0/spec/appendix/`. This is F-186's *real* defect
   and it is not the two cards'.
2. **`##STACK-SHIPS-ITS-OWN-CARDS-PROJECTION` in all three boot snippets** and
   **`core-ai-native`'s own boot snippet** (`10-flow-core-ai-native.md:9-18,38`)
   carry F-153's exact defect and carry **no verdict**.
3. **Five inconsistently-judged siblings**, tabled above — each needs a
   re-judgement in the same pass as its twin's diff, or the family ships two
   answers to one question.
4. **`typescript-ai-native-lang/v0.6.0` is the only one of the repository's 42
   shipped package versions with no `README.md`**, and the file has never existed
   in git history.
5. **`spec/common/PROP-031-algorithmic-refactoring.md:21` rests a host PROP's
   «the beachhead exists» on the card's `rename-seam` command**, then corrects
   itself one line down (`##BEACHHEAD-LIMITS`). The card's repair and the PROP's
   citation should move together.
