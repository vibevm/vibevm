# the three discipline-mcp briefs â€” three-way diff

_Captured 2026-07-28 against the three `-mcp` packages' `spec/tools/`._

C7's evidence, and a re-derivation of **F-116** by machine. That finding was filed
from reading the three briefs side by side during B16; this is the same comparison
run as a command, and it reproduces all three of the finding's normative items
while adding the counts.

```console
$ python campaigns/packages-2026-09/tasks/scaffold-three-way.py discipline-mcp
three-way scaffold diff — 1 card(s) × 3 languages, view --anchors

scaffold-discipline-mcp: rust=33, ts=32, go=34; shared 29, divergent 9
    only in go+rust          missing from ts           — REPORTS-CARRY-THE-RUNS-ENTIRE-STORY
    only in rust             missing from go+ts        — ROW-LEDGER-RENDER
    only in go               missing from rust+ts      — THE-GO-ORACLE-STANDS-ON-GO-TYPES
    only in go               missing from rust+ts      — THE-GO-UMBRELLA-HAS-NO-LEDGER-COMMAND
    only in ts               missing from go+rust      — THE-TS-ORACLE-IS-THE-COMPILER
    only in ts               missing from go+rust      — THE-TS-UMBRELLA-HAS-NO-LEDGER-COMMAND
    only in rust             missing from go+ts        — THIRTEEN-THIN-ADAPTERS-OVER-THE-LIB-FNS
    only in go+ts            missing from rust         — TWELVE-THIN-ADAPTERS-OVER-THE-LIB-FNS
    only in go+rust          missing from ts           — parity-map-lead

anchors present in some languages and not others: 9
EXIT=0
```

```console
$ python campaigns/packages-2026-09/tasks/scaffold-three-way.py --words discipline-mcp
        rust-only vs go: seam variant
        go-only        : (none)
    WORDS DIFFER  ROW-FLOOR
        rust-only vs ts: fast loop
        ts-only        : (none)
        rust-only vs go: fast loop
        go-only        : (none)
    WORDS DIFFER  SERVED-OVER-MCP-AS-ONE-STDIO-BINARY
        rust-only vs ts: eighteen
        ts-only        : seventeen
        rust-only vs go: eighteen
        go-only        : seventeen
    WORDS DIFFER  SERVING-NEEDS-NO-VIBE-ON-THE-MACHINE
        rust-only vs ts: in the consuming repo
        ts-only        : (none)
        rust-only vs go: in the consuming repo the live chain scrubs path to prove it
        go-only        : (none)
    WORDS DIFFER  TCG-VALIDATE-ISERROR-MIRRORS-THE-ONE-SHOT-EXIT-CONTRACT
        rust-only vs ts: grade
        ts-only        : (none)
        rust-only vs go: grade
        go-only        : the filled markers stream rides every validate the relay s named delta over the one
    WORDS DIFFER  THE-SPECMAP-GATE-FORM-STAYS-CLI-ONLY
        rust-only vs ts: its audience is package gates not agents
        ts-only        : as on the side
        rust-only vs go: its audience is package gates not agents
        go-only        : as on the sibling servers
    WORDS DIFFER  THE-TOOLS-CALL-THE-SAME-LIB-FNS-THE-CLIS-CALL
        rust-only vs ts: (none)
        ts-only        : (none)
        rust-only vs go: x y z
        go-only        : 0 0
    WORDS DIFFER  TOOL-LEVEL-FAILURE-IS-AN-ISERROR-RESULT
        rust-only vs ts: a refusing oracle carrying the report
        ts-only        : an absent toolchain refusal with its recipe
        rust-only vs go: a refusing oracle carrying the report
        go-only        : an absent toolchain refusal with its recipe

anchors present in some languages and not others: 9
shared anchors whose content words differ:        12
```

**Which divergences are language and which are drift.** Legitimate: rust ships a
`ledger` command and the other two say they do not (`ROW-LEDGER-RENDER` against the
two `â€¦-UMBRELLA-HAS-NO-LEDGER-COMMAND` anchors), and the adapter and tool counts move
with it coherently â€” rust Â«thirteenÂ»/Â«eighteenÂ», ts and go Â«twelveÂ»/Â«seventeenÂ».
Drift: **`REPORTS-CARRY-THE-RUNS-ENTIRE-STORY` and `parity-map-lead` are in rust and
go and absent from TypeScript** â€” the capture guarantee and the parity-map claim, both
of them normative, missing from one projection of three; and the `force`-class rule
inside `HEAVY-TOOLS-SAY-EXPECT-MINUTES-AND-NOTHING-PROMPTS` is stated by rust alone.

**Scope:** every fact in the three `discipline-mcp-*.md` briefs. The anchor list is not maintained here â€” a verdict cites this file in its `ev[]`, and the reverse index is derived at the phase close (PHASE-C-BATCH-PLAN.md Â§5).
