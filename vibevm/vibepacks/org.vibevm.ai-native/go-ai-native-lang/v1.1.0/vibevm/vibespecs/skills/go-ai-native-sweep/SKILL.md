---
name: go-ai-native-sweep
description: Run the recurring AI-Native discipline sweep on this Go project — the seven-step floor first, then the health collector's ratchet items, weekly drift and judgment tiers. Use daily or several times a day on an active tree; any single item is a safe stop.
---

<status stage="impl" state="done"/>

# The discipline sweep (Go stack) {#root}

@fact:RUNNING-THE-STANDING-SWEEP You are running the standing sweep from the Discipline's Sweep Playbook
(`spec://org.vibevm.ai-native/core-ai-native/04-SWEEP-PLAYBOOK` — the shipped copy is at
`vibedeps/flow-core-ai-native/<version>/spec/04-SWEEP-PLAYBOOK.md`; read it
once per session if you have not). @status:impl/done

@fact:two-truths-lead The two truths: @status:impl/done

- @fact:TRUTH-GATES-ARE-THE-FLOOR **the gates are the
  floor, the sweep is the ceiling**, @status:impl/done
- @fact:TRUTH-GATE-IS-TRUTH and **the gate is truth, the collector is
  a guide**. @status:impl/done

@fact:NEVER-SWEEP-ON-A-RED-TREE Never sweep on a red tree. @status:impl/done

@fact:ACT-ON-COLLECTOR-FACTS Act on collector facts, never on
memory. @status:impl/done

@fact:ALL-COMMANDS-ARE-THE-SHIPPED-TOOLCHAIN All commands below are the shipped toolchain. @status:impl/done

@fact:IF-NOT-ON-PATH-INSTALL-OR-RUN-IN-PLACE If `go-ai-native` is not on
PATH, either install it once —
`cargo install --path vibedeps/<stack-slot>/crates/go-ai-native-cli` — or
run it in place: `cargo run --manifest-path vibedeps/<stack-slot>/Cargo.toml
-p go-ai-native-cli --bin go-ai-native -- <args>`. @status:impl/done

## Tier 0 — the hard floor (ALWAYS first) {#tier-zero}

```sh
go-ai-native floor
```

@fact:FLOOR-HAS-SEVEN-STEPS Seven steps: gofmt → go vet → go test → staticcheck+exhaustive → conform →
specmap → test-gate. @status:impl/done

@fact:RED-FLOOR-ADMITS-ONLY-GREENING-WORK Red? The only legal work is making it green — fix, do
not proceed. @status:impl/done

@fact:CHECK-THE-PRINTED-POLICY-LINES Check the printed policy lines: a
`NO conform.toml — topology default in force` line means the project is not
bootstrapped (`go-ai-native init`) and any green under it is vacuous, and every
`DISABLED by policy` line is a standing decision to re-question weekly — a
floor that shrank quietly is the failure mode this line exists to catch. @status:impl/done

## Tier 1 — the ratchet (every run) {#tier-one}

```sh
go-ai-native health
```

@fact:READ-THE-HEALTH-SUMMARY Read the summary (the JSON at `discipline/health/latest-go.json` is the
work-list; its git diff is the trend). @status:impl/done

@fact:take-cheapest-wins-lead Take one or two cheapest wins: @status:impl/done

1. @fact:RATCHET-DANGER-BAND-FILES **danger-band files** — split any file at the top of the [540,600) band
   before an edit trips the 600 budget. Go packages are natively
   multi-file: move a cohesive slice into a sibling file of the SAME
   package (GUIDE §15 — the cheapest split of the three stacks); item-level
   `//spec:` tags move with their items. @status:impl/done
2. @fact:RATCHET-SUPPRESSION-CENSUS **suppression census** — every reasonless `//lint:ignore` /
   `//exhaustive:ignore` is unrecorded testimony: add the reason or fix the
   underlying finding. A `t.Skip` on a known-failing test moves to
   `discipline/registry/tests-baseline.json` the day it is found (GUIDE
   §10 — Go has no in-source xfail twin; the registry carries full weight). @status:impl/done
3. @fact:RATCHET-EXAMPLE-COVERAGE **example coverage** — exported seam items without an `Example` are
   retrieval gaps; document the highest-traffic seam first (the four
   Example idioms, GUIDE §15). @status:impl/done
4. @fact:RATCHET-ORPHAN-BACKLOG **orphan backlog** — untagged exported identifiers the ratchet will
   block on: tag the item (`//spec:implements …`), `//spec:scope` its
   package (doc.go), or unexport it. @status:impl/done
5. @fact:RATCHET-CENSUS-REGRESSIONS **census regressions** — `go-ai-native health`'s printed
   `ban census {N} reasoned / {M} unreasoned` (and `ban_census` in the
   snapshot): every `go_unsafe` fact without a `//spec:deviates` reason
   counts unreasoned. The kinds behind the count are `init_decl`,
   `blank_import`, `ambient_call`, `naked_go`, `error_string_match` and
   `seam_error_missing_req`; the collector reports one project-wide total,
   **not** a per-kind or per-package split, so compare the figure against
   the previous run rather than expecting a breakdown. Drain immediately;
   restructure beats testify. Outside the gate they are the adoption
   backlog: **flip a package into `[go] gated` only after it drains to
   zero.** @status:impl/done

## Tier 2 — drift (weekly) {#tier-two}

- @fact:DRIFT-TRIPWIRE `go-ai-native tripwire --base origin/main` — re-disposition every
  touched-and-open debt entry; file new deficiencies into
  `discipline/registry/debt.json`, never leave them as prose. @status:impl/done
- @fact:DRIFT-DOC-CODE Doc/code drift: WAL freshness (if the project keeps one — see
  `06-WAL-CONVENTION`), architecture docs vs the real package layout,
  roadmap staleness. File `stale-doc` debt. @status:impl/done
- @fact:DRIFT-MARKER-CENSUS Marker census: `rg -n 'TODO|FIXME|REVIEW|XXX|HACK'` over the source
  roots — graduate load-bearing markers into the registries, delete
  trivial ones. @status:impl/done
- @fact:DRIFT-GOLDEN-TRANSCRIPTS Golden transcripts (`discipline/golden/`, `testdata/` goldens): must
  fail loudly, re-captured deliberately, never auto-updated — the
  `-update` flag never runs in CI. @status:impl/done

## Tier 3 — deep judgment (weekly) {#tier-three}

@fact:WALK-THE-WISH-RULES Walk the WISH rules over the week's diff (typed seams, cell isolation and
oracles, goroutine ownership, uniformity, contract-first ordering, lying
godoc, closed-vocabulary naming — GUIDE §2–§9). @status:impl/done

@fact:CAMPAIGN-SIZED-BACKLOG-BECOMES-A-RAID If a Tier-1 backlog has
grown campaign-sized, plan a raid instead: `03-RAID-PLAYBOOK` +
`05-CAMPAIGN-FORM`. @status:impl/done

## Closing a sweep {#closing-a-sweep}

@fact:TOPIC-GROUPED-COMMITS Topic-grouped commits, one logical unit each, citing the sweep item. @status:impl/done

@fact:COMMIT-THE-REFRESHED-HEALTH-JSON Commit the refreshed `discipline/health/latest-go.json` in the same run. @status:impl/done

@fact:RESUME-POINTER Resume pointer: **with a WAL** — bump its standing line at any milestone
move; **without** — the closing commit message carries the summary (floor
state, items taken, next candidate). @status:impl/done

@fact:NEVER-LEAVE-STATE-ONLY-IN-THE-CONVERSATION Never leave the sweep's state only in
this conversation. @status:impl/done

## The generation-time assistant (before you edit, not instead of the floor) {#generation-time-assistant}

@fact:STACK-SHIPS-AN-AGENTIC-TYPE-ORACLE The stack ships an agentic type oracle. @status:impl/done

@fact:check-the-hypothetical-content-lead Before writing a nontrivial `.go`
edit, check the HYPOTHETICAL content instead of paying a red floor
iteration: @status:impl/done

```sh
vibe bin exec go-ai-native-tcg -- validate internal/cells/<cell>/<file>.go \
    --content-from - --root .   # the edit on stdin; exit 1 = would fail
```

@fact:MCP-ALTERNATIVE or, when the vibevm MCP server is mounted, call `tcg_validate` with
`language: "go"` and the `content` argument (plus `tcg_scope` /
`tcg_complete` / `tcg_type` for in-scope symbols, type-valid completions,
and quick info). @status:impl/done

@fact:RESPONSES-CARRY-THE-SAME-CONFORM-FINDINGS Responses carry the SAME conform findings as the gate,
flagged `baselined` or new, with guide-citing advice — a new finding in
the answer means the floor WILL go red if you write that edit. @status:impl/done

@fact:ORACLE-PREREQUISITES Prerequisites: go ≥ 1.24 + gopls (`go install
golang.org/x/tools/gopls@latest` — a stack obligation). @status:impl/done

@fact:ORACLE-HONESTY Honesty: gopls
stands on go/types, the reference implementation of the spec — tighter
than rust-analyzer↔rustc, still not the compiler; the floor stays the
truth (TCG-ORACLE-GO §5). @status:impl/done
