---
name: typescript-ai-native-sweep
description: Run the recurring AI-Native discipline sweep on this TypeScript project — the seven-step floor first, then the health collector's ratchet items. Use daily or several times a day on an active tree; any single item is a safe stop.
---

<status stage="impl" state="done"/>

# The discipline sweep (TypeScript stack) {#root}

##RUNNING-THE-STANDING-SWEEP You are running the standing sweep from the Discipline's Sweep Playbook
(`spec://org.vibevm.ai-native/core-ai-native/04-SWEEP-PLAYBOOK` — the shipped copy is at
`vibedeps/flow-core-ai-native/<version>/spec/04-SWEEP-PLAYBOOK.md`; read it
once per session if you have not). @impl/done

##two-truths-lead The two truths: @impl/done

- ##TRUTH-GATES-ARE-THE-FLOOR **the gates are the
  floor, the sweep is the ceiling**, @impl/done
- ##TRUTH-GATE-IS-TRUTH and **the gate is truth, the collector is
  a guide**. @impl/done

##NEVER-SWEEP-ON-A-RED-TREE Never sweep on a red tree. @impl/done

##ACT-ON-COLLECTOR-FACTS Act on collector facts, never on
memory. @impl/done

##ALL-COMMANDS-ARE-THE-SHIPPED-TOOLCHAIN All commands below are the shipped toolchain. @impl/done

##IF-NOT-ON-PATH-INSTALL-OR-RUN-IN-PLACE If `typescript-ai-native` is
not on PATH, either install it once —
`cargo install --path vibedeps/<stack-slot>/crates/typescript-ai-native-cli`
— or run it in place: `cargo run --manifest-path
vibedeps/<stack-slot>/Cargo.toml -p typescript-ai-native-cli --bin
typescript-ai-native -- <args>`. @impl/done

## Tier 0 — the hard floor (ALWAYS first) {#tier-zero}

```sh
typescript-ai-native floor
```

##FLOOR-HAS-SEVEN-STEPS Seven steps: prettier → tsc → tests → eslint → conform → specmap →
test-gate. @impl/done

##RED-FLOOR-ADMITS-ONLY-GREENING-WORK Red? The only legal work is making it green — fix, do not
proceed. @impl/done

##CHECK-THE-PRINTED-POLICY-LINES Check the printed policy lines: a
`NO conform.toml — topology default in force` line means the project is not
bootstrapped (`typescript-ai-native init`) and any green under it is vacuous,
and every `DISABLED by policy` line is a standing decision to re-question
weekly — a floor that shrank quietly is the failure mode this line exists
to catch. @impl/done

## Tier 1 — the ratchet (every run) {#tier-one}

```sh
typescript-ai-native health
```

##READ-THE-HEALTH-SUMMARY Read the summary (the JSON at `discipline/health/latest-typescript.json` is
the work-list; its git diff is the trend). @impl/done

##take-cheapest-wins-lead Take one or two cheapest wins: @impl/done

1. ##RATCHET-DANGER-BAND-FILES **danger-band files** — split any file at the top of the [540,600) band
   before an edit trips the 600 budget; the new module keeps (or gains) its
   own `@scope` marker so the orphan gate never regresses. @impl/done
2. ##RATCHET-UNREASONED-SUPPRESSIONS **unreasoned suppressions** — every `@ts-expect-error` WITHOUT
   `-- reason` in the census is unrecorded testimony: add the reason or fix
   the underlying type. `@ts-ignore` is never acceptable — replace with
   `@ts-expect-error -- reason` and watch it fail when the error goes. @impl/done
3. ##RATCHET-EXPORT-DOC-EXAMPLE-COVERAGE **export doc-example coverage** — exports without an `@example` (or
   fenced block) are retrieval gaps; document the highest-traffic seam
   first. @impl/done
4. ##RATCHET-ORPHAN-BACKLOG **orphan backlog** — untagged exports the ratchet will block on: tag the
   export (`@implements spec://…`), `@scope` its file, or move it out of
   the public surface. @impl/done

## Tier 2 — weekly {#tier-two}

- ##WEEKLY-FAST-LOOP `typescript-ai-native fast-loop` — every cell answers inside the budget;
  a cell with NO tests fails the check (the loop must exist). @impl/done
- ##WEEKLY-TRIPWIRE `typescript-ai-native tripwire --base origin/main` — debt that this
  week's changes touched; each fired entry is addressed in the PR text:
  pulled-in, re-dispositioned, or consciously deferred. @impl/done
- ##WEEKLY-REREAD-DISABLE-AND-EXEMPT-LISTS Re-read the `floor_disable` list and the `[[typescript.exempt]]` list: does each reason
  still hold? @impl/done

## Output contract {#output-contract}

##END-EVERY-SWEEP-WITH-THE-OUTCOME-TABLE End every sweep with an outcome table — this stack's own
addition, and not something the Sweep Playbook prescribes: per tier, what ran,
what was found, the ONE ratchet item taken, and what was deliberately left
(with why). The Playbook's own closing contract is its §4 «Output of a sweep»
— topic-grouped commits, the refreshed health snapshot, the resume pointer —
and it holds here unchanged. @impl/done

##green-gates-only-is-not-a-sweep A sweep that only reports green gates did the
floor's job, not the sweep's. @spec/done

## The generation-time assistant (before you edit, not instead of the floor) {#generation-time-assistant}

##STACK-SHIPS-AN-AGENTIC-TYPE-ORACLE The stack ships an agentic type oracle. @impl/done

##check-the-hypothetical-content-lead Before writing a nontrivial `.ts`
edit, check the HYPOTHETICAL content instead of paying a red floor
iteration: @impl/done

```sh
vibe bin exec typescript-ai-native-tcg -- validate src/cells/<cell>/index.ts \
    --content-from - --root .   # the edit on stdin; exit 1 = would fail
```

##MCP-ALTERNATIVE or, when the vibevm MCP server is mounted, call `tcg_validate` with the
`content` argument (plus `tcg_scope` / `tcg_complete` / `tcg_type` for
in-scope symbols, type-valid completions, and quick info). @impl/done

##RESPONSES-CARRY-THE-SAME-CONFORM-FINDINGS Responses
carry the SAME conform findings as the gate, flagged `baselined` or new,
with guide-citing advice — a new finding in the answer means the floor
WILL go red if you write that edit. @impl/done

##FLOOR-STAYS-THE-TRUTH The floor stays the truth; the
oracle exists so you reach it green on the first try. @impl/done
