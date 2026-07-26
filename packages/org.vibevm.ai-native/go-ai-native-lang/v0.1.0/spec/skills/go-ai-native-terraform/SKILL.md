---
name: go-ai-native-terraform
description: Adopt the AI-Native discipline on an existing (brownfield) Go codebase — inventory-not-gate, the three registries, characterization goldens, then card raids package by package. Use once per codebase (or to resume a partial adoption); the recurring counterpart is /go-ai-native-sweep.
---

<status stage="impl" state="done"/>

# Terraform a Go codebase (brownfield adoption) {#root}

##EXECUTING-THE-BROWNFIELD-PROTOCOL You are executing the BROWNFIELD protocol
(`spec://org.vibevm.ai-native/core-ai-native/mechanisms/BROWNFIELD-PROTOCOL-v0.1` — shipped
copy under `vibedeps/flow-core-ai-native/<version>/spec/mechanisms/`;
read it before the first phase, and skim `03-RAID-PLAYBOOK` +
`05-CAMPAIGN-FORM` for the campaign machinery). @impl/done

##founding-principles-lead The founding principles: @impl/done

- ##PRINCIPLE-INVENTORY-NOT-GATE **inventory, not gate** (the only precondition is "the module builds"); @impl/done
- ##PRINCIPLE-ASPIRATION-MUST-BE-LABELED **aspiration is legal only when labeled**; @impl/done
- ##PRINCIPLE-CONTRADICTION-IS-DATA **contradiction is data**; @impl/done
- ##PRINCIPLE-CHARACTERIZATION-IS-THE-TRUTH-OF-RECORD **characterization is the truth-of-record**; @impl/done
- ##PRINCIPLE-MONOTONE-UTILITY **monotone utility**. @impl/done

##DO-NOT-BULLDOZE-AN-INHABITED-WORLD Do not
bulldoze an inhabited world. @impl/done

##TOOLCHAIN-INSTALL-OR-RUN-IN-PLACE Toolchain: `go-ai-native` (install once:
`cargo install --path vibedeps/<stack-slot>/crates/go-ai-native-cli`, or
run via `cargo run --manifest-path vibedeps/<stack-slot>/Cargo.toml -p
go-ai-native-cli --bin go-ai-native -- <args>`). @impl/done

##MACHINE-NEEDS-GO-AND-GOPLS The machine needs
go ≥ 1.24 and gopls (stack obligations). @impl/done

## Phase −1 — inventory (record reality; change nothing) {#phase-inventory}

1. ##INVENTORY-PRECONDITION Precondition: `go build ./...` succeeds. (Red build → fix the build
   first; that is the one true gate.) @impl/done
2. ##INVENTORY-INIT `go-ai-native init` — policies + empty registries. Every package
   starts exempt-with-a-reason; nothing is gated yet, and that is correct. @impl/done
3. ##INVENTORY-TESTS-BASELINE **Fill `discipline/registry/tests-baseline.json` with reality:** run
   the suite once (`go test ./... -json`), record every failing test as
   `failing-known` with a `since` date and a debt id — do NOT fix them now
   (drive-by repairs destroy the accounting). Delete no `t.Skip` yet, but
   file each one as debt: skips on known-failing tests are the pattern
   this stack bans (GUIDE §10). @impl/done
4. ##INVENTORY-HARVEST-INTENT **Harvest intent** into `discipline/registry/intent.json`: README
   roadmaps, TODO/FIXME that carry design, open issues you will honour.
   The carry-over guarantee: at exit every harvested intention is
   done | rescoped | rejected(reason) — zero unaccounted. @impl/done
5. ##INVENTORY-FILE-DEBT **File debt** into `discipline/registry/debt.json`: failing tests, the
   `init()`/ambient/naked-`go` hotspots `go-ai-native health` counts,
   loose boundary decoding (no `DisallowUnknownFields`), reasonless
   suppressions — each with severity, evidence, disposition, and `touch:`
   tripwires. @impl/done
6. ##INVENTORY-CHARACTERIZE **Characterize** currently-passing observable behavior (golden
   transcripts under `discipline/golden/`, normalized for volatile
   fields). A pinned bug is visible debt; an unpinned bug is a landmine. @impl/done
7. ##INVENTORY-SPECMAP `go-ai-native specmap` — mint the (initially small) index; commit the
   whole inventory as its own topic commits. @impl/done

## Phase 0 — the first spec units {#phase-first-spec-units}

##WRITE-THE-FIRST-SPEC-DOCUMENTS Write the project's first `spec/` documents for the subsystems you will
touch first: anchored headings (`{#req-…}`), kind lines (`` `req r1` ``). @impl/done

##TAG-IMPLEMENTING-PACKAGES-AS-YOU-GO Tag implementing packages as you go (`//spec:scope <uri> r=1` in doc.go;
item-level `//spec:implements` where precision pays);
`go-ai-native specmap` after each batch keeps the index green. @impl/done

## Phases 1…N — card raids, package by package {#card-raids}

##raid-one-package-at-a-time-lead Per the Raid Playbook skeleton (scope & freeze → card order → phases →
acceptance), raid one package at a time toward the cell layout: @impl/done

- ##RAID-CARVE-THE-CELL-LAYOUT carve `internal/cells/<name>/` with seams in a neutral package and the
  registry as the only cell importer (`go-cell-isolation` starts enforcing
  the moment `cells_dir` is set in conform.toml); @impl/done
- ##RAID-DEFINE-TYPES-AT-SEAMS define types for meaning-bearing primitives at seams (card B); add the
  loud-conformance assertions; validate boundary decodes explicitly; @impl/done
- ##RAID-DRAIN-THE-BAN-CENSUS drain the ban census (`init()` → composition root; ambient calls →
  injected capabilities; naked `go` → owned groups; error-string matches
  → `errors.As` on closed sets), recording the irreducible remainder as
  reasoned `//spec:deviates` testimony; @impl/done
- ##RAID-CLOSED-ERROR-SETS-AND-EXAMPLES give each seam its closed error set with REQ-citing messages (card F)
  and each exported seam item its `Example` (card G); @impl/done
- ##RAID-FREEZE-ONCE-PER-LANDING `go-ai-native conform freeze` once per raid landing, then the ratchet
  only shrinks; @impl/done
- ##RAID-BEHAVIOR-CHANGES-CARRY-AN-ORACLE behavior changes carry a differential fuzz oracle (card D) with a
  committed seed corpus; goldens follow the promotion protocol; @impl/done
- ##RAID-KEEP-THE-FLOOR-GREEN keep `go-ai-native floor` green at every raid boundary — that is the
  campaign's safe-stop invariant. @impl/done

##TRACK-THE-CAMPAIGN-PER-CAMPAIGN-FORM Track the campaign per `05-CAMPAIGN-FORM`: a cold-executable PLAN, the
BASELINE numbers you started from, PREDICTIONS, a LOG, and a closing
REPORT. @impl/done

##CAMPAIGN-RESUME-POINTER Resume pointer: with a WAL — the standing line at each phase
boundary; without — the PLAN's status line + LOG tail
(`06-WAL-CONVENTION` §4). @impl/done

## Exit criteria {#exit-criteria}

##EXIT-CRITERIA `go-ai-native floor` green with every package either gated or
exempt-with-a-living-reason; the test-gate baseline shrunk truthfully
(promotions, not silence); zero `t.Skip`-hidden failures; the carry-over
guarantee met (every intent done | rescoped | rejected); the REPORT
written. @impl/done

##HELD-BY-THE-RECURRING-SWEEP From here the tree is held by the recurring sweep:
/go-ai-native-sweep. @impl/done

## The generation-time assistant during raids {#generation-time-assistant}

##RAIDS-ARE-WHERE-THE-ORACLE-PAYS Card raids rewrite cells wholesale — exactly where the oracle pays for
itself. @impl/done

##VALIDATE-THE-DRAFT-BEFORE-IT-LANDS While drafting a cell replacement, validate the draft BEFORE it
lands: `vibe bin exec go-ai-native-tcg -- validate <file> --content-from -`
(or the `tcg_validate` MCP tool with `language: "go"` and `content`). @impl/done

##NON-BASELINED-FINDING-IS-THE-RATCHET-EARLY A
non-baselined census finding in the answer is the ratchet telling you
early; `tcg_scope` lists the defined types at the seams you are about to
cross. @impl/done

##SAFE-STOP-INVARIANT-IS-UNCHANGED The raid's safe-stop invariant is unchanged — the floor gates the
landing; the oracle just makes the landing green on the first attempt. @impl/done
