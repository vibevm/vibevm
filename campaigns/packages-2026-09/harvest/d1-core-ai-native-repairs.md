# D1 — `core-ai-native` v0.8.0 repairs (eleven self-falsifier obligations)

_Worked 2026-07-29. Subject:
`vibevm/vibepacks/org.vibevm.ai-native/core-ai-native/v0.8.0/`. Every obligation here
carries `falsifier: self` — the falsifying reference sits inside the package,
so route (a) of [§3.6](../PHASE-D-BATCH-PLAN.md#which-side) applies without a
judgement call and the package is what changes. No code was written; no `git`
was run. Written incrementally, one entry per obligation, as each finished._

Obligations: F-121 · F-151 · F-152 · F-159 · F-207 · F-259 · F-260 · F-263 ·
F-266 · F-267 · F-268.

---

## F-260 — the README's mechanism roster names four of the five specs that ship

**Outcome:** EDITED
**Files touched:** `vibevm/vibepacks/org.vibevm.ai-native/core-ai-native/v0.8.0/README.md`
**Re-verification:**

```console
$ cd vibevm/vibepacks/org.vibevm.ai-native/core-ai-native/v0.8.0 && ls -1 spec/mechanisms/
BROWNFIELD-PROTOCOL-v0.1.xml
ENGINE-CONFORM-v0.1.xml
LEDGER-INTENT-v0.1.xml
MCP-CORE-v0.1.xml
PROP-014-specmap-bidirectional-traceability.xml

$ grep -rn "MCP-CORE" --include=*.md .
./spec/mechanisms/MCP-CORE-v0.1.md:1:# MCP-CORE v0.1 — the neutral MCP server transport {#root}
```

The perimeter of that second search is every `.md` file in the package, and it
returns exactly one hit: the mechanism's own title line. Before the edit,
`MCP-CORE-v0.1.xml` was named by no other prose in the package at all — the
README was not merely incomplete, it was the only place a reader would have
looked. The verdict's reason holds exactly as written.

**What changed and why:** `README.md:34` (`##READ-MECHANISMS`) listed
`ENGINE-CONFORM, PROP-014 (specmap), BROWNFIELD-PROTOCOL, LEDGER-INTENT`;
`MCP-CORE` is now the fifth name in that list. Nothing else on the line moved.
The marker stays `@impl/done`: the second half of the fact — that
`spec://org.vibevm.ai-native/core-ai-native/mechanisms/…` is what code tags
cite — is true and independently evidenced at
`crates/core-ai-native-specmark/tests/usage.rs:14`, which cites exactly that
URI form.

**New obligations noticed:** F-260 is **typed wrongly in the registry**. It
carries `type: missing-support`, `rule: r-nothing-exists`, `route:
build-or-demote` — the class whose closure is demotion because nothing
implements the fact. The actual defect is a roster short by one name, with the
fifth mechanism present on disk: that is `reality-mismatch` under
`r-default-described-wrongly`, and its route is a prose edit. Demoting
`##READ-MECHANISMS` to `@spec/done` would have made the README *less* true.
Recorded, not fixed — the registry is derived and never hand-edited
([§3.1](../PHASE-D-BATCH-PLAN.md#closure)).

---

## F-263 — the README's front door says "prompt content only" while five Rust crates ship inside

**Outcome:** EDITED
**Files touched:** `vibevm/vibepacks/org.vibevm.ai-native/core-ai-native/v0.8.0/README.md`
**Re-verification:**

```console
$ cd vibevm/vibepacks/org.vibevm.ai-native/core-ai-native/v0.8.0 && ls -1 crates/*/src/lib.rs
crates/core-ai-native-conform/src/lib.rs
crates/core-ai-native-mcp/src/lib.rs
crates/core-ai-native-specmap/src/lib.rs
crates/core-ai-native-specmark-grammar/src/lib.rs
crates/core-ai-native-specmark/src/lib.rs

$ ls crates/*/src/main.rs; ls crates/*/src/bin
ls: cannot access 'crates/*/src/main.rs': No such file or directory
ls: cannot access 'crates/*/src/bin': No such file or directory

$ grep -rn "\[\[bin\]\]" crates/ Cargo.toml
(no output)
```

`Cargo.toml:11-17` lists all five as `[workspace] members`, and its own header
comment already says the opposite of the README: *«code-bearing since 0.4.0
(PROP-024) … It AUTHORS the neutral engine crates»*. The verdict's reason
holds, including its nuance: the perimeter of the last two searches is every
crate manifest and every `src/` in the package, and neither finds a binary
target — so the claim was *nearly* true and still wrong in the first sentence a
consumer reads.

**What changed and why:** `README.md:19` said «This package is prompt content
only.» It now says the package is prompt content **plus** the five neutral
engine crates it authors, names them, and states the part that was actually
true — all five are libraries, with no binary, no `[[bin]]` target and no CLI
of its own. The marker stays `@impl/done`: the fact describes what the package
contains, and after the edit it describes it correctly. The anchor id
`##PROMPT-CONTENT-ONLY` is **deliberately left unchanged** even though it now
reads against its own sentence — renaming it would delete an anchor the
obligation's `anchors` list names, which would break the re-judge and force a
`vibe progress mirror` before `merge-verdicts.py` ([§3.1](../PHASE-D-BATCH-PLAN.md#closure)'s
revisit clause). A stale label is the cheaper defect than a changed addressable
set; flagging it here rather than acting on it.

**New obligations noticed:** the anchor id `PROMPT-CONTENT-ONLY` now contradicts
the fact it names, and no rule in this campaign covers renaming a fact id whose
text has been corrected. That is a real question for the owner — it is the same
shape as `RULE-ANCHORS-IMMUTABLE` and probably belongs beside it — and it will
recur every time a `reality-mismatch` inverts a fact whose id encoded the wrong
claim.

---

## F-266 — the ATLAS reports 16 status-less records as `new`, overstating genuinely-new findings by sixteen

**Outcome:** EDITED
**Files touched:** `vibevm/vibepacks/org.vibevm.ai-native/core-ai-native/v0.8.0/vibevm/vibespecs/appendix/ATLAS.xml`
**Re-verification:** counted every `_class · confidence · status_` line in the
file and bucketed by the status token:

```console
$ cd vibevm/vibepacks/org.vibevm.ai-native/core-ai-native/v0.8.0 && python - <<'PY'
… (regex `^\s+_([^·]+)·([^·]+)·(.*?)_ —` over spec/appendix/ATLAS.md) …
PY
records with a _..._ line: 87
status buckets: {'contradicts': 5, 'new': 15, 'refines': 46, 'known': 5, '(EMPTY)': 16}
EMPTY status count: 16
EMPTY ids: ['BLD-001', 'BLD-003', 'BLD-004', 'BLD-007', 'BLD-008', 'BLD-009',
 'BLD-010', 'BLD-011', 'BLD-012', 'BLD-014', 'BLD-018', 'BLD-019', 'BLD-020',
 'BLD-021', 'BLD-022', 'BLD-023']
evidence class: {'manifesto': 7, 'production': 6, 'benchmark': 48, 'theory': 17,
 'case-study': 6, 'anecdote': 3}

$ … (per-axis count from the same file) …
actual per axis: {'A': 10, 'B': 8, 'C': 9, 'D': 10, 'E': 8, 'F': 4, 'G': 8, 'H': 30} total 87
```

The reason holds in every particular, including the sixteen ids and the fact
that all sixteen are `BLD-*` in axis H. The two distributions the reason calls
sound are sound: the evidence-class counts reproduce exactly, and the per-axis
counts match both the eight section headings' parenthesised numbers and
`##distribution-by-axis`. 46 + 15 + 5 + 5 + 16 = 87, so the old `new=31` was
15 + 16.

**What changed and why:** `ATLAS.xml:14` now reads `refines=46, new=15,
contradicts=5, known=5, unclassified=16`, with a parenthetical naming the
sixteen as the status-less `BLD-*` records in axis H. The total still sums to
87, so the line stays consistent with `##totals-and-research-passes`. The
marker stays `@impl/done` — it is a count of what is in the file, and it is now
the right count. An absent status is reported as absent instead of being folded
into a present one.

**New obligations noticed:** two, both outside my eleven.

1. **`ATLAS.xml:5` `##status-line` claims a generator input that does not
   exist.** It reads *«GENERATED from findings.jsonl (A2: derived, do not
   hand-edit)»*. `find . -name "findings.jsonl" -not -path "./.git/*"` over the
   **entire repository working tree** returns nothing — not in the package, not
   in `crates/`, not in `vibedeps/`, nowhere. So the file is not derived from
   anything that ships, no generator can regenerate it, and «do not hand-edit»
   describes a workflow that cannot be performed. That is a `missing-support`
   defect on a different anchor. It is also why the repair above *had* to be a
   hand-edit: there is no source to fix upstream of the document.
2. **Sixteen records carry no status at all.** The empty third field in every
   `BLD-*` line is the underlying data defect the distribution was papering
   over; classifying them is research work, not a Phase D edit.

---

## F-267 — C-7 calls H6-uniformity unmeasured while the map's own index carries a measurement of it

**Outcome:** EDITED
**Files touched:** `vibevm/vibepacks/org.vibevm.ai-native/core-ai-native/v0.8.0/vibevm/vibespecs/appendix/CONTRADICTION-MAP.xml`
**Re-verification:**

```console
$ cd vibevm/vibepacks/org.vibevm.ai-native/core-ai-native/v0.8.0 && grep -rn "H6" --include=*.md .
./spec/appendix/ATLAS.md:32:  _benchmark · low · refines:H6_ — Surveys + studies: code data improves LLM reasoning/planning…
./spec/appendix/ATLAS.md:38:  _theory · high · refines:H6_ — Models imitate visible neighbors. One way per operation…
./spec/appendix/ATLAS.md:130:  _benchmark · high · refines:H6_ — Name-only obfuscation collapses intent-level summaries…
./spec/appendix/ATLAS.md:132:  _benchmark · med · refines:H6_ — LLM-aided code profiling finds that keeping prompt and test
  examples in the same syntactic paradigm improves rule-extraction accuracy…
./spec/appendix/CONTRADICTION-MAP.md:46:- ##C-7-OPEN-H6-UNIFORMITY **H6 uniformity:** no controlled
  measurement of internal-uniformity effect exists in the literature…
```

Perimeter: every `.md` file in the package. Four ATLAS records are filed under
H6 — DR2-024 (benchmark, low), R3-006 (theory, high), DR1-010 (benchmark,
high), DR1-022 (benchmark, med). DR1-022's own text says it is *«Quantitative
support that intra-corpus syntactic uniformity is an in-context signal»* with a
2.3 %–125 % effect size. So the reason holds: the map's flat *«no controlled
measurement … exists»* is falsified by a record the map's own index carries.

**The reason holds but is slightly over-stated, and the edit is written for the
real cause.** DR1-022 measures uniformity *between prompt and test examples*,
not the internal uniformity of a codebase, so the open question is **narrowed,
not answered**. Deleting the open item would have been the wrong repair —
it would trade a false absolute for a false closure.

**What changed and why:** `CONTRADICTION-MAP.xml:46` now opens «partly measured,
not settled», names DR1-022 with its evidence class and effect size in the
map's own `(benchmark, med)` style, says what that record actually measured,
and scopes what is still open to a codebase's own internal uniformity. The
marker stays `@spec/done` — it remains an open-question record, which is
exactly what `@spec/done` means here.

**New obligations noticed:** **the hypothesis roster H1–H6 is never defined in
the shipped package.** Searching every `.md` for each of `H1`…`H6` outside the
`refines:` / `contradicts:` citation fields returns: H1 and H5 glossed only
inline inside C-1's heading and sides; H6 glossed only inside the line repaired
above; **H2, H3 and H4 matched nothing at all**. Both appendices cite these
hypotheses as though a consumer could look them up, and there is nothing to look
up. Recorded, not fixed — it touches anchors outside my eleven and probably
wants a new fact rather than an edit.

---

## F-259 — C-6 side B cites a `4–7/80` score that no record in the package carries

**Outcome:** EDITED
**Files touched:** `vibevm/vibepacks/org.vibevm.ai-native/core-ai-native/v0.8.0/vibevm/vibespecs/appendix/CONTRADICTION-MAP.xml`
**Re-verification:**

```console
$ cd vibevm/vibepacks/org.vibevm.ai-native/core-ai-native/v0.8.0
$ for d in "4-7" "4–7" "4—7"; do echo "[$d]"; grep -rnF "$d" --include=*.md . ; done
[4-7]
[4–7]
./spec/appendix/CONTRADICTION-MAP.md:40:- ##c-6-side-b **Side B:** even WITH executable scaffolds,
  Haiku-4.5 stayed near the floor (4–7/80); resources amplify, don't create capability. @spec/done
[4—7]

$ grep -rnF "/80" --include=*.md .
./spec/appendix/CONTRADICTION-MAP.md:40:  (the same single line)

$ grep -rn "Haiku" --include=*.md .
./spec/00-MANIFESTO.md:144:  ##BOUNDARY-THERE-IS-A-FLOOR … the weakest models (Haiku-4.5-class …) did *not* recover
./spec/02-EXECUTABLE-SCAFFOLDS.md:33:  ##floor-haiku-stayed-near-it **The floor.** … Haiku-4.5 stayed near the floor.
./spec/02-EXECUTABLE-SCAFFOLDS.md:71:  ##evidence-haiku-did-not-improve Evidence: Haiku did not improve …
./spec/appendix/ATLAS.md:194:  R2C-003 … capability ladder: Claude Opus/Sonnet/Haiku 4.x, GPT-5.4 …
./spec/appendix/ATLAS.md:198:  R2C-008 … weak agents (Sonnet 4.6, GPT-5.4 mini, Haiku 4.5) … (Sonnet 12->64, GPT-5.4-mini 5->53)
./spec/appendix/CONTRADICTION-MAP.md:40:  (the line under judgement)
```

Perimeter: every `.md` file in the package, all three dash variants plus the
denominator on its own. The figure occurs **once**, inside the sentence being
judged. The reason holds exactly.

**The reason holds, and re-verification sharpened the cause.** The *position* is
not baseless — `spec/00-MANIFESTO.md:144` and `spec/02-EXECUTABLE-SCAFFOLDS.md:33`
both assert it, and ATLAS record R2C-008 is the experiment behind it. What has
no record is the **number**: R2C-008 tested Haiku 4.5 as one of three weak
agents and publishes per-model figures for Sonnet 4.6 (12→64) and GPT-5.4-mini
(5→53) only. So the defect is a fabricated-looking precision on a sound
position, not a wrong position.

**What changed and why:** the unsourced `(4–7/80)` is gone from
`CONTRADICTION-MAP.xml:40`. In its place the side now carries an `Evidence:`
clause in the same shape side A already uses (`Evidence: R2C-007 (…)`), naming
R2C-008, giving the two figures that record does publish, and stating plainly
that no Haiku score is on record. This also brings the entry into compliance
with the map's own `##ENTRY-CARRIES-FOUR-PARTS` law, which requires the
evidence on each side. The marker stays `@spec/done`.

**New obligations noticed:** `spec/02-EXECUTABLE-SCAFFOLDS.md:33`
(`##floor-haiku-stayed-near-it`) and `:71` (`##evidence-haiku-did-not-improve`)
assert the same Haiku-stayed-near-the-floor result, and neither cites a record
either. They are outside my eleven and I have not touched them, but they are the
same defect one document over, and closing F-259 alone leaves the package
asserting the result twice more without a source.

---

## F-159 — LEDGER-INTENT describes an entry struct, a GC, a cost metric and a signed release slice, none of which exist

**Outcome:** EDITED
**Files touched:** `vibevm/vibepacks/org.vibevm.ai-native/core-ai-native/v0.8.0/vibevm/vibespecs/mechanisms/LEDGER-INTENT-v0.1.xml`
**Re-verification:** perimeter is every `.rs` file under `crates/` plus every
`Cargo.toml` in the package.

```console
$ cd vibevm/vibepacks/org.vibevm.ai-native/core-ai-native/v0.8.0
$ for t in confidence created_at timestamp model_id prompt_rev; do
    printf '%-12s hits=%s\n' "$t" "$(grep -rn --include=*.rs --include=Cargo.toml -i "$t" crates/ Cargo.toml | wc -l)"; done
confidence   hits=0
created_at   hits=0
timestamp    hits=0
model_id     hits=0
prompt_rev   hits=0

$ for t in '\bgc\b' lru least-recently evict prune size_budget pin_set QueryKind 'enum Query' \
           release_slice sign_ signature ed25519 minisign '\bgpg\b' export; do … done
\bgc\b 0 · lru 0 · least-recently 0 · evict 0 · prune 1 · size_budget 0 · pin_set 0
QueryKind 0 · enum Query 0 · release_slice 0 · sign_ 0 · signature 1 · ed25519 0
minisign 0 · gpg 0 · export 4

$ grep -rniE 'lru|evict|prune|size_budget|pin_set|signature|export' crates/ Cargo.toml
crates/core-ai-native-conform/src/baseline.rs:51:/// fail the gate; stale entries are prune candidates (the file may
crates/core-ai-native-conform/src/sarif.rs:59:    // branch is unreachable; a Result signature would thread an
crates/core-ai-native-conform/src/rules/diagnostics.rs:17,110,111 + rules/mod.rs:9  (all "re-exports", Rust doc prose)
```

Every one of the four non-zero hits is unrelated: `prune` is conform's
baseline-file prose, `signature` is a Rust function signature, `export` is four
doc-comment mentions of Rust re-exports. So the reason holds on all five
anchors. Reading `crates/core-ai-native-specmap/src/ledger.rs` confirms the
rest directly: the stored object is written by `std::fs::write(&slot, &text)`
at line 155 — a text blob, no struct; `Telemetry` at lines 82-88 has exactly
`hits, misses, rot_checks, rot_changed`; the single query kind is
`const PRODUCER: &str = "explain.item/prose-template-1"` at line 132; and the
module header at lines 17-18 says of itself *«Local per checkout; never
shipped, never signed, never exposed»*.

**One thing the reason under-states, recorded because it changes a word in the
edit:** the rot counters exist but are dead. `ledger.rs:79-81` says they are
incremented *«when a re-verification of an epoch-invalidated entry runs (none do
yet — the template producer recomputes from scratch)»*, so of the four shipped
counters only `hits` and `misses` ever move. The edit says that rather than
counting the rot pair as working telemetry.

**What changed and why:** five facts now separate what is designed from what is
built, and all five markers drop `@impl/done` → `@spec/done`.
`##ENTRY-CARRIES-ITS-PROVENANCE-FIELDS` keeps its designed field set and adds
that there is no entry type at all — only `producer`, `epoch` and the cited
spec URIs survive, in the trailing provenance line.
`##GC-IS-LRU-WITH-A-PIN-SET` keeps the LRU-with-pin-set design and states that
`.ledger/objects/` grows without bound. `##TELEMETRY-FEEDS-THE-HEADLINE-METRIC`
names the four shipped counters, says only two move, and marks both cost
measures unbuilt. `##RELEASE-SLICE-IS-EXPORTED-SIGNED-AND-SHIPPED` keeps the
design and states the ledger is local and git-ignored.
`##FAILURE-KEY-UNDER-SPECIFICATION` keeps the closed-enum mitigation and
records that the one kind is a string constant — the very under-specification
the entry was written to close. **No prescription was deleted**: every design
sentence survives verbatim inside its fact.

**New obligations noticed:** `##UNSIGNED-SLICES-ARE-NEVER-EXPOSED-REMOTELY`
(line 87, `@impl/done`) is vacuously true only because nothing is exposed at
all — there is no remote path to guard. It is outside my eleven and untouched,
but it reads as an enforced control and is not one.

---

## F-152 — six LEDGER-INTENT facts depend on machinery that does not exist: two query kinds, the index, the warm copy, the draft-input recompute, the poisoning predicate

**Outcome:** EDITED
**Files touched:** `vibevm/vibepacks/org.vibevm.ai-native/core-ai-native/v0.8.0/vibevm/vibespecs/mechanisms/LEDGER-INTENT-v0.1.xml`
**Re-verification:** same perimeter — every `.rs` under `crates/`.

```console
$ grep -niE 'index' crates/core-ai-native-specmap/src/ledger.rs ; echo "(exit $?)"
(exit 1)

$ grep -rniE 'legacy_unit|classify\.legacy|propose\.links|propose_links' --include=*.rs crates/
(no output)

$ grep -rniE 'proposal' --include=*.rs crates/
(no output)

$ grep -rniE 'warm|action-cache|sccache|bazel' --include=*.rs --include=*.toml crates/ Cargo.toml
crates/core-ai-native-conform/src/facts.rs:182:    /// Batch warm-up: the store calls this ONCE per run with every
crates/core-ai-native-conform/src/facts.rs:188:    fn warm(&self, _pending_files: &[String]) {}
crates/core-ai-native-conform/src/store.rs:129,153  (the same Frontend::warm call)
```

The four `warm` hits are a batch warm-up hook on the **conform fact store**, a
different store this module explicitly does not touch (`ledger.rs:2-5`), so
there is no CI warm copy of `.ledger/`. `EdgeProvenance::Proposed` does exist as
a wire variant (`generated/specmap/mod.rs:62-63`) — a value the format can
carry, with no producer that writes it, exactly as the reason said. The reason
holds on all six anchors.

**Where the reason holds but the cause differs.** On `##ROW-CLASS-INTERPRETATIONS`
the reason says *«three [of six key components] are absent»*. Only **two** are
absent outright — `prompt rev` and `model id`, both zero hits crate-wide. The
third, «spec revs touched», is not a distinct key component but does enter the
key: `ledger.rs:136` hashes `PRODUCER + epoch + subject`, and the subject is the
serialised explain subgraph, whose edges carry `pinned_r`. The edit says that,
rather than repeating the reason's count.

**What changed and why:** six facts demoted `@impl/done` → `@spec/done`, each
keeping its design sentence and gaining a plain statement of what is not built.
`##ROW-CLASS-INTERPRETATIONS` splits designed key vs shipped key and designed
examples vs the one shipped kind. `##RECOMPUTE-DECISION-HAPPENS-ABOVE-THE-FLOOR`
records that the producer renders from scratch on every miss.
`##STORAGE-LAYOUT-IS-SHARDED-LIKE-GIT-OBJECTS` marks the sharded layout built
and the index and warm copy not. `##QUERY-CLASSIFY-LEGACY-UNIT` and
`##QUERY-PROPOSE-LINKS` are marked specified-not-shipped.
`##FAILURE-CACHE-POISONING` records that `prompt_rev` is not in the key and the
only remedy today is deleting `.ledger/`. No code was written.

**One edit outside the six anchors, flagged deliberately.** The section heading
at line 74 read **«## 6. Query kinds shipped in v0.1 {#queries}»**. With two of
its four members now marked not-shipped, that heading asserted the opposite of
the facts beneath it — the demotion would have *created* a fresh intra-document
contradiction. I changed it to «## 6. Query kinds in v0.1». **The `{#queries}`
anchor is unchanged**, so the file's addressable set is untouched; only the
unit's heading text and content hash move. Reverting this is a one-line change
if the boss would rather keep the heading and open a separate obligation.

**New obligations noticed:** none beyond the heading above — the two shipped
kinds, `facts.extract` and `explain.item`, were not in my anchors and I did not
verify them.

---

## F-151 — six BROWNFIELD facts rest on machinery nothing implements: golden transcripts, conflict detection, the REPORT, the reconciliation check, the close quota

**Outcome:** EDITED
**Files touched:** `vibevm/vibepacks/org.vibevm.ai-native/core-ai-native/v0.8.0/vibevm/vibespecs/mechanisms/BROWNFIELD-PROTOCOL-v0.1.xml`
**Re-verification:** two perimeters, because the reason names the Rust stack's
CLI and that lives in a different package.

```console
### perimeter A — this package: crates/**/*.rs + *.toml + *.json
$ cd vibevm/vibepacks/org.vibevm.ai-native/core-ai-native/v0.8.0
$ for t in golden characterization transcript snapshot 'burn.down' 'half.life' \
           shrinkage unaccounted quota flatline entrench; do … done
golden 2 · characterization 3 · transcript 0 · snapshot 0 · burn.down 0
half.life 0 · shrinkage 0 · unaccounted 0 · quota 0 · flatline 0 · entrench 0

$ grep -rniE 'golden|characterization' --include=*.rs crates/
crates/core-ai-native-conform/src/config.rs:110:  /// substrings is skipped (fixtures, goldens, vendored trees).
crates/core-ai-native-conform/src/rules/structure.rs:150,179,224  (rule text telling a user to write one)
crates/core-ai-native-conform/src/store.rs:370:  /// goldens/fixtures, and build output, mirroring go-extract's own skip

### perimeter B — the Rust stack's CLI (read-only; NOT edited)
$ grep -rlniE 'golden|characterization|transcript' \
    vibevm/vibepacks/org.vibevm.ai-native/rust-ai-native-lang/*/crates/ --include=*.rs
…/rust-ai-native-cli/src/codemod.rs        (one hit: a doc string it emits)
…/crates/vendor/core-ai-native-conform/…   (the vendored copy of perimeter A)

$ find vibevm/vibepacks/org.vibevm.ai-native -iname "*golden*"
(no output)

$ grep -rniE 'insta::|expect_test|trycmd|assert_cmd|stdout_snapshot' \
    …/rust-ai-native-cli/src/ …/core-ai-native/v0.8.0/crates/ --include=*.rs
(no output)
```

Every `golden`/`characterization` hit in both perimeters is prose — a skip-list
comment, a conform rule telling the *user* to write a characterization test, and
the vendored duplicate of the same. No golden file exists anywhere under
`vibevm/vibepacks/org.vibevm.ai-native/`, and no snapshot-testing crate is in use. The
reason holds on all six anchors.

**Where the cause is more precise than the reason.** On `##STATUS-DISPUTED` the
reason says the status is real and the detection is not; re-verification says
exactly *how much* is real, and it is less than «the status»:

```console
$ grep -rn "disputed" --include=*.rs crates/
crates/core-ai-native-specmap/src/mdspec.rs:101:  Some(w) if w.starts_with("disputed(#") && w.ends_with(')') => {
crates/core-ai-native-specmap/src/mdspec.rs:112:  "kind line `{decl}` has an unknown status `{w}` (expected `planned` or `disputed(#anchor)`)"
crates/core-ai-native-specmap/src/generated/specmap/mod.rs:139:  #[serde(rename = "disputed")]
crates/core-ai-native-specmap/src/generated/specmap/mod.rs:184:  /// For `disputed` status: the other anchor of the conflicts_with pair.

$ grep -rniE 'conflicts_with|MUST-NOT|must_not|heuristic' --include=*.rs crates/
crates/core-ai-native-specmap/src/generated/specmap/mod.rs:184  (the doc comment above — the only occurrence)
crates/core-ai-native-specmark-grammar/src/lib/tests.rs:193  (unrelated: fn spec_args_pin_conflict_and_agreement)
```

So what ships is the **declaration**: an author hand-writes `disputed(#anchor)`
and the parser records it. There is no `conflicts_with` edge *type* — only a doc
comment naming one — no MUST/MUST-NOT collision pass, and the duplicate-anchor
check (`mdspec.rs:207`) emits a generic warning wired to nothing. The edit says
that rather than the reason's shorter version.

**What changed and why:** six facts demoted `@impl/done` → `@spec/done`, each
keeping its design sentence and gaining an explicit *Specified, not built*
clause. B4 says it has nothing to pin with yet; `##STATUS-DISPUTED` separates
the shipped declaration from the unbuilt detection;
`##CAPTURE-GOLDEN-TRANSCRIPTS-AT-INVENTORY-TIME`,
`##REPORT-PUBLISHES-THE-EXIT-NUMBERS`, `##EVERYTHING-PLANNED-LANDS-OR-IS-LET-GO`
and `##ANTI-ENTRENCHMENT-CLOSE-QUOTA` each name what has no producer. **Nothing
prescriptive was removed** — this is the §3.3 demotion, not a rule weakening,
and no code was written.

**New obligations noticed:** `##TRIPWIRES-ARE-CHEAP-AND-MECHANICAL` (line 79,
`@impl/done`) sits next to a real `tripwire.rs`, so it is probably fine, but the
three registry facts around it (`##REGISTRY-TESTS-BASELINE`,
`##REGISTRY-DEBT-JSON`, `##REGISTRY-INTENT-JSON`) claim a committed
`discipline/registry/` triple, and only `tests-baseline.json` has visible code
support (`testgate.rs:23`). Outside my eleven, unverified, recorded so someone
checks whether `debt.json` and `intent.json` have producers.

---

## F-268 — a phase gate is redefined to mean "snapshots unchanged" where nothing captures a snapshot

**Outcome:** EDITED
**Files touched:** `vibevm/vibepacks/org.vibevm.ai-native/core-ai-native/v0.8.0/vibevm/vibespecs/mechanisms/BROWNFIELD-PROTOCOL-v0.1.xml`
**Re-verification:** the absence half is the same measurement as F-151 above —
`transcript` and `snapshot` return **0** across this package's crates, no
`*golden*` file exists under `vibevm/vibepacks/org.vibevm.ai-native/`, and no
snapshot-testing crate is in use in either perimeter. The reason's *second*
half — «the floor's actual gate is fmt/test/clippy/conform/specmap» — checks out
and is slightly incomplete:

```console
$ sed -n '1,7p' vibevm/vibepacks/org.vibevm.ai-native/rust-ai-native-lang/v0.7.0/crates/rust-ai-native-cli/src/floor.rs
//! `rust-ai-native floor` — the portable verification floor (Sweep
//! Playbook Tier 0): format → tests → lints → the conform gate → the
//! specmap check → the xfail-strict test-gate (when a baseline registry
//! exists). One command, one exit code, per-step headers, …
```

Six steps, not five: the reason omits the xfail-strict test-gate. The edit uses
the six.

**What changed and why:** `##PHASE-GATES-NOW-MEAN-SNAPSHOTS-UNCHANGED` no longer
says gates *«now mean»* snapshots unchanged — it says they *are to mean* it,
adds that nothing captures a snapshot so no gate can compare one, and states
what a phase gate is today, quoting the floor's six steps. Marker `@impl/done` →
`@spec/done`. The redefinition is kept as the intent it is; only the claim that
it is in force was removed.

**New obligations noticed:** none new — F-268 and F-151's
`##CAPTURE-GOLDEN-TRANSCRIPTS-AT-INVENTORY-TIME` are the same absence seen from
two sentences, and the registry pointed them at two obligations. Worth the
boss's attention only in that closing one without the other would leave the
document self-contradictory again.

---

## F-207 — PROP-014's edge model claims a Brownfield amendment that landed in prose and not in code, and a Phase-0 acceptance that is impossible by design

**Outcome:** EDITED
**Files touched:** `vibevm/vibepacks/org.vibevm.ai-native/core-ai-native/v0.8.0/vibevm/vibespecs/mechanisms/PROP-014-specmap-bidirectional-traceability.xml`
**Re-verification:** perimeter is every `.rs` under the package's `crates/`.

```console
$ grep -rniE 'retired|tombstone' --include=*.rs crates/
(no output — the four-value lifecycle is two values plus an absence)

$ grep -rniE 'conflicts_with' --include=*.rs crates/
crates/core-ai-native-specmap/src/generated/specmap/mod.rs:184:
    /// For `disputed` status: the other anchor of the conflicts_with pair.
(a doc comment on the `disputes` field — no edge type)

$ grep -n "status" crates/core-ai-native-specmap/src/index.rs
(no output — the suspect computation never consults status)

$ grep -rniE 'coverage' --include=*.rs crates/
crates/core-ai-native-conform/src/store.rs:293:    "coverage",           (a word in a skip list)
crates/core-ai-native-specmap/src/explain.rs:121:  /// … (coverage        (a doc comment)
crates/core-ai-native-specmap/src/tripwire.rs:95:  "kind": "coverage-gap" (a JSON test fixture)
```

Reading `index.rs:92-135` confirms the freeze directly: the suspect loop matches
on `unit_revisions.get(uri)` and compares pinned `r` to current `r`, with no
status branch anywhere — a `disputed` unit's edges go suspect exactly like any
other's. The built half of the fact is equally real:
`generated/specmap/mod.rs` carries all five verbs, all three provenance values,
and the `reason` field documented *«Mandatory for `deviates`»*. The reason holds
on both anchors.

**`##PHASE-0-ACCEPTANCE` — the reason holds and the edit sharpens which half.**

```console
$ sed -n '419,424p' crates/core-ai-native-specmap/src/rscan.rs
    fn untagged_items_are_not_inventoried() {
        let src = "pub fn plain() {}\npub struct Plain;\n";
        let (items, edges, _) = scan_source("f.rs", "x", "x", src);
        assert!(items.is_empty());

$ sed -n '456,463p' crates/core-ai-native-specmap/src/index.rs
    fn index_is_deterministic() { … assert_eq!(a, b); … }
```

Determinism is a tested property, so that half stands. «Full node inventory»
cannot: nodes are spec units *and* code items, and the scanner excludes untagged
items by design — `generated/specmap/mod.rs:32` says *«Only items carrying at
least one edge appear; the full-orphan inventory is a later-phase table.»* On an
untouched repo the spec-unit half is full and the code-item half is necessarily
empty. The edit says that instead of «full node inventory».

**What changed and why:** `##EDGE-MODEL-EDGES` now marks the base edge model as
built, labels the Brownfield amendment *specified and not built* with its text
intact, and states precisely what ships of it — the status field alone, two of
four values, no `conflicts_with` type, a status-blind suspect table, no coverage
math. Marker `@impl/done` → `@spec/done`; the fact is mixed, and `@spec/done` is
the honest floor for a fact that promises three unbuilt things.
`##PHASE-0-ACCEPTANCE` keeps its `@spec/done` marker — it was already honest
about stage — and its content is corrected to the split inventory.

**New obligations noticed:** `##INDEX-CONTENTS` (line 315-ish of the same file,
`@impl/done`) promises *«computed tables: coverage per REQ (`{implemented,
verified, documented}` bits), orphans …»*. The coverage grep above covers that
claim too and finds no computation. Outside my eleven, untouched, recorded.

---

## F-121 — the same self-falsifying closing rule in four documents: "anything unexercised is removed", marked done, with the unexercised things still above it

**Outcome:** EDITED
**Files touched:**
`vibevm/vibepacks/org.vibevm.ai-native/core-ai-native/v0.8.0/vibevm/vibespecs/mechanisms/BROWNFIELD-PROTOCOL-v0.1.xml`,
`vibevm/vibepacks/org.vibevm.ai-native/core-ai-native/v0.8.0/vibevm/vibespecs/mechanisms/LEDGER-INTENT-v0.1.xml`,
`vibevm/vibepacks/org.vibevm.ai-native/core-ai-native/v0.8.0/vibevm/vibespecs/mechanisms/PROP-014-specmap-bidirectional-traceability.xml`,
`vibevm/vibepacks/org.vibevm.ai-native/core-ai-native/v0.8.0/vibevm/vibespecs/appendix/CONTRADICTION-MAP.xml`

**Re-verification:** the three closing-rule anchors are settled by the work
recorded under F-151, F-152, F-159 and F-207 above — each of those obligations
found, with commands, that the very items the closing rule promises to have
deleted are still present in the same document. Nothing enforces the rule
either:

```console
$ cd vibevm/vibepacks/org.vibevm.ai-native/core-ai-native/v0.8.0
$ grep -rniE 'unexercised|aspiration' --include=*.rs crates/; echo "(exit $?)"
(exit 1)
```

Perimeter: every `.rs` file under the package's `crates/`. No checker knows the
words, so no checker can enforce the rule.

The fourth anchor is a different defect and I re-verified it separately:

```console
$ grep -n "^## C-" spec/appendix/CONTRADICTION-MAP.md
13:## C-1 — "AI-native = stricter/more meta" (H1) vs "engineered dialects underperform" (H5) …
18:## C-2 — "Specs/context files help" vs "AGENTbench: context files barely help, cost +20%" …
23:## C-3 — "Written strategy transfers capability" vs "only executable scaffolds transfer" …
28:## C-4 — "Type-constrained decoding cuts errors 74.8%" vs "no Rust implementation exists" …
33:## C-5 — Rust benchmark conflict: 58% (SWE-bench Multilingual) vs 10–17% (Multi-SWE-bench) …
38:## C-6 — Optimism vs the floor: "current models are fine" vs "weak models stay near floor" …
43:## C-7 — Unresolved / open (honest) {#c-7-unresolved-open}

$ grep -nE "^- ##(c-|C-)" spec/appendix/CONTRADICTION-MAP.md | cut -c1-46
14:- ##c-1-side-a   15:- ##c-1-side-b   16:- ##C-1-RESOLUTION
19:- ##c-2-side-a   20:- ##c-2-side-b   21:- ##C-2-RESOLUTION
24:- ##c-3-side-a   25:- ##c-3-side-b   26:- ##C-3-RESOLUTION
29:- ##c-4-side-a   30:- ##c-4-side-b   31:- ##C-4-RESOLUTION
34:- ##c-5-side-a   35:- ##c-5-side-b   36:- ##C-5-RESOLUTION
39:- ##c-6-side-a   40:- ##c-6-side-b   41:- ##C-6-RESOLUTION
44:- ##C-7-OPEN-TRANSFER
45:- ##C-7-OPEN-BUILD-USE-BOUNDARY
46:- ##C-7-OPEN-H6-UNIFORMITY
```

(the second listing is one bullet per line in the real output; folded to three
columns here). C-1…C-6 each carry side-a + side-b + a RESOLUTION whose text ends
in a `→ <decision>` clause — all four parts. C-7 carries three open bullets and
no side or resolution at all. So the reason holds: a four-part format law stated
in a document holding a three-part entry.

**What changed and why:** two different repairs for two different defects.

1. **The three closing rules** — `##UNEXERCISED-FIELD-STATUS-OR-POLICY-IS-REMOVED`
   (BROWNFIELD), `##UNEXERCISED-POLICY-IS-REMOVED-NOT-CARRIED` (LEDGER-INTENT),
   `##UNEXERCISED-MECHANISM-IS-REMOVED-FROM-THE-SPEC` (PROP-014). **The rule is
   kept verbatim in all three** — it is a good rule and deleting it to pass a
   measurement is exactly what §3.3 forbids. Each now adds, in identical words,
   that it is standing policy, unenforced, that the sweep has not been run
   against this revision, and that until it is the unexercised items above are
   marked `@spec/done` and named as designed-not-built rather than silently
   carried. Marker `@impl/done` → `@spec/done` on all three. This is what makes
   the demotions in F-151/F-152/F-159/F-207 self-consistent: without it, every
   one of those `@spec/done` markers would itself violate the closing rule
   sitting at the foot of the same file.
2. **The format law** — `##ENTRY-CARRIES-FOUR-PARTS` is scoped to the resolved
   entries C-1…C-6, which do all carry four parts, and now names C-7's shape:
   the open register, question plus why it is still open, no side/resolution
   triple *because there is nothing yet to resolve*. Marker stays `@impl/done` —
   with the scope corrected, the law describes the document accurately.

**New obligations noticed: a fifth instance of the same closing rule that is not
in this obligation's anchor set.** The verdict reasons call the repaired ones the
«THIRD» and «FOURTH» instances «after ENGINE-CONFORM», so I checked:

```console
$ grep -n "unexercised\|not exercised\|aspiration" spec/mechanisms/ENGINE-CONFORM-v0.1.md
109:##UNEXERCISED-FRONTEND-OR-TIER-IS-REMOVED *Any frontend or tier specified here
    that is not exercised by Playbook Phase 4 is removed from this document rather
    than carried as aspiration.* @impl/done
```

`ENGINE-CONFORM-v0.1.xml:109` carries the identical rule, still `@impl/done`, and
is **not** one of F-121's four anchors. It needs the identical treatment or the
package will ship four honest closing rules and one that still claims to be
enforced. I did not touch it. Related and smaller: the four rules name three
different deadlines — «Playbook (v0.2) Phase 2» (BROWNFIELD), «Playbook Phase 5»
(LEDGER-INTENT), «end of Phase 2» (PROP-014), «Playbook Phase 4»
(ENGINE-CONFORM) — so «the sweep» is four sweeps, and nothing says which has run.

---

## Batch verification

All eleven obligations closed as edits; none re-judged confirmed, none blocked.
Six files touched, all inside
`vibevm/vibepacks/org.vibevm.ai-native/core-ai-native/v0.8.0/`:

| file | obligations |
|---|---|
| `README.md` | F-260, F-263 |
| `spec/appendix/ATLAS.md` | F-266 |
| `spec/appendix/CONTRADICTION-MAP.md` | F-259, F-267, F-121 (1 of 4 anchors) |
| `spec/mechanisms/LEDGER-INTENT-v0.1.md` | F-152, F-159, F-121 (1 of 4) |
| `spec/mechanisms/BROWNFIELD-PROTOCOL-v0.1.md` | F-151, F-268, F-121 (1 of 4) |
| `spec/mechanisms/PROP-014-…-traceability.md` | F-207, F-121 (1 of 4) |

**Markup well-formedness, run after the last edit:**

```console
$ cargo run -q -p vibe-cli --bin vibe -- progress check \
    --path vibevm/vibepacks/org.vibevm.ai-native/core-ai-native/v0.8.0 --no-cache
progress check: clean (26 files, 0 warning(s))
EXIT=0
```

All six touched files carry a `<status>` block and are inside the observed set
(the package holds 28 `.md` files; `progress check` observes 26).

**Marker transitions, verified anchor by anchor after the edits.** 21 facts moved
`@impl/done` → `@spec/done`; 4 kept `@impl/done` because the corrected sentence
describes something that *is* built; 2 were already `@spec/done` and stayed.

| stayed `@impl/done` | why |
|---|---|
| `README.md#READ-MECHANISMS` | roster completed; all five mechanism documents ship |
| `README.md#PROMPT-CONTENT-ONLY` | now describes the package's actual contents |
| `ATLAS.xml#distribution-by-status-vs-prior-knowledge` | a count of the file, now the right count |
| `CONTRADICTION-MAP.xml#ENTRY-CARRIES-FOUR-PARTS` | scope corrected; the law now describes the document |

`ROW-CLASS-INTERPRETATIONS`'s fourth cell («**Yes** — hence the epoch in the
key») also keeps `@impl/done`: the epoch genuinely is in the shipped key. The
other three cells of that row are `@spec/done`.

**No code was written, no `git` command was run, and nothing outside
`vibevm/vibepacks/org.vibevm.ai-native/core-ai-native/v0.8.0/` was edited.** Two edits
reach beyond an obligation's own anchor and are flagged in their entries for the
boss to accept or revert: the `## 6. Query kinds in v0.1` heading (F-152) and
the four-instance closing-rule treatment being applied to three of five
instances (F-121, with `ENGINE-CONFORM-v0.1.xml:109` named and untouched).
