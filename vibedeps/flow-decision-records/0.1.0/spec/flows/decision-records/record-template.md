# Decision record template {#root}

<status stage="spec" state="done"/>

##scope-of-this-document **Scope of this document.** The copy-ready shape of a decision
record, what each of the four fields must contain, two fully worked
examples, and the anti-pattern table. @impl/done

##companion-document-pointers The reasoning behind the
practice lives in
[`DECISION-RECORDS-PROTOCOL.md`](DECISION-RECORDS-PROTOCOL.md);
trigger design in [`revisit-triggers.md`](revisit-triggers.md). @impl/done

## The template {#template}

##PASTE-UNDER-THE-GOVERNING-SPEC-HEADING Paste under the spec heading that governs the value. @impl/done

##GIVE-THAT-HEADING-AN-EXPLICIT-ANCHOR Give that
heading an explicit anchor — a record is only as good as its
address. @impl/done

```markdown
### <The thing decided> {#<stable-anchor>}

**Decision:** <the chosen value or approach, one line>.

**Why:** <the observation that forced the choice — with data:
log path, sample size, benchmark numbers, incident reference,
upstream constraint and version>.

**Considered and rejected:**
- <alternative 1> — rejected: <reason>.
- <alternative 2> — rejected: <reason>.

**When to revisit:** <metric + threshold + where it is observed>.
```

## What each field must contain {#fields}

| Field | Passes when | Fails when |
|-------|-------------|------------|
| ##ROW-FIELD-DECISION **Decision** @impl/done | A reader can act on it without asking anyone. @impl/done | It hedges ("probably", "for now") without a trigger. @impl/done |
| ##ROW-FIELD-WHY **Why** @impl/done | It cites data someone could check: a log, a count, a benchmark, a version. @impl/done | It appeals to taste ("cleaner") or restates the decision. @impl/done |
| ##ROW-FIELD-CONSIDERED-AND-REJECTED **Considered and rejected** @impl/done | Each line names the loser *and* the reason it lost. @impl/done | It lists losers without reasons — or lists nothing. @impl/done |
| ##ROW-FIELD-WHEN-TO-REVISIT **When to revisit** @impl/done | Metric + threshold + observation point; a stranger can answer "has it fired?". @impl/done | "Later", "when it breaks", "when we refactor". @impl/done |

## Worked example: a constant with consequences {#example-timeout}

##worked-example-session-narration The session that produced this record: the human measured VPN
delivery latency, raised a timeout from 300 s to 600 s, and wrote
the record into the spec section governing verification timing —
same session, before close. @unknown

```markdown
### Verification timeout {#verification.timeout}

**Decision:** 600 seconds.

**Why:** Testing showed ~15 % of users on corporate VPNs see
delivery latency above 300 s; their messages were flagged TIMEOUT
before the transport confirmed delivery. Measured on
logs/vpn-test-2026-03-05.log, 847 messages, 128 users.

**Considered and rejected:**
- Adaptive timeout keyed to observed latency — rejected:
  unpredictable UX.
- 300 s plus retry — rejected: adds complexity, does not fix the
  root cause.

**When to revisit:** when p99 delivery latency drops below 100 s
per the network monitoring dashboard.
```

##side-by-side-lead The difference this buys, side by side: @impl/done

```
Before:  "Timeout: 600 s"
After:   "600 s, because VPN false positives — measured, here is
          the data, here is what lost, here is when to reconsider."
```

##the-first-line-is-a-fact The first line is a fact; the code already says it. @spec/done

##the-second-line-is-a-decision The second is a
decision: the next session that reads `TIMEOUT = 600` and feels the
urge to "optimise" it back to 300 finds an 847-message measurement
standing in the way. @spec/done

##that-is-the-immunity-working That is the immunity working. @spec/done

##field-by-field-lead Field by field: @impl/done

- ##WHY-SURVIVES-AUDIT-BECAUSE-IT-IS-CHECKABLE The **why** survives audit because it is *checkable*: the log file
  is named, the sample size is stated. A why with data can be
  re-verified or outgrown; a why without data can only be believed
  or ignored. @impl/done
- ##REJECTIONS-ARE-ONE-LINE-EACH The **rejections** are one line each — enough to stop the
  re-proposal ("what about adaptive timeouts?"), cheap enough to
  write in the same minute. @impl/done
- ##TRIGGER-NAMES-THE-WORLD-STATE-THAT-REOPENS-THE-QUESTION The **trigger** names the exact world-state that reopens the
  question, and where to look for it. @impl/done

## Worked example: a library choice {#example-library}

```markdown
### Content hashing {#hashing}

**Decision:** blake3 for every content hash.

**Why:** SHA-256 through the platform library drags in an OpenSSL
dependency; we need minimal binary size and no system-library
coupling for edge servers on weak hardware. blake3 also measured
~3x faster on the 1-MiB payload benchmark (bench/hashing, run
2026-02-11).

**Considered and rejected:**
- SHA-256 via platform OpenSSL — rejected: the OpenSSL dependency
  is exactly what we are avoiding.
- SHA-256, pure-language implementation — rejected: ~3x slower on
  the payload benchmark; no compliance requirement compels it.

**When to revisit:** if a compliance requirement mandates a
NIST-approved hash, or blake3 upstream ships no release for
24 months.
```

##note-the-trigger-shape Note the trigger's shape: two disjunct conditions, both observable —
one an unambiguous external event, one a threshold on a fact anyone
can check from the upstream repository today. @spec/done

##either-condition-fires-unprompted Either can be answered
yes-or-no by a stranger without the project instrumenting anything —
though neither fires itself: what fires a trigger is a re-read
([`revisit-triggers.md` §periodic-sweep](revisit-triggers.md#periodic-sweep)). @spec/done

## Anti-patterns {#anti-patterns}

| Anti-pattern | Example | Why it fails | Fix |
|--------------|---------|--------------|-----|
| ##ROW-ANTI-TAUTOLOGY Tautology @impl/done | "600 s, because that is our timeout." @impl/done | We do X because we do X — restates the decision, zero information. @impl/done | Name the observation that forced the value. @impl/done |
| ##ROW-ANTI-UNFALSIFIABLE-WHY Unfalsifiable why @impl/done | "blake3 felt cleaner." / "because it is better." @impl/done | Cannot be checked, cannot be outgrown; blocks revisiting forever without justifying anything. @impl/done | Cite a measurement, constraint, or incident. @impl/done |
| ##ROW-ANTI-REJECTIONS-WITHOUT-REASONS Rejections without reasons @impl/done | "Considered: adaptive timeout, 300 s + retry." @impl/done | The evaluation gets re-run; the bare list answers nothing. @impl/done | One line per loser, each with the reason it lost. @impl/done |
| ##ROW-ANTI-REVISIT-LATER "Revisit: later" @impl/done | "Revisit when needed." @impl/done | Never fires; the record hardens into a sacred cow. @impl/done | Metric + threshold + observation point. @impl/done |
| ##ROW-ANTI-WHY-IN-THE-COMMIT-ONLY Why lives in the commit only @impl/done | Reasoning in the commit body; spec carries the bare value. @impl/done | Commit history is not in the reading path at the anchor; the agent reads the section, not `git log`. @impl/done | The spec carries the why; the commit cites the anchor. @impl/done |
| ##ROW-ANTI-BACKFILLED-MEMORY Backfilled memory @impl/done | Writing the why a week later, from recollection. @impl/done | Reconstructed reasoning is fiction with confidence; the data is gone. @impl/done | Record in the session that decides — or mark the why TODO(owner). @impl/done |

## Summary {#summary}

- ##SUM-PASTE-AT-THE-GOVERNING-ANCHOR Paste the template at the governing anchor; never under a heading
  without one. @impl/done
- ##SUM-DATA-REASONS-AND-A-MEASURABLE-TRIGGER Why with data; rejections with reasons; trigger with metric,
  threshold, and observation point. @impl/done
- ##SUM-THE-BEFORE-AFTER-TEST The before/after test: if the record only says what the code
  already says, it is a fact with decoration — complete it or delete
  it. @impl/done
- ##SUM-RECORD-IN-THE-DECIDING-SESSION Record in the same session the decision is made. Backfilled whys
  are fiction. @impl/done
