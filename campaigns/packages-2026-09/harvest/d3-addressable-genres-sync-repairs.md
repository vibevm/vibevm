# D3 — addressable-specs · spec-genres · sync-from-code · two-process-model · wal-specspaces

_Wave 3 of Phase D. Sixteen `prose-edit` obligations over five `org.vibevm.world`
packages. Route check run first for all sixteen:_

```console
$ for t in F-170 F-286 F-289 F-290 F-247 F-248 F-249 F-335 \
           F-252 F-253 F-340 F-341 F-346 F-347 F-353 F-354; do
    python campaigns/packages-2026-09/tasks/drift-registry.py --task "$t" | head -1
  done   # (fields re-read from run/state/obligations.json to tabulate)
F-170  route=prose-edit release_event=False cross_package=False
F-286  route=prose-edit release_event=False cross_package=False
F-289  route=prose-edit release_event=False cross_package=False
F-290  route=prose-edit release_event=False cross_package=False
F-247  route=prose-edit release_event=False cross_package=False
F-248  route=prose-edit release_event=False cross_package=False
F-249  route=prose-edit release_event=False cross_package=False
F-335  route=prose-edit release_event=False cross_package=False
F-252  route=prose-edit release_event=False cross_package=False
F-253  route=prose-edit release_event=False cross_package=False
F-340  route=prose-edit release_event=False cross_package=False
F-341  route=prose-edit release_event=False cross_package=False
F-346  route=prose-edit release_event=False cross_package=False
F-347  route=prose-edit release_event=False cross_package=False
F-353  route=prose-edit release_event=False cross_package=False
F-354  route=prose-edit release_event=False cross_package=False
```

_No obligation was out of route by `closure_route`, and none is a release event;
every OUT-OF-ROUTE verdict below is §3.6 **route (b)** — the rule is sound and
the falsifier is the host._

**Tally: 3 EDITED, 13 OUT-OF-ROUTE, 0 RE-JUDGE, 0 BLOCKED. 3 anchors moved of
20.** The ratio is the expected one and is the finding, not a shortfall: these
are shipped normative flows, and «the consumer does less than the rule asks» is
route (b) by §3.6, not a licence to loosen the rule. Every one of the three
edits was licensed by a falsifier *inside* `packages/` — a sibling package that
disclaims what it was credited with (F-253), a sibling package that states the
opposite (F-346), and a registry format four files away in the same package that
cannot express the escape hatch offered (F-353). None of the three makes a rule
admit anything the host happens to do.

**Two Phase C reasons need correcting, and both are recorded in place rather
than quietly worked around:** F-335 says the design doc is unlinked when the
contract exists and the design doc names it — the real defect is a missing
return leg (see below); and F-249's «5 of 42» is not reproducible by any single
command (3 by the strict header, 9 by any `design/` mention). Neither changes a
route.

---

## F-170 — the size budgets (boot ≤ 500 / WAL ≤ 3000 / module spec ≤ 5000) are 32× over on this host

**Outcome:** OUT-OF-ROUTE
**Anchors:** 0 edited of 4 — `authoring-rules.xml#ROW-BUDGET-BOOT-FILE`,
`#ROW-BUDGET-WAL`, `#ROW-BUDGET-MODULE-SPEC`, `#SUM-THE-SIZE-BUDGETS`
**Files touched:** none
**Re-verification:**

```console
$ for f in vibevm/vibepacks/org.vibevm.world/*/v0.1.0/spec/boot/*.md; do \
    w=$(wc -w < "$f"); echo "$w words ~$((w*4/3)) tok $f"; done | sort -rn | head -3
565 words  ~753 tok  vibevm/vibepacks/org.vibevm.world/secrets-hygiene/v0.1.0/vibevm/vibespecs/boot/57-flow-secrets-hygiene.xml
506 words  ~674 tok  vibevm/vibepacks/org.vibevm.world/wal-specspaces/v0.1.0/vibevm/vibespecs/boot/11-flow-wal-specspaces.xml
499 words  ~665 tok  vibevm/vibepacks/org.vibevm.world/decision-records/v0.1.0/vibevm/vibespecs/boot/25-flow-decision-records.xml

$ w=$(wc -w < vibevm/vibepacks/org.vibevm.world/addressable-specs/v0.1.0/vibevm/vibespecs/boot/15-flow-addressable-specs.xml); echo "$w words ~$((w*4/3)) tok"
360 words ~480 tok

$ lane=$(cat vibevm/vibespecs/boot/STATIC.xml vibevm/vibespecs/boot/00-core.xml vibevm/vibespecs/boot/90-user.xml vibevm/vibespecs/boot/INDEX.md | wc -w)
$ echo "boot lane words: $lane  ~tokens: $((lane*500/375))"
boot lane words: 12102  ~tokens: 16136

$ for f in vibevm/vibespecs/modules/*/*.md vibevm/vibespecs/common/*.md; do w=$(wc -w < "$f"); \
    [ "$w" -gt 3750 ] && echo "$f"; done | wc -l
9
```

**Who falsifies it:** the host — its boot lane, its WAL and 9 of its 43 module
and common specs are over; the package's own boot snippet is **inside** the
budget it sets.
**Perimeter searched:** the whole boot lane a session loads unconditionally
(`vibevm/vibespecs/boot/STATIC.xml` + `00-core.xml` + `90-user.xml` + `INDEX.md`), `vibevm/vibespecs/WAL.xml`,
and `vibevm/vibespecs/modules/*/*.md` + `vibevm/vibespecs/common/*.md` for the 5 000-token ceiling —
measured at the flow's own conversion (`##rule-of-thumb-500-tokens-is-a-page`:
500 tokens ≈ 375 English words), not at chars/token.
**What changed and why:** nothing. This is a budget with a stated remedy
(«split when over»), and the whole of the verdict is that the consumer has not
split. Raising 500 to admit ~16 136 is the exact shape wave 2 reverted three
times. The package obeys its own rule in its own tree — 480 tokens against its
own 500 — so there is nothing here for the package to yield. Host obligation:
the boot lane (~32× over), `vibevm/vibespecs/WAL.xml` and the nine oversized specs are over
budget and the split has never fired. Phase C's «9 of 47» reproduces as **9 of
43** on today's tree — the numerator, which is what the verdict turns on, is
exact.
**New obligations noticed:** the same budget triple is authored twice in the
collection — here at `authoring-rules.xml#SUM-THE-SIZE-BUDGETS` and again at
`two-process-model/…/files-as-ipc.xml#ROW-PLANE-CONTROL` (F-347) — with nothing
forcing them to agree. That is a `duplication` across two packages and therefore
a release event, not a wave-3 edit. Two sibling boot snippets are themselves
over the 500-token budget this package sets: `secrets-hygiene` (~753) and
`wal-specspaces` (~674).

---

## F-286 — harness entry files are «thin redirects into `spec/BOOT.md`»; the host has three fat doors and no `spec/BOOT.md`

**Outcome:** OUT-OF-ROUTE
**Anchors:** 0 edited of 1 —
`spec-tree-layout.xml#HARNESS-ENTRY-FILES-ARE-THIN-REDIRECTS`
**Files touched:** none
**Re-verification:**

```console
$ wc -l CLAUDE.md AGENTS.md GEMINI.md && ls spec/BOOT.md
  228 CLAUDE.md
  228 AGENTS.md
  228 GEMINI.md
  684 total
ls: cannot access 'spec/BOOT.md': No such file or directory
```

**Who falsifies it:** the host — it keeps three full-contract entry files and
boots from a directory rather than a file.
**Perimeter searched:** repo root (`CLAUDE.md`, `AGENTS.md`, `GEMINI.md`),
`spec/` root for `BOOT.md`, and `vibevm/vibespecs/boot/**` for the actual lane. The host's
entry point is the generated `vibevm/vibespecs/boot/INDEX.md` manifest plus
`vibevm/vibespecs/boot/STATIC.xml`, not a single file.
**What changed and why:** nothing. Both halves are the host declining a `should`.
The package is internally consistent about `spec/BOOT.md` — its own reference
tree at `spec-tree-layout.xml:19` names the same file the rule points at, and
`:35` repeats the redirect — so no sentence in this package is false about
anything in this package. Broadening «thin redirects» to admit 228 lines of
contract, or broadening `spec/BOOT.md` to admit a directory, is a rule rewritten
to fit the consumer. Host obligation: three byte-identical fat entry files and
no single boot document.
**New obligations noticed:** `AGENTS.md` and `GEMINI.md` being byte-identical
copies of `CLAUDE.md` is the `duplication` this rule exists to prevent, and it
is a host defect with no registry row that I could find under
`spec/flows/addressable-specs/`.

---

## F-289 — «every normative statement is addressable», and the two files every session reads first carry no anchors at all

**Outcome:** OUT-OF-ROUTE
**Anchors:** 0 edited of 1 —
`spec/boot/15-flow-addressable-specs.md#EVERY-NORMATIVE-STATEMENT-IS-ADDRESSABLE`
**Files touched:** none
**Re-verification:**

```console
$ grep -h "^#" vibevm/vibespecs/boot/00-core.xml vibevm/vibespecs/boot/90-user.xml vibevm/vibespecs/boot/INDEX.md | grep -c "{#"
0
$ grep -c "{#" vibevm/vibepacks/org.vibevm.world/addressable-specs/v0.1.0/vibevm/vibespecs/boot/15-flow-addressable-specs.xml
7
```

**Who falsifies it:** the host — its own `vibevm/vibespecs/boot/` and `vibevm/vibespecs/WAL.xml` headings
are unanchored; the package's boot snippet stating the rule is fully anchored.
**Perimeter searched:** `vibevm/vibespecs/boot/00-core.xml`, `vibevm/vibespecs/boot/90-user.xml`,
`vibevm/vibespecs/boot/INDEX.md` (host-authored boot lane; `STATIC.md` is generated from
installed packages and is not host-authored), and the package's own
`vibevm/vibespecs/boot/*.md`.
**What changed and why:** nothing. The verdict is `DRIFT on coverage, confirmed
on form` — 519 anchors, zero malformed — and the coverage gap is 125 host
headings, all of them in host-authored files. There is no way to repair this in
the package that does not amount to exempting the boot lane from
addressability, which is the one place the flow most wants it. Host obligation:
anchor the 23 headings in `vibevm/vibespecs/boot/` and the 8 in `vibevm/vibespecs/WAL.xml`.
**New obligations noticed:** the WAL's Constraints section is the most normative
content the sibling `flow:wal` names and cannot be cited at all today.

---

## F-290 — «protocol docs at the spec root» names a home this host does not have

**Outcome:** OUT-OF-ROUTE
**Anchors:** 0 edited of 1 — `spec-tree-layout.xml#ROW-HOME-SPEC-PROTOCOL`
**Files touched:** none
**Re-verification:**

```console
$ ls spec/*PROTOCOL*.md
ls: cannot access 'spec/*PROTOCOL*.md': No such file or directory
$ ls vibevm/vibepacks/org.vibevm.world/*/v0.1.0/spec/flows/*/*PROTOCOL*.md | wc -l
18
```

**Who falsifies it:** the host — it receives conflict and sync rules as installed
flows under `spec/flows/<flow>/`, compiled into `vibevm/vibespecs/boot/STATIC.xml`.
**Perimeter searched:** `spec/` root (glob `spec/*PROTOCOL*.md`), `vibevm/vibespecs/common/`,
`vibevm/vibespecs/boot/**`, and `vibevm/vibepacks/org.vibevm.world/*/v0.1.0/spec/flows/*/`.
**What changed and why:** nothing, and this was the closest call in the batch.
The row is internally consistent with its own reference tree at
`spec-tree-layout.xml:21`, so the package does not contradict itself. The
tempting route (a) reading is that 18 of the collection's own protocol documents
sit at `spec/flows/<flow>/` rather than any spec root — but the only edit that
observation licenses is «…or `spec/flows/<flow>/` when installed as a flow»,
which is precisely a rule broadened until the consumer's layout fits inside it.
Left alone. Host obligation: the host has no protocol document at its spec root
and receives that content through the install lane instead — a candidate for a
§3.6 route (c) written exception rather than a package edit.
**New obligations noticed:** if the owner takes route (c) here, the same
exception covers `##ROW-HOME-BOOT-MINIMUM` («boot entry file (≤ 500 tokens)»),
which names a single file for the same reason and meets the same directory.

---

## F-247 — «genre typing removes all three» removes two; one live counter-instance survives in the host

**Outcome:** OUT-OF-ROUTE
**Anchors:** 0 edited of 2 —
`SPEC-GENRES-PROTOCOL.xml#GENRE-TYPING-REMOVES-ALL-THREE`,
`SPEC-GENRES-PROTOCOL.xml#PLACE-DO-NOT-BLEND`
**Files touched:** none
**Re-verification:**

```console
$ grep -c "MUST" vibevm/vibespecs/design/structural-loader.xml
2
$ grep -n "idx-structural-loader" vibevm/vibespecs/design/README.md
50:- ##idx-structural-loader [Structural loader](structural-loader.xml) — provisional loader instructions held for PROP-035; not yet wired into any live boot. @spec/hold
```

**Who falsifies it:** the host — one of its design docs carries uppercase MUSTs
inside a non-binding genre and has not been split.
**What changed and why:** nothing. `##PLACE-DO-NOT-BLEND` is a bare imperative —
«do not blend» — and the only edit an unsplit host document licenses is to soften
the imperative, which is the reverted shape exactly. `##GENRE-TYPING-REMOVES-ALL-THREE`
is the thesis those steps serve, and the verdict's own words are that «genre
typing gave the host the vocabulary to SEE the failure and the file is still in
it» — that is the host not applying the typing it has, not the typing failing.
Host obligation: `vibevm/vibespecs/design/structural-loader.xml` is half requirement and half
story and needs splitting along the binding line.
**New obligations noticed — the README arithmetic, checked and left alone.** The
brief named a route-(a) over-count in this package's own README. It is real and
it is measured:

```console
$ grep -rh "This package ships \(three\|four\) pieces of content plus a boot snippet" \
    vibevm/vibepacks/org.vibevm.world/*/v0.1.0/README.md | sort | uniq -c
      2 This package ships four pieces of content plus a boot snippet: @impl/done
     14 This package ships three pieces of content plus a boot snippet: @impl/done
$ ls vibevm/vibepacks/org.vibevm.world/spec-genres/v0.1.0/vibevm/vibespecs/flows/spec-genres/ | wc -l
3
$ ls vibevm/vibepacks/org.vibevm.world/tool-design-lessons/v0.1.0/vibevm/vibespecs/flows/*/ | wc -l
3
```

`spec-genres/README.md:32` says «four pieces of content plus a boot snippet» and
lists four bullets of which one *is* the boot snippet — so it ships three, like
the fourteen siblings that say so. **Both** «four» claims over-count:
`tool-design-lessons` ships three as well. I did not touch it: the anchor is
`README.md#package-contents-lead`, which the registry already carries as
**F-251, `closure_route: release`** — outside this batch's route and behind the
owner under §5. It is the same shared anchor in two packages, which is why it
routes that way.

---

## F-248 — the five-field fork skeleton, and 18 host forks in three other shapes

**Outcome:** OUT-OF-ROUTE
**Anchors:** 0 edited of 2 —
`design-docs.xml#RECORD-EACH-FORK-AS-A-FIXED-SKELETON`,
`design-docs.xml#SUM-RECORD-FORKS-AS-A-SKELETON`
**Files touched:** none
**Re-verification:**

```console
$ grep -rno "Consequence" vibevm/vibespecs/design/
vibevm/vibespecs/design/loading-and-boot-model.xml:196:Consequence
$ grep -o "Chosen:" vibevm/vibespecs/design/workspace-and-qualified-naming.xml | wc -l
13
```

_The single «Consequence» in `vibevm/vibespecs/design/` is the heading `## 7. Consequences
and findings` at :196, covering a whole document — not a fork field. Phase C's
verdict says 14 `Chosen:` bullets; the reproducible count is 13, which does not
change the finding (13 forks in a one-field shape is the same defect) but is
recorded because the number was quoted._

**Who falsifies it:** the host — it records forks in a two-field and a one-line
shape, and «Consequence» never appears as a fork field.
**What changed and why:** nothing. This is a prescribed template with a stated
rationale (scannable rather than buried in prose), and the host's forks are
buried in prose — which is the failure the template names, not a counter-example
to it. Widening the skeleton to admit «Options offered:» + «Resolution —» is a
rule rewritten around a consumer's habit. Host obligation: 18 forks across
`vibevm/vibespecs/design/loading-and-boot-model.xml` and
`vibevm/vibespecs/design/workspace-and-qualified-naming.xml` carry no Rejected or Consequence
lines.

---

## F-249 — «most real decisions produce both» measures 5 of 42 on this host

**Outcome:** OUT-OF-ROUTE
**Anchors:** 0 edited of 2 —
`when-to-write-what.xml#MOST-DECISIONS-PRODUCE-BOTH`,
`when-to-write-what.xml#SUM-A-DECISION-PRODUCES-BOTH`
**Files touched:** none
**Re-verification:**

```console
$ ls vibevm/vibespecs/modules/*/PROP-*.md spec/common/PROP-*.md | wc -l
42
$ grep -rln "design-rationale" vibevm/vibespecs/modules/ vibevm/vibespecs/common/ | wc -l
3
$ grep -rln "design/" vibevm/vibespecs/modules/*/PROP-*.md spec/common/PROP-*.md | wc -l
9
$ ls vibevm/vibepacks/org.vibevm.world/spec-genres/v0.1.0/vibevm/vibespecs/flows/spec-genres/
SPEC-GENRES-PROTOCOL.xml
design-docs.xml
when-to-write-what.xml
$ grep -rl "vibevm/vibespecs/design/" vibevm/vibepacks/org.vibevm.world/spec-genres/ | wc -l
0
```

**Who falsifies it:** the host — its own PROP-to-design-doc ratio, i.e. its
authoring practice.
**Perimeter searched:** `vibevm/vibespecs/modules/**/PROP-*.md`, `spec/common/PROP-*.md`
(42 contracts), and `vibevm/vibespecs/design/**` for the linked side; then the whole of
`vibevm/vibepacks/org.vibevm.world/spec-genres/v0.1.0/` for anything that could measure
the quantifier from inside the package — nothing does, it ships three flow docs,
a boot snippet and a README, and no contracts or design docs at all. Note the
verdict's «5 of 42» is not reproducible by one command: the strict
`design-rationale` back-link header gives **3**, and any mention of a `design/`
path gives **9**. The quantifier fails on every one of those readings (3, 5 or 9
against 42), so the finding stands while the exact figure does not.
**What changed and why:** nothing, and this is the entry I would most expect to
be re-judged. The sentence is descriptive rather than normative, which argues for
route (a) — but the only measurable population is the host's authoring practice,
and my contract is that a package yields only where its own sentence is false
about something inside its own tree. Nothing in
`vibevm/vibepacks/org.vibevm.world/spec-genres/v0.1.0/` measures decisions against design
docs. Recorded for the owner instead of guessed at.
**New obligations noticed:** the same passage's routing table already says the
long-form story goes in a design doc «if any»
(`##ROW-SITUATION-DURABLE-CHOICE`), which sits uneasily beside «most decisions
produce both» four lines later. If a later wave rules F-249 route (a), that
internal tension is the evidence to cite, not the host's 12 %.

---

## F-335 — «an unlinked design doc is a defect», and the host's one unlinked doc was parked rather than filed

**Outcome:** OUT-OF-ROUTE
**Anchors:** 0 edited of 1 —
`SPEC-GENRES-PROTOCOL.xml#AN-UNLINKED-DESIGN-DOC-IS-A-DEFECT`
**Files touched:** none
**Re-verification:**

```console
$ grep -rn "structural-loader" vibevm/vibespecs/terraforms/SPEC-ACTUALIZATION-CAMPAIGN-v0.1.xml BACKLOG.md
vibevm/vibespecs/terraforms/SPEC-ACTUALIZATION-CAMPAIGN-v0.1.xml:770:  design↔PROP backlinks hold 4/4 (structural-loader parked by its own
$ grep -c "PROP-035" vibevm/vibespecs/design/structural-loader.xml
3
$ grep -c "structural-loader" vibevm/vibespecs/modules/vibe-workspace/PROP-035-spec-compiler.xml
0
```

**Who falsifies it:** the host — its record classifies the instance as parked,
where the flow classifies it as a defect.
**Perimeter searched:** `vibevm/vibespecs/terraforms/SPEC-ACTUALIZATION-CAMPAIGN-v0.1.xml`
and `BACKLOG.md` for a finding id (none in either), `vibevm/vibespecs/design/README.md` for
the index entry, and `vibevm/vibespecs/modules/*/` + `vibevm/vibespecs/common/` for the contract.
**Correction to the Phase C reason, and it matters:** the reason reads as though
the doc is unlinked in both directions. It is not. `PROP-035-spec-compiler.xml`
**exists** at `vibevm/vibespecs/modules/vibe-workspace/`, 37 KB, and
`vibevm/vibespecs/design/structural-loader.xml` names it three times — its own status line
says «DESIGN — provisional (PROP-035 §13)». What is missing is only the return
leg: PROP-035's `##related` header at :8 links
`vibevm/vibespecs/design/loading-and-boot-model.xml` and never mentions `structural-loader`.
So the instance is a **one-directional** link, not an unlinked doc.
**What changed and why:** nothing, and I considered `RE-JUDGE: confirmed` here
under §3.6 route (c) — the host *does* carry a marked exception
(`vibevm/vibespecs/design/README.md:50` marks the doc `@spec/hold`, and the campaign log at
:770 says it was «parked by its own» marker). I did not take it: route (c)
requires the exception to be *written down as an exception to this rule*, and
`@spec/hold` is a progress marker about a document's own readiness, not a ruling
that an unlinked design doc is exempt. That distinction is the owner's to make,
so it is recorded rather than assumed. The package sentence is not false about
anything in its own tree either way.
**New obligations noticed:** the instance is a one-directional link, which the
host adopts verbatim as
`SPEC-GENRES-PROTOCOL.xml#A-ONE-DIRECTIONAL-LINK-IS-A-LATENT-BREAK` (`confirmed`
in Phase C on this same file). Host obligation: add the return leg to
`vibevm/vibespecs/modules/vibe-workspace/PROP-035-spec-compiler.xml`'s `##related` line — one
link closes it. That is a cheaper and better-founded closure than either reading
of F-335, and whichever wave re-judges this anchor should be told the contract
half exists.

---

## F-252 — the mandatory third part of a sync draft is the one the host's two syncs never produced

**Outcome:** OUT-OF-ROUTE
**Anchors:** 0 edited of 2 —
`SYNC-PROTOCOL.xml#PART-THE-REVISIT-TRIGGER`,
`SYNC-PROTOCOL.xml#SUM-OUTPUT-IS-VALUE-REASON-AND-REVISIT-TRIGGER`
**Files touched:** none
**Re-verification:**

```console
$ git show 4ea09ad0 -- spec/ | grep -ci revisit
0
$ git show 04d7e4ae -- spec/ | grep -ci revisit
0
```

**Who falsifies it:** the host — both recorded syncs shipped without a revisit
trigger.
**Perimeter searched:** the two commits the campaign records as applied syncs,
**scoped to `-- spec/`**. Scoping matters and is the reason the command is
written this way: unscoped, `git show 4ea09ad0 | grep -ci revisit` returns
**13**, every one of them an anchor id inside the 94 481-line
`campaigns/progress-2026-08/run/cache.json` blob the same commit carries. A
reader checking this verdict with the obvious command would conclude it was
false. It is not — the spec diff is clean of the word.
**What changed and why:** nothing. The registry's mechanical `falsifier` field
reads `self` because every evidence ref sits inside the package, but the verdict
prose is measured entirely on host commits and host spec sections — a reminder
that §6.1's `ROUTE-BEFORE-FALSIFIER` cuts both ways and `falsifier` is not the
routing decision. Dropping the trigger from the mandatory triple would delete
the rule the sibling `review-workflow.xml#NO-TRIGGER-MEANS-NO-AUDIT-PATH` exists
to enforce. Host obligation: 2 of 2 applied syncs are incomplete by this
document's own definition, and no sync since has been run.

---

## F-253 — Conventional Commits attributed to the atomicity flow, which itself says it is not the owner

**Outcome:** EDITED
**Anchors:** 1 edited of 2 — **edited**
`when-to-apply.xml#BOUNDARY-FLOW-ATOMIC-COMMITS`; **not edited**
`when-to-apply.xml#BOUNDARY-VIBE-BUILD`
**Files touched:**
`vibevm/vibepacks/org.vibevm.world/sync-from-code/v0.1.0/vibevm/vibespecs/flows/sync-from-code/when-to-apply.xml`
**Re-verification:**

```console
$ grep -n "CONVENTIONAL-COMMITS-IS-THE-FORMAT-THIS-FLOW-IS-THE-ATOMICITY" vibevm/vibepacks/org.vibevm.world/git-atomic-commits/v0.1.0/vibevm/vibespecs/boot/30-flow-atomic-commits.xml
26:##CONVENTIONAL-COMMITS-IS-THE-FORMAT-THIS-FLOW-IS-THE-ATOMICITY Conventional Commits is the *format*; this flow is the *atomicity* (one commit, one idea). @impl/done

$ ls -d vibevm/vibepacks/org.vibevm.world/git-conventional-commits/v0.1.0/vibevm/vibespecs/flows/conventional-commits/
vibevm/vibepacks/org.vibevm.world/git-conventional-commits/v0.1.0/vibevm/vibespecs/flows/conventional-commits/

$ diff <(git show HEAD:vibevm/vibepacks/org.vibevm.world/sync-from-code/v0.1.0/vibevm/vibespecs/flows/sync-from-code/when-to-apply.xml | grep -o '##[A-Za-z0-9-]*' | sort) <(grep -o '##[A-Za-z0-9-]*' vibevm/vibepacks/org.vibevm.world/sync-from-code/v0.1.0/vibevm/vibespecs/flows/sync-from-code/when-to-apply.xml | sort) && echo IDENTICAL
IDENTICAL
```

**Who falsifies it:** the document itself — the sibling package it names is in
`packages/`, ships a boot snippet that disclaims the format in as many words,
and the flow that does own the format ships beside it.
**What changed and why:** `##BOUNDARY-FLOW-ATOMIC-COMMITS` said the Conventional
Commits framing «is defined by the git-atomic-commits flow». It is not:
`git-atomic-commits` states at its own `30-flow-atomic-commits.xml:23-26` that
messages follow the **git-conventional-commits** sibling and that «Conventional
Commits is the *format*; this flow is the *atomicity*». The row now draws the
same line — atomicity to `flow:git-atomic-commits`, the `docs(spec)` message
format to `flow:git-conventional-commits` — and cites both by flow id, so no
`../` cross-package link is introduced. Anchor set byte-identical to HEAD.
`##BOUNDARY-VIBE-BUILD` was left alone: its falsifier is host code
(`crates/vibe-cli/src/cli.rs` carries no `Build` variant), a fact about the
host's product surface, and the closure §3.3 prescribes for a promised-but-
unbuilt mechanism is a demotion on the `build-or-demote` route — which this
`prose-edit` obligation does not carry.
**New obligations noticed:** `##BOUNDARY-VIBE-BUILD` names `vibe build` «(M1.5+)»
as the flow generating code from spec; the milestone has passed by fourteen
minor versions and the subcommand does not exist. That belongs on
`build-or-demote`, not here.

---

## F-340 — the boot snippet prints a third commit-subject grammar, and the host has never written any of them

**Outcome:** OUT-OF-ROUTE
**Anchors:** 0 edited of 1 —
`spec/boot/20-flow-sync-from-code.md#STEP-ON-APPROVAL-APPLY-AND-COMMIT-ON-REJECT-REVERT-OR-REDRAFT`
**Files touched:** none
**Re-verification:**

```console
$ git log --format=%s | grep -c '^docs(spec): sync'
0
$ git rev-list --count HEAD
2175
```

_The plan's §5 quotes 2 041; the tree has moved 134 commits since. The ratio is
unchanged: still zero._

**Who falsifies it:** the host — the prescribed subject grammar has never been
typed in 2 041 commits.
**Perimeter searched:** full `git log` over `HEAD` for `^docs(spec): sync`, and
`packages/**` + `vibedeps/**` for every occurrence of the string `docs(spec): sync`.
**What changed and why:** nothing. I found a package-internal defect while
looking — the collection carries **three** subject grammars for the same commit:
`docs(spec): sync <section> with code` (this boot snippet, :41),
`docs(spec): sync timeout to 600s in PROP-003 §verification.timeout`
(`SYNC-PROTOCOL.xml:171`), and `docs(spec): sync timeout into PROP-003
§verification.timeout` (`review-workflow.xml:119`). I did not repair it, for two
reasons. The registry's own falsifiers for this row (`SYNC-PROTOCOL.xml:165`,
`:178`) *confirm* the step's substance and say nothing about the subject line, so
the grammar mismatch is not what this evidence falsifies. And the second grammar
is also authored in a different package —
`git-conventional-commits/…/conventional-commits.xml:169` carries the identical
example — which makes the family cross-package and its repair a release event
under §4.5, not a wave-3 edit.
**New obligations noticed:** three commit-subject grammars for one commit across
two packages (`sync-from-code` boot snippet and its two flow docs;
`git-conventional-commits/…/conventional-commits.xml:169`) — a `duplication`
family with a release-event closure. Recorded, not fixed.

---

## F-341 — the third of the approval step's three requirements: no sync commit has ever carried a `spec://` URI

**Outcome:** OUT-OF-ROUTE
**Anchors:** 0 edited of 1 —
`review-workflow.xml#APPROVAL-STEP-COMMITS-WITH-CONVENTIONAL-COMMITS`
**Files touched:** none
**Re-verification:**

```console
$ git log -1 --format=%B 4ea09ad0 | grep -c 'spec://'
0
$ git log -1 --format=%B 04d7e4ae | grep -c 'spec://'
0
```

**Who falsifies it:** the host — two of its three stated requirements hold
(Conventional Commits at 397/400, a body citing the driving code change), and
the third is a habit the sync path never acquired.
**Perimeter searched:** the two commits the campaign records as applied syncs
(`4ea09ad0`, `04d7e4ae`) and the last 200 commits for `docs(spec)` bodies
carrying `spec://`.
**What changed and why:** nothing. The requirement is sound and the notation is
in daily use elsewhere in the tree, so this is a consumer that does less than the
rule asks. Striking the `spec://` clause would delete the citation contract the
sibling `flow:addressable-specs` exists to establish. Host obligation: sync
commits must carry the affected anchor's `spec://` URI in the body.

---

## F-346 — «works for the AI, works for the other two for free», contradicted inside the collection

**Outcome:** EDITED
**Anchors:** 1 edited of 1 —
`cognitive-load-split.xml#TEXT-THAT-WORKS-FOR-THE-AI-WORKS-FOR-THE-OTHER-TWO`
**Files touched:**
`vibevm/vibepacks/org.vibevm.world/two-process-model/v0.1.0/vibevm/vibespecs/flows/two-process-model/cognitive-load-split.xml`
**Re-verification:**

```console
$ grep -n "THE-REPORT-IS-FOR-THE-HUMAN\|they-serve-different-readers" vibevm/vibepacks/org.vibevm.world/wal/v0.2.0/vibevm/vibespecs/flows/wal/session-end-hook.xml
140:##THE-REPORT-IS-FOR-THE-HUMAN The report is for the human's quick scan. @impl/done
145:##they-serve-different-readers They serve different readers. @impl/done

$ grep -n "READ-THE-END-OF-SESSION-REPORT-EVERY-TIME" vibevm/vibepacks/org.vibevm.world/two-process-model/v0.1.0/vibevm/vibespecs/flows/two-process-model/files-as-ipc.xml
67:##READ-THE-END-OF-SESSION-REPORT-EVERY-TIME It is structured input for the next decision

$ diff <(git show HEAD:vibevm/vibepacks/org.vibevm.world/two-process-model/v0.1.0/vibevm/vibespecs/flows/two-process-model/cognitive-load-split.xml | grep -o '##[A-Za-z0-9-]*' | sort) <(grep -o '##[A-Za-z0-9-]*' vibevm/vibepacks/org.vibevm.world/two-process-model/v0.1.0/vibevm/vibespecs/flows/two-process-model/cognitive-load-split.xml | sort) && echo IDENTICAL
IDENTICAL
```

**Who falsifies it:** the document itself — a sibling package in `packages/`
says the opposite in its own words, and this package's own `files-as-ipc.xml`
already treats the human report and the machine checkpoint as two artefacts on
two different planes.
**What changed and why:** «for free» was the false half. The collection does not
believe it: `wal/…/session-end-hook.xml:140,145` specifies a human-scan report
*and* a machine-read WAL for the same state and says «They serve different
readers», and this package's own `files-as-ipc.xml:60,64-68` puts the checkpoint
on the Control plane and the end-of-session report on the Signals plane, to be
read «with your eyes, every time». The sentence now says the transfer carries
most of the way but is paid for with a second *rendering* where a reader needs a
different shape — while keeping the neighbouring norm
`##WRITE-EVERY-LOAD-BEARING-FILE-FOR-THREE-READERS` («from one source») intact,
and saying so explicitly: «never a second source». The host's CLAUDE.md TL;DR
requirement is consistent with the repair but is not what licenses it — the
sibling package is. Anchor set byte-identical to HEAD; no `../` link added.

---

## F-347 — the Control plane's three budgets, broken by the consumer at ~32×

**Outcome:** OUT-OF-ROUTE
**Anchors:** 0 edited of 1 — `files-as-ipc.xml#ROW-PLANE-CONTROL`
**Files touched:** none
**Re-verification:**

```console
$ wc -w vibevm/vibespecs/boot/STATIC.xml vibevm/vibespecs/boot/00-core.xml vibevm/vibespecs/boot/90-user.xml vibevm/vibespecs/WAL.xml
  9860 vibevm/vibespecs/boot/STATIC.xml
   827 vibevm/vibespecs/boot/00-core.xml
  1282 vibevm/vibespecs/boot/90-user.xml
  3296 vibevm/vibespecs/WAL.xml
 15265 total
```

_At the sibling flow's conversion (`500 tokens ≈ 375 English words`): the boot
lane totals ~16 136 tokens against ~500 (the F-170 measurement, which adds
`vibevm/vibespecs/boot/INDEX.md` to the four files above), and `vibevm/vibespecs/WAL.xml` is ~4 395
against ~3 000. The overrun is **~32×** on boot, not the 45× the Phase C reason
states — that reason counted at ~4 chars/token rather than the collection's own
rule of thumb. Either figure falsifies the row; the direction is not in doubt._

**Who falsifies it:** the host — its boot lane, its checkpoint and nine of its
module and common specs are over the budgets; the plane's membership and
direction, which is the rest of the row, are exactly right at
`vibevm/vibespecs/boot/00-core.xml:9-13`.
**Perimeter searched:** identical to F-170's — same three budgets, same tree.
**What changed and why:** nothing, for F-170's reason and by the same rule. The
row's own remedy — «split when over» — is the repair, and it belongs to the
consumer. Host obligation: the same one F-170 opens; these are one budget stated
in two packages.
**New obligations noticed:** the duplication itself. `##ROW-PLANE-CONTROL` and
`addressable-specs/…/authoring-rules.xml#SUM-THE-SIZE-BUDGETS` author the same
three numbers in two packages with nothing forcing them to agree — a
`duplication` whose closure is cross-package and therefore a release event.

---

## F-353 — the boot snippet offers an escape hatch its own registry format cannot express

**Outcome:** EDITED
**Anchors:** 1 edited of 1 —
`spec/boot/11-flow-wal-specspaces.md#READS-THE-SPECSPACES-OWN-BOOT-CONTRACT`
**Files touched:**
`vibevm/vibepacks/org.vibevm.world/wal-specspaces/v0.1.0/vibevm/vibespecs/boot/11-flow-wal-specspaces.xml`
**Re-verification:**

```console
$ grep -n "^- ##FIELD-" vibevm/vibepacks/org.vibevm.world/wal-specspaces/v0.1.0/vibevm/vibespecs/flows/wal-specspaces/SPECSPACES-PROTOCOL.xml
60:- ##FIELD-DEFAULT **`default:`** (optional, above the table) — which target a **bare**
66:- ##FIELD-NAME **name** — the word used in session phrases. Short, unique,
68:- ##FIELD-ROOT **root** — the specspace root, relative to the host root. The
70:- ##FIELD-WAL-AND-CONTINUE **wal**, **continue** — paths relative to root. Defaults are
74:- ##FIELD-STATUS **status** — one line, refreshed at every specspace wind-down:

$ sed -n '55,57p' vibevm/vibepacks/org.vibevm.world/wal-specspaces/v0.1.0/vibevm/vibespecs/flows/wal-specspaces/SPECSPACES-PROTOCOL.xml
| name | root | wal | continue | status |
|---|---|---|---|---|
| fractality | vibevm/vibepacks/org.vibevm.fractality/ | WAL.md | CONTINUE.md | 2026-07-09 — ignition PLANNED; next: Phase 0 |

$ diff <(git show HEAD:vibevm/vibepacks/org.vibevm.world/wal-specspaces/v0.1.0/vibevm/vibespecs/boot/11-flow-wal-specspaces.xml | grep -o '##[A-Za-z0-9-]*' | sort) <(grep -o '##[A-Za-z0-9-]*' vibevm/vibepacks/org.vibevm.world/wal-specspaces/v0.1.0/vibevm/vibespecs/boot/11-flow-wal-specspaces.xml | sort) && echo IDENTICAL
IDENTICAL
```

**Who falsifies it:** the document itself — the registry format is defined four
files away *in the same package*, and it has no column that could name a boot
contract.
**What changed and why:** the step read «(`CLAUDE.md` at the specspace root, **or
the file the registry names**)». The registry this same package defines carries
`default`, `name`, `root`, `wal`, `continue`, `status` and nothing else, so the
alternative is unimplementable — and the package's own `##FIELD-ROOT` says
something different again, placing «the specspace's `CLAUDE.md` (or equivalent
boot contract)» *at the root* rather than naming it in a registry column. Two
statements, one package, one of them impossible. The snippet now reads «or the
equivalent boot contract living there», which is `##FIELD-ROOT`'s own wording and
leaves the same latitude without promising a column that does not exist. This
tightens rather than loosens: no host practice becomes admissible that was not
already. Anchor set byte-identical to HEAD; no `../` link added.
**New obligations noticed:** none new — but note the repair deliberately does not
touch `##FIELD-ROOT`, which is already the correct half.

---

## F-354 — «its status column is a pointer, never canonical state»; the live column is 1 062 characters of state

**Outcome:** OUT-OF-ROUTE
**Anchors:** 0 edited of 1 — `SPECSPACES-PROTOCOL.xml#SUM-THE-REGISTRY`
**Files touched:** none
**Re-verification:**

```console
$ awk -F'|' '/^\| fractality/ {print length($6)" chars"}' SPECSPACES.md
1062 chars
```

**Who falsifies it:** the host — it wrote a paragraph of state into a cell the
rule reserves for a one-line pointer.
**What changed and why:** nothing. The summary restates two body rules,
`##FIELD-STATUS` («one line … A pointer for the *host's* readers; never the
specspace's canonical state») and `##LAW-STATE-LOCALITY`, and both are sound;
what is broken is the one host cell that fills them. Broadening «never canonical
state» to admit 1 062 characters would repeal state locality — the second of the
five laws — to accommodate one row. Host obligation: `SPECSPACES.md`'s
`fractality` status cell must collapse to one line: date, phase, next step.
**New obligations noticed:** the first two clauses of the summary hold exactly
(one registry at the host root, a `default:` line governing bare phrases), so
this row is a candidate for a partial re-judge if a later wave splits summary
anchors per clause.
