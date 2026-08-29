---
name: steward-goal
description: Render or refresh the deterministic user-local GOAL.md from a selected campaign node in plan.toml. Use for an owner-facing central session at start/resume or after plan, goal-node, or planning-profile changes; diagnose only when the session does not hold custody.
---

<status stage="impl" state="done"/>

# Steward goal {#root}

@fact:GOAL-SKILL-PURPOSE Produce the canonical local campaign goal specified by
`spec://org.vibevm.world/multi-user-planning/flows/multi-user-planning/goal-projection#root`.
The plan remains authority; do not paraphrase with an LLM. @status:impl/done

@fact:GOAL-SKILL-REFERENCE-RENDERER When Python 3.11+ is available, prefer the
bundled deterministic `scripts/render_goal.py` and pass the exact context,
holder and session ids. Its output is the protocol's reference implementation;
inspect the protocol before modifying or replacing it. Without Python, follow
the same byte rules directly and perform the double-render check.
@status:impl/done

## Procedure {#procedure}

1. @fact:GOAL-SKILL-RESOLVE Resolve the exact stewardship context and read its
   settings, custody and raw plan bytes once. Validate the plan graph before
   deriving anything. @status:impl/done
2. @fact:GOAL-SKILL-SELECT Use valid `goal_node`; infer only a single
   unambiguous top-level non-terminal campaign. On ambiguity or invalid id,
   report the typed refusal and do not write. @status:impl/done
3. @fact:GOAL-SKILL-AUTH Compare the current session/holder with held custody.
   A worker, non-holder, vacant context or `offering` holder may report whether
   the goal is stale but must not create or replace `GOAL.md`.
   @status:impl/done
4. @fact:GOAL-SKILL-RENDER Follow the protocol's exact subtree, ordering,
   frontier, candidate, route, closure, profile and fixed-text rules. Render
   canonical `GOAL.md` plus the bounded one-line `GOAL-CLAUDE.txt` `/goal`
   adapter. Emit UTF-8 without BOM, LF endings, no timestamp and one trailing
   newline per file.
   @status:impl/done
5. @fact:GOAL-SKILL-PUBLISH Write a same-directory temporary file, recheck plan
   hash, selection, profile and custody epoch/holder, then publish the Claude
   command first and `GOAL.md` marker last. Retry one moved snapshot at most;
   refuse the second.
   @status:impl/done
6. @fact:GOAL-SKILL-VERIFY Re-read the marker and both outputs, recompute raw
   plan and Claude-condition SHA, count condition UTF-16 units, and render a
   second time in memory. Accept only exact hashes, a condition at most 4000
   units, and byte-identical second rendering. @status:impl/done

## Output {#output}

@fact:GOAL-SKILL-REPORT Report the selected node, plan revision/hash, profile,
current/stale result, both output paths, condition length and whether bytes
changed. Print the exact manual continuation recipe: copy trimmed
`GOAL-CLAUDE.txt`, run `claude -c` in the bound worktree, paste, Enter. Do not
start the campaign merely because the goal was refreshed. @status:impl/done
