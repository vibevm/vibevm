---
name: typescript-ai-native-sweep
description: Run the recurring AI-Native discipline sweep on this TypeScript project — the seven-step floor first, then the health collector's ratchet items. Use daily or several times a day on an active tree; any single item is a safe stop.
---

# The discipline sweep (TypeScript stack)

You are running the standing sweep from the Discipline's Sweep Playbook
(`spec://core-ai-native/04-SWEEP-PLAYBOOK` — the shipped copy is at
`vibedeps/flow-core-ai-native/<version>/spec/04-SWEEP-PLAYBOOK.md`; read it
once per session if you have not). The two truths: **the gates are the
floor, the sweep is the ceiling**, and **the gate is truth, the collector is
a guide**. Never sweep on a red tree. Act on collector facts, never on
memory.

All commands below are the shipped toolchain. If `typescript-ai-native` is
not on PATH, either install it once —
`cargo install --path vibedeps/<stack-slot>/crates/typescript-ai-native-cli`
— or run it in place: `cargo run --manifest-path
vibedeps/<stack-slot>/Cargo.toml -p typescript-ai-native-cli --bin
typescript-ai-native -- <args>`.

## Tier 0 — the hard floor (ALWAYS first)

```sh
typescript-ai-native floor
```

Seven steps: prettier → tsc → tests → eslint → conform → specmap →
test-gate. Red? The only legal work is making it green — fix, do not
proceed. Check the printed policy lines: a `Defaulted` conform policy means
the project is not bootstrapped (`typescript-ai-native init`), and every
`DISABLED by policy` line is a standing decision to re-question weekly —
a floor that shrank quietly is the failure mode this line exists to catch.

## Tier 1 — the ratchet (every run)

```sh
typescript-ai-native health
```

Read the summary (the JSON at `discipline/health/latest-typescript.json` is
the work-list; its git diff is the trend). Take one or two cheapest wins:

1. **danger-band files** — split any file at the top of the [540,600) band
   before an edit trips the 600 budget; the new module keeps (or gains) its
   own `@scope` marker so the orphan gate never regresses.
2. **unreasoned suppressions** — every `@ts-expect-error` WITHOUT
   `-- reason` in the census is unrecorded testimony: add the reason or fix
   the underlying type. `@ts-ignore` is never acceptable — replace with
   `@ts-expect-error -- reason` and watch it fail when the error goes.
3. **export doc-example coverage** — exports without an `@example` (or
   fenced block) are retrieval gaps; document the highest-traffic seam
   first.
4. **orphan backlog** — untagged exports the ratchet will block on: tag the
   export (`@implements spec://…`), `@scope` its file, or move it out of
   the public surface.

## Tier 2 — weekly

- `typescript-ai-native fast-loop` — every cell answers inside the budget;
  a cell with NO tests fails the check (the loop must exist).
- `typescript-ai-native tripwire --base origin/main` — debt that this
  week's changes touched; each fired entry is addressed in the PR text:
  pulled-in, re-dispositioned, or consciously deferred.
- Re-read the `floor_disable` list and the exempt lists: does each reason
  still hold?

## Output contract

End every sweep with the outcome table the Playbook §5 specifies: per tier,
what ran, what was found, the ONE ratchet item taken, and what was
deliberately left (with why). A sweep that only reports green gates did the
floor's job, not the sweep's.

## The generation-time assistant (before you edit, not instead of the floor)

The stack ships an agentic type oracle. Before writing a nontrivial `.ts`
edit, check the HYPOTHETICAL content instead of paying a red floor
iteration:

```sh
vibe bin exec typescript-ai-native-tcg -- validate src/cells/<cell>/index.ts \
    --content-from - --root .   # the edit on stdin; exit 1 = would fail
```

or, when the vibevm MCP server is mounted, call `tcg_validate` with the
`content` argument (plus `tcg_scope` / `tcg_complete` / `tcg_type` for
in-scope symbols, type-valid completions, and quick info). Responses
carry the SAME conform findings as the gate, flagged `baselined` or new,
with guide-citing advice — a new finding in the answer means the floor
WILL go red if you write that edit. The floor stays the truth; the
oracle exists so you reach it green on the first try.
