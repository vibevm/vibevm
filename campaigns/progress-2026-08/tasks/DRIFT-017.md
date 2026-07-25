# DRIFT-017 — a run that changes nothing writes nothing {#root}

<status stage="impl" state="plan" ref="DRIFT-017"/>

**Status:** queued (blocked on DRIFT-016 — same files)
**Executor:** Opus. **Reviewer:** Fable, against §6 verbatim.
**Cluster:** cli (progress-core state + cache writes)
**Unit-stability check:** no spec anchor moves.

## 1. Goal {#goal}

`vibe progress scan` over an unchanged tree stops rewriting `cache.json`
and the five state files — so the campaign's real cost stops being the
JSON it emits when it has nothing new to say.

## 2. Contract {#contract}

> All formats are schema-versioned (`"schema": 1`); all writes are atomic.
> — `spec://vibevm/modules/vibe-progress/PROP-043#data`

> Everything else can be erased at any moment — no knowledge is lost.
> — `spec://vibevm/modules/vibe-progress/PROP-043#erasure`

**Owner's ruling, 2026-07-25:** take the lever DRIFT-010 identified and
declined to pull — «не переписывать state-файлы при отсутствии изменений».

## 3. Current state {#current}

From DRIFT-010's own measurement — do not re-discover:

- Parsing was never the expensive part. In release, over this corpus, the
  parse is **10.3 ms**. What dominates a run is the JSON of `cache.json` /
  `corpus.json` and their fsync'd atomic writes.
- `refresh_state` writes cache + all five state files unconditionally on
  every `scan`, whether or not a single byte of the corpus moved.
- DRIFT-010 named this as the real lever and deliberately left it: §5 of
  that task fixed the state shapes, and `updated_at` semantics move if a
  write is skipped. That is the design question below, and it is why this
  is its own task.

## 4. Required behavior {#behavior}

1. Before writing any of the six files, serialise the candidate content and
   compare it with what is on disk. Identical ⇒ skip the write entirely, do
   not touch the file, do not fsync.
2. **The `updated_at` question, which is the whole task.** A naive
   implementation stamps `updated_at = now()` first and then compares, so
   nothing is ever identical and nothing is ever skipped. Decide and state
   the semantics in §9, choosing between:
   - **(a) `updated_at` means "when the content last changed."** Compare
     everything *except* `updated_at`; if the rest is identical, skip the
     write and the old stamp stands. Simple, and it makes the field mean
     what its name says.
   - **(b) `updated_at` means "when the tool last looked."** Then a skip is
     a lie and the field must move, which defeats the task.
   **(a) is the reviewer's reading** — a freshness plaque that advances
   while nothing changed is not freshness. But if a dashboard screen or the
   RESUME generator depends on (b), say so and stop rather than silently
   changing what a reader is told.
3. The skip must be observable: `--json` output gains a per-file or
   per-artifact `written: bool`, and the human line says how many files were
   skipped. A silent optimisation is one nobody can debug.
4. `--no-cache` keeps forcing the parse; it does **not** force a write of
   identical content — those are different questions and conflating them
   would make the flag mean two things.

Edge cases: a file absent on disk is always written. A file present but
unreadable is written (a corrupt projection should be replaced). The journal
is append-only and out of scope — never skip a journal append.

Error paths: unchanged. A comparison that fails for any reason falls back to
writing, never to skipping — the safe direction is the one that costs
milliseconds, not the one that loses state.

## 5. Boundaries {#boundaries}

- Do not change any file's *shape*. This is about whether a write happens,
  not what it contains.
- Do not touch the journal.
- Never skip a write of `cache.json` whose `campaign` maps differ — the
  verdicts are the one thing worth an unconditional fsync. If the
  comparison cannot prove they are identical, write.
- Never edit spec text.

## 6. Acceptance {#acceptance}

```bash
cargo test -p progress-core -p vibe-cli
cargo run -q -p vibe-cli --bin vibe -- progress scan   # twice
bash tools/self-check.sh
```

- New test: `second_scan_writes_nothing` — scan a fixture twice, capture
  every mtime after the first, assert none moved after the second.
- New test: `edited_file_forces_the_write` — touch one source file, assert
  cache and the affected projections are rewritten.
- New test: `verdict_change_always_writes`.
- New test: `absent_file_is_always_written`.
- **Measured, reported in §9:** wall time of a second consecutive `scan` on
  this repository, before and after, in **release** — debug numbers were
  what made DRIFT-010's headline misleading, and this task exists because of
  that lesson.
- Discipline: `cargo fmt --all`, clippy clean, atomic commits, no AI
  attribution.

## 7. Analogies {#analogies}

`crates/vibe-install`'s freshness skip (PROP-011) — this project's existing
"do nothing when nothing changed", including the fact that it is observable
rather than silent. And the managed-blocks flow's law, which is the same
idea one layer up: *never rewrite a file when the result is byte-identical.*

## 8. Stop rule {#stop}

If any consumer — the dashboard, `resume`, the freshness plaque — depends on
`updated_at` advancing on every run: STOP, name the consumer in §9, and
return. Changing what a reader is told about freshness is a contract
question, not an optimisation.

Budget signal: past ~5 files or ~350 lines, stop and return.

## 9. Log {#log}

- queued 2026-07-25 (Fable), on the owner's ruling. Blocked on DRIFT-016:
  both tasks rewrite the same write paths, and landing them concurrently
  would produce a diff nobody can review.
