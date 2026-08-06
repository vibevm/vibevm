# SPEC-NNN — <one-line goal: which documents converge on what> {#root}

<status stage="spec" state="done" comment="B2 2026-07-24: template in force since Phase A; the task's own document marker is the fenced example below"/>

```
<status stage="spec" state="plan" ref="SPEC-NNN"/>
```

@fact:status-legend **Status:** queued | in-progress | review | done | returned @status:spec/done
@fact:EXECUTOR-SPLIT **Executor:** Fable when budget allows, otherwise Opus. **Reviewer:** Fable
always (normative text is judgment territory). @status:spec/done
@fact:wave-field **Wave:** <stitching wave number> @status:spec/done

## 1. Goal {#goal}

@fact:goal-format One sentence: which incoming obligations this task closes. @status:spec/done

## 2. Incoming obligations {#obligations}

@fact:obligations-source From the findings ledger, verbatim rows: @status:spec/done

```
| id | from | to | type |
|---|---|---|---|
| F-NNN | `spec://…#anchor` | `spec://…#anchor` | contradiction | duplication | missing-support | terminology | relocation | reality-mismatch |
```

## 3. Rules of engagement {#rules}

- @fact:RULE-GENRE-FIRST **Genre first:** name the genre of every document touched; contract wins
  over lore; normative language never enters design docs. @status:spec/done
- @fact:RULE-ANCHORS-IMMUTABLE **Anchors are immutable:** a relocated unit leaves a tombstone
  (`<!-- RETIRED: superseded by #new-anchor -->`); never rename or reuse. @status:spec/done
- @fact:RULE-SINGLE-SOURCE **Single source:** a duplicated norm survives in exactly one anchor; the
  other side becomes a citation. @status:spec/done
- @fact:RULE-REALITY-MISMATCH-FLOW **`reality-mismatch` closes via the sync-from-code flow** — draft the spec
  diff, surface for owner approval, never apply silently. @status:spec/done
- @fact:RULE-NO-SILENT-REPAIRS **No silent repairs beyond scope:** discover a problem in a unit not named
  in §2 → record a NEW obligation in the ledger; do not fix it here. Sprawl
  of work is forbidden; sprawl of records is mandatory. @status:spec/done
- @fact:RULE-UPDATE-MARKERS Update the `<status>` markers of every touched unit in the same edit. @status:spec/done

## 4. Acceptance {#acceptance}

- @fact:ACC-CHECK-GREEN `vibe progress check` green on all touched files. @status:spec/done
- @fact:ACC-OBLIGATIONS-RESOLVED Every §2 obligation status = resolved in the ledger, with the resolving
  edit referenced. @status:spec/done
- @fact:ACC-NO-NEW-CONTRADICTION No NEW contradiction against the neighbour documents listed here: <list>. @status:spec/done
- @fact:ACC-MARKERS-COMMITS Markers updated; commit(s) atomic, conventional, no AI attribution. @status:spec/done

## 5. Stop rule {#stop}

@fact:STOP-TWO-WAVES A conflict that resists two waves (the counter for this doc pair did not
fall) is conceptual, not editorial: STOP, set `returned`, escalate to the
owner with both readings stated fairly. @status:spec/done

## 6. Log {#log}

@fact:log-format claimed <ts> · notes · resolved obligations · done <ts> @status:spec/done
