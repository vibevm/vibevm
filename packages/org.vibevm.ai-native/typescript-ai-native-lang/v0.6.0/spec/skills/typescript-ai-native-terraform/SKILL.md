---
name: typescript-ai-native-terraform
description: Adopt the AI-Native discipline on an existing (brownfield) TypeScript codebase — inventory-not-gate, the three registries, characterization goldens, then card raids cell by cell. Use once per codebase (or to resume a partial adoption); the recurring counterpart is /typescript-ai-native-sweep.
---

# Terraform a TypeScript codebase (brownfield adoption)

You are executing the BROWNFIELD protocol
(`spec://org.vibevm.ai-native/core-ai-native/mechanisms/BROWNFIELD-PROTOCOL-v0.1` — shipped
copy under `vibedeps/flow-core-ai-native/<version>/spec/mechanisms/`;
read it before the first phase, and skim `03-RAID-PLAYBOOK` +
`05-CAMPAIGN-FORM` for the campaign machinery). The founding principles:
**inventory, not gate** (the only precondition is "the project
type-checks or at least runs"); **aspiration is legal only when
labeled**; **contradiction is data**; **characterization is the
truth-of-record**; **monotone utility**. Do not bulldoze an inhabited
world.

Toolchain: `typescript-ai-native` (install once:
`cargo install --path vibedeps/<stack-slot>/crates/typescript-ai-native-cli`,
or run via `cargo run --manifest-path vibedeps/<stack-slot>/Cargo.toml -p
typescript-ai-native-cli --bin typescript-ai-native -- <args>`). The
project itself needs node ≥ 22.6 and its own `typescript` devDependency —
the structural gate parses with the project's own compiler.

## Phase −1 — inventory (record reality; change nothing)

1. Precondition: `npx tsc --noEmit` runs (errors are FINDINGS to record,
   not blockers) and the test runner starts.
2. `typescript-ai-native init` — policies + empty registries. Nothing is
   gated yet, and that is correct.
3. **Fill `discipline/registry/tests-baseline.json` with reality:** run
   the suite once (`node --test --test-reporter=tap`), record every
   failing test as `failing-known` with a `since` date and a debt id — do
   NOT fix them now (drive-by repairs destroy the accounting).
4. **Harvest intent** into `discipline/registry/intent.json`: README
   roadmaps, TODO/FIXME that carry design, open issues you will honour.
   The carry-over guarantee: at exit every harvested intention is
   done | rescoped | rejected(reason) — zero unaccounted.
5. **File debt** into `discipline/registry/debt.json`: failing tests, the
   `any`/`as`/`!`/`@ts-ignore` hotspots `typescript-ai-native health`
   counts, missing runtime validation at erasure boundaries — each with
   severity, evidence, disposition, and `touch:` tripwires.
6. **Characterize** currently-passing observable behavior (golden
   transcripts under `discipline/golden/`, normalized for volatile
   fields). A pinned bug is visible debt; an unpinned bug is a landmine.
7. `typescript-ai-native specmap` — mint the (initially small) index;
   commit the whole inventory as its own topic commits.

## Phase 0 — the first spec units + the tsconfig floor

1. Write the project's first `spec/` documents for the subsystems you
   will touch first: anchored headings (`{#req-…}`), kind lines
   (`` `req r1` ``). Tag implementing exports as you go
   (`/** @implements spec://<ns>/… */`, or a file-level `@scope`);
   `typescript-ai-native specmap` after each batch keeps the index green.
2. Raise `tsconfig.json` toward the GUIDE §1 floor (`strict`,
   `noUncheckedIndexedAccess`, `exactOptionalPropertyTypes`,
   `erasableSyntaxOnly`) — one flag at a time; each flag's fallout is
   inventory (debt entries), not a fix-everything-now mandate.

## Phases 1…N — card raids, cell by cell

Per the Raid Playbook skeleton (scope & freeze → card order → phases →
acceptance), raid one directory-at-a-time toward the cell layout:

- carve `src/cells/<name>/` with `index.ts` seams; imports cross seams
  only (`ts-cell-isolation` starts enforcing the moment `cells_dir` is
  set in conform.toml);
- brand meaning-bearing primitives at the seams (card B); validate
  external data at erasure boundaries through a single-source schema;
- drain the unsafe set (`any` → `unknown`+narrowing, checked `as`,
  assertion functions for `!`), recording the irreducible remainder as
  reasoned `@ts-expect-error` testimony;
- `typescript-ai-native conform freeze` once per raid landing, then the
  ratchet only shrinks;
- keep `typescript-ai-native floor` green at every raid boundary —
  that is the campaign's safe-stop invariant.

## Exit

The BROWNFIELD §8 carry-over reconciliation (zero unaccounted intent),
the floor green with every step armed (an empty `floor_disable`), and
the sweep skill (`/typescript-ai-native-sweep`) taking over as the
recurring posture.

## The generation-time assistant during raids

Card raids rewrite cells wholesale — exactly where the oracle pays for
itself. While drafting a cell replacement, validate the draft BEFORE it
lands: `vibe bin exec typescript-ai-native-tcg -- validate <file> --content-from -`
(or the `tcg_validate` MCP tool with `content`). A non-baselined
`ts-unsafe-in-domain` or `ts-cell-isolation` finding in the answer is
the ratchet telling you early; `tcg_scope` lists the branded types at
the seams you are about to cross. The raid's safe-stop invariant is
unchanged — the floor gates the landing; the oracle just makes the
landing green on the first attempt.
