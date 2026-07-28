# W5 — operating-modes, health-audit, manual-tests, secrets-hygiene: the three sources

_Captured 2026-07-28 at the W5 opening. Every number below is the output of the
command printed above it._

W5 is the batch where **source 2 is at its most literal anywhere in `world`**.
All four flows describe practices the host performs and leaves artefacts for:

- **`operating-modes`** — the host keeps a codeword catalogue in its own contract
  tree (`spec/common/PROP-006-operating-modes.md`) and surfaces it at boot from
  `spec/boot/90-user.md`. One codeword is in force and its invocations are in the
  commit record.
- **`health-audit`** — the host keeps `AUDIT.md` at the root, 24 363 bytes, three
  dated runs. The flow's cadence rule is checkable against the gap since the last.
- **`manual-tests`** — the host has **three** manual-test homes with two numbering
  schemes, and the flow asks for one index that marks required runs.
- **`secrets-hygiene`** — the flow's fourth law is «redaction is tested, not
  promised», and the host's crates either carry those tests or do not.

None of this is a reading habit. **This is the batch where a rule-fact can be
settled against an artefact rather than argued about** — and by the same token,
the batch where «non-adoption is not drift» will do the least work, because these
practices are adopted.

## Source 1 — the package agreeing with itself {#source-1}

```console
$ python campaigns/packages-2026-09/tasks/source1-join.py \
    packages/org.vibevm.world/operating-modes \
    packages/org.vibevm.world/health-audit \
    packages/org.vibevm.world/manual-tests \
    packages/org.vibevm.world/secrets-hygiene
source-1 join over 25 file(s) under packages/org.vibevm.world/operating-modes, packages/org.vibevm.world/health-audit, packages/org.vibevm.world/manual-tests, packages/org.vibevm.world/secrets-hygiene
  relative .md citations resolved: 35
  broken: 0
```

**Thirty-five relative citations, none broken** — the largest clean count of any
world batch (W1 11, W2 23, W3 24, W4 37 with 2 broken). The mechanical half of
source 1 is clean.

## Source 3 — the installed reality {#source-3}

```console
$ python campaigns/packages-2026-09/tasks/source23-boot-join.py | grep -E 'operating-modes|health-audit|manual-tests|secrets-hygiene'
  org.vibevm.world/operating-modes  [INSTALLED SOURCED WORDS-DIFFER]
    installed: vibedeps/flow-operating-modes/0.1.0/spec/boot/45-flow-operating-modes.md
    source   : packages/org.vibevm.world/operating-modes/v0.1.0/spec/boot/45-flow-operating-modes.md
```

**Three of the four are clean** — `health-audit`, `manual-tests` and
`secrets-hygiene` do not appear on the join's problem list at all, so each is
INSTALLED, SOURCED and word-identical to what the host boots.

### `operating-modes`' eight differing words are the instrument, not the corpus {#instrument}

```console
$ python - <<'PY'   # word-stream diff, package source vs the compiled host lane
  only in package: ['recognise', 'a', 'codeword', 'by', 'intent', 'not', 'exact', 'wording']
  only in host   : []
$ grep -nE '^\s*-?\s*##[A-Za-z][A-Za-z0-9_.:-]*\s*$' packages/org.vibevm.world/operating-modes/v0.1.0/spec/boot/45-flow-operating-modes.md
30:##RECOGNISE-A-CODEWORD-BY-INTENT-NOT-EXACT-WORDING
```

The join's `strip_markup` removes a fact anchor only when a space or tab follows
it (`##[A-Za-z][A-Za-z0-9_-]*[ \t]+`), so an anchor alone on its line survives the
strip and is counted as prose. All eight words are that one anchor. **Do not write
a drift row on it.** Same class as W4's `##COLD-FACTS-VERIFIED-AT-WRITING-TIME`
and `##sibling-document-pointers`.

### The shipped skill reaches `vibedeps/` and stops there {#skill-gap}

`health-audit`'s boot snippet says «Use the **`health-audit`** skill».

```console
$ ls packages/org.vibevm.world/health-audit/v0.1.0/spec/skills/health-audit/
SKILL.md
$ ls vibedeps/flow-health-audit/0.1.0/spec/skills/
health-audit
$ ls .claude/skills/
rust-ai-native-sweep
rust-ai-native-terraform
typescript-ai-native-sweep
typescript-ai-native-terraform
vibevm
```

**The skill ships, installs, and is not materialised into the harness the host
actually runs.** Five skills are projected into `.claude/skills/`; `health-audit`
is not among them. Check the same for any other skill a W5 fact names, and note
that `vibe skill install` is the mechanism — its absence here is a fact about the
host's projection step, not about the package.

### The sibling-pointer family, per file {#dangling}

```console
$ for f in <the four boot snippets>; do grep -oE '\.\./flows/[^)]*' "$f" | wc -l; done
  operating-modes    2
  health-audit       3
  manual-tests       1
  secrets-hygiene    3
$ ls spec/
WAL.md  boot  common  design  manual-tests  modules  terraforms
```

**The host has no `spec/flows/` directory**, so all nine pointers resolve nowhere
in the consuming project — W1's 69-dangling finding, in its ninth and tenth
packages. It is a fact about the pointer, not about the rule the pointer sits
under.

**One trap specific to this batch:** the host DOES have `spec/manual-tests/`, so a
`manual-tests` pointer of the form `../flows/manual-tests/…` is still dangling
while a claim about `spec/manual-tests/` is not. Read which one the fact makes.

## Source 2 — the host's observed conformance {#source-2}

### operating-modes — the catalogue exists and is a pointer {#s2-modes}

```console
$ sed -n '5p;13p' spec/common/PROP-006-operating-modes.md
##status-line **Status:** accepted 2026-05-06; the framework and its codewords were extracted to the `operating-modes` flow 2026-07-14 (reached via the redbook dependency). This entry is now a thin pointer.
##CATALOGUE-AT-BOOT The codeword catalogue is surfaced at session boot by [`spec/boot/90-user.md`](../boot/90-user.md). @freeze/done
```

The host's own PROP was **extracted into this flow on 2026-07-14** and reduced to a
pointer that cites the flow by qualified `spec://` URI. `spec/boot/90-user.md`
carries the catalogue at boot with one codeword in force —
«move fast and break things» — and restates the red-lines law verbatim.

```console
$ git log --oneline --all -i --grep='mfbt\|move fast and break' | wc -l
14
```

**Fourteen commits mention the codeword**, so the mode is not theoretical. Whether
each of the flow's activation-lifecycle steps (acknowledge which mode is active,
apply, drop back at cycle end) leaves a trace is a per-fact search — the
acknowledgement is a chat act and may leave none, which is a **non-adoption**
shape, not drift.

### health-audit — the artefact exists and the cadence is measurable {#s2-audit}

```console
$ ls -la AUDIT.md
-rw-r--r-- 1 olegc 197121 24363 Jul 12 22:54 AUDIT.md
$ grep -nE '^## ' AUDIT.md
20:## Audit run — 2026-05-23 (seed)
154:## Audit run — 2026-06-10 (terraform close-out, instrumented category C)
191:## Audit run — 2026-06-12 (discipline depth — the full AI-Native sweep)
$ grep -oiE '\b(fixed|filed|accepted|open)\b' AUDIT.md | tr 'A-Z' 'a-z' | sort | uniq -c | sort -rn
     37 open
     25 filed
     20 fixed
      8 accepted
```

**Three dated runs, every finding carrying a disposition from the flow's own
four-word vocabulary.** The flow's «never let a finding vanish without a
disposition» is checkable row by row here.

The cadence is the other half:

```console
$ git log -1 --format='%h %ad  %s' --date=short -- AUDIT.md
3656f362 2026-06-12  docs(audit): AUD-0016 dispositioned fixed - the posture is live
$ git log --oneline --since=2026-06-12 | wc -l
1544
$ git log --oneline --since=2026-06-12 -i --grep='M1\.\|milestone' | wc -l
33
```

**The last audit is 2026-06-12 and 1 544 commits have landed since**, 33 of them
naming a milestone. The flow's floor is «at least once per milestone» and its
`#never` is «never declare a milestone done on an un-audited base». Judge each of
those facts on its own sentence: the floor is a cadence claim, the `#never` is a
prohibition, and they can land differently.

### manual-tests — three homes, two numbering schemes, no index {#s2-manual}

```console
$ ls manual-tests/
M1.1-git-registry-smoke.md   M1.15-git-source-smoke.md   M1.16-redirect-smoke.md
M1.17-workspace-publish-smoke.md   M1.5-gate-multi-package-smoke.md
M1.5-gate-v2-per-package-smoke.md  M1.6-mirror-vendor-smoke.md
M2.10-index-smoke.md   README.md
$ ls spec/manual-tests/
MT-01-vibe-tree.md   MT-02-vibe-tree-tui.md   MT-03-vibe-prefs-tui.md
$ grep -n 'MT-05' CLAUDE.md | head -1
131:  2026-07-12 (MT-05 run `01KXBEHEYJCQ1RNJ5657Q31HVA`; host crates inherit via
```

Three homes: `manual-tests/` at the root (8 tests, `M<milestone>` numbering),
`spec/manual-tests/` (3 tests, `MT-NN` numbering, the ones the campaign scope
includes), and `packages/org.vibevm.fractality/…/spec/manual-tests/MT-05-…` inside
the specspace.

**`manual-tests/README.md` is an index in prose and it states the flow's own
three triggers**, in the same order, in the host's words:

```console
$ sed -n '14,22p' manual-tests/README.md
- **Before tagging a milestone.** Walk the tests for every feature
  the milestone claims to ship.
- **After touching an integration surface** (git backend, CLI arg
  parsing, lockfile format, registry layout) even if `cargo test`
  stays green — unit tests use fakes and tempdirs; the real world is
  messier.
- **When a user reports a reproducer.** Add their steps here so the
  next session can replay them.
$ grep -cE '^\|' manual-tests/README.md ; grep -ciE 'required' manual-tests/README.md
9
1
```

**This README predates the flow and the flow was extracted from it**, so the
match is not coincidence — it is provenance. What the flow asks for and this does
not clearly do is mark *which* runs are **required** for a milestone: nine table
rows, one occurrence of «required». And the `MT-NN` home has no index at all.

The host also records outstanding runs outside the tier, in its checkpoint —
MT-02 and MT-03 await owner sign-off. **Cite the durable side of that fact**
(`spec/manual-tests/MT-02-vibe-tree-tui.md` and `MT-03-vibe-prefs-tui.md`
themselves), never the checkpoint.

`grep -rn 'Expected' spec/manual-tests/*.md | wc -l` returns **10** — the flow's
«never write a step without an Expected paragraph» is countable per file.

### secrets-hygiene — the fourth law has machinery {#s2-secrets}

> **CORRECTED by W5d's worker, 2026-07-29 — and this is the worst defect in any
> harvest I have written, because it would have sent the search to the wrong
> files.** The block below originally printed TEN paths and concluded «ten source
> files». The command returns **nineteen**, and my paste was truncated:
> ```console
> $ grep -rln 'redact\|Redact' crates/ --include='*.rs' | wc -l
> 19
> ```
> The truncation dropped **`crates/vibe-publish/src/token.rs`** — the one file
> that matters, where the `Token` wrapper, both hand-written `Debug`/`Display`
> impls and both Law-4 tests actually live. It also dropped `git_publish.rs` (the
> `redact_credentials` scrubber), `git_publish/tests.rs` (six tests of it),
> `lib.rs`, `orchestrator.rs`, `repo_creator_oracle.rs` and three `vibe-registry`
> files. A worker following the truncated list would have checked Law 4 against
> files that only *mention* redaction and concluded it unimplemented. **The
> function count of 11 was right; the file count and the list were not.** Run the
> command; do not read the list below as complete.

```console
$ grep -rln 'redact\|Redact' crates/ --include='*.rs'    # 19 files — first ten only
crates/vibe-cli/src/commands/registry/publish.rs
crates/vibe-cli/src/commands/registry/redirect/create.rs
crates/vibe-cli/src/commands/registry/redirect/update.rs
crates/vibe-cli/src/commands/show/config.rs
crates/vibe-cli/src/commands/workspace/publish.rs
crates/vibe-cli/tests/cli_registry_mgmt.rs
crates/vibe-index/src/scanner/from_github.rs
crates/vibe-publish/src/creator.rs
crates/vibe-publish/src/direct_git.rs
crates/vibe-publish/src/github.rs
   … plus vibe-publish/src/token.rs, git_publish.rs, git_publish/tests.rs,
     lib.rs, orchestrator.rs, repo_creator_oracle.rs and three vibe-registry files
$ grep -rn 'fn .*redact' crates/ --include='*.rs' | wc -l
11
```

**Nineteen source files and eleven redaction functions/tests.**
The flow's law 4 — «a wrapper type that redacts the value on display is backed by
a unit test asserting the value never appears» — is the one law in `world` with a
compiled checker behind it, and it is in this batch. Verify the assertion shape
before writing a row; a `fn redact_*` that is a helper is not the test the law
names.

The host's written contract on the same surface:

```console
$ grep -cE 'publish.token|VIBEVM_PUBLISH_TOKEN' spec/common/PROP-000.md spec/boot/90-user.md
spec/common/PROP-000.md:2
spec/boot/90-user.md:5
```

`spec/boot/90-user.md` carries `##TOKEN-DISCIPLINE`, `##TOKEN-FILE-CONVENTION` and
`##SCOPE-DISCIPLINE` — the host restating this flow's laws in its own vocabulary,
including the sanctioned at-rest location, the env-var precedence, and the
scope-escalation refusal. `spec/common/PROP-000.md` §20 is the governing anchor.
**Both are durable citation targets; prefer them.**

## The twenty-one files and their anchor counts {#files}

Measured from `campaigns/packages-2026-09/run/mirror/`; the total agrees with
`tasks/PHASE-C-BATCHES.json` (`W5 … 21 files, 775 markers, 697 anchors`).

```
health-audit (217)
  20  packages/org.vibevm.world/health-audit/v0.1.0/README.md
  17  …/health-audit/v0.1.0/spec/boot/42-flow-health-audit.md
  71  …/spec/flows/health-audit/HEALTH-AUDIT-PROTOCOL.md
  65  …/spec/flows/health-audit/audit-checklist.md
  30  …/spec/flows/health-audit/running-an-audit.md
  14  …/spec/skills/health-audit/SKILL.md
manual-tests (123)
  18  packages/org.vibevm.world/manual-tests/v0.1.0/README.md
  16  …/spec/boot/44-flow-manual-tests.md
  37  …/spec/flows/manual-tests/MANUAL-TESTS-PROTOCOL.md
  35  …/spec/flows/manual-tests/authoring-rules.md
  17  …/spec/flows/manual-tests/test-template.md
operating-modes (166)
  20  packages/org.vibevm.world/operating-modes/v0.1.0/README.md
  24  …/spec/boot/45-flow-operating-modes.md
  52  …/spec/flows/operating-modes/OPERATING-MODES-PROTOCOL.md
  33  …/spec/flows/operating-modes/mfbt-mode.md
  37  …/spec/flows/operating-modes/writing-a-codeword.md
secrets-hygiene (191)
  20  packages/org.vibevm.world/secrets-hygiene/v0.1.0/README.md
  21  …/spec/boot/57-flow-secrets-hygiene.md
  63  …/spec/flows/secrets-hygiene/SECRETS-HYGIENE-PROTOCOL.md
  47  …/spec/flows/secrets-hygiene/scope-discipline.md
  40  …/spec/flows/secrets-hygiene/third-party-code-consent.md
```

**`health-audit` ships a `SKILL.md` with 14 anchors, and F-092 is already filed
against that shape**: a `SKILL.md`'s YAML frontmatter cannot carry a fact anchor,
across 9 files in the corpus. Confirm rather than re-discover.

**One standing rule for this batch above all others: never print a secret value.**
`secrets-hygiene`'s own subject is credentials. Cite the *source* of a token — a
path, an env-var name — and never its contents. `~/.vibe/*.token` files are not to
be read.

**Scope:** §3.1 sources 1, 2 and 3 for the four flows of batch W5.
