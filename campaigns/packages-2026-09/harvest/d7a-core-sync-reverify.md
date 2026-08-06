# D7a — `core-ai-native` on the `sync-from-code` route: 38 verdicts re-verified before any diff is prepared

_Worked 2026-07-31 at HEAD `9f79acf1` (`fix(campaign): the last two boss-closable
obligations, and neither one moved a package`), working tree clean. Subject:
`packages/org.vibevm.ai-native/core-ai-native/v0.8.0/` — every open
`sync-from-code` obligation whose package is `core-ai-native`, enumerated from
`run/state/obligations.json` minus everything already in `run/state/routing.json`:
**8 obligations, 38 anchors**._

| obligation | type | falsifier | anchors | document |
|---|---|---|---:|---|
| F-132 | `reality-mismatch` | mixed | 14 | `spec/mechanisms/PROP-014-specmap-bidirectional-traceability.md` |
| F-146 | `reality-mismatch` | mixed | 7 | `spec/mechanisms/ENGINE-CONFORM-v0.1.md` |
| F-159 | `reality-mismatch` | self | 5 | `spec/mechanisms/LEDGER-INTENT-v0.1.md` |
| F-121 | `contradiction` | self | 4 | `spec/appendix/CONTRADICTION-MAP.md` + 3 closing rules |
| F-120 | `reality-mismatch` | mixed | 3 | `spec/00-MANIFESTO.md`, `spec/01-PATTERN-CARD-FORMAT.md`, `spec/mechanisms/BROWNFIELD-PROTOCOL-v0.1.md` |
| F-206 | `contradiction` | mixed | 2 | `spec/mechanisms/ENGINE-CONFORM-v0.1.md` |
| F-207 | `contradiction` | self | 2 | `spec/mechanisms/PROP-014-specmap-bidirectional-traceability.md` |
| F-263 | `reality-mismatch` | self | 1 | `README.md` |

**Nothing in this record was applied.** No file under `packages/` was edited, no
verdict JSON was written, nothing under `campaigns/packages-2026-09/run/` was
touched, and no `git` command that writes was run. Corrections that survive are
written out below as **proposed text** for the boss to carry to the owner.

---

## Owner ruling received mid-batch (2026-07-31)

_Delivered while F-132 was in measurement; F-263, F-120, F-121, F-207, F-159,
F-146 and F-206 were already written and have each been **re-read against it**,
with a `Re-read under the ruling` line added to every entry saying whether the
outcome moved. In my own words, the five points and what they change:_

1. **The `ai-native` packages are built first and foremost for EXTERNAL
   consumers** — projects we cannot see, in trees we do not have. So a verdict
   that convicts a `core-ai-native` sentence **because this repository does not
   do the thing** may simply be measuring the wrong consumer. Where a sentence
   describes what an *adopting project* does and the only evidence against it is
   that *this host* does not do it, that evidence is weak to void, and the
   outcome is FALSE or FALSE PREMISE with the ruling as the reason.
2. **One exception, and it is the biggest one for this package: part of VibeVM
   itself is written in AI-Native Rust.** Where a core fact is exercised through
   the **Rust** stack, the host genuinely is a consumer and host evidence counts
   normally. **Go and TypeScript run the other way** — Go is a prototype
   specification, deliberately unused here and not to be used here — so a core
   fact whose only possible host witness would be a Go or TypeScript deployment
   **has no host witness by design, and that absence is not drift.**
3. **The legitimate bench for the externally-aimed parts is the package's own
   artefacts and its own TESTS.** Does the package's tree say what it says it
   says, and do its tests exercise it.
4. **Skill-directory evidence is void as a signal about a package.**
   `.claude/skills/`, `.agents/skills/`, `.opencode/skills/` hold whatever this
   host's agents happen to use. *(No entry in this batch rested on such
   evidence; recorded so the record shows the constraint was in force.)*
5. **`legacy-spec/**` is legacy — out of the perimeter and out of every count**
   as evidence of a practice being present or absent. Where an entry below cited
   it, the citation is demoted to corroboration and the load is carried by
   evidence that stands without it.

**What this changed, in one line each:** no outcome reversed, and two were
re-grounded. F-120 now rests on read-only `git log` rather than on the
`legacy-spec/` disposition table (point 5). F-121 and F-206's closing rules now
lead with the **package's own nine «Specified, not built» annotations** — a
package-internal falsification — and demote the host's playbook-phase deadline
to corroboration, because a different consumer's Phase 2 is a different date
(points 1 and 3). F-207's CI clause and F-159's host-disk observations move the
same way. Everything the batch turns on was already package-internal or
Rust-stack, which is why nothing moved.

---

## The standing perimeter

Run from the repository root; referred to below as *the standing perimeter*:

```
packages/**  (INCLUDING packages/org.vibevm.fractality/** — a second adopter, not just a subject)
vibedeps/**  crates/**  xtask/**  tools/**  spec/**  discipline/**  terraform/**
research/**  campaigns/**  fixtures/**  schemas/**  docs/**  manual-tests/**
and the repository root's own *.md / *.toml / *.json / *.sh / *.ps1
minus  **/target/**  .git/**  **/node_modules/**  campaigns/*/run/**
minus  legacy-spec/**   (owner ruling point 5 — legacy; out of every count)
```

`refs/**` is searched but reported **separately** — third-party study corpus; a
hit there is not an implementation of ours.

**Two weightings the owner ruling imposes on everything below.** Evidence drawn
from the **package's own tree and its own tests** is full-strength (ruling point
3). Evidence that amounts to «this host does not do it» is full-strength only
where the host is a genuine consumer — which for `core-ai-native` means **through
the Rust stack** (point 2) — and is weak to void where the sentence is about what
an external adopting project does. Every entry below now says which kind it used.

**The perimeter that bites hardest in this batch is one directory, and it is
inside the subject package itself.** Eight of these thirty-eight verdicts scope a
search to «the host's crates» or to `crates/` — and `core-ai-native` **is a Cargo
workspace with five member crates of its own**, one of which is an MCP server:

```console
$ ls packages/org.vibevm.ai-native/core-ai-native/v0.8.0/crates
core-ai-native-conform
core-ai-native-mcp
core-ai-native-specmap
core-ai-native-specmark
core-ai-native-specmark-grammar
```

That is the four-layer rule of §3.7 in its sharpest form: this package is
simultaneously the **SPEC** layer and the **ENGINE** layer, and a verdict that
searches only the host's `crates/` for the engine is searching the wrong project.
Every entry below names the perimeter it used.

**Three further layer facts, measured once here and cited by several entries:**

```console
$ ls packages/org.vibevm.ai-native/rust-ai-native-lang/v0.7.0/crates/
rust-ai-native-cli            rust-ai-native-conform-frontend  rust-ai-native-specmap
rust-ai-native-conform        rust-ai-native-env-audit         rust-ai-native-tcg
rust-ai-native-tcg-bridge     vendor
$ ls packages/org.vibevm.ai-native/rust-ai-native-lang/v0.7.0/crates/vendor/
core-ai-native-conform  core-ai-native-specmap  core-ai-native-specmark  core-ai-native-specmark-grammar
```

The **DRIVER** layer is each language stack's CLI; the engine crates are
byte-identically vendored into every stack by `cargo xtask sync-engines`. So «no
binary here» is true of `core-ai-native` and says nothing about whether the
described command runs — that question lives one layer over.

---

## F-263 — the front door still says «prompt content only» over five crates and 10 072 lines of Rust; the verdict is right

**Outcome:** SURVIVES
**Anchors:** 1 of 1 — `README.md#PROMPT-CONTENT-ONLY` → SURVIVES
**Perimeter searched:** the standing perimeter for `prompt content only` and
`PROMPT-CONTENT-ONLY`; plus a direct enumeration of the package's own
`Cargo.toml` workspace members and a line/file count of each crate's `src/`.
**The verdict's own command, re-run:** the verdict quotes none — it cites
`Cargo.toml:2` of one crate. That citation reproduces, and the claim is larger
than the citation.

**What the measurement shows:**

The sentence, in full, at `packages/org.vibevm.ai-native/core-ai-native/v0.8.0/README.md:19`:

```
##PROMPT-CONTENT-ONLY This package is prompt content only. @impl/done
```

Its own `Cargo.toml` declares five workspace members
(`packages/org.vibevm.ai-native/core-ai-native/v0.8.0/Cargo.toml:11-17`), and
every one of them is real code:

```console
$ P=packages/org.vibevm.ai-native/core-ai-native/v0.8.0
$ for c in $P/crates/*/; do n=$(basename $c); \
    echo "$n rs=$(find $c/src -name '*.rs'|wc -l) loc=$(find $c/src -name '*.rs' -exec cat {} + |wc -l) bin=$(grep -c '\[\[bin\]\]' $c/Cargo.toml) main=$(test -f $c/src/main.rs && echo yes || echo no)"; done
core-ai-native-conform  rs=14 loc=3928 bin=0 main=no
core-ai-native-mcp      rs=6  loc=1035 bin=0 main=no
core-ai-native-specmap  rs=14 loc=4168 bin=0 main=no
core-ai-native-specmark-grammar rs=2 loc=790 bin=0 main=no
core-ai-native-specmark rs=1  loc=151  bin=0 main=no
```

**37 Rust files, 10 072 lines, five crates, zero binaries.** The package's own
`Cargo.toml:1-8` header says so in its own words — *«the language-neutral
Discipline package, **code-bearing since 0.4.0** (PROP-024 …). It AUTHORS the
neutral engine crates»*. The claim is therefore falsified by the file that sits
two directories from it, and it has been false for four minor versions.

The verdict's parenthetical — «library crates rather than binaries, which is why
the claim is nearly true» — is exactly right and is the reason the correction
must not overshoot: `##RUNNABLE-HALF-IN-STACKS` two lines below is judged
`confirmed` and stays true, because the **drivers** (the CLIs, the binaries) do
ship in the stacks. What ships here is the ENGINE layer, and it is vendored
byte-identically into six sibling packages:

```console
$ find packages/ -type d -path '*/crates/vendor/*' -maxdepth 6 | grep -v /target/ | wc -l
27
```

— four engine crates into each of the three `-lang` stacks, five (the four plus
`-mcp`) into each of the three `-mcp` packages. A sentence that says this package
is prompt content only is wrong about the origin of 27 vendored crate copies.

**Proposed correction (NOT APPLIED):** replace `README.md:19` with

```
##PROMPT-CONTENT-ONLY This package ships prompt content and the neutral engine
crates it authors — five **library** crates (`core-ai-native-conform`,
`-mcp`, `-specmap`, `-specmark`, `-specmark-grammar`); it ships no binary.
@impl/done
```

This keeps the contrast the next anchor draws (`##RUNNABLE-HALF-IN-STACKS`: the
runnable half — the CLI drivers — ships in each stack) while removing the false
half. *Note for the boss:* the same sentence is live in the superseded v0.7.0
slot (`packages/org.vibevm.ai-native/core-ai-native/v0.7.0/README.md:10`) and in
every `vibedeps/` copy; §3.3 of the campaign spec marks superseded slots rather
than verifying them, and the `vibedeps/` copies move only through a re-vendor, so
neither is part of this repair.

**Re-read under the ruling:** unchanged. Every piece of evidence here is the
package's own tree — its `Cargo.toml`, its five crate directories, its `README`.
No host adoption, installation or dispatch is involved, so points 1, 2, 4 and 5
have nothing to bite on. This is ruling point 3's bench exactly: *does the
package's own tree say what it says it says*, and it does not.

**Recommendation per anchor:** `##PROMPT-CONTENT-ONLY` → **drift stands,
correction prepared**.

---

## F-120 — one reason pasted over three different sentences; the supersession it calls unrecorded is planned in a table, executed by a named commit, and built in the parser

**Outcome:** FALSE — 3 of 3
**Anchors:** 3 of 3, each with its own outcome
- `spec/00-MANIFESTO.md#status-line` → **FALSE** (the reason is about a sentence this anchor does not contain)
- `spec/01-PATTERN-CARD-FORMAT.md#status-line` → **FALSE**
- `spec/mechanisms/BROWNFIELD-PROTOCOL-v0.1.md#AMENDMENT-GUIDE-SPEC-AUTHORING-LIFECYCLE` → **FALSE**

**Perimeter searched:** the standing perimeter for `GUIDE-SPEC-AUTHORING`,
`DISCIPLINE-CHARTER`, `req r2`, `disputed(#`, `kind line`, `tombstone`; **plus
read-only `git log --all` over the whole history**, which no verdict in this
batch consulted and which is where a superseded document necessarily lives.

**The verdict's own command, re-run:**

```console
$ find . -iname '*SPEC-AUTHORING*'
$ echo $?
0
```

**It reproduces — zero hits — and it is the wrong question.** The registry itself
flags why: this obligation's `merged_by` is `['cites F-120', 'reason-text
j=1.00']`, i.e. **one reason text at Jaccard 1.00 across three anchors**. That is
§3.7's corollary signature, and here it fired on sentences that are not the same
sentence.

**What the measurement shows:**

**(1) `00-MANIFESTO.md#status-line` does not mention GUIDE-SPEC-AUTHORING at all.**

```console
$ sed -n '5p' packages/org.vibevm.ai-native/core-ai-native/v0.8.0/spec/00-MANIFESTO.md
##status-line **Discipline v0.2 · status: BETA · supersedes DISCIPLINE-CHARTER-v0.1** @impl/done
```

The reason attached to it — *«supersedes part of GUIDE-SPEC-AUTHORING-v0.1», and
that document is not in this repository* — quotes a string that is not in this
fact. What this fact claims is that v0.2 supersedes **DISCIPLINE-CHARTER-v0.1**,
and the verdict's own second evidence ref confirms it verbatim:

```console
$ sed -n '23p' legacy-spec/terraforms/TERRAFORM-PLAN-v0.3.md
| `DISCIPLINE-CHARTER-v0.1.md` | **superseded** | `discipline-v0.2/00-MANIFESTO.md` (the new charter; axioms retained, projected to language level) |
```

A verdict that files a fact as drift while attaching, as evidence, the row that
states the fact.

**(2) The supersession is not unrecorded — it is planned in a table and executed
by a commit whose subject is the claim.**

```console
$ sed -n '25p' legacy-spec/terraforms/TERRAFORM-PLAN-v0.3.md
| `GUIDE-SPEC-AUTHORING-v0.1.md` | **superseded + extended** | `discipline-v0.2/01-PATTERN-CARD-FORMAT.md` (spec authoring = card authoring now) |
```

```console
$ git log --all --diff-filter=A --format="%h %ad %s" --date=short -- 'spec/neworder/GUIDE-SPEC-AUTHORING-v0.1.md'
689113ad 2026-06-10 docs(spec): add the Discipline terraform package v0.2-beta
$ git log --all --diff-filter=D --format="%h %ad %s" --date=short -- 'spec/neworder/GUIDE-SPEC-AUTHORING-v0.1.md'
7ca98728 2026-06-11 docs(spec): Discipline v0.2 BETA supersedes the v0.1 package
```

The document existed at `spec/neworder/GUIDE-SPEC-AUTHORING-v0.1.md` and was
removed by `7ca98728`, whose subject line **is** the supersession. `DISCIPLINE-CHARTER-v0.1.md`
was removed by the same commit, which is why the two `##status-line`s are one
event and not two coincidences.

**This is a category error in the verdict, not a near miss.** «X supersedes Y» is
a **lineage claim about the past**. Its truth condition is that Y existed and was
replaced by X — which is precisely what the absence of Y evidences. Requiring the
superseded document to still be present would make every correct supersession
statement in this repository false, and there are two of them in this obligation
alone.

**(3) The amendment did land, and it landed in the parser.** BROWNFIELD's own
statement of the amended grammar is `##UNIT-STATUSES-ARE-KIND-LINE-GRAMMAR`
(`spec/mechanisms/BROWNFIELD-PROTOCOL-v0.1.md:94`) — *«`req r2` (default:
ratified) · `req r1 planned` · `req r2 disputed(#other-anchor)` · retired
(tombstone)»*. The scanner implements exactly that vocabulary, including the
anchor-id validation on the disputed argument:

```console
$ sed -n '98,113p' packages/org.vibevm.ai-native/core-ai-native/v0.8.0/crates/core-ai-native-specmap/src/mdspec.rs
    let (status, disputes) = match words.next() {
        None => (None, None),
        Some("planned") => (Some(SpecUnitStatus::Planned), None),
        Some(w) if w.starts_with("disputed(#") && w.ends_with(')') => {
            let other = &w["disputed(#".len()..w.len() - 1];
            if !is_valid_anchor(other) {
                return Err(format!(
                    "kind line `{decl}`: disputed(...) must name an anchor id `[A-Za-z][A-Za-z0-9_-]*`, got `{other}`"
                ));
            }
            (Some(SpecUnitStatus::Disputed), Some(other.to_string()))
        }
        Some(w) => {
            return Err(format!(
                "kind line `{decl}` has an unknown status `{w}` (expected `planned` or `disputed(#anchor)`)"
            ));
        }
    };
```

`SpecUnitStatus { Disputed, Planned }` is the wire type
(`crates/core-ai-native-specmap/src/generated/specmap/mod.rs:138-144`); `ratified`
is the default and therefore absent by design, and `retired` is a tombstone
comment, specified at `PROP-014-specmap-bidirectional-traceability.md:80`
(`##HEADING-ANCHOR-GRAMMAR-AND-IMMUTABILITY`, *«retiring a unit tombstones the
anchor (`<!-- RETIRED: superseded by #new-anchor -->`)»*). The kind line itself
is practised in the host's live specs — `spec/common/PROP-018-agentic-standalone-modes.md:115`,
`:139`, `spec/common/PROP-019-version-manager.md:83`,
`spec/modules/vibe-mcp/PROP-015-mcp-integration.md:125`,
`spec/modules/vibe-resolver/PROP-003-dep-evolution.md:454` — and the module that
parses it opens with a doc comment citing the amended document by name
(`mdspec.rs:4-9`).

So «this one amends a document that is not here» is answered three ways: the
document was here, its replacement is named in the plan that replaced it, and the
amendment's content is in the code.

**`refs/**`, reported separately:** `grep -rn 'GUIDE-SPEC-AUTHORING' refs/` returns
**2** hits, both inside one third-party transcript (`refs/ts/talk.json:31`, `:171`)
which happens to discuss this very supersession — *«GUIDE-SPEC-AUTHORING-v0.1.md →
заменён 01-PATTERN-CARD-FORMAT.md»*. It is study corpus, not an artefact of ours,
and is **not** counted in the evidence above; it is reported because it
independently corroborates the disposition the terraform plan recorded.

**Proposed correction (NOT APPLIED):** none — all three facts are correct as
written.

*One thing the boss may want to route separately, and it is not this
obligation's:* `##UNIT-STATUSES-ARE-KIND-LINE-GRAMMAR`
(`BROWNFIELD-PROTOCOL-v0.1.md:94`) says *«see GUIDE-SPEC-AUTHORING amendment»* —
a **pointer** to a document that no longer resolves. That is a `relocation`-shaped
defect in a different anchor, not in any of the three here, and the honest repair
is re-pointing it at `01-PATTERN-CARD-FORMAT.md`. It is noted, not acted on.

**Re-read under the ruling:** outcome unchanged, **evidence re-grounded** under
point 5. The `legacy-spec/terraforms/TERRAFORM-PLAN-v0.3.md` rows at `:23` and
`:25` are now **corroboration only** and are excluded from any count. The load is
carried entirely by read-only `git log`, which is not `legacy-spec/` and is the
strongest evidence available for a supersession anyway: the document existed at
`spec/neworder/GUIDE-SPEC-AUTHORING-v0.1.md` (added `689113ad`) and was removed
by **`7ca98728`, whose commit subject is «docs(spec): Discipline v0.2 BETA
supersedes the v0.1 package»**. Both `##status-line`s stand on that alone. The
third anchor stands on the package's own parser (`mdspec.rs:98-113`), which is
point 3's bench; its supporting host-spec-unit examples are Rust-stack spec units
in a repository that is a genuine AI-Native Rust consumer (point 2), so they
count normally — but they are not load-bearing either way.

**Recommendation per anchor:**
`spec/00-MANIFESTO.md#status-line` → **re-judge confirmed**; the reason quotes a
sentence this anchor does not contain, and the claim it does make is stated by
the verdict's own evidence ref.
`spec/01-PATTERN-CARD-FORMAT.md#status-line` → **re-judge confirmed**;
supersession planned at `TERRAFORM-PLAN-v0.3.md:25`, executed by `7ca98728`.
`spec/mechanisms/BROWNFIELD-PROTOCOL-v0.1.md#AMENDMENT-GUIDE-SPEC-AUTHORING-LIFECYCLE`
→ **re-judge confirmed**; the amended grammar is parsed at `mdspec.rs:98-113` and
practised in five host spec units.

---

## F-121 — the three closing rules survive, and they survive on the documents' own annotations; the fourth anchor survives narrowly on a labelled exception

**Outcome:** SURVIVES — 4 of 4
**Anchors:** 4 of 4, each with its own outcome
- `spec/mechanisms/BROWNFIELD-PROTOCOL-v0.1.md#UNEXERCISED-FIELD-STATUS-OR-POLICY-IS-REMOVED` → **SURVIVES**
- `spec/mechanisms/LEDGER-INTENT-v0.1.md#UNEXERCISED-POLICY-IS-REMOVED-NOT-CARRIED` → **SURVIVES**
- `spec/mechanisms/PROP-014-specmap-bidirectional-traceability.md#UNEXERCISED-MECHANISM-IS-REMOVED-FROM-THE-SPEC` → **SURVIVES**
- `spec/appendix/CONTRADICTION-MAP.md#ENTRY-CARRIES-FOUR-PARTS` → **SURVIVES, narrowly** (counter-reading recorded, not suppressed)

**Perimeter searched:** for the three closing rules the perimeter is **the
documents themselves plus `terraform/`** — the host's record of executing the
playbook whose phase these rules name as their deadline. I did **not** take the
verdicts' «none of which exist» lists on trust; §3.7 has invalidated exactly that
move twice, and wave 5 proved several such lists wrong. Instead the falsification
below needs no absence claim at all. For `##ENTRY-CARRIES-FOUR-PARTS` the
perimeter is one 48-line file, read in full.
**The verdict's own command, re-run:** the verdicts quote none; they cite
`file:line` refs, and every ref I checked resolves.

**What the measurement shows:**

**(1) The deadline these three rules name has passed, and the host recorded it
passing.** This is the check none of the three verdicts performed, and it is the
one that could have saved them: a rule of the form *«any X not exercised **by
Phase N** is removed»* cannot be falsified before Phase N. Phase N is over.

```console
$ sed -n '3,4p;11,22p' terraform/REPORT.md
_2026-06-10, branch `new`. The beta-exit deliverable of
[`PLAYBOOK-TERRAFORM-VIBEVM-v0.2.md`](../spec/neworder/PLAYBOOK-TERRAFORM-VIBEVM-v0.2.md)
## Phase ledger

| Phase | Status | Evidence |
|---|---|---|
| -1 inventory | done | ... |
| 0 tooling skeleton | done | ... |
| 1 pilot + drill | done | ... |
| 2 backfill vibe-resolver | done | 54 proposals (all owner-APPROVED), 6 affirmation commits, PRP-0054 ratchet catch, orphan ratchet flipped blocking |
...
| 5 ledger MVP | done | `.ledger/` interpretations store; `trace explain --prose` epoch-keyed cache + provenance line; telemetry; facts proven epoch-immune |
| 6 expansion + reconciliation + report | done | ratchet 15->8 exemptions (each with reason); intent unaccounted = 0; instrumented category-C audit; this report |
```

`terraform/REPORT.md:4` names **`PLAYBOOK-TERRAFORM-VIBEVM-v0.2.md`** — the exact
playbook BROWNFIELD and LEDGER-INTENT cite — and `:18` books **Phase 2 done**,
`:21` **Phase 5 done**, both on 2026-06-10. BROWNFIELD's trigger («Playbook
(v0.2) Phase 2»), LEDGER-INTENT's («Playbook Phase 5») and PROP-014's («the end
of Phase 2» — its own §4 backfill phase, the same event `:18` books) have **all
fired, seven weeks before Phase C measured these documents**.

**(2) So the only remaining question is whether unexercised content was removed —
and each document answers that against itself, in writing.** No absence claim is
needed, and none is made here:

```console
$ P=packages/org.vibevm.ai-native/core-ai-native/v0.8.0
$ for f in PROP-014-specmap-bidirectional-traceability LEDGER-INTENT-v0.1 BROWNFIELD-PROTOCOL-v0.1; do
    printf "%s: %s\n" "$f" "$(grep -ciE 'specified, not built' $P/spec/mechanisms/$f.md)"; done
PROP-014-specmap-bidirectional-traceability: 5
LEDGER-INTENT-v0.1: 3
BROWNFIELD-PROTOCOL-v0.1: 1
```

Nine passages across the three documents that carry a mechanism **and say in the
same breath that it is not built**. A rule saying such content *«is removed
rather than carried as aspiration»* is falsified by any one of them. The
sharpest, quoted:

- `PROP-014-…md:120` — *«**Specified, not built:** `decides` is not a verb this
  system can emit. The `Verb` enum in `specmark-grammar` is `Implements ·
  Verifies · Documents · Deviates · Informs`, and its own doctest states the verb
  set is closed»* — and this sits **inside `##ROW-KIND-PROP`**, a row of the
  four-unit-kinds table §2.2 is built on.
- `PROP-014-…md:242` — *«**Specified, not built — all three clauses.** (a)
  Nothing is signed: no signing or verification path exists in `vibe-publish`,
  `vibe-registry` …»*
- `LEDGER-INTENT-v0.1.md:78` — *«**Specified, not built:** this query kind has
  never been run.»*
- `BROWNFIELD-PROTOCOL-v0.1.md:132` — *«**Specified, not built:** nothing detects
  a flatline and nothing activates a quota»*

**These annotations are this campaign's own Phase B/C markup**, which makes the
contradiction sharper rather than softer: the campaign that exists to remove
false claims has written into each document a standing admission that the
document's own closing rule is unkept. All three rules are `@impl/done`
(`BROWNFIELD-…:152`, `LEDGER-INTENT-…:100`, `PROP-014-…:425`), and `@impl/done`
on a rule whose practice is nine documented exceptions is the defect.

**(3) `##ENTRY-CARRIES-FOUR-PARTS` — the measurement reproduces; the inference is
contestable; both readings are recorded.** The file is 48 lines and was read in
full. C-1 … C-6 each carry `##c-N-side-a`, `##c-N-side-b`, `##C-N-RESOLUTION`,
and each resolution names the discipline decision it drove (`→ **Central law**`,
`→ **Minimal-sufficiency delivery**`, `→ **Runnable capital**`, `→ Guide §12, tcg
spec`, `→ drives the locality/size/ownership rules`, `→ the discipline **lowers**
the floor`). **C-7 (`:43-46`) carries none of the three.** The verdict's claim —
«C-7 carries no side/resolution triple» — is exactly true.

*The counter-reading, stated because it is not weak.* C-7 is titled **«Unresolved
/ open (honest)»**, which is a label rather than an oversight; its three items
each name a conflict and its evidence and state the empty resolution explicitly
(`:44` *«**No source resolves this.** It is the central pilot validation
target»*); and the document's closing fact supplies the fourth part for the entry
as a whole — `:48` `##open-items-are-why-the-package-is-beta` *«These open items
are why the package is BETA and why every card carries a falsifiable prediction
in place of a present measurement»*. Under §3.6(c) a **marked** exception is not
drift, and C-7's title is a mark.

*Why I still recommend SURVIVES.* §3.6(c)'s ruling is that the exception is
written down **as an exception**; C-7's title announces its own state, but the
**format rule** admits none — it quantifies over «each entry» without
qualification, and one entry of seven departs. Both the defect and the repair are
one clause, and the repair loses nothing, which is the test for whether an edit
is worth the owner's read.

**Proposed correction (NOT APPLIED):**

*For `CONTRADICTION-MAP.md:11` — replace:*

```
##ENTRY-CARRIES-FOUR-PARTS *Each entry: the conflict, the evidence on each side, the resolution, and which discipline decision it drove.* @impl/done
```

*with:*

```
##ENTRY-CARRIES-FOUR-PARTS *Each resolved entry: the conflict, the evidence on each side, the resolution, and which discipline decision it drove. One entry is deliberately unresolved — C-7 holds the open questions, and the fourth part it drove is the package's BETA status (below).* @impl/done
```

*For the three closing rules there are two honest repairs, and choosing between
them is a policy decision, so it is put to the owner rather than picked here:*

**(i) demote the marker** — `@impl/done` → `@spec/done` on all three, since the
rule is stated and not enforced; **or (ii) narrow the rule to what is actually
practised**, e.g. for `PROP-014-…md:425`:

```
##UNEXERCISED-MECHANISM-IS-REMOVED-FROM-THE-SPEC *Any mechanism specified here that is not exercised by the end of Phase 2 is either removed from the spec or annotated in place as **specified, not built** — never carried as unmarked aspiration.* @impl/done
```

with the same shape for `BROWNFIELD-…:152` and `LEDGER-INTENT-…:100`.

**(ii) is the option that describes reality**: the nine annotations *are* the
practice that replaced removal, and they are strictly more informative to a
consumer than deletion would have been — but adopting it changes the policy, and
only the owner can do that. **Note for the boss:** whichever is chosen must land
on all three as a set, because they are one rule written three times. §3.7's
corollary cuts both ways: a repair landed in one of three is precisely the
`duplication` obligation the registry will mint on the next run.

**Re-read under the ruling:** outcomes unchanged, **argument re-ordered** under
points 1 and 3, and the change is worth stating because it makes the finding
stronger rather than weaker.

Section (1) above — the deadline — rests on `terraform/REPORT.md`, i.e. on **this
host's** execution of `PLAYBOOK-TERRAFORM-VIBEVM-v0.2.md`, which is a
vibevm-specific playbook. Under ruling point 1 that is the wrong instrument to
convict an externally-aimed package with: an external adopter's «Phase 2» is its
own date, or has not happened at all. **So the deadline argument is demoted to
corroboration**, and it corroborates only that the rule has had at least one
consumer reach its trigger.

**Section (2) carries the whole finding, and it needs no consumer at all.** Nine
passages in the package's own three documents carry a mechanism *and say in the
same breath that it is not built* — `PROP-014-…:120`, `:242`, `LEDGER-INTENT-…:78`,
`BROWNFIELD-…:132` among them. A rule saying such content «is removed rather than
carried as aspiration», sitting in a document that carries nine such passages, is
falsified by the document itself. That is ruling point 3's bench in its purest
form: the package's own tree against the package's own sentence, with this
repository playing no part. `##ENTRY-CARRIES-FOUR-PARTS` is the same — one
48-line file read against itself.

**Recommendation per anchor:**
`##UNEXERCISED-FIELD-STATUS-OR-POLICY-IS-REMOVED` → **drift stands, correction
prepared (owner picks (i) or (ii))**; deadline fired at `terraform/REPORT.md:18`,
live annotation at `BROWNFIELD-…:132`.
`##UNEXERCISED-POLICY-IS-REMOVED-NOT-CARRIED` → **drift stands, correction
prepared**; deadline fired at `terraform/REPORT.md:21`, three live annotations.
`##UNEXERCISED-MECHANISM-IS-REMOVED-FROM-THE-SPEC` → **drift stands, correction
prepared**; deadline fired at `terraform/REPORT.md:18`, five live annotations,
one of them inside the document's own kind table.
`##ENTRY-CARRIES-FOUR-PARTS` → **drift stands, correction prepared**; one-clause
repair, with the counter-reading recorded above for the owner to overrule if he
reads C-7's title as a marked exception.

---

## F-207 — one anchor survives with a sharper description than the verdict gave it; the other is false, and the code cites the very clause it is accused of contradicting

**Outcome:** MIXED — 1/2 SURVIVES, 1/2 FALSE
**Anchors:** 2 of 2, each with its own outcome
- `spec/mechanisms/PROP-014-…md#EDGE-MODEL-EDGES` → **SURVIVES** (2 of the 4 amendment clauses are unbuilt; a third is built in a different shape than the sentence names)
- `spec/mechanisms/PROP-014-…md#PHASE-0-ACCEPTANCE` → **FALSE**

**Perimeter searched:** the whole of `core-ai-native/v0.8.0/crates/` (the ENGINE
layer — **inside the subject package**, which is the perimeter these verdicts got
right), plus `rust-ai-native-lang/v0.7.0/crates/rust-ai-native-cli/` (the DRIVER
layer) for the coverage clause, plus `terraform/` (the DEPLOYMENT layer) for
Phase 0's acceptance record. Terms: `conflicts_with`, `disputed` / `Disputed`,
`suspect` / `Suspect`, `planned` / `Planned`, `coverage`, `full node inventory`.
**The verdict's own command, re-run:** the verdicts quote none; both cite
`file:line`, and all six refs resolve.

**What the measurement shows:**

**(1) `##EDGE-MODEL-EDGES` — the verdict's conclusion holds and its description
is imprecise in a way that matters for the diff.** The sentence has a base clause
and four amendment clauses; they are in four different states, and only a
clause-by-clause reading produces a correct repair.

| clause | state | evidence |
|---|---|---|
| base: `(CodeItem) --verb--> (SpecUnit @ r)`, provenance, mandatory `deviates` reason | **built** | `generated/specmap/mod.rs:87-105` (`Edge { file, fromSymbol, line, provenance, uri, verb }`) |
| spec units carry lifecycle status `ratified\|planned\|disputed\|retired` | **built, in three mechanisms** | `SpecUnitStatus { Disputed, Planned }` (`mod.rs:138-144`); `ratified` = **absent** by design (`mod.rs:202` *«Lifecycle status (brownfield amendment); absent = ratified»*); `retired` = tombstone comment, specified at `PROP-014-…md:80` |
| a spec↔spec **edge** `conflicts_with` records contradictions | **built in a different shape** — the *pairing* is a field, not an edge | `mod.rs:184-187` `pub disputes: Option<Box<String>>` *«For `disputed` status: the other anchor of the conflicts_with pair»*; parsed at `mdspec.rs:101-108`; rendered at `explain.rs:49-54` as `[DISPUTED ↔ #other]` |
| edges into `disputed` units are **frozen**, exempt from suspect-clearing | **not built** | `index.rs:118-131` — the only `suspects.push(…)` in the engine, gated on `p < rev` alone, with no read of `unit.status` anywhere in the function |
| coverage math reports `planned` scope separately and never penalizes it | **not built** | no coverage math over spec units exists: `ratchet.rs` contains no occurrence of `status` or `planned`; the only `coverage` in a CLI is doctest/type coverage (`rust-ai-native-cli/src/health.rs:53`, `:97`), a different quantity |

So the verdict is right that the anchor is false, and **wrong in the detail that
would have driven the edit**: `conflicts_with` is not merely *«a doc comment»*.
The pair is a first-class serialized field on `SpecUnit`, validated by the parser
against the anchor grammar, and printed by `explain`. What does not exist is the
**edge**. A repair written from the verdict's description would delete a
mechanism that is two-thirds built.

**(2) `##PHASE-0-ACCEPTANCE` — the scanner cites this clause by name while
implementing it.** The fact reads: *«Acceptance: index builds deterministically
twice on the untouched repo (zero edges, full node inventory); CI job wired but
non-blocking.»* The verdict grants the determinism half and rejects *«full node
inventory»* as *«contradicted by design — untagged items are excluded by the
scanner, so the inventory is of TAGGED items and cannot be full»*.

That reads the phrase against the **code** side. The markdown scanner reads it
the other way, and says so, quoting the clause and its section number:

```console
$ sed -n '1,9p' packages/org.vibevm.ai-native/core-ai-native/v0.8.0/crates/core-ai-native-specmap/src/mdspec.rs
//! Markdown side of the scanner: anchored spec units.
//!
//! A unit is the span from an anchored heading (`### Title {#anchor}`)
//! to the next same-or-higher heading — anchored or not (GUIDE-SPEC-
//! AUTHORING §1). The first non-blank body line may be a kind line:
//! `` `req r2` ``, `` `req r1 planned` ``, `` `req r2 disputed(#other)` ``
//! — optionally followed by prose on the same line. Units without a kind
//! line are legacy-unmarked and still inventoried (full node inventory,
//! PROP-014 §4 Phase 0).
```

**«full node inventory, PROP-014 §4 Phase 0»** — the engine names this exact
acceptance clause as the reason it inventories unmarked units. And the reading is
forced by the sentence's own scope: the acceptance is over **the untouched
repo**, where by construction no `#[spec]` tag exists, so the code-item side has
nothing to inventory and *«zero edges»* in the same parenthesis says so. On an
untouched repo the only inventory there can be is the spec-unit one, and it is
full.

The acceptance was then **executed and booked by the consumer**:

```console
$ sed -n '16p' terraform/REPORT.md
| 0 tooling skeleton | done | specmark-grammar / specmark / specmap-core crates; `specmap [--check]` / `test-gate` / `tripwire`; first committed index (408 units) |
```

408 units, zero edges, on the untouched repo — the clause, discharged. The
determinism half the verdict grants is tested at `index.rs:457`
(`fn index_is_deterministic()`).

*The one half that is genuinely unmet is a marked exception, not drift.* «CI job
wired but non-blocking» has no instance because **this repository has no CI at
all**, by a standing owner decision the same report records as a finding:

```console
$ sed -n '81,84p' terraform/REPORT.md
8. **The CI bullets repeat in three phases against a repo whose owner
   decision is no-CI.** v0.2 should parameterise the carrier ("gate
   command runs in CI *where CI exists*; locally otherwise") instead
   of re-deferring per phase.
```

§3.6(c) is explicit that a **marked** exception is not drift, and wave 6 applied
that ruling to a CI clause in another package already
(`spec/terraforms/PACKAGES-ACTUALIZATION-CAMPAIGN-v0.1.md:3488`). This is the
same shape, and the marking is seven weeks older than the verdict.

**`refs/**`, reported separately:** `grep -rl 'conflicts_with' refs/` returns
**328** files, all in `refs/src/cargo`, `refs/src/uv`, `refs/src/warp` and
`refs/study/cargo` — clap's argument-conflict builder API in third-party code, an
unrelated sense of the identifier. Not an instance of ours and not counted above.
Recorded because a bare hit count here would look like a finding and is not.

**Proposed correction (NOT APPLIED):** for `##EDGE-MODEL-EDGES` only. Replace the
amendment sentence at `PROP-014-specmap-bidirectional-traceability.md:199` —

```
*(Brownfield amendment:)* spec units additionally carry a lifecycle status (`ratified` | `planned` | `disputed` | `retired`), and a spec↔spec edge `conflicts_with` records detected contradictions; edges into `disputed` units are frozen — exempt from suspect-clearing — until adjudication. Coverage math reports `planned` scope separately and never penalizes it.
```

— with:

```
*(Brownfield amendment:)* spec units additionally carry a lifecycle status (`planned` | `disputed`; `ratified` is the absent default, `retired` a tombstone), and a `disputed` unit names the other anchor of its pair in a `disputes` field. *Specified, not built: the pairing is a unit field and not yet a spec↔spec `conflicts_with` edge; edges into `disputed` units are not frozen — suspect detection reads only the pinned revision (`index.rs:118-131`); and no coverage math over spec units exists, so `planned` scope is neither reported separately nor penalized.*
```

This keeps the three states that are built, states the two that are not in the
document's own established «Specified, not built» form, and corrects the shape of
the third rather than deleting it. **Note for the boss:** if the owner adopts
option (i) on F-121 (demote the closing rules rather than legalise the
annotation), this correction must be re-cast, because it *uses* the annotation
form F-121's rule forbids. The two obligations are coupled and should be
presented together.

For `##PHASE-0-ACCEPTANCE`: **none — the fact is correct as written**, with the
CI half standing as a §3.6(c) marked exception.

**Re-read under the ruling:** both outcomes unchanged; the FALSE one gets
**stronger** and the SURVIVES one is untouched.

`##EDGE-MODEL-EDGES` is decided entirely inside the package's own engine crates —
`generated/specmap/mod.rs`, `mdspec.rs`, `index.rs`, `explain.rs`, `ratchet.rs` —
plus the Rust stack's CLI, which ruling point 2 makes a genuine consumer. Nothing
rests on host adoption.

`##PHASE-0-ACCEPTANCE`'s decisive evidence is `mdspec.rs:8-9`, the package's own
scanner citing this exact acceptance clause as the reason for its behaviour —
point 3's bench. **The CI half moves from «marked exception» to «doubly not
drift».** It was already a §3.6(c) marked exception on the host's recorded no-CI
decision; ruling point 1 adds that «CI job wired but non-blocking» describes what
an **adopting project** wires, so *this* host having no CI is weak-to-void
evidence about the package in the first place. An external adopter with CI would
satisfy the clause unchanged. `terraform/REPORT.md:16` stays as corroboration
that one consumer discharged the acceptance, not as the proof.

**Recommendation per anchor:**
`##EDGE-MODEL-EDGES` → **drift stands, correction prepared**; two clauses unbuilt,
one built as a field rather than an edge — the verdict's «doc comment only» is
itself wrong and the prepared text corrects it.
`##PHASE-0-ACCEPTANCE` → **re-judge confirmed**; the scanner cites this clause by
name at `mdspec.rs:8-9` as the reason for the behaviour the verdict calls a
contradiction, and `terraform/REPORT.md:16` books the acceptance as met.

---

## F-159 — five for five, and the engine's own module header contradicts the spec in one line

**Outcome:** SURVIVES — 5 of 5
**Anchors:** 5 of 5, each SURVIVES
- `#ENTRY-CARRIES-ITS-PROVENANCE-FIELDS` · `#GC-IS-LRU-WITH-A-PIN-SET` ·
  `#TELEMETRY-FEEDS-THE-HEADLINE-METRIC` ·
  `#RELEASE-SLICE-IS-EXPORTED-SIGNED-AND-SHIPPED` ·
  `#FAILURE-KEY-UNDER-SPECIFICATION` — all in `spec/mechanisms/LEDGER-INTENT-v0.1.md`

**Perimeter searched — deliberately wider than every verdict here used.** Each of
these five verdicts says «anywhere in the crate» or «over the crates», singular
scope. §3.7 and the wave-6 extension both say that is the failure mode, so this
entry re-measured over **four layers**: the ENGINE
(`core-ai-native/v0.8.0/crates/`), the DRIVER
(`rust-ai-native-lang/v0.7.0/crates/`, which turns out to carry its **own**
`src/ledger.rs` — 199 lines — that a crate-scoped search would have missed), the
HOST (`crates/`, `xtask/`), and the DEPLOYMENT (`.ledger/` live on disk,
`terraform/REPORT.md`). Terms: `confidence`, `created_at`, `model_id`,
`prompt_rev`, `cost`, `QueryKind`, `enum Query`, `gc`, `garbage`, `lru`,
`pin_set`, `evict`, `prune`, `size_budget`, `release_slice`, `ed25519`,
`minisign`, `gpg`, `sign_`, `signature`.
**The verdict's own command, re-run:** the verdicts describe greps rather than
quoting them; every one reproduces on the **wider** perimeter, which is the
outcome that matters — a `not-found` that survives a perimeter it did not need.

**The DRIVER-layer file that could have overturned this and does not.**
`packages/org.vibevm.ai-native/rust-ai-native-lang/v0.7.0/crates/rust-ai-native-cli/src/ledger.rs`
is a **different ledger** and must not be mistaken for this one — its own header
(`:1-8`) says it renders *«the two BROWNFIELD §3 registries: `discipline/DEBT.md`
from `debt.json` … and `discipline/INTENT.md` from `intent.json`»*. It is the
registry view, not the LEDGER-INTENT interpretation cache. Named here because it
is exactly the artefact a §3.7-aware reader would expect to rescue these five,
and it does not.

**What the measurement shows, per anchor:**

**`##ENTRY-CARRIES-ITS-PROVENANCE-FIELDS`** (`LEDGER-INTENT-v0.1.md:61`) — *«Each
entry carries `{producer, model_id?, prompt_rev?, inputs (hashes + spec URIs ~r),
epoch, cost, created_at, confidence}`»*. There is no entry type. The store writes
the render itself:

```console
$ sed -n '155p' packages/org.vibevm.ai-native/core-ai-native/v0.8.0/crates/core-ai-native-specmap/src/ledger.rs
    std::fs::write(&slot, &text).with_context(|| format!("writing {}", slot.display()))?;
```

`text` is the prose string; the slot is `.ledger/objects/<sha[0..2]>/<sha>`. The
only structs in the module are `Telemetry` (`:83-88`) and `ProseRender { text,
cached, epoch }` (`:113-117`), neither of which is a stored entry. Of the eight
named fields, **three survive only as ingredients of the cache key** and five
have no representation at all:

```console
$ sed -n '132,136p' .../core-ai-native-specmap/src/ledger.rs
    const PRODUCER: &str = "explain.item/prose-template-1";
    let subgraph = crate::explain::explain_json(map, target)?;
    let subject = serde_json::to_string(&subgraph)?;
    let epoch = epoch(root);
    let key = content_hash(&format!("{PRODUCER}\n{}\n{subject}", epoch.0));
```

`producer`, `epoch` and `inputs` are hashed **into the key** — which means they
are unreadable from the entry, the opposite of «each entry carries». Over all
four layers: `created_at` **0**, `model_id` **0**, `prompt_rev` **0**;
`confidence` **1** (an unrelated doc comment in `crates/progress-core/src/baseline/project.rs:19`);
`cost` **3**, all of them doc comments inside `ledger.rs` itself (`:16`, `:77`,
`:81`) and never a field.

**`##GC-IS-LRU-WITH-A-PIN-SET`** (`:70`) — over all four layers: `lru` **0**,
`pin_set` **0**, `size_budget` **0**. `evict` returns 7 hits, every one in
`crates/vibe-index/src/server/rate_limit.rs` (HTTP rate-limit bucket eviction);
`gc` returns 9, every one `vibe self gc` — the PROP-019 version-manager instance
pruner (`crates/vibe-cli/src/commands/vvm/remove.rs:1-3`). Neither touches
`.ledger/`. **Nothing evicts a ledger object, ever**, which is visible on disk:
`.ledger/objects/` holds exactly 1 object today.

**`##TELEMETRY-FEEDS-THE-HEADLINE-METRIC`** (`:72`) — the fact names four
measures: hit rate, **cost per query kind**, **LLM-$ per merged change**,
contextual-rot rate. Two exist:

```console
$ sed -n '83,88p' .../core-ai-native-specmap/src/ledger.rs
pub struct Telemetry {
    pub hits: u64,
    pub misses: u64,
    pub rot_checks: u64,
    pub rot_changed: u64,
}
$ cat .ledger/telemetry.json
{ "hits": 1, "misses": 1, "rot_checks": 0, "rot_changed": 0 }
```

No cost field, and — a point the verdict did not make and which strengthens it —
a *per query kind* breakdown is unreachable in principle here, because
`ledger.rs:7` says *«One query kind ships: `explain.item`»*. The consumer's own
close-out already booked the same gap: `terraform/REPORT.md`'s metrics table,
row **«LLM-$ per merged change | n/a | not instrumented — no `vibe-llm` runtime
exists»**. The struct's doc comment at `:77` still asserts *«hit rate and cost
feed the Charter's headline metric»* over a struct with no cost field, so the
engine carries the same claim the spec does and neither is true.

**`##RELEASE-SLICE-IS-EXPORTED-SIGNED-AND-SHIPPED`** (`:85`) — *«At tag time, a
frozen subset … is exported, **signed**, and shipped with the package»*. The
implementing module's header says the opposite in one line:

```console
$ sed -n '15,18p' .../core-ai-native-specmap/src/ledger.rs
//! Storage: `.ledger/objects/<sha256[0..2]>/<sha256>` plus
//! `.ledger/telemetry.json` (hit rate, cost, rot-rate plumbing).
//! Local per checkout; never shipped, never signed, never exposed —
//! `.ledger/` is git-ignored.
```

And it is git-ignored, verifiably:

```console
$ git check-ignore -v .ledger/telemetry.json
.gitignore:50:/.ledger/	.ledger/telemetry.json
```

with `.gitignore:49` naming the reason — *«The local intent ledger (LEDGER-INTENT
v0.1) — derived interpretations cache, never shipped.»* Over the engine, the
host's `crates/` and `xtask/`: `release_slice` **0**, `ed25519` **0**, `minisign`
**0**, `gpg` **0**, `sign_`/`signature` **0** in any cryptographic sense (the
only hits are `render signature` / `DFS signature` in doc comments). **This is
the strongest of the five**: the spec, the engine header and the repository's own
ignore file give three different answers, and two of them agree against the spec.

**`##FAILURE-KEY-UNDER-SPECIFICATION`** (`:95`) — *«query kinds are a closed enum
with reviewed key schemas; adding a kind is a PR, not a string.»* Over all four
layers `QueryKind` returns **0** and `enum Query` returns **0**. The one kind is
a string constant declared inside a function body (`ledger.rs:132`, quoted
above) — which is not merely «not an enum» but the precise failure the fact
claims to have designed out: adding a second kind today *is* adding a string.

**`refs/**`, reported separately:** `grep -rn 'LEDGER-INTENT' refs/` returns **2**
hits, both in the same third-party transcript as F-120's (`refs/ts/talk.json`),
which merely names the document while summarising the tree. Not an
implementation, not counted. The `lru` / `evict` sweeps over `refs/` return
third-party cache code (`refs/src/uv`, `refs/src/cargo`), also not ours.

**Proposed correction (NOT APPLIED).** All five are one document and should move
as one diff. The honest form follows this document's existing convention (three
«Specified, not built» annotations already live in it at `:38`, `:57`, `:78`), so
the correction **annotates rather than deletes** — and, as with F-207, this is
coupled to F-121: if the owner picks option (i) there, these must be re-cast as
marker demotions instead.

*`:61`* — append: *«Specified, not built: no entry type exists. The stored object
is the render text itself (`ledger.rs:155`); `producer`, `epoch` and `inputs` are
hashed into the key (`:136`) rather than carried on the entry, and `model_id`,
`prompt_rev`, `cost`, `created_at` and `confidence` have no representation in any
layer.»*

*`:70`* — append: *«Specified, not built: no eviction of any kind exists — no
LRU, no pin set, no size budget. Ledger objects accumulate without bound.»*

*`:72`* — replace the measure list with *«hit rate and the **contextual-rot
rate**»*, then append: *«Specified, not built: cost per query kind and LLM-$ per
merged change have no field — `Telemetry` is `{hits, misses, rot_checks,
rot_changed}` (`ledger.rs:83-88`) — and with one query kind shipping, a per-kind
breakdown has nothing to break down.»*

*`:85`* — append: *«Specified, not built: the ledger is local per checkout and
git-ignored (`.gitignore:50`); nothing exports, freezes, signs or ships a slice,
and no signing path exists anywhere in the tree.»*

*`:95`* — append: *«Specified, not built: query kinds are not an enum. The one
kind is a string constant in a function body (`ledger.rs:132`), which is the
under-specification this entry claims to have closed.»*

**Re-read under the ruling:** all five outcomes unchanged; two citations demoted.

Every one of the five is decided by **the package's own `ledger.rs`** — no entry
type (`:155`), the key composition (`:136`), the `Telemetry` struct (`:83-88`),
the module header (`:17`), the `const PRODUCER` string (`:132`). That is ruling
point 3's bench, and it is sufficient on its own for all five.

Two host observations are therefore demoted to corroboration: `.ledger/objects/`
holding one uncollected object, and `.gitignore:49-50`. Both are «this host's
deployment», which ruling point 1 makes weak evidence about an externally-aimed
package — though neither was load-bearing, since the engine contains no eviction
code and the module header already says «never shipped, never signed» in the
package's own words. `terraform/REPORT.md`'s «LLM-$ … not instrumented» row is
corroboration for the same reason. The DRIVER-layer file I checked and rejected
(`rust-ai-native-cli/src/ledger.rs`) is Rust-stack and counts fully under point
2 — it simply turns out to be a different ledger.
`##ENTRY-CARRIES-ITS-PROVENANCE-FIELDS` — 3 of 8 fields survive only inside the
cache key, 5 have no representation on any layer.
`##GC-IS-LRU-WITH-A-PIN-SET` — zero eviction machinery on four layers; 1 object
on disk, never collected.
`##TELEMETRY-FEEDS-THE-HEADLINE-METRIC` — 2 of 4 measures; the consumer booked
the same gap at `terraform/REPORT.md`.
`##RELEASE-SLICE-IS-EXPORTED-SIGNED-AND-SHIPPED` — contradicted by
`ledger.rs:17` and by `.gitignore:49-50`.
`##FAILURE-KEY-UNDER-SPECIFICATION` — `QueryKind` 0, `enum Query` 0; the kind is
the string the fact forbids.

---

## F-146 — five survive, and two fall to the DRIVER layer; one of the two is falsified by the verdict's own evidence ref

**Outcome:** MIXED — 5/7 SURVIVE, 2/7 FALSE
**Anchors:** 7 of 7, each with its own outcome
- `#ROW-FRONTEND-TS-JS` → **SURVIVES** (T-syn column wrong; T-sem column exact)
- `#ROW-FRONTEND-PYTHON` → **SURVIVES** (whole row absent)
- `#FRONTEND-CRASH-DEGRADES-VISIBLY-NEVER-SILENTLY` → **SURVIVES** (the guarantee is kept by a different and stronger mechanism than the one named)
- `#RULES-ARE-RUST-TRAIT-IMPLS-COMPILED-IN` → **SURVIVES** (shape right, signature wrong)
- `#FINDINGS-CARRY-THE-A1-CHAIN` → **SURVIVES** (3 of 5 links)
- `#DETERMINISM-TESTED-BY-RUN-TWICE-DIFF` → **FALSE**
- `#GATE-EXIT-CODE-IS-THE-ACCEPTANCE-CRITERION` → **FALSE**

**Perimeter searched:** the ENGINE (`core-ai-native/v0.8.0/crates/`), **all three
DRIVER stacks** (`rust-ai-native-lang/v0.7.0/`, `typescript-ai-native-lang/v0.6.0/`,
`go-ai-native-lang/v0.1.0/` — including their `crates/`, their `tools/` sidecars
and their `tests/`), and the HOST (`crates/`, `xtask/`). Terms: `tree.sitter`,
`swc`, `rustpython`, `symtable`, `frontend unavailable`, `skipped (`,
`deviation-acknowledged`, `in_deviation`, `twice`, `findings_a`/`findings_b`,
`byte_stable`, `deterministic`, `--scope`, `--baseline`, `sarif`.
**Two of these seven verdicts confine themselves to «the crate» or «the engine»,
and both are the two that fall.** That is the wave-6 pattern with no ambiguity:
`core-ai-native` ships **library crates with no binary**, so anything that
requires *running the thing* — an end-to-end determinism test, a gate command —
can only exist one layer over, in the driver. Searching the engine for it is
searching where it structurally cannot be.

**What the measurement shows:**

### The two that are FALSE

**`##DETERMINISM-TESTED-BY-RUN-TWICE-DIFF`** (`ENGINE-CONFORM-v0.1.md:96`) —
*«Tested the way vibevm tests its resolver and codegen: run twice, diff.»* The
verdict: *«the crate carries 30 tests and none of them runs anything twice: grep
for twice / double-run / same_inputs over the crate returns 0. The claim that it
is tested survives only as a doc comment.»*

**It fails on the verdict's own perimeter, and the verdict attached the
falsifying evidence.** Its first two evidence refs are `sarif.rs:82
fn sarif_is_byte_stable()` and `sarif.rs:99 assert_eq!(a, b)`. Read together:

```console
$ sed -n '81,101p' packages/org.vibevm.ai-native/core-ai-native/v0.8.0/crates/core-ai-native-conform/src/sarif.rs
    #[test]
    fn sarif_is_byte_stable() {
        …
        let findings = check(&[&gate], &facts, None);
        let a = sarif::render(&[&gate], &findings);
        let b = sarif::render(&[&gate], &findings);
        assert_eq!(a, b);
        assert!(a.contains("\"ruleId\": \"unsafe-gate\""));
    }
```

Render, render again, diff — *inside the crate the verdict searched*. The grep
returned 0 because it searched for the words `twice` / `double-run` /
`same_inputs`, and the test is named `sarif_is_byte_stable`. §3.7's rule is
**search for the thing, not for the string the verdict used**, and this is that
rule failing at the smallest possible scale.

The **whole-pipeline** version — the one the fact actually promises — is in the
driver, and it too is a verdict evidence ref (`tests/engine.rs:123`):

```console
$ sed -n '103p;121,124p' packages/org.vibevm.ai-native/rust-ai-native-lang/v0.7.0/crates/rust-ai-native-conform-frontend/tests/engine.rs
fn findings_and_sarif_are_deterministic_and_baseline_gates() {
    let (findings_a, sarif_a) = run();
    let (findings_b, sarif_b) = run();
    assert_eq!(findings_a, findings_b);
    assert_eq!(sarif_a, sarif_b, "same inputs — byte-identical SARIF");
```

An integration test that runs the extraction-plus-rules-plus-SARIF pipeline
twice and diffs both outputs. A third instance sits in the TypeScript stack:
`typescript-ai-native-specmap/tests/gate.rs:17`
`fn clean_fixture_check_is_byte_stable_and_gate_green()`. **Three run-twice-diff
tests, one of them in the crate the verdict declared empty.**

*The near-miss is worth recording in full, because it is the cheapest kind of
error to avoid.* The verdict greps for `same_inputs`. Line 124 carries the
assertion message **`"same inputs — byte-identical SARIF"`** — the same two words
with a space instead of an underscore, on the line immediately after the one the
verdict cites as evidence.

**`##GATE-EXIT-CODE-IS-THE-ACCEPTANCE-CRITERION`** (`:98`) — *«Gate command:
`conform check --baseline conform-baseline.json --scope crates/vibe-resolver` —
exit code is the acceptance criterion … no human judgment in the loop (A3).»*
The verdict: *«the command as written cannot be run … the language stacks'
`<lang>-ai-native conform check` takes only `--path`.»*

That last clause is checkable and false. The Rust driver:

```console
$ sed -n '142,152p' packages/org.vibevm.ai-native/rust-ai-native-lang/v0.7.0/crates/rust-ai-native-cli/src/main.rs
enum ConformCmd {
    /// Extract facts, run the rules, fail on any new finding past the
    /// baseline.
    Check {
        /// Limit the gate to one crate by name.
        #[arg(long)]
        scope: Option<String>,
        /// The ratchet baseline file, project-relative.
        #[arg(long, default_value = rust_ai_native_cli::DEFAULT_CONFORM_BASELINE)]
        baseline: String,
    },
```

Both named flags, and the Go stack ships a **standalone binary whose name is the
family's `conform`**, with the third flag as well and the scope semantics spelled
out exactly as the document uses them:

```console
$ sed -n '24,33p' packages/org.vibevm.ai-native/go-ai-native-lang/v0.1.0/crates/go-ai-native-conform/src/main.rs
    Check {
        /// Project root.
        #[arg(long, default_value = ".")]
        path: String,
        /// Path to the frozen-findings baseline, root-relative.
        #[arg(long, default_value = go_ai_native_conform::DEFAULT_GO_BASELINE)]
        baseline: String,
        /// Report findings only under this root-relative path prefix.
        #[arg(long)]
        scope: Option<String>,
    },
```

*«Report findings only under this root-relative **path prefix**»* — so
`--scope crates/vibe-resolver`, the document's own example, is exactly the
accepted form. (The Rust driver's help text says «one crate by name», but it
passes the value straight through — `main.rs:222`
`rust_ai_native_conform::run_check(&root, &baseline, scope.as_deref())` →
`lib.rs:136` `check(&rule_refs, &facts, scope)` → `finding.rs:79`, documented
*«a repo-relative path prefix»*. The help text is imprecise; the behaviour is
the document's.)

And the exit-code half — the actual load-bearing claim — is literal:

```console
$ sed -n '172,174p' packages/org.vibevm.ai-native/rust-ai-native-lang/v0.7.0/crates/rust-ai-native-conform/src/lib.rs
    if !new.is_empty() {
        bail!("conform: {} new finding(s) against the baseline", new.len());
    }
```

Non-zero on any new finding, no human in the loop. The fact is correct in the
sense that matters; the only inexactness is the **bare** verb `conform check`
where each stack prefixes its family name, which is the neutral-spec convention
this document uses throughout and not a defect.

### The five that SURVIVE

**`##ROW-FRONTEND-TS-JS`** (`:55`) — the row's **T-sem** column is exact
(*«TypeScript compiler API via a Node sidecar process»*: the extractor is
`tools/ts-extract/`, `package.json:10` `"typescript": "^6.0.0"`, run through
`typescript-ai-native-conform-frontend`). The **T-syn** column names
`tree-sitter / SWC`, and over the whole tree — every package, `crates/`,
`xtask/`, `tools/`, all manifests, `.ts` included —
`tree.sitter` returns **0** and `\bswc\b` returns **0**. There is no syntactic
tier for TS/JS; the compiler API is the only frontend.

**`##ROW-FRONTEND-PYTHON`** (`:57`) — both columns absent: `rustpython` **0**,
`symtable` **0** over the same perimeter, and no Python stack exists (the three
shipped are Rust, TypeScript, Go).

**`##FRONTEND-CRASH-DEGRADES-VISIBLY-NEVER-SILENTLY`** (`:61`) — the *promise*
is kept; the *mechanism named* does not exist and has been replaced by a
stricter one. `frontend unavailable` returns **0** over every layer, and
`Finding` (`finding.rs:27-36`) carries no status field, so there is no report
line to degrade to. What happens instead is fail-fast:

```console
$ sed -n '65,70p' packages/org.vibevm.ai-native/typescript-ai-native-lang/v0.6.0/crates/typescript-ai-native-conform/src/lib.rs
    // Fail HARD on a broken toolchain before the gate can run on zero
    // facts — the bridge's taxonomy carries the fix surface.
    frontend
        .probe()
        .map_err(|e| anyhow::anyhow!(e.to_string()))?;
```

with the frontend's own header (`…-conform-frontend/src/lib.rs:47-50`)
explaining the policy: *«Failures surface on stderr here and again as an empty
fact set per file — the gate itself stays running (B5); the CLI drivers probe the
bridge FIRST so a broken toolchain is a hard error there, not a silent green
here.»* «Never silent» holds. «Degrades to `skipped (frontend unavailable)`,
visible in the report» does not — the implementation refuses to run rather than
degrading, which is the better behaviour and the wrong description.

**`##RULES-ARE-RUST-TRAIT-IMPLS-COMPILED-IN`** (`:84`) — the shape is right
(rules **are** Rust impls of one trait, compiled in) and the quoted signature is
wrong in both parameters:

```console
$ sed -n '51,56p' packages/org.vibevm.ai-native/core-ai-native/v0.8.0/crates/core-ai-native-conform/src/finding.rs
pub trait Rule {
    fn id(&self) -> &'static str;
    fn why(&self) -> &'static str;
    fn check(&self, facts: &[SourceFacts]) -> Vec<Finding>;
}
```

against the document's `fn check(&self, facts: &FactStore, specmap: &Index) ->
Vec<Finding>`. The `specmap` parameter is not merely absent but impossible: the
conform crate does not depend on the specmap crate — verified in
`core-ai-native-conform/Cargo.toml`, which lists no specmap dependency. The
trait also has two members the document omits (`id`, `why`).

**`##FINDINGS-CARRY-THE-A1-CHAIN`** (`:88`) — five links claimed, three present:

```console
$ sed -n '27,36p' packages/org.vibevm.ai-native/core-ai-native/v0.8.0/crates/core-ai-native-conform/src/finding.rs
pub struct Finding {
    pub rule: &'static str,
    pub file: String,
    pub line: u32,
    pub message: String,
    /// Why the rule exists — the axiom trace rendered into SARIF.
    pub why: &'static str,
    /// Stable identity for the baseline: `rule|file|carrier`.
    pub fingerprint: String,
}
```

rule id ✓, why ✓, span (`file` + `line`) ✓; **involved facts ✗**, **deviation
status ✗**. `deviation-acknowledged` returns **0** across every layer. The
deviation concept does exist one level down — `Fact` variants carry an
`in_deviation: bool` (`facts.rs:62`, `:89`, `:101`) and `budget.rs:19` says *«the
rule honors it»* — but it **suppresses** a finding at rule time rather than
**downgrading** an emitted one, so nothing is ever labelled
`deviation-acknowledged` and the chain's fifth link has no carrier.

**`refs/**`, reported separately:** `tree.sitter` matches **17** files under
`refs/` and `sarif` matches **2** — third-party study corpus in both cases, and
worth naming precisely because they are exactly the two terms this obligation
turns on: a reader who forgets the `refs/` split could read those 17 as evidence
that tree-sitter is in this tree. It is not, on our shipped surface, anywhere.

**Proposed correction (NOT APPLIED)** — for the five that survive only. Same
coupling note as F-207: this uses the «Specified, not built» form, which is
exactly what F-121's closing rule forbids, so the two must be decided together.

*`:55` — `##ROW-FRONTEND-TS-JS`, T-syn cell:*

```
tree-sitter / SWC (Apache-2.0) — *Specified, not built: there is no syntactic tier for TS/JS. `tree-sitter` and `swc` return no hit anywhere in the repository. The T-sem column is the only frontend that ships, and it is exact.*
```

*`:57` — `##ROW-FRONTEND-PYTHON`, whole row, matching the form already used on
`##ROW-FRONTEND-CPP` two rows up:*

```
| ##ROW-FRONTEND-PYTHON Python — *Specified, not built, in full: there is no Python frontend at either depth. `rustpython`, `symtable` and any CPython sidecar return no hit anywhere in the repository, and no Python stack exists; the three that ship are Rust, TypeScript and Go.* | RustPython parser (MIT) in-process | CPython `ast`/`symtable` via sidecar | PSF / MIT — clean |
```

*`:61`:* replace with — *«A frontend whose toolchain is broken is a **hard
error**: each stack's driver probes its frontend before extraction and fails the
run (`typescript-ai-native-conform/src/lib.rs:66-70`), so the gate can never
report green over zero facts. A per-file extraction failure surfaces on stderr
and yields an empty fact set for that file. Specified, not built: there is no
`skipped (frontend unavailable)` report status — `Finding` carries no status
field.»*

*`:84`:* correct the signature to
`fn check(&self, facts: &[SourceFacts]) -> Vec<Finding>` and note that the trait
also carries `fn id()` and `fn why()`; drop the `specmap: &Index` parameter,
which the crate's dependency graph forbids.

*`:88`:* append — *«Specified, not built: three of the five links ship —
`Finding` is `{rule, file, line, message, why, fingerprint}` (`finding.rs:27-36`).
There is no involved-facts field, and no deviation status: a `deviates` record
suppresses the finding at rule time (`in_deviation`, `facts.rs:62`) rather than
downgrading an emitted one, so nothing is ever labelled
`deviation-acknowledged`.»*

**Re-read under the ruling:** all seven outcomes unchanged, and the two FALSE
ones are **exactly what ruling point 3 predicts**.

Point 3 says the bench for the externally-aimed parts is the package's own
artefacts and **its tests** — and both falls here are tests and CLI surfaces
inside the package family: `sarif.rs:81-101` (a test in `core-ai-native`'s own
crate), `tests/engine.rs:121-124` (a test in the Rust stack), and the
`ConformCmd` / `go-ai-native-conform` flag definitions. Nothing rests on host
adoption.

**A caution the ruling forces, and it does not change the answer.** Point 2 says
Go and TypeScript are not host consumers, so an *absence observed in a host Go or
TypeScript deployment* would prove nothing. Three of the five survivors —
`##ROW-FRONTEND-TS-JS`, `##ROW-FRONTEND-PYTHON`,
`##FRONTEND-CRASH-DEGRADES-VISIBLY-NEVER-SILENTLY` — touch those languages, so
the distinction has to be drawn explicitly: **none of them rests on a host
deployment.** They rest on what the **package family ships** — whether a
tree-sitter or SWC frontend exists as a crate or a manifest dependency anywhere
(it does not), whether a Python stack package exists at all (it does not), and
whether `Finding` carries a status field (it does not). Those are package
artefacts under point 3, not deployments under point 1, and they would read the
same in any external consumer's tree.

**Recommendation per anchor:**
`##ROW-FRONTEND-TS-JS` → **drift stands, correction prepared** (T-syn only).
`##ROW-FRONTEND-PYTHON` → **drift stands, correction prepared** (whole row).
`##FRONTEND-CRASH-DEGRADES-VISIBLY-NEVER-SILENTLY` → **drift stands, correction
prepared**; the promise holds by fail-fast, the named mechanism does not exist.
`##RULES-ARE-RUST-TRAIT-IMPLS-COMPILED-IN` → **drift stands, correction
prepared**; signature only.
`##FINDINGS-CARRY-THE-A1-CHAIN` → **drift stands, correction prepared**; 3 of 5.
`##DETERMINISM-TESTED-BY-RUN-TWICE-DIFF` → **re-judge confirmed**; three
run-twice-diff tests, one of them at `sarif.rs:81-101` inside the crate the
verdict searched and cited.
`##GATE-EXIT-CODE-IS-THE-ACCEPTANCE-CRITERION` → **re-judge confirmed**; both
flags exist in every driver, `--scope` takes the path prefix the example uses,
and `lib.rs:172-174` makes the exit code the criterion.

---

## F-206 — both survive; the SARIF claim points the wrong way down a real pipe, and the closing rule is F-121's fourth instance with its deadline already fired

**Outcome:** SURVIVES — 2 of 2
**Anchors:** 2 of 2, each SURVIVES
- `spec/mechanisms/ENGINE-CONFORM-v0.1.md#FOREIGN-LINTERS-ARE-EVIDENCE-PROVIDERS` → **SURVIVES** (partial: the linters do run; nothing ingests them)
- `spec/mechanisms/ENGINE-CONFORM-v0.1.md#UNEXERCISED-FRONTEND-OR-TIER-IS-REMOVED` → **SURVIVES**

**Perimeter searched:** ENGINE, all three DRIVER stacks (including their
`floor.rs` step lists), and the HOST (`crates/`, `xtask/`). Terms: `sarif` in
every casing, `clippy`, `eslint`, `ruff`, `clang-tidy`, plus a read of every
`sarif` occurrence in the tree to separate **render** from **parse**. For the
closing rule, `terraform/` — the consumer's record of the phase it names.
**The verdict's own command, re-run:** the verdict describes a grep for
`clippy / eslint / ruff / clang-tidy` over the engine; it reproduces, and the
finding it reports (`#[allow(clippy::…)]` plus `eslint` as a floor step name at
`config.rs:180`) is exactly what is there.

**What the measurement shows:**

**`##FOREIGN-LINTERS-ARE-EVIDENCE-PROVIDERS`** (`ENGINE-CONFORM-v0.1.md:63`) —
*«clippy, eslint, ruff, clang-tidy run as-is; their output is ingested as facts
via **SARIF**.»* Two clauses, and they are in opposite states — which the
verdict's summary («the direction of the claim is the opposite of the direction
of the code») captures correctly but under-describes.

*Clause 1 is true.* The linters do run as-is, as named floor steps:

```console
$ grep -rn 'clippy\|eslint' .../rust-ai-native-cli/src/floor.rs .../typescript-ai-native-cli/src/floor.rs
rust-ai-native-cli/src/floor.rs:73:        "cargo clippy --workspace --all-targets -- -D warnings",
rust-ai-native-cli/src/floor.rs:87:    if !record(&mut outcomes, "clippy", ok) && !opts.keep_going {
typescript-ai-native-cli/src/floor.rs:30:    "eslint",
typescript-ai-native-cli/src/floor.rs:141:    if !is_disabled("eslint") {
```

*Clause 2 is false, and false in the exact direction the verdict names.* Every
`sarif` occurrence in the tree was read; **all of them are on the render side**.
`core-ai-native-conform/src/sarif.rs` exposes one public function —
`pub fn render(rules: &[&dyn Rule], findings: &[Finding]) -> String` (`:18`) —
and the drivers write its output to `target/conform/report.sarif`
(`rust-ai-native-conform/src/lib.rs:137-142`; the Go sibling at
`go-ai-native-conform/src/lib.rs:84-85`). **There is no SARIF parser, reader,
deserializer or ingest path anywhere in any layer.** conform *emits* the format
this fact says it *consumes*, so no clippy diagnostic ever becomes a `Fact`.

That gap is also already conceded one row up in the same document:
`##ROW-FRONTEND-GO`'s annotation says of `gopls` / `go vet` — *«Built, but at
another layer, and not as evidence providers. … Neither reaches conform —
conform ingests no output from either, so nothing they know becomes a fact.»*
The same sentence is true of clippy and eslint, and this anchor has not received
it.

**`##UNEXERCISED-FRONTEND-OR-TIER-IS-REMOVED`** (`:109`) — *«Any frontend or tier
specified here that is not exercised by Playbook Phase 4 is removed from this
document rather than carried as aspiration.»* This is **F-121's rule in a fourth
document**, and it falls the same way and for the same two reasons.

*The deadline fired.* Playbook Phase 4 is booked done by the consumer:

```console
$ sed -n '20p' terraform/REPORT.md
| 4 conform MVP | done | conform-core + conform-frontend-rust; 3 rules; SARIF; baseline (6 frozen unsafe); 1-file-diff = 1 re-extract proven; conform-lite retired |
```

*And the document carries more unexercised material than any of its siblings —
by its own admission:*

```console
$ grep -ci 'specified, not built' packages/org.vibevm.ai-native/core-ai-native/v0.8.0/spec/mechanisms/ENGINE-CONFORM-v0.1.md
8
```

Eight standing annotations, three of them **in the frontend table this very rule
governs** — `##ROW-FRONTEND-RUST`'s T-sem cell (*«the T-sem column names software
that is absent … `rust-analyzer`, `rustc_driver`, `ra_ap` and `hir` return no
hit»*), `##ROW-FRONTEND-CPP` (*«Specified, not built, in full: there is no C++
frontend at either depth»*), and `##ROW-FRONTEND-GO`'s T-sem cell. Add
`##ROW-FRONTEND-TS-JS`'s T-syn column and `##ROW-FRONTEND-PYTHON` (both from
F-146 above, both unannotated and both absent), and the tally is: **of five
frontend rows, one is fully built (Rust T-syn / `syn`), one is fully absent
(Python), and the other three are half-absent.** A rule that says an unexercised
frontend is removed, sitting under a table where four of five rows carry
unexercised cells, is the same self-falsification F-121 books three times.

**«Tier» is worse than unexercised — it does not exist in the code at all.** The
document's §3 sketch is:

```
trait Frontend {
    fn lang(&self) -> Lang;
    fn tier(&self) -> Tier;
    fn extract(&self, files: &[SourceFile]) -> Result<Vec<Fact>, FrontendError>;
}
```

The shipped trait is:

```console
$ sed -n '176,189p' packages/org.vibevm.ai-native/core-ai-native/v0.8.0/crates/core-ai-native-conform/src/facts.rs
pub trait Frontend {
    fn id(&self) -> &'static str;
    fn version(&self) -> &'static str;
    …
    fn extract(&self, file: &str, crate_name: &str, module: &str, text: &str) -> Vec<Fact>;
    …
    fn warm(&self, _pending_files: &[String]) {}
}
```

`enum Tier` and `enum Lang` both return **0** over the engine. The T-syn / T-sem
tier split is a **document-only taxonomy** with no representation in the type
system — which means the rule's own subject («any frontend **or tier** … is
removed») quantifies over a thing the engine cannot exercise even in principle.
*(This is a finding about the §3 trait sketch, which is a different anchor and
not in this batch; it is recorded because it sharpens this one and because the
boss may want it routed.)*

**`refs/**`, reported separately:** `sarif` matches **2** files under `refs/`,
third-party; `clippy` matches widely in the vendored Rust study corpus. Neither
is an ingest path of ours.

**Proposed correction (NOT APPLIED):**

*`:63` — replace:*

```
##FOREIGN-LINTERS-ARE-EVIDENCE-PROVIDERS **Foreign linters as evidence providers.** clippy, eslint, ruff, clang-tidy run as-is; their output is ingested as facts via **SARIF** (the OASIS static-analysis interchange format). @impl/done
```

*with:*

```
##FOREIGN-LINTERS-ARE-EVIDENCE-PROVIDERS **Foreign linters run alongside the gate.** clippy and eslint run as-is as floor steps (`rust-ai-native-cli/src/floor.rs:73`, `typescript-ai-native-cli/src/floor.rs:30`); ruff and clang-tidy have no stack. *Specified, not built: nothing ingests their output. conform **emits** SARIF 2.1.0 (`sarif::render`, the only public function of that module) and reads none — there is no SARIF parser in any layer — so no foreign diagnostic becomes a `Fact`. The same gap is recorded one row up for `gopls` / `go vet`.* @spec/done
```

*`:109`:* the same two options as F-121 — **(i)** demote `@impl/done` →
`@spec/done`, or **(ii)** legalise the annotation form — and **it must take
whichever answer F-121's three take.** This is one rule in four documents; a
repair in three of four is the `duplication` obligation the registry mints next
run. The `(ii)` text for this instance:

```
##UNEXERCISED-FRONTEND-OR-TIER-IS-REMOVED *Any frontend or tier specified here that is not exercised by Playbook Phase 4 is either removed from this document or annotated in place as **specified, not built** — never carried as unmarked aspiration.* @impl/done
```

**Re-read under the ruling:** both outcomes unchanged; the second is re-ordered
the same way F-121's three were.

`##FOREIGN-LINTERS-ARE-EVIDENCE-PROVIDERS` is decided inside the package:
`sarif.rs` exposes exactly one public function, `render`, and no layer contains a
SARIF parser. The floor-step citations (`floor.rs` in the Rust and TypeScript
stacks) are **package artefacts** — the shipped floor definitions, not this
host's practice — so point 2's Go/TypeScript caution does not reach them.

`##UNEXERCISED-FRONTEND-OR-TIER-IS-REMOVED` now **leads with the eight «Specified,
not built» annotations in this document**, three of them inside the frontend
table the rule governs, plus the fact that `enum Tier` and `enum Lang` do not
exist in the engine at all. The `terraform/REPORT.md:20` Phase-4 deadline is
demoted to corroboration for the reason given under F-121: a different consumer's
Phase 4 is a different date, and this package is written for consumers we cannot
see.

**Recommendation per anchor:**
`##FOREIGN-LINTERS-ARE-EVIDENCE-PROVIDERS` → **drift stands, correction
prepared**; clause 1 true, clause 2 inverted — SARIF is rendered, never parsed,
on every layer.
`##UNEXERCISED-FRONTEND-OR-TIER-IS-REMOVED` → **drift stands, correction
prepared, decided together with F-121's three**; deadline fired at
`terraform/REPORT.md:20`, eight live annotations in this document, and «tier» has
no type in the engine at all.

---

## F-132 — the batch's biggest obligation, worked after the owner ruling landed: four false, one route (b), nine survive

**Outcome:** MIXED — 4/14 FALSE, 1/14 SURVIVES BUT IS NOT THE PACKAGE'S DEFECT (route (b)), 9/14 SURVIVE
**Anchors:** 14 of 14, each with its own outcome (all in
`spec/mechanisms/PROP-014-specmap-bidirectional-traceability.md`)

| anchor | outcome |
|---|---|
| `##CONSEQUENCE-M1-5-CONVERGENCE` | **FALSE** — a future-conditional convicted for its antecedent not having fired |
| `##RUNTIME-TRANSPORT` | **FALSE** — the tools exist, in the discipline MCP packages the search never entered |
| `##DIFFERENTIATOR-CONSUMED-AT-RUNTIME` | **FALSE** — same evidence; runtime agent consumption is an MCP tool |
| `##EDGE-MODEL-DIRECTION-OF-AUTHORITY` | **FALSE** — the `proposed` pool is a file by design, and the file exists with 54 entries |
| `##RUST-PRINCIPLE-GENERATOR-INPUT-IS-TAGGED` | **SURVIVES — ROUTE (b)** |
| `##DISTRIBUTION-RIDES-THE-EXISTING-REGISTRY` | SURVIVES |
| `##RULE-MULTIPLICITY-LINT` | SURVIVES |
| `##EDGE-MODEL-NODES` | SURVIVES |
| `##QUERY-ERROR-PROVENANCE` | SURVIVES (mechanism absent; the practice it exists for is live and tested) |
| `##LLM-AS-RENDERER` | SURVIVES |
| `##RUNTIME-PROFILES` | SURVIVES |
| `##SPEC-PRINCIPLE-UNITS-FIT-A-PAGE` | SURVIVES |
| `##RUST-PRINCIPLE-RUSTDOC-IS-THE-DETAIL-LAYER` | SURVIVES |
| `##ROW-PRIOR-ART-SYN-TREE-SITTER` | SURVIVES (half — `syn` is real, tree-sitter is not) |

**Perimeter searched:** the standing perimeter, and for this obligation
specifically **all four `-mcp` packages** (`core-ai-native-mcp` inside the subject
itself, plus `rust-`, `typescript-` and `go-ai-native-mcp`), all three `-lang`
stacks, the host's `crates/` and `xtask/`, `schemas/`, and `terraform/`. Terms:
`specmap_query`, `specmap_explain`, `specmap_source`, `trace_explain`,
`EdgeProvenance`, `Proposed`, `Generated`, `multiplicity`, `max_edges`,
`edge_budget`, `more than 3`, `120`, `metamodel`, `tree.sitter`, `violates spec`,
`vibe-llm`, `ErrorVariant`, `Command`, `content_hash`, `/generated/`,
`fetch.*fragment`.
**The verdict's own commands, re-run:** two are quoted and both reproduce as
strings — `specmap_query|specmap_explain|specmap_source` over the tree returns
**0**, and `tree-sitter` in any `Cargo.toml`/`package.json` returns **0**. The
first one reproducing is *why* it is wrong: it searched for the names, not the
thing, and the thing ships under another name in another package.

**This entry was measured after the owner ruling arrived**, so points 1–3 are
applied inline rather than in a re-read note.

### The four that are FALSE

**`##RUNTIME-TRANSPORT`** (`:239`) — *«`vibe-mcp` (M1.7) gains tools:
`specmap_query(query) -> subgraph`, `specmap_explain(target, format)`,
`specmap_source(content_hash) -> fragment` … An agent that drives `vibe` as a
CLI gets the same via `vibe explain --json`.»* The verdict: *«vibe-mcp gained no
such tools: grep for specmap_query / specmap_explain / specmap_source **over the
host's crates** returns 0.»*

**The host's crates are the wrong project, and this is the wave-6 error in its
purest form.** The discipline's MCP surface is not in `vibe-mcp` — PROP-028 put it
in family-prefixed **packages**, three of them, and the subject package itself
authors the neutral server crate they build on (`core-ai-native-mcp`, 1 035 lines).
Their declared inventory:

```console
$ sed -n '48,67p' packages/org.vibevm.ai-native/rust-ai-native-mcp/v0.7.0/crates/rust-ai-native-mcp/src/lib.rs
pub const TOOL_NAMES: [&str; 18] = [
    "codemod_add_cell", "conform_check", "conform_freeze", "fast_loop",
    "floor", "health", "init", "ledger_render",
    "specmap_check", "specmap_write",
    "tcg_bench", "tcg_complete", "tcg_scope", "tcg_type", "tcg_validate",
    "test_gate", "trace_explain", "tripwire",
];
```

Eighteen tools, three of them specmap tools, and the middle one of the fact's
three is there under the CLI's name for it:

```console
$ sed -n '206,216p' .../rust-ai-native-mcp/src/tools_discipline.rs
        t(
            "trace_explain",
            "Explain one symbol or spec unit through the index: what implements, \
             verifies, documents it. = `rust-ai-native trace <target>`.",
            properties! {
                "target": {"type": "string", "description": "a symbol or spec:// URI (required)"},
                "json": {"type": "boolean"},
                "prose": {"type": "boolean"},
            },
```

`trace_explain(target, {json|prose})` **is** `specmap_explain(target, format)`,
argument for argument. It is registered in all three MCP packages
(`typescript-ai-native-mcp/.../tools_discipline.rs`,
`go-ai-native-mcp/.../lib.rs`) and pinned by a test that asserts `tools/list` is
exactly the declared inventory (`lib.rs:76`,
`fn the_tool_set_is_exactly_the_declared_inventory`). That test is ruling point
3's bench: the package's own tree, exercised by the package's own test.

The fact is not wholly right — `specmap_source(content_hash) -> fragment` has no
tool, and there is no general `specmap_query`; the closest is `trace_explain`'s
`--json` subgraph. But the verdict's claim is *«gained no such tools»*, and the
transport, the server crate and two of the three capabilities ship. The residue
is a naming-and-one-missing-tool correction, not the absence the verdict records
— and it is what the prepared text below says.

**`##DIFFERENTIATOR-CONSUMED-AT-RUNTIME`** (`:403`) — *«the map is consumed at
runtime by agents using the tool, not only at audit time»*. The verdict rests
entirely on `##RUNTIME-TRANSPORT`: *«the only channel is a CLI flag — the runtime
tools that would make it true are the ones RUNTIME-TRANSPORT also claims and that
do not exist»*. They exist. **An MCP tool an agent calls mid-session is the
definition of runtime consumption by an agent**, and there are three of them
(`trace_explain`, `specmap_check`, `specmap_write`) in three servers. This is
also §3.7's corollary again — one premise, propagated to a second verdict, both
falling together.

**`##EDGE-MODEL-DIRECTION-OF-AUTHORITY`** (`:200`) — *«…plus one social channel: a
`proposed` edge pool (§4, Phase 2) feeding the Sync-from-Code protocol»*. The
verdict: *«the `proposed` pool that feeds Sync-from-Code is an enum variant
nothing constructs: every edge in the host's 912 is authored»*.

**The pool is not supposed to be an edge in the index, and this document says so
two hundred lines earlier.** `##LLM-AS-PROPOSER` (`:233`): *«Link mining (Phase 2)
produces edges with provenance `proposed`, stored in `specmap-proposals.json`,
**never in code**. A human … *affirms* a proposal by writing the actual `#[spec]`
attribute — the affirmation IS the code change.»* An affirmed proposal becomes an
**authored** edge by construction. So «every edge in the index is authored» is not
evidence against the pool — **it is the pool working as specified.**

And the pool exists, as the file the design names:

```console
$ ls -l terraform/specmap-proposals.json
-rw-r--r-- … 40058 … terraform/specmap-proposals.json
$ python -c "import json;d=json.load(open('terraform/specmap-proposals.json',encoding='utf-8'));print(list(d.keys()))"
['schema', 'scope', 'note', 'mined_commits', 'proposals', 'candidate_orphans']
```

with `mined_commits` and `candidate_orphans` — the two artefacts
`##PHASE-2-LATENT-CORPUS-MINING` and `##PHASE-2-CRATE-SWEEP-PROMPT` specify.
Wave 5 already confirmed the loop end-to-end with **54 owner-approved proposals**
(F-139), and `terraform/REPORT.md:18` books Phase 2 on them. The `Proposed` enum
variant being unconstructed in the *index* is the design, not the defect.

**`##CONSEQUENCE-M1-5-CONVERGENCE`** (`:52`) — read it whole: *«**M1.5
convergence.** **Once** `vibe build` generates code from specs, the generator
emits specmap edges as a true compiler byproduct — the analogy becomes literal.
Hand-authored tags remain as the human-override lane. **This PROP defines the
format that the future generator will target.**»*

The verdict: *«nothing constructs `EdgeProvenance::Generated` … The convergence
this fact marks @status:impl/done is **conditional on** a `vibe build` that does not
generate code yet.»* **The verdict states the sentence's own antecedent and files
it as the defect.** «Once X, then Y» is not falsified by ¬X; the sentence
explicitly calls the generator *future* and claims only that the format is
defined for it. And the format *is* defined — `EdgeProvenance::Generated` exists
as a variant (`generated/specmap/mod.rs:60`), which is precisely «this PROP
defines the format that the future generator will target», discharged.

*What may remain is a marker question, not a prose one.* `@impl/done` on a
sentence about a future milestone is arguable, but that is a different defect on a
different route, and `reality-mismatch`/`sync-from-code` is not the instrument for
it. Recorded for the boss rather than acted on.

### The one that survives but is not the package's defect

**`##RUST-PRINCIPLE-GENERATOR-INPUT-IS-TAGGED`** (`:284`) — *«Generated code is
excluded; its generator input is tagged. Schema files and macro definitions carry
the edges; expansion output is marked generated.»*

The verdict grants half and rejects half: *«the exclusion of generated code is
real; the tagging of generator input is not — **the host's schemas carry no spec
tags**, so the edges the generated code would inherit do not exist.»*

Both halves check out as measurements. Exclusion is real and doubled:

```console
$ sed -n '313,315p' .../core-ai-native-specmap/src/rscan.rs
            // Generated trees are excluded from scanning wholesale.
            if fwd(rel_in_crate).contains("/generated/") {
                continue;
$ sed -n '81,83p' .../core-ai-native-specmap/src/ratchet.rs
            if fwd(rel_in_crate).contains("/generated/") {
                continue;
```

And `schemas/specmap.jtd.json` carries `spec://` only inside two `description`
strings (`:17`, `:23`) — prose about the URI format, not a tag. So the host's
schemas are untagged.

**But this is a rule addressed to the adopting project, and the only evidence
against it is that this host does not keep it** — which is exactly ruling point
1. A `rust-ai-native` principle telling a project to tag its schema files and
macro definitions is not falsified by *this* project not having done so; an
external adopter tags its own. Under §3.6(b) the rule is sound and the consumer
does not keep it, so **the package does not move** and the compliance work is a
host obligation.

*Two things push the same way.* The macro half is **kept** in the package's own
tree — `core-ai-native-specmark/src/lib.rs` is the macro definition crate, and its
`spec` attribute macro carries the grammar and worked examples at `:13`, `:62`,
`:71-78`. And this repository is a genuine AI-Native **Rust** consumer under
ruling point 2, so its untagged `schemas/` is a real compliance gap worth
booking — as a **host** obligation, in
`PHASE-D-HOST-OBLIGATIONS.md`, not as a package edit.

### The nine that SURVIVE

Each is decided on the **package's own tree** (ruling point 3) or on the Rust
stack (point 2). None rests on a Go or TypeScript deployment, and none rests on a
skills directory or on `legacy-spec/`.

**`##DISTRIBUTION-RIDES-THE-EXISTING-REGISTRY`** (`:58`) — *«the metamodel index
ships with the package; source fragments are fetched by content hash.»* No
package's `vibe.toml` lists `specmap.json` in a payload —
`grep -rn 'specmap.json' --include=vibe.toml packages/` returns nothing — and the
only `specmap.json` files under `packages/` are four extractor **test fixtures**
and the `fractality` specspace's own project index, i.e. a consumer's artefact,
not a shipped package payload. Clause two has no implementation at all: no
fetch-by-content-hash path exists (`content_hash` is a hashing function, never a
retrieval).

**`##RULE-MULTIPLICITY-LINT`** (`:190`) — *«An item carrying more than 3 spec
edges is flagged by `vibe check`.»* Searched for the **thing**, not the string:
`multiplicity`, `max_edges`, `edge_budget`, `more than 3` over every package,
`crates/`, `xtask/` — **0**. No checker in any layer counts edges per item.

**`##SPEC-PRINCIPLE-UNITS-FIT-A-PAGE`** (`:272`) — *«Soft target ≤ 120 lines per
unit; `vibe check` warns beyond.»* `120` over the engine, the Rust stack and
`crates/vibe-check/` returns only two unrelated doctest literals in
`conform/src/rules/budget.rs`. There is a file-length budget rule for **code**;
there is no length warning for **spec units** anywhere.

**`##EDGE-MODEL-NODES`** (`:198`) — *«`SpecUnit { uri, kind, r, content_hash }`,
`CodeItem { symbol_path, item_kind, crate, content_hash }`, plus derived
`Command`, `ErrorVariant` views.»* `CodeItem` in the package's own wire type
(`generated/specmap/mod.rs:34-52`) is `{ crateName, file, itemKind, line, symbol }`
— **no content hash**, which is exactly what the document itself already concedes
elsewhere (`:141`, `##INVALIDATION-CODE-CHANGE-KEEPS-EDGES-VALID`: *«`CodeItem` in the
committed index carries `crate_name`, `file`, `item_kind`, `line`, `symbol` and
**no content hash**»*). No `Command` node type exists.

*One precision the verdict missed and the correction must keep.* `ErrorVariant`
**does** exist — as a `Fact` variant in the **conform** engine
(`core-ai-native-conform/src/facts.rs:66`) with two rules over it in
`rules/diagnostics.rs`. That is a different graph from the specmap node model, so
the verdict's conclusion holds; but a repair written as «ErrorVariant does not
exist» would be false about a live type in a sibling crate of the same package.

**`##QUERY-ERROR-PROVENANCE`** (`:226`) — *«`vibe`'s error rendering looks up the
failing error variant **in the index** and appends `violates spec://…#req-… (r2)
— run: vibe explain <uri>`.»* The named mechanism is absent: no error rendering
performs an index lookup, and no rendered error carries a revision or a
`run: vibe explain` hint.

**But the verdict's flat *«No error rendering consults the map»* understates what
ships, and the correction must not delete it.** The package's own MCP crate
renders **every** transport error with spec provenance, and a doctest pins it:

```console
$ sed -n '14,20p;23,28p' .../core-ai-native-mcp/src/error.rs
/// let e = core_ai_native_mcp::McpCoreError::Io { … };
/// assert!(e.to_string().contains("MCP-CORE-v0.1#wire"));
    #[error(
        "stdio transport {op} failed: {source} \
         (violates spec://org.vibevm.ai-native/core-ai-native/mechanisms/MCP-CORE-v0.1#wire; \
          fix surface: the host closed the pipe — check the agent host's \
          server log, then the [[mcp_server]] command line it launched)"
    )]
```

with the module header stating the intent this fact is about — *«every variant
cites the MCP-CORE requirement it guards and names a fix surface, so a failing
server run is navigable without this crate's source»*. Conform does the same for
rule findings (`rules/mod.rs:49-50`, `req_message`). So *«every failure becomes a
doorway into the metamodel»* is **live and tested in the package**; the URIs are
compile-time constants rather than index lookups, and that is the whole defect.

**`##LLM-AS-RENDERER`** (`:234`) — *«`vibe explain --prose` feeds the subgraph …
to the provider behind `vibe-llm`.»* Decided package-internally: `ledger.rs:8-11`
says the producer *«is a deterministic template (the tool MUST be fully useful
without an LLM; an LLM prose producer **slots in later** under its own producer id
+ model id)»*, and `render_prose` (`:168`) is that template. The host crate the
fact names is a stub — `crates/vibe-llm/src/lib.rs` is **9 lines**, header
*«**STATUS: M0 stub.** Concrete providers (Anthropic, OpenAI, OpenRouter, …»* —
which corroborates without being needed.

**`##RUNTIME-PROFILES`** (`:241`) — *«`open` / `contract` / `none`. Declared in
`vibe.toml` `[metamodel] profile = "open"`.»* `[metamodel]` appears in no
manifest, no schema and no parser across every package, `crates/` and `schemas/`;
the only `metamodel` hits in the tree are one doc-comment phrase in `ledger.rs:34`
and its vendored copies. The three profile values have no representation.

**`##RUST-PRINCIPLE-RUSTDOC-IS-THE-DETAIL-LAYER`** (`:282`) — *«`vibe explain`
composes spec (contract) + rustdoc (detail)»*. `explain.rs` contains **no `doc`
field access at all**, and `CodeItem` (above) carries none to access; the renderer
emits symbol, kind, crate, file, line and edges. The first half of the sentence —
that every tagged item's doc comment states the practically important behaviour —
is a rule for the adopting project and is **not** what fails; the composition
claim is.

**`##ROW-PRIOR-ART-SYN-TREE-SITTER`** (`:398`) — *«syn / tree-sitter | MIT/Apache-2.0;
MIT | Implementation dependencies for the scanner»*. `syn` is real and load-bearing
(`Cargo.toml:44`, `core-ai-native-specmap/Cargo.toml:16`, `rscan.rs:243`
`syn::parse_file`). `tree-sitter` appears in **no `Cargo.toml` or `package.json`
anywhere in the tree** — 0. Half the row is a present-tense claim about a
dependency that is not one. Already `@spec/done`.

**`refs/**`, reported separately:** `tree.sitter` matches 17 files and
`conflicts_with` 328, all third-party; `specmap_query` / `specmap_explain` /
`specmap_source` / `metamodel` return nothing there. None is ours.

**Proposed corrections (NOT APPLIED)** — nine anchors plus the naming half of
`##RUNTIME-TRANSPORT`. All follow the document's own established «Specified, not
built» form, and all are coupled to F-121's decision exactly as F-207's and
F-146's are.

*`:239` `##RUNTIME-TRANSPORT` — the one FALSE anchor that still needs a text
change, because the tool names and the server are wrong even though the capability
is built:*

```
##RUNTIME-TRANSPORT **Transport.** Each stack's discipline MCP server (`rust-ai-native-mcp`, `typescript-ai-native-mcp`, `go-ai-native-mcp`, all built on this package's `core-ai-native-mcp`) exposes `trace_explain(target, {json|prose})` — the subgraph and the prose render — alongside `specmap_check` and `specmap_write`. An agent that drives the stack CLI gets the same via `<lang>-ai-native trace <target> --json`. *Specified, not built: there is no `specmap_source(content_hash) -> fragment` tool and no general `specmap_query`.* @impl/done
```

*`:58`:* append — *«Specified, not built: no package ships an index (no `vibe.toml` lists `specmap.json` in a payload), and there is no fetch-by-content-hash path — `content_hash` hashes, it does not retrieve.»*
*`:190`:* append — *«Specified, not built: no checker in any layer counts edges per item; `vibe check`'s checks do not include a multiplicity lint.»*
*`:198`:* replace the node list with *«`SpecUnit { uri, kind, r, content_hash }`, `CodeItem { symbol, item_kind, crate_name, file, line }`»* and append — *«Specified, not built: `CodeItem` carries no content hash (see §2.2's own note), and there are no derived `Command` or `ErrorVariant` node views. (`ErrorVariant` exists as a conform **fact** — `conform/src/facts.rs:66` — which is a different graph.)»*
*`:226`:* append — *«Specified, not built as an index lookup: error renderings cite `violates spec://…` from compile-time constants — `core-ai-native-mcp/src/error.rs:25`, `:37`, `:48`, pinned by a doctest, and conform's `req_message` (`rules/mod.rs:49`) — with no revision and no `run: vibe explain` hint. The doorway is real; the lookup is not.»*
*`:234`:* append — *«Specified, not built: the prose producer is a deterministic template (`ledger.rs:168`) and the crate's own header says an LLM producer slots in later.»*
*`:241`:* append — *«Specified, not built: `[metamodel]` is in no manifest, no schema and no parser; the three profile values have no representation.»*
*`:272`:* append — *«Specified, not built: no checker warns on spec-unit length; the 120-line figure is a target with no enforcement.»*
*`:282`:* append — *«Specified, not built: `explain` cannot compose rustdoc — `CodeItem` carries no doc field and the renderer emits symbol, kind, crate, file, line and edges only.»*
*`:398`:* append to the row — *«`syn` is the live scanner dependency (`Cargo.toml:44`); tree-sitter is in no manifest in this repository.»*

**Recommendation per anchor:**
`##RUNTIME-TRANSPORT` → **re-judge confirmed on the absence claim; drift stands
on the names** — the correction above is a *renaming*, not a demotion. If the
boss prefers one verdict per anchor, the honest call is **drift stands, correction
prepared**, with the record showing the verdict's stated reason («gained no such
tools») is false.
`##DIFFERENTIATOR-CONSUMED-AT-RUNTIME` → **re-judge confirmed**; three MCP tools
in three servers are runtime consumption by agents.
`##EDGE-MODEL-DIRECTION-OF-AUTHORITY` → **re-judge confirmed**; the pool is a
file by design (`##LLM-AS-PROPOSER`, «never in code») and the file holds 54
approved proposals.
`##CONSEQUENCE-M1-5-CONVERGENCE` → **re-judge confirmed**; the verdict states the
sentence's own antecedent as its defect, and the format the sentence promises to
define exists.
`##RUST-PRINCIPLE-GENERATOR-INPUT-IS-TAGGED` → **drift stands, route (b)**; book
the untagged host `schemas/` in `PHASE-D-HOST-OBLIGATIONS.md`. **No package edit.**
`##DISTRIBUTION-RIDES-THE-EXISTING-REGISTRY` · `##RULE-MULTIPLICITY-LINT` ·
`##EDGE-MODEL-NODES` · `##QUERY-ERROR-PROVENANCE` · `##LLM-AS-RENDERER` ·
`##RUNTIME-PROFILES` · `##SPEC-PRINCIPLE-UNITS-FIT-A-PAGE` ·
`##RUST-PRINCIPLE-RUSTDOC-IS-THE-DETAIL-LAYER` · `##ROW-PRIOR-ART-SYN-TREE-SITTER`
→ **drift stands, correction prepared** (nine).

---

## Summary of dispositions

| obligation | anchors | FALSE | route (b) | SURVIVES | outcome |
|---|---:|---:|---:|---:|---|
| F-132 | 14 | 4 | 1 | 9 | MIXED |
| F-146 | 7 | 2 | 0 | 5 | MIXED |
| F-159 | 5 | 0 | 0 | 5 | SURVIVES |
| F-121 | 4 | 0 | 0 | 4 | SURVIVES |
| F-120 | 3 | 3 | 0 | 0 | FALSE |
| F-206 | 2 | 0 | 0 | 2 | SURVIVES |
| F-207 | 2 | 1 | 0 | 1 | MIXED |
| F-263 | 1 | 0 | 0 | 1 | SURVIVES |
| **total** | **38** | **9** | **1** | **28** | |

**Nine verdicts turned out FALSE** — the number this wave exists to produce.
**Nine of thirty-eight is 23.7 %**, against wave 6's 31 of 59 (52.5 %) and wave
7's sibling batch d6a at 16 of 16. The rate falls because this route is different
in kind: `sync-from-code` obligations are **discrepancies about things that
exist**, and a discrepancy is much harder to be wrong about than an absence. Four
of the nine are still perimeter misses of exactly the §3.7 shape; the other five
are the four cheaper classes the brief named.

**No file under `packages/` was edited.** No verdict JSON was written, nothing
under `campaigns/packages-2026-09/run/` was touched, and no `git` command that
writes was run — only `log`, `show`, `status` and `check-ignore`.

### The nine, and what killed each

| anchor | why it fell |
|---|---|
| `PROP-014#RUNTIME-TRANSPORT` | searched **the host's `crates/`** for a tool surface that lives in three `-mcp` **packages** |
| `PROP-014#DIFFERENTIATOR-CONSUMED-AT-RUNTIME` | inherited that premise verbatim |
| `PROP-014#EDGE-MODEL-DIRECTION-OF-AUTHORITY` | «every edge is authored» **is** the design (`##LLM-AS-PROPOSER`: proposals live in a file, «never in code»); the file exists with 54 approved entries |
| `PROP-014#CONSEQUENCE-M1-5-CONVERGENCE` | a future-conditional («**Once** `vibe build` generates code…») convicted for its antecedent not having fired |
| `ENGINE-CONFORM#DETERMINISM-TESTED-BY-RUN-TWICE-DIFF` | grepped `twice` / `same_inputs`; the test is `sarif_is_byte_stable` **in the crate searched**, and the driver's message reads `"same inputs — byte-identical SARIF"` with a space |
| `ENGINE-CONFORM#GATE-EXIT-CODE-IS-THE-ACCEPTANCE-CRITERION` | «the CLI takes only `--path`» — every driver takes `--scope` and `--baseline`, and Go ships a standalone `conform` binary |
| `PROP-014#PHASE-0-ACCEPTANCE` | the scanner cites this clause **by name** (`mdspec.rs:8-9`) as the reason for the behaviour called a contradiction |
| `00-MANIFESTO#status-line` | the reason quotes a sentence this anchor does not contain, and the claim it does make is stated by the verdict's own evidence ref |
| `01-PATTERN-CARD-FORMAT#status-line` | «X supersedes Y» is a lineage claim; Y's absence is what supersession *means*, and `7ca98728`'s subject line is the event |

### Three patterns the boss should carry forward

**1. On this route the perimeter miss has a new address, and it is inside the
subject.** §3.7 says «search the host too»; wave 6 says «search `packages/` too».
This batch found the third case: **`core-ai-native` is simultaneously the SPEC
layer and the ENGINE layer**, a Cargo workspace with five crates of its own, and
its capabilities are then vendored into six sibling packages and driven by three
CLIs and three MCP servers. Eight of these thirty-eight verdicts scoped to «the
host's crates» or «the crate», and **six of the nine falls came from that one
move**. The invariant that would have caught all six: *before recording an
absence about a mechanism, name which of the four layers you searched — and if
the answer is fewer than all four, you have not measured it.*

**2. Re-running the verdict's own command is still the cheapest check there is,
and it caught two here.** `sarif_is_byte_stable` sits in the file the verdict
cited as evidence, twelve lines from the line number it quoted. The `--path`-only
claim is refuted by a `clap` enum. Neither needed any widening at all.

**3. A conditional is not a claim, and this route is full of them.** Three of the
nine — `##CONSEQUENCE-M1-5-CONVERGENCE`, and (by the same logic, though they
survived on other grounds) the three F-121 closing rules — are sentences of the
form «once X, then Y» or «any X not done by phase N is removed». A verdict that
proves ¬X has proved the sentence's own premise, not its falsehood. Worth a line
in §3.7 next to the perimeter rule.

### What the boss owes the queues if these are accepted

**Nine re-judges, no diff, no owner read.** That is the whole point of this pass:
a re-judge that edits nothing produces no spec diff and therefore needs no
approval on the `sync-from-code` route.

**One route-(b) record**, and one only:
`PROP-014#RUST-PRINCIPLE-GENERATOR-INPUT-IS-TAGGED` → `run/state/routing.json`,
plus a host obligation in `PHASE-D-HOST-OBLIGATIONS.md`: **this repository's
`schemas/*.jtd.json` carry no spec tags**, so generated code inherits no edges.
The rule is sound; under the owner ruling the host is a genuine AI-Native Rust
consumer, so the gap is real — and it is the *host's*, not the package's. The
macro half of the same rule is already kept, in
`core-ai-native-specmark/src/lib.rs`.

**Twenty-eight prepared corrections in five documents, and they must go to the
owner as three coupled groups, not twenty-eight rows:**

- **Group A — the closing-rule family (4 anchors, 4 documents).** F-121's three
  plus F-206's `##UNEXERCISED-FRONTEND-OR-TIER-IS-REMOVED`. One rule written four
  times, falsified four times by its own documents' **eighteen** «Specified, not
  built» annotations (5 + 3 + 1 + 8 + 1 in CONTRADICTION-MAP's neighbourhood).
  The owner picks **(i) demote the markers** or **(ii) legalise the annotation
  form**, and the answer applies to all four. **Nothing else can be decided
  first**, because —
- **Group B — the twenty-three annotate-in-place corrections** (F-132's nine,
  F-146's five, F-159's five, F-207's one, F-206's one, F-263's one, F-121's
  `##ENTRY-CARRIES-FOUR-PARTS`) are all written in the «Specified, not built»
  form that Group A's rule currently forbids. **If the owner picks (i), every one
  of them must be re-cast as a marker demotion.** Present A first.
- **Group C — F-263 alone**, `README.md#PROMPT-CONTENT-ONLY`. Independent of A
  and B, one sentence, the package's front door, wrong for four minor versions
  over 10 072 lines of its own Rust. The cheapest approval in the batch and the
  one a consumer meets first.

### One measurement worth re-taking elsewhere

`PROP-014`'s §3 `trait Frontend` sketch and `ENGINE-CONFORM`'s §2 sketch both
show `fn lang(&self) -> Lang; fn tier(&self) -> Tier`. **Neither `enum Tier` nor
`enum Lang` exists in the engine** — the shipped trait is
`{ id, version, extract, warm }` (`conform/src/facts.rs:176-189`). The T-syn /
T-sem tier split that four documents reason over is a **document-only taxonomy**
with no type behind it. Those trait blocks are fenced code, carry no anchor, and
so were never judged — which is B-004's problem shape (a claim inside a fenced
block that no anchor covers) at a new site. Worth booking rather than
re-deriving.
