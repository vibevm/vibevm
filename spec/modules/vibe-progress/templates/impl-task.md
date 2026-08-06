# DRIFT-NNN — <one-line goal: which promise becomes true> {#root}

<status stage="spec" state="done" comment="B2 2026-07-24: template in force since Phase A (DRIFT-001..003 written from it); the task's own document marker is the fenced example below"/>

```
<status stage="impl" state="plan" ref="DRIFT-NNN"/>
```

@fact:status-legend **Status:** queued | in-progress | review | done | returned @status:spec/done
@fact:EXECUTOR-SPLIT **Executor:** Opus. **Reviewer:** Fable, against §6 verbatim. @status:spec/done
@fact:cluster-field **Cluster:** <registry | workspace | resolver | cli | common> @status:spec/done
@fact:UNIT-STABILITY **Unit-stability check (release precondition):** every anchor cited in §2 has
no open obligation in the findings ledger and no `unknown` marker. @status:spec/done

## 1. Goal {#goal}

@fact:goal-format One sentence. What promise, made where, becomes true. @status:spec/done

## 2. Contract {#contract}

@fact:CONTRACT-VERBATIM Verbatim quotes of the governing spec units — never paraphrase: @status:spec/done

```
> <quoted unit text>
> — `spec://org.vibevm.core/vibevm/…#anchor`
```

- @fact:CONTRACT-LIST-ALL List every anchor this task realises. @status:spec/done
- @fact:PREMATURE-RETURN If units disagree, the task is premature — return it to stitching. @status:spec/done

## 3. Current state {#current}

@fact:current-evidence From campaign verification evidence (do not re-discover): @status:spec/done

- @fact:cs-what-exists `crates/<…>/src/<file>.rs:<line>` — what exists; @status:spec/done
- @fact:cs-what-missing what is missing / broken, with the verification verdict refs. @status:spec/done

## 4. Required behavior {#behavior}

@fact:BEHAVIOR-EXHAUSTIVE Step by step, exhaustively — inputs, outputs, error paths, edge cases. Spell
out everything the spec left implied; nothing here may require judgment: @status:spec/done

```
1. …
2. …
```

@fact:behavior-edges Edge cases: … Error paths (exact error types/codes): … @status:spec/done

## 5. Boundaries {#boundaries}

- @fact:bd-not-touch Files/subsystems NOT to touch: … @status:spec/done
- @fact:bd-construction-sites Sanctioned construction sites to respect (e.g. conform R-001): … @status:spec/done
- @fact:BD-NEVER-EDIT-SPEC Never edit spec text or golden tests. Spec doubts → §8, not improvisation. @status:spec/done

## 6. Acceptance {#acceptance}

@fact:ACCEPTANCE-EXECUTABLE Executable, complete — the review runs exactly this: @status:spec/done

```bash
cargo test -p <crate> <filter>        # new tests listed below must pass
bash tools/self-check.sh              # floor stays green
```

- @fact:acc-new-tests New tests to write: `<test_name>` asserts <what>; … @status:spec/done
- @fact:acc-cli-scenario CLI scenario (when applicable): `vibe <…>` → expected output verbatim. @status:spec/done
- @fact:ACC-DISCIPLINE Discipline: `#[spec(implements = "spec://…#anchor")]` on new items (closes
  the specmap orphan), `cargo fmt --all`, clippy clean, atomic commits per
  the repo rules, no AI attribution anywhere. @status:spec/done
- @fact:ACC-UPDATE-MARKERS On completion update the unit markers: `impl/work → impl/done` (+ next
  marker per plan, typically `test/plan`), same commit discipline. @status:spec/done

## 7. Analogies {#analogies}

@fact:analogies-format "Do it like X": `crates/<…>` — the closest existing shape to imitate. @status:spec/done

## 8. Stop rule {#stop}

- @fact:STOP-RULE If the spec is silent or ambiguous on a point you need: STOP, mark
  `<!-- REVIEW: <question> -->` at the code point, record the question in the
  task file under this section, set status `returned`. Do not invent semantics. @status:spec/done
- @fact:BUDGET-SIGNAL Budget signal: if the change grows past <N> files / <M> lines, stop and
  return with findings — the task was mis-scoped. @status:spec/done

## 9. Log {#log}

@fact:log-format Appended by executor/reviewer: claimed <ts>, returned/review notes, done <ts>. @status:spec/done
