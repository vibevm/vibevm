# SPEC-NNN — <one-line goal: which documents converge on what> {#root}

<status stage="spec" state="plan" ref="SPEC-NNN"/>

**Status:** queued | in-progress | review | done | returned
**Executor:** Fable when budget allows, otherwise Opus. **Reviewer:** Fable
always (normative text is judgment territory).
**Wave:** <stitching wave number>

## 1. Goal {#goal}

One sentence: which incoming obligations this task closes.

## 2. Incoming obligations {#obligations}

From the findings ledger, verbatim rows:

| id | from | to | type |
|---|---|---|---|
| F-NNN | `spec://…#anchor` | `spec://…#anchor` | contradiction \| duplication \| missing-support \| terminology \| relocation \| reality-mismatch |

## 3. Rules of engagement {#rules}

- **Genre first:** name the genre of every document touched; contract wins
  over lore; normative language never enters design docs.
- **Anchors are immutable:** a relocated unit leaves a tombstone
  (`<!-- RETIRED: superseded by #new-anchor -->`); never rename or reuse.
- **Single source:** a duplicated norm survives in exactly one anchor; the
  other side becomes a citation.
- **`reality-mismatch` closes via the sync-from-code flow** — draft the spec
  diff, surface for owner approval, never apply silently.
- **No silent repairs beyond scope:** discover a problem in a unit not named
  in §2 → record a NEW obligation in the ledger; do not fix it here. Sprawl
  of work is forbidden; sprawl of records is mandatory.
- Update the `<status>` markers of every touched unit in the same edit.

## 4. Acceptance {#acceptance}

- `vibe progress check` green on all touched files.
- Every §2 obligation status = resolved in the ledger, with the resolving
  edit referenced.
- No NEW contradiction against the neighbour documents listed here: <list>.
- Markers updated; commit(s) atomic, conventional, no AI attribution.

## 5. Stop rule {#stop}

A conflict that resists two waves (the counter for this doc pair did not
fall) is conceptual, not editorial: STOP, set `returned`, escalate to the
owner with both readings stated fairly.

## 6. Log {#log}

claimed <ts> · notes · resolved obligations · done <ts>
