---
name: go-ai-native-terraform
description: Adopt the AI-Native discipline on an existing (brownfield) Go codebase — inventory-not-gate, the three registries, characterization goldens, then card raids package by package. Use once per codebase (or to resume a partial adoption); the recurring counterpart is /go-ai-native-sweep.
---

<status stage="impl" state="done"/>

# Terraform a Go codebase (brownfield adoption) {#root}

@fact:EXECUTING-THE-BROWNFIELD-PROTOCOL You are executing the BROWNFIELD protocol
(`spec://org.vibevm.ai-native/core-ai-native/mechanisms/BROWNFIELD-PROTOCOL-v0.1` — shipped
copy under `vibedeps/flow-core-ai-native/<version>/spec/mechanisms/`;
read it before the first phase, and skim `03-RAID-PLAYBOOK` +
`05-CAMPAIGN-FORM` for the campaign machinery). @status:impl/done

@fact:founding-principles-lead The founding principles: @status:impl/done

- @fact:PRINCIPLE-INVENTORY-NOT-GATE **inventory, not gate** (the only precondition is "the module builds"); @status:impl/done
- @fact:PRINCIPLE-ASPIRATION-MUST-BE-LABELED **aspiration is legal only when labeled**; @status:impl/done
- @fact:PRINCIPLE-CONTRADICTION-IS-DATA **contradiction is data**; @status:impl/done
- @fact:PRINCIPLE-CHARACTERIZATION-IS-THE-TRUTH-OF-RECORD **characterization is the truth-of-record**; @status:impl/done
- @fact:PRINCIPLE-MONOTONE-UTILITY **monotone utility**. @status:impl/done

@fact:DO-NOT-BULLDOZE-AN-INHABITED-WORLD Do not
bulldoze an inhabited world. @status:impl/done

@fact:TOOLCHAIN-INSTALL-OR-RUN-IN-PLACE Toolchain: `go-ai-native` (install once:
`cargo install --path vibedeps/<stack-slot>/crates/go-ai-native-cli`, or
run via `cargo run --manifest-path vibedeps/<stack-slot>/Cargo.toml -p
go-ai-native-cli --bin go-ai-native -- <args>`). @status:impl/done

@fact:MACHINE-NEEDS-GO-AND-GOPLS The machine needs
go ≥ 1.24 and gopls (stack obligations). @status:impl/done

## Phase −1 — inventory (record reality; change nothing) {#phase-inventory}

1. @fact:INVENTORY-PRECONDITION Precondition: `go build ./...` succeeds. (Red build → fix the build
   first; that is the one true gate.) @status:impl/done
2. @fact:INVENTORY-INIT `go-ai-native init` — policies + empty registries. Every package
   starts exempt-with-a-reason; nothing is gated yet, and that is correct. @status:impl/done
3. @fact:INVENTORY-TESTS-BASELINE **Fill `discipline/registry/tests-baseline.json` with reality:** run
   the suite once (`go test ./... -json`), record every failing test as
   `failing-known` with a `since` date and a debt id — do NOT fix them now
   (drive-by repairs destroy the accounting). Delete no `t.Skip` yet, but
   file each one as debt: skips on known-failing tests are the pattern
   this stack bans (GUIDE §10). @status:impl/done
4. @fact:INVENTORY-HARVEST-INTENT **Harvest intent** into `discipline/registry/intent.json`: README
   roadmaps, TODO/FIXME that carry design, open issues you will honour.
   The carry-over guarantee: at exit every harvested intention is
   done | rescoped | rejected(reason) — zero unaccounted. @status:impl/done
5. @fact:INVENTORY-FILE-DEBT **File debt** into `discipline/registry/debt.json`: failing tests, the
   `init()`/ambient/naked-`go` hotspots `go-ai-native health` counts,
   loose boundary decoding (no `DisallowUnknownFields`), reasonless
   suppressions — each with severity, evidence, disposition, and `touch:`
   tripwires. @status:impl/done
6. @fact:INVENTORY-CHARACTERIZE **Characterize** currently-passing observable behavior (golden
   transcripts under `discipline/golden/`, normalized for volatile
   fields). A pinned bug is visible debt; an unpinned bug is a landmine. @status:impl/done
7. @fact:INVENTORY-SPECMAP `go-ai-native specmap` — mint the (initially small) index; commit the
   whole inventory as its own topic commits. @status:impl/done

## Phase 0 — the first spec units {#phase-first-spec-units}

@fact:WRITE-THE-FIRST-SPEC-DOCUMENTS Write the project's first `spec/` documents for the subsystems you will
touch first: anchored headings (`{#req-…}`), kind lines (`` `req r1` ``). @status:impl/done

@fact:TAG-IMPLEMENTING-PACKAGES-AS-YOU-GO Tag implementing packages as you go (`//spec:scope <uri> r=1` in doc.go;
item-level `//spec:implements` where precision pays);
`go-ai-native specmap` after each batch keeps the index green. @status:impl/done

## Phases 1…N — card raids, package by package {#card-raids}

@fact:raid-one-package-at-a-time-lead Per the Raid Playbook skeleton (scope & freeze → card order → phases →
acceptance), raid one package at a time toward the cell layout: @status:impl/done

- @fact:RAID-CARVE-THE-CELL-LAYOUT carve `internal/cells/<name>/` with seams in a neutral package and the
  registry as the only cell importer (`go-cell-isolation` starts enforcing
  the moment `cells_dir` is set in conform.toml); @status:impl/done
- @fact:RAID-DEFINE-TYPES-AT-SEAMS define types for meaning-bearing primitives at seams (card B); add the
  loud-conformance assertions; validate boundary decodes explicitly; @status:impl/done
- @fact:RAID-DRAIN-THE-BAN-CENSUS drain the ban census (`init()` → composition root; ambient calls →
  injected capabilities; naked `go` → owned groups; error-string matches
  → `errors.As` on closed sets), recording the irreducible remainder as
  reasoned `//spec:deviates` testimony; @status:impl/done
- @fact:RAID-CLOSED-ERROR-SETS-AND-EXAMPLES give each seam its closed error set with REQ-citing messages (card F)
  and each exported seam item its `Example` (card G); @status:impl/done
- @fact:RAID-FREEZE-ONCE-PER-LANDING `go-ai-native conform freeze` once per raid landing, then the ratchet
  only shrinks; @status:impl/done
- @fact:RAID-BEHAVIOR-CHANGES-CARRY-AN-ORACLE behavior changes carry a differential fuzz oracle (card D) with a
  committed seed corpus; goldens follow the promotion protocol; @status:impl/done
- @fact:RAID-KEEP-THE-FLOOR-GREEN keep `go-ai-native floor` green at every raid boundary — that is the
  campaign's safe-stop invariant. @status:impl/done

@fact:TRACK-THE-CAMPAIGN-PER-CAMPAIGN-FORM Track the campaign per `05-CAMPAIGN-FORM`: a cold-executable PLAN, the
BASELINE numbers you started from, PREDICTIONS, a LOG, and a closing
REPORT. @status:impl/done

@fact:CAMPAIGN-RESUME-POINTER Resume pointer: with a WAL — the standing line at each phase
boundary; without — the PLAN's status line + LOG tail
(`06-WAL-CONVENTION` §4). @status:impl/done

## Exit criteria {#exit-criteria}

@fact:EXIT-CRITERIA `go-ai-native floor` green with every package either gated or
exempt-with-a-living-reason; the test-gate baseline shrunk truthfully
(promotions, not silence); zero `t.Skip`-hidden failures; the carry-over
guarantee met (every intent done | rescoped | rejected); the REPORT
written. @status:impl/done

@fact:HELD-BY-THE-RECURRING-SWEEP From here the tree is held by the recurring sweep:
/go-ai-native-sweep. @status:impl/done

## The generation-time assistant during raids {#generation-time-assistant}

@fact:RAIDS-ARE-WHERE-THE-ORACLE-PAYS Card raids rewrite cells wholesale — exactly where the oracle pays for
itself. @status:impl/done

@fact:VALIDATE-THE-DRAFT-BEFORE-IT-LANDS While drafting a cell replacement, validate the draft BEFORE it
lands: `vibe bin exec go-ai-native-tcg -- validate <file> --content-from -`
(or the `tcg_validate` MCP tool with `language: "go"` and `content`). @status:impl/done

@fact:NON-BASELINED-FINDING-IS-THE-RATCHET-EARLY A
non-baselined census finding in the answer is the ratchet telling you
early; `tcg_scope` lists the defined types at the seams you are about to
cross. @status:impl/done

@fact:SAFE-STOP-INVARIANT-IS-UNCHANGED The raid's safe-stop invariant is unchanged — the floor gates the
landing; the oracle just makes the landing green on the first attempt. @status:impl/done
