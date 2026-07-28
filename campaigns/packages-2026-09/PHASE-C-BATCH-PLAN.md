# Phase C — the batch plan {#root}

_Written 2026-07-28 at the phase opening, from measurement taken the same day.
Ratified by the owner before the first verdict. The machine-readable file
assignment is [`tasks/PHASE-C-BATCHES.json`](tasks/PHASE-C-BATCHES.json), generated
from `run/mirror/`; this document is the reasoning around it._

Governing contract:
[spec://vibevm/terraforms/packages-actualization#phase-c](../../spec/terraforms/PACKAGES-ACTUALIZATION-CAMPAIGN-v0.1.md#phase-c),
with §3.1 [#world-verdicts](../../spec/terraforms/PACKAGES-ACTUALIZATION-CAMPAIGN-v0.1.md#world-verdicts)
and §3.2 [#ai-native-verdicts](../../spec/terraforms/PACKAGES-ACTUALIZATION-CAMPAIGN-v0.1.md#ai-native-verdicts).
Mechanics are wave 1's and are not re-invented: PROP-043
[§7.1](../../spec/modules/vibe-progress/PROP-043-progress-markup.md#cache) /
[§7.5](../../spec/modules/vibe-progress/PROP-043-progress-markup.md#erasure).

## 1. What the phase actually owes {#size}

Two instruments were run and agreed on the marker count byte for byte:

```bash
cargo run -q -p vibe-cli --bin vibe -- progress report --json --campaign campaigns/packages-2026-09
```

```bash
cargo run -q -p vibe-cli --bin vibe -- progress mirror --campaign campaigns/packages-2026-09
```

| namespace | files | markers | addressable anchors | judged at opening |
|---|---:|---:|---:|---:|
| host | 58 | 4 988 | 4 441 | 4 440 |
| ai-native | 80 | 2 993 | 2 697 | 0 |
| world | 121 | 4 816 | 4 150 | 0 |
| **total** | **259** | **12 797** | **11 288** | **4 440** |

**`addressable` is the unit a verdict is written in**, and it is not the marker
count. A verdict map keys on facts that are marked *and* carry an `##ID`; a marked
table cell inherits its row's anchor and a document- or section-level marker goes
into the per-file `_elements` bundle. Both are excluded by
`progress_core::seal::addressable`, which is the function the exit gate's coverage
check runs through — so the number that matters is 11 288, not 12 797.

**The phase owes 6 848 anchor verdicts** — 2 697 ai-native + 4 150 world + one on
the host — plus 201 `_elements` bundles, one per package file. Call it **≈ 7 049
records**.

### The kick-off's headline was a counting-unit slip {#unit-slip}

`PHASE-C-KICKOFF.md` says «roughly 11 900 markers carry no verdict», derived as
`12 797 − 921`. But **921 is a count of units** — `baseline.json` carries a
`units` dict of exactly that length — while 12 797 counts markers. The subtraction
crosses units, and the error is the same class B16's executor named at the close of
Phase B: *state the unit with every count*. The honest figures are 7 809 markers
without a verdict, or 6 848 in the unit a verdict is actually written in.

The consequence is a smaller phase than advertised. Wave 1's Phase C judged 4 455
units; **wave 2 is 1.54× that, not 2.6×.**

### What the host still owes {#host-debt}

The host is 4 440 of 4 441 — Phase C inherits three small debts, not a cluster:

- one unjudged anchor, `CACHE-TALLY-COMPUTED` in PROP-043, added by wave 1's own
  close-out after the file had been judged;
- five files whose `processed_hash` no longer matches their `content_hash` —
  `spec/boot/00-core.md`, `MT-02-vibe-tree-tui.md`, `PROP-026-tcg-tool-family.md`,
  `PROP-043-progress-markup.md`, `PROP-003-dep-evolution.md`;
- five orphan verdict keys naming anchors that no longer exist — `authority-line`
  and `status-line` in both `loading-and-boot-model.md` and
  `workspace-and-qualified-naming.md`, and `related` in `tui-visual-language.md` —
  left by Phase D/E re-authoring.

The host's cache arithmetic closes exactly on those three debts, which is the
check that they are the whole list: **4 440 judged anchors + 53 `_elements`
bundles + 5 orphans = 4 498**, the verdict-key count in `run/cache.json`.

## 2. Three decisions taken at the opening {#decisions}

*All three were surfaced with their measurements and ratified by the owner,
2026-07-28, before the first verdict was written.*

### 2.1 A verdict records its source class in a field, not in prose {#src-field}

**Decision.** The verdict object grows a third key: `{v, ev[], src:[…]}`, where
`src` lists which of §3.1's three source classes the evidence rests on. A `world`
verdict whose `src` is `[1]` alone is **self-referential** — the package agreeing
with itself — and is counted separately in the phase summary, per amendment A2.

**Why:** the exit gate's clause (iv) has to be *counted*, and counting is what
picks the representation. Both readers of a verdict take only `v` and `ev`
(`baseline/project.rs`), so a third key is inert to everything shipped.
**Considered and rejected:** encoding the class as a prefix inside each `ev`
string. That makes the gate's own count a substring match over a data file, which
is the trap this campaign has now walked into three times — most recently inside
its own correction (F-065). **Revisit when:** a fourth source class is admitted,
which §3.1 says would itself be the signal that the genre needs one.

### 2.2 The subject is not modified to make the measurement pass {#no-init}

**Decision.** `<lang>-ai-native init` is **not** run against any package under
verification. The checkers are run as the packages ship, and their real output —
including the refusal — is the evidence.

**Why:** the first probe of §3.2 returned this, in 0.03 s:

```console
$ ./target/debug/rust-ai-native.exe conform check
conform: NO conform.toml — topology default in force, nothing is gated; run `rust-ai-native init` to write a starting policy.
Error: conform.toml: crate `rust-ai-native-cli` is neither gated nor exempt — classify it
```

**No slot under `packages/org.vibevm.ai-native/` carries a `conform.toml` or a
`discipline/` directory** (`specmap.toml` is present in six of eleven). Running
`init` would create the policy the measurement is asking about — a green result
manufactured by the act of measuring. The mandate asks whether the discipline holds
itself to its own rule; the answer is the unmodified run.
**Considered and rejected:** initialising a scratch copy of each package and
gating that. It measures a package that nobody ships. **Revisit when:** a package
lands a `conform.toml` on its own release line — then the run measures the policy
rather than its absence.

### 2.3 `vibedeps/` stands in for §3.1's third source {#source-three}

**Decision.** «The installed reality» is read from `vibedeps/<slot>/` on disk plus
the generated `spec/boot/STATIC.md` and `spec/boot/INDEX.md`, **not** from the
lockfile's `files_written`.

**Why:** the field §3.1 names is empty for every package that exists here.

```bash
grep -c "files_written = \[\]" vibe.lock
```

returns **36** against 36 `[[package]]` blocks. Taking §3.1 literally would leave
source class 3 unusable, and every `world` verdict would be self-referential by
construction — an artefact of the instrument, not a fact about the corpus. All 27
`world` flows are materialised under `vibedeps/`, so the substitute is available
for the whole cluster; the three `go-*` slots are **not** installed and their
checkers are run in place. **Considered and rejected:** running a real
`vibe install` to populate the field. That is a tree-wide side effect in service of
a measurement, and it is the same error as 2.2 wearing different clothes.
**Revisit when:** `files_written` is populated by the installer — at which point
this becomes the stronger evidence and should be preferred.

*A finding follows from this decision and is filed rather than fixed here: the
lockfile documents a field it does not write.*

## 3. How the corpus is cut {#principle}

Batches are cut by **evidence locality** — files whose verdicts rest on the same
run travel together. That is what makes §3.2 cheap: one captured `floor` answers
for a whole slot's claims about gates and engines, and a batch that spans two slots
pays for two.

Two structural facts found while measuring shaped the ai-native cut:

- **The three language stacks carry a parallel corpus.** Eight `spec/cards/scaffold-*.md`
  files at *exactly* 13 anchors in each of rust, typescript and go, plus
  `scaffold-d` at 51/52/52. That is 467 anchors — 17 % of the cluster — in three
  near-copies, and the instrument for it is a three-way diff: a divergence with no
  language reason is drift.
- **`core-ai-native` has no umbrella CLI and therefore no floor.** Its crates are
  engines (`-conform`, `-specmap`, `-specmark`, `-specmark-grammar`, `-mcp`); §3.2's
  floor step does not exist for it, and its evidence is the engines run directly.

## 4. The batches {#batches}

Order: **C0 → C1…C7 → W1…W7.** The §3.2 cluster runs first because its captured
runs are the evidence for most of that namespace, per §5-C of the plan.

### C0 — spike: the harvest, and a control on the mechanism {#c0}

No verdicts. Two jobs:

1. **The captured runs** (exit-gate clause (iii)) — `floor`, `conform check`,
   `specmap --check` / `--gate` and `health` over every code-bearing slot, each
   written to `harvest/<package>-<what>.md` as *command → real output*, verbatim.
   Seven slots carry an umbrella CLI; `core-ai-native` is captured through its
   engines.
2. **A control on the cache mechanism, on ground where the right answer is already
   known.** Re-derive the five stale host files, write the one missing verdict,
   `vibe progress seal` them, and diff `run/cache.json`: the 53 campaign maps the
   batch does not touch must come through byte for byte. The load-and-merge law is
   asserted before 7 000 records ride on it, not after.

### The §3.2 cluster — ai-native {#ai-native-batches}

| batch | what | files | markers | anchors |
|---|---|---:|---:|---:|
| **C1** | `core-ai-native` — the mechanism specs (PROP-014, BROWNFIELD, ENGINE-CONFORM, LEDGER-INTENT, MCP-CORE) | 5 | 412 | 353 |
| **C2** | `core-ai-native` — guiding layer, playbooks, appendix, README, boot | 11 | 548 | 485 |
| **C3** | the three language GUIDEs | 3 | 393 | 390 |
| **C4** | the nine scaffolds × three languages | 27 | 494 | 467 |
| **C5** | TCG mechanisms and tools × three | 14 | 617 | 573 |
| **C6** | skills, boot snippets, READMEs | 17 | 375 | 330 |
| **C7** | the three `discipline-mcp-*.md` briefs | 3 | 154 | 99 |
| | **cluster** | **80** | **2 993** | **2 697** |

Two batches open with a filed finding already pointed at them: **C7 is F-116**
(twelve divergences across the three briefs, three of them normative), and **C6
contains no README for `typescript-ai-native-lang`** — F-115, confirmed by the
file's absence from the generated assignment rather than by re-reading the finding.

### The §3.1 cluster — world {#world-batches}

| batch | packages | files | markers | anchors |
|---|---|---:|---:|---:|
| **W1** | the git family — `git-practices`, `-atomic-commits`, `-conventional-commits`, `-attribution-policy`, `-autonomy` | 16 | 472 | 407 |
| **W2** | `two-process-model`, `wal`, `wal-specspaces`, `sync-from-code` | 20 | 745 | 692 |
| **W3** | `addressable-specs`, `decision-records`, `conflict-protocol` | 15 | 770 | 615 |
| **W4** | `campaign-plans`, `discovery-prompt`, `comparative-research`, `redbook` | 15 | 651 | 564 |
| **W5** | `operating-modes`, `health-audit`, `manual-tests`, `secrets-hygiene` | 21 | 775 | 697 |
| **W6** | `licensing`, `source-mirrors`, `spec-genres`, `dev-runtime-docs` | 19 | 700 | 572 |
| **W7** | `managed-blocks`, `qualified-naming`, `tool-design-lessons` | 15 | 703 | 603 |
| | **cluster** | **121** | **4 816** | **4 150** |

W1 leads the cluster for a reason unrelated to its size: for the git family,
§3.1's source 2 — the host's observed conformance — is literally this repository's
`git log`, which is the cheapest and most independent evidence anywhere in `world`.

**W2 and W5 are provisional at 692 and 697.** The unit is anchors. After the first
world batch closes, the per-anchor cost is re-measured and they split if it is
higher than C1's; a derived number in a plan goes stale and nothing recomputes it,
so this one is recomputed on purpose.

## 5. Evidence, and how a harvest file is cited {#evidence}

A verdict's `ev[]` cites the harvest file by path — `harvest/rust-ai-native-lang-floor.md` —
and the harvest file carries the command, the output, and the scope it covers. It
does **not** carry the list of anchors it proves: that list is derivable from the
verdict maps at the phase close and is back-filled there mechanically. A
hand-maintained list on both sides of the same fact is two writers for one
statement, and this campaign has already measured three of those going stale
(F-077).

Running `health` writes `discipline/health/latest.json` **into the package it
measures**. The harvest run records that it does, and the artefact is removed
afterwards — it is an output, not a policy, so removing it leaves the subject as
shipped (§2.2).

## 6. Delegation {#delegation}

**Owner ruling, 2026-07-28: delegated execution in this phase routes to the
harness's built-in `opus5` subagent, not to fractality.** This is a deliberate
local override of the delegation-first flow's `#route` rule for this campaign, and
it is recorded here rather than assumed.

What that changes and what it does not:

- **Verdicts are never delegated.** The plan names the executor as «Fable + machine
  evidence»; a verdict is judgment, and judgment plus the review of delegated output
  are both in the never-delegate set.
- **The captured runs are not delegated either** — for economics rather than
  principle. Each is a sub-second command; packaging the task costs more than the
  task.
- **Two bulk jobs are delegated,** both scoring high on verifiability: the world
  cluster's source-1 join (for every `spec://` URI and relative link in 121 files,
  does the target exist and carry the cited anchor?) and the three-way diff of C4's
  parallel corpus. Each returns a table the boss spot-checks by resolving rows at
  random. **The verdicts written from those tables are the boss's.**

## 7. The exit gate, restated as five checks {#gate}

1. **100 % of markers carry verdicts** — measured by `vibe progress seal` refusing
   nothing across all 259 files.
2. **The X/Y/Z summary in the LOG** — confirmed / drift / unverifiable, per cluster.
3. **The §3.2 runs exist as files** under `harvest/`, each `command → real output`.
4. **Every `world` verdict carries `src`**, and the summary counts `src == [1]`
   separately as self-referential (§2.1 above). No shipped command counts this; the
   phase writes the script that does.
5. **`baseline.json` written at the close** (amendment A6).

**The plan's falsifiable prediction stands unchanged:** `world` measures higher
than `ai-native`. C0's first probe is already evidence against the *reason* given
for it — the ai-native packages do not merely make checkable claims, they make
claims their own checkers currently decline to check.
