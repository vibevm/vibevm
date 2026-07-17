---
name: go-ai-native-sweep
description: Run the recurring AI-Native discipline sweep on this Go project — the seven-step floor first, then the health collector's ratchet items, weekly drift and judgment tiers. Use daily or several times a day on an active tree; any single item is a safe stop.
---

# The discipline sweep (Go stack)

You are running the standing sweep from the Discipline's Sweep Playbook
(`spec://org.vibevm.ai-native/core-ai-native/04-SWEEP-PLAYBOOK` — the shipped copy is at
`vibedeps/flow-core-ai-native/<version>/spec/04-SWEEP-PLAYBOOK.md`; read it
once per session if you have not). The two truths: **the gates are the
floor, the sweep is the ceiling**, and **the gate is truth, the collector is
a guide**. Never sweep on a red tree. Act on collector facts, never on
memory.

All commands below are the shipped toolchain. If `go-ai-native` is not on
PATH, either install it once —
`cargo install --path vibedeps/<stack-slot>/crates/go-ai-native-cli` — or
run it in place: `cargo run --manifest-path vibedeps/<stack-slot>/Cargo.toml
-p go-ai-native-cli --bin go-ai-native -- <args>`.

## Tier 0 — the hard floor (ALWAYS first)

```sh
go-ai-native floor
```

Seven steps: gofmt → go vet → go test → staticcheck+exhaustive → conform →
specmap → test-gate. Red? The only legal work is making it green — fix, do
not proceed. Check the printed policy lines: a `Defaulted` conform policy
means the project is not bootstrapped (`go-ai-native init`), and every
`DISABLED by policy` line is a standing decision to re-question weekly — a
floor that shrank quietly is the failure mode this line exists to catch.

## Tier 1 — the ratchet (every run)

```sh
go-ai-native health
```

Read the summary (the JSON at `discipline/health/latest-go.json` is the
work-list; its git diff is the trend). Take one or two cheapest wins:

1. **danger-band files** — split any file at the top of the [540,600) band
   before an edit trips the 600 budget. Go packages are natively
   multi-file: move a cohesive slice into a sibling file of the SAME
   package (GUIDE §15 — the cheapest split of the three stacks); item-level
   `//spec:` tags move with their items.
2. **suppression census** — every reasonless `//lint:ignore` /
   `//exhaustive:ignore` is unrecorded testimony: add the reason or fix the
   underlying finding. A `t.Skip` on a known-failing test moves to
   `discipline/registry/tests-baseline.json` the day it is found (GUIDE
   §10 — Go has no in-source xfail twin; the registry carries full weight).
3. **example coverage** — exported seam items without an `Example` are
   retrieval gaps; document the highest-traffic seam first (the four
   Example idioms, GUIDE §15).
4. **orphan backlog** — untagged exported identifiers the ratchet will
   block on: tag the item (`//spec:implements …`), `//spec:scope` its
   package (doc.go), or unexport it.
5. **census regressions** (`init_in_cell` / `ambient_call_in_cell` /
   `naked_go_in_cell` / `error_string_match` / `seam_error_missing_req`
   non-zero on a gated package) — drain immediately; restructure beats
   testify. On an ungated package they are the adoption backlog: **flip a
   package into `gated_packages` only after it drains to zero.**

## Tier 2 — drift (weekly)

- `go-ai-native tripwire --base origin/main` — re-disposition every
  touched-and-open debt entry; file new deficiencies into
  `discipline/registry/debt.json`, never leave them as prose.
- Doc/code drift: WAL freshness (if the project keeps one — see
  `06-WAL-CONVENTION`), architecture docs vs the real package layout,
  roadmap staleness. File `stale-doc` debt.
- Marker census: `rg -n 'TODO|FIXME|REVIEW|XXX|HACK'` over the source
  roots — graduate load-bearing markers into the registries, delete
  trivial ones.
- Golden transcripts (`discipline/golden/`, `testdata/` goldens): must
  fail loudly, re-captured deliberately, never auto-updated — the
  `-update` flag never runs in CI.

## Tier 3 — deep judgment (weekly)

Walk the WISH rules over the week's diff (typed seams, cell isolation and
oracles, goroutine ownership, uniformity, contract-first ordering, lying
godoc, closed-vocabulary naming — GUIDE §2–§9). If a Tier-1 backlog has
grown campaign-sized, plan a raid instead: `03-RAID-PLAYBOOK` +
`05-CAMPAIGN-FORM`.

## Closing a sweep

Topic-grouped commits, one logical unit each, citing the sweep item.
Commit the refreshed `discipline/health/latest-go.json` in the same run.
Resume pointer: **with a WAL** — bump its standing line at any milestone
move; **without** — the closing commit message carries the summary (floor
state, items taken, next candidate). Never leave the sweep's state only in
this conversation.

## The generation-time assistant (before you edit, not instead of the floor)

The stack ships an agentic type oracle. Before writing a nontrivial `.go`
edit, check the HYPOTHETICAL content instead of paying a red floor
iteration:

```sh
vibe bin exec go-ai-native-tcg -- validate internal/cells/<cell>/<file>.go \
    --content-from - --root .   # the edit on stdin; exit 1 = would fail
```

or, when the vibevm MCP server is mounted, call `tcg_validate` with
`language: "go"` and the `content` argument (plus `tcg_scope` /
`tcg_complete` / `tcg_type` for in-scope symbols, type-valid completions,
and quick info). Responses carry the SAME conform findings as the gate,
flagged `baselined` or new, with guide-citing advice — a new finding in
the answer means the floor WILL go red if you write that edit.
Prerequisites: go ≥ 1.24 + gopls (`go install
golang.org/x/tools/gopls@latest` — a stack obligation). Honesty: gopls
stands on go/types, the reference implementation of the spec — tighter
than rust-analyzer↔rustc, still not the compiler; the floor stays the
truth (TCG-ORACLE-GO §5).
