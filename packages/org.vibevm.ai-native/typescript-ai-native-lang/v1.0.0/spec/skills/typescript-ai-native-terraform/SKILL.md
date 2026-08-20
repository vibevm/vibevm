---
name: typescript-ai-native-terraform
description: Adopt the AI-Native discipline on an existing (brownfield) TypeScript codebase — inventory-not-gate, the three registries, characterization goldens, then card raids cell by cell. Use once per codebase (or to resume a partial adoption); the recurring counterpart is /typescript-ai-native-sweep.
---

<status stage="impl" state="done"/>

# Terraform a TypeScript codebase (brownfield adoption) {#root}

@fact:EXECUTING-THE-BROWNFIELD-PROTOCOL You are executing the BROWNFIELD protocol
(`spec://org.vibevm.ai-native/core-ai-native/mechanisms/BROWNFIELD-PROTOCOL-v0.1` — shipped
copy under `vibedeps/flow-core-ai-native/<version>/spec/mechanisms/`;
read it before the first phase, and skim `03-RAID-PLAYBOOK` +
`05-CAMPAIGN-FORM` for the campaign machinery). @status:impl/done

@fact:founding-principles-lead The founding principles: @status:impl/done

- @fact:PRINCIPLE-INVENTORY-NOT-GATE **inventory, not gate** (the only precondition is "the project
  type-checks or at least runs"); @status:impl/done
- @fact:PRINCIPLE-ASPIRATION-MUST-BE-LABELED **aspiration is legal only when
  labeled**; @status:impl/done
- @fact:PRINCIPLE-CONTRADICTION-IS-DATA **contradiction is data**; @status:impl/done
- @fact:PRINCIPLE-CHARACTERIZATION-IS-THE-TRUTH-OF-RECORD **characterization is the
  truth-of-record**; @status:impl/done
- @fact:PRINCIPLE-MONOTONE-UTILITY **monotone utility**. @status:impl/done

@fact:DO-NOT-BULLDOZE-AN-INHABITED-WORLD Do not bulldoze an inhabited
world. @status:impl/done

@fact:TOOLCHAIN-INSTALL-OR-RUN-IN-PLACE Toolchain: `typescript-ai-native` (install once:
`cargo install --path vibedeps/<stack-slot>/crates/typescript-ai-native-cli`,
or run via `cargo run --manifest-path vibedeps/<stack-slot>/Cargo.toml -p
typescript-ai-native-cli --bin typescript-ai-native -- <args>`). @status:impl/done

@fact:PROJECT-NEEDS-NODE-AND-ITS-OWN-TYPESCRIPT The
project itself needs node ≥ 22.6 and its own `typescript` devDependency —
the structural gate parses with the project's own compiler. @status:impl/done

## Phase −1 — inventory (record reality; change nothing) {#phase-inventory}

1. @fact:INVENTORY-PRECONDITION Precondition: `npx tsc --noEmit` runs (errors are FINDINGS to record,
   not blockers) and the test runner starts. @status:impl/done
2. @fact:INVENTORY-INIT `typescript-ai-native init` — policies + empty registries. Nothing is
   gated yet, and that is correct. @status:impl/done
3. @fact:INVENTORY-TESTS-BASELINE **Fill `discipline/registry/tests-baseline.json` with reality:** run
   the suite once (`node --test --test-reporter=tap`), record every
   failing test as `failing-known` with a `since` date and a debt id — do
   NOT fix them now (drive-by repairs destroy the accounting). @status:impl/done
4. @fact:INVENTORY-HARVEST-INTENT **Harvest intent** into `discipline/registry/intent.json`: README
   roadmaps, TODO/FIXME that carry design, open issues you will honour.
   The carry-over guarantee: at exit every harvested intention is
   done | rescoped | rejected(reason) — zero unaccounted. @status:impl/done
5. @fact:INVENTORY-FILE-DEBT **File debt** into `discipline/registry/debt.json`: failing tests, the
   `any`/`as`/`!`/`@ts-ignore` hotspots `typescript-ai-native health`
   counts, missing runtime validation at erasure boundaries — each with
   severity, evidence, disposition, and `touch:` tripwires. @status:impl/done
6. @fact:INVENTORY-CHARACTERIZE **Characterize** currently-passing observable behavior (golden
   transcripts under `discipline/golden/`, normalized for volatile
   fields). A pinned bug is visible debt; an unpinned bug is a landmine. @status:impl/done
7. @fact:INVENTORY-SPECMAP `typescript-ai-native specmap` — mint the (initially small) index;
   commit the whole inventory as its own topic commits. @status:impl/done

## Phase 0 — the first spec units + the tsconfig floor {#phase-first-spec-units}

1. @fact:PHASE-ZERO-FIRST-SPEC-DOCUMENTS Write the project's first `spec/` documents for the subsystems you
   will touch first: anchored headings (`{#req-…}`), kind lines
   (`` `req r1` ``). Tag implementing exports as you go
   (`/** @implements spec://<ns>/… */`, or a file-level `@scope`);
   `typescript-ai-native specmap` after each batch keeps the index green. @status:impl/done
2. @fact:PHASE-ZERO-RAISE-THE-TSCONFIG-FLOOR Raise `tsconfig.json` toward the GUIDE §1 floor (`strict`,
   `noUncheckedIndexedAccess`, `exactOptionalPropertyTypes`,
   `erasableSyntaxOnly`) — one flag at a time; each flag's fallout is
   inventory (debt entries), not a fix-everything-now mandate. @status:impl/done

## Phases 1…N — card raids, cell by cell {#card-raids}

@fact:raid-one-directory-at-a-time-lead Per the Raid Playbook skeleton (scope & freeze → card order → phases →
acceptance), raid one directory-at-a-time toward the cell layout: @status:impl/done

- @fact:RAID-CARVE-THE-CELL-LAYOUT carve `src/cells/<name>/` with `index.ts` seams; imports cross seams
  only (`ts-cell-isolation` starts enforcing the moment `cells_dir` is
  set in conform.toml); @status:impl/done
- @fact:RAID-BRAND-PRIMITIVES-AT-SEAMS brand meaning-bearing primitives at the seams (card B); validate
  external data at erasure boundaries through a single-source schema; @status:impl/done
- @fact:RAID-DRAIN-THE-UNSAFE-SET drain the unsafe set (`any` → `unknown`+narrowing, checked `as`,
  assertion functions for `!`), recording the irreducible remainder as
  reasoned `@ts-expect-error` testimony; @status:impl/done
- @fact:RAID-FREEZE-ONCE-PER-LANDING `typescript-ai-native conform freeze` once per raid landing, then the
  ratchet only shrinks; @status:impl/done
- @fact:RAID-KEEP-THE-FLOOR-GREEN keep `typescript-ai-native floor` green at every raid boundary —
  that is the campaign's safe-stop invariant. @status:impl/done

## Exit {#exit}

@fact:EXIT-CRITERIA The BROWNFIELD §8 carry-over reconciliation (zero unaccounted intent),
the floor green with every step armed (an empty `floor_disable`), and
the sweep skill (`/typescript-ai-native-sweep`) taking over as the
recurring posture. @status:impl/done

## The generation-time assistant during raids {#generation-time-assistant}

@fact:RAIDS-ARE-WHERE-THE-ORACLE-PAYS Card raids rewrite cells wholesale — exactly where the oracle pays for
itself. @status:impl/done

@fact:VALIDATE-THE-DRAFT-BEFORE-IT-LANDS While drafting a cell replacement, validate the draft BEFORE it
lands: `vibe bin exec typescript-ai-native-tcg -- validate <file> --content-from -`
(or the `tcg_validate` MCP tool with `content`). @status:impl/done

@fact:NON-BASELINED-FINDING-IS-THE-RATCHET-EARLY A non-baselined
`ts-unsafe-in-domain` or `ts-cell-isolation` finding in the answer is
the ratchet telling you early; `tcg_scope` lists the branded types at
the seams you are about to cross. @status:impl/done

@fact:SAFE-STOP-INVARIANT-IS-UNCHANGED The raid's safe-stop invariant is
unchanged — the floor gates the landing; the oracle just makes the
landing green on the first attempt. @status:impl/done
