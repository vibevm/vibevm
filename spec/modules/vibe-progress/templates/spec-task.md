# SPEC-NNN — <one-line goal: which documents converge on what> {#root}

<status stage="spec" state="done" comment="B2 2026-07-24: template in force since Phase A; the task's own document marker is the fenced example below"/>

```
<status stage="spec" state="plan" ref="SPEC-NNN"/>
```

##status-legend **Status:** queued | in-progress | review | done | returned @spec/done
##EXECUTOR-SPLIT **Executor:** Fable when budget allows, otherwise Opus. **Reviewer:** Fable
always (normative text is judgment territory). @spec/done
##wave-field **Wave:** <stitching wave number> @spec/done

## 1. Goal {#goal}

##goal-format One sentence: which incoming obligations this task closes. @spec/done

## 2. Incoming obligations {#obligations}

##obligations-source From the findings ledger, verbatim rows: @spec/done

```
| id | from | to | type |
|---|---|---|---|
| F-NNN | `spec://…#anchor` | `spec://…#anchor` | contradiction | duplication | missing-support | terminology | relocation | reality-mismatch |
```

## 3. Rules of engagement {#rules}

- ##RULE-GENRE-FIRST **Genre first:** name the genre of every document touched; contract wins
  over lore; normative language never enters design docs. @spec/done
- ##RULE-ANCHORS-IMMUTABLE **Anchors are immutable:** a relocated unit leaves a tombstone
  (`<!-- RETIRED: superseded by #new-anchor -->`); never rename or reuse. @spec/done
- ##RULE-SINGLE-SOURCE **Single source:** a duplicated norm survives in exactly one anchor; the
  other side becomes a citation. @spec/done
- ##RULE-REALITY-MISMATCH-FLOW **`reality-mismatch` closes via the sync-from-code flow** — draft the spec
  diff, surface for owner approval, never apply silently. @spec/done
- ##RULE-NO-SILENT-REPAIRS **No silent repairs beyond scope:** discover a problem in a unit not named
  in §2 → record a NEW obligation in the ledger; do not fix it here. Sprawl
  of work is forbidden; sprawl of records is mandatory. @spec/done
- ##RULE-UPDATE-MARKERS Update the `<status>` markers of every touched unit in the same edit. @spec/done

## 4. Acceptance {#acceptance}

- ##ACC-CHECK-GREEN `vibe progress check` green on all touched files. @spec/done
- ##ACC-OBLIGATIONS-RESOLVED Every §2 obligation status = resolved in the ledger, with the resolving
  edit referenced. @spec/done
- ##ACC-NO-NEW-CONTRADICTION No NEW contradiction against the neighbour documents listed here: <list>. @spec/done
- ##ACC-MARKERS-COMMITS Markers updated; commit(s) atomic, conventional, no AI attribution. @spec/done

## 5. Stop rule {#stop}

##STOP-TWO-WAVES A conflict that resists two waves (the counter for this doc pair did not
fall) is conceptual, not editorial: STOP, set `returned`, escalate to the
owner with both readings stated fairly. @spec/done

## 6. Log {#log}

##log-format claimed <ts> · notes · resolved obligations · done <ts> @spec/done
