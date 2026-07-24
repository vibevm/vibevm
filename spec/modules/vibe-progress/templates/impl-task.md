# DRIFT-NNN — <one-line goal: which promise becomes true> {#root}

<status stage="impl" state="plan" ref="DRIFT-NNN"/>

**Status:** queued | in-progress | review | done | returned
**Executor:** Opus. **Reviewer:** Fable, against §6 verbatim.
**Cluster:** <registry | workspace | resolver | cli | common>
**Unit-stability check (release precondition):** every anchor cited in §2 has
no open obligation in the findings ledger and no `unknown` marker.

## 1. Goal {#goal}

One sentence. What promise, made where, becomes true.

## 2. Contract {#contract}

Verbatim quotes of the governing spec units — never paraphrase:

> <quoted unit text>
> — `spec://vibevm/…#anchor`

List every anchor this task realises. If units disagree, the task is
premature — return it to stitching.

## 3. Current state {#current}

From campaign verification evidence (do not re-discover):

- `crates/<…>/src/<file>.rs:<line>` — what exists;
- what is missing / broken, with the verification verdict refs.

## 4. Required behavior {#behavior}

Step by step, exhaustively — inputs, outputs, error paths, edge cases. Spell
out everything the spec left implied; nothing here may require judgment:

1. …
2. …

Edge cases: … Error paths (exact error types/codes): …

## 5. Boundaries {#boundaries}

- Files/subsystems NOT to touch: …
- Sanctioned construction sites to respect (e.g. conform R-001): …
- Never edit spec text or golden tests. Spec doubts → §8, not improvisation.

## 6. Acceptance {#acceptance}

Executable, complete — the review runs exactly this:

```bash
cargo test -p <crate> <filter>        # new tests listed below must pass
bash tools/self-check.sh              # floor stays green
```

- New tests to write: `<test_name>` asserts <what>; …
- CLI scenario (when applicable): `vibe <…>` → expected output verbatim.
- Discipline: `#[spec(implements = "spec://…#anchor")]` on new items (closes
  the specmap orphan), `cargo fmt --all`, clippy clean, atomic commits per
  the repo rules, no AI attribution anywhere.
- On completion update the unit markers: `impl/work → impl/done` (+ next
  marker per plan, typically `test/plan`), same commit discipline.

## 7. Analogies {#analogies}

"Do it like X": `crates/<…>` — the closest existing shape to imitate.

## 8. Stop rule {#stop}

If the spec is silent or ambiguous on a point you need: STOP, mark
`<!-- REVIEW: <question> -->` at the code point, record the question in the
task file under this section, set status `returned`. Do not invent semantics.
Budget signal: if the change grows past <N> files / <M> lines, stop and
return with findings — the task was mis-scoped.

## 9. Log {#log}

Appended by executor/reviewer: claimed <ts>, returned/review notes, done <ts>.
