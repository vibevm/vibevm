# Decision record template {#root}

**Scope of this document.** The copy-ready shape of a decision
record, what each of the four fields must contain, two fully worked
examples, and the anti-pattern table. The reasoning behind the
practice lives in
[`DECISION-RECORDS-PROTOCOL.md`](DECISION-RECORDS-PROTOCOL.md);
trigger design in [`revisit-triggers.md`](revisit-triggers.md).

## The template {#template}

Paste under the spec heading that governs the value. Give that
heading an explicit anchor — a record is only as good as its
address.

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
| **Decision** | A reader can act on it without asking anyone. | It hedges ("probably", "for now") without a trigger. |
| **Why** | It cites data someone could check: a log, a count, a benchmark, a version. | It appeals to taste ("cleaner") or restates the decision. |
| **Considered and rejected** | Each line names the loser *and* the reason it lost. | It lists losers without reasons — or lists nothing. |
| **When to revisit** | Metric + threshold + observation point; a stranger can answer "has it fired?". | "Later", "when it breaks", "when we refactor". |

## Worked example: a constant with consequences {#example-timeout}

The session that produced this record: the human measured VPN
delivery latency, raised a timeout from 300 s to 600 s, and wrote
the record into the spec section governing verification timing —
same session, before close.

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

The difference this buys, side by side:

```
Before:  "Timeout: 600 s"
After:   "600 s, because VPN false positives — measured, here is
          the data, here is what lost, here is when to reconsider."
```

The first line is a fact; the code already says it. The second is a
decision: the next session that reads `TIMEOUT = 600` and feels the
urge to "optimise" it back to 300 finds an 847-message measurement
standing in the way. That is the immunity working.

Field by field:

- The **why** survives audit because it is *checkable*: the log file
  is named, the sample size is stated. A why with data can be
  re-verified or outgrown; a why without data can only be believed
  or ignored.
- The **rejections** are one line each — enough to stop the
  re-proposal ("what about adaptive timeouts?"), cheap enough to
  write in the same minute.
- The **trigger** names the exact world-state that reopens the
  question, and where to look for it.

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

Note the trigger's shape: two disjunct conditions, both observable —
one an unambiguous external event, one a threshold on a fact anyone
can check from the upstream repository today. Either fires without
anyone having to remember to wonder.

## Anti-patterns {#anti-patterns}

| Anti-pattern | Example | Why it fails | Fix |
|--------------|---------|--------------|-----|
| Tautology | "600 s, because that is our timeout." | We do X because we do X — restates the decision, zero information. | Name the observation that forced the value. |
| Unfalsifiable why | "blake3 felt cleaner." / "because it is better." | Cannot be checked, cannot be outgrown; blocks revisiting forever without justifying anything. | Cite a measurement, constraint, or incident. |
| Rejections without reasons | "Considered: adaptive timeout, 300 s + retry." | The evaluation gets re-run; the bare list answers nothing. | One line per loser, each with the reason it lost. |
| "Revisit: later" | "Revisit when needed." | Never fires; the record hardens into a sacred cow. | Metric + threshold + observation point. |
| Why lives in the commit only | Reasoning in the commit body; spec carries the bare value. | Commit history is not in the reading path at the anchor; the agent reads the section, not `git log`. | The spec carries the why; the commit cites the anchor. |
| Backfilled memory | Writing the why a week later, from recollection. | Reconstructed reasoning is fiction with confidence; the data is gone. | Record in the session that decides — or mark the why TODO(owner). |

## Summary {#summary}

- Paste the template at the governing anchor; never under a heading
  without one.
- Why with data; rejections with reasons; trigger with metric,
  threshold, and observation point.
- The before/after test: if the record only says what the code
  already says, it is a fact with decoration — complete it or delete
  it.
- Record in the same session the decision is made. Backfilled whys
  are fiction.
