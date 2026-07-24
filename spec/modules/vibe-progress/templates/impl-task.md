# DRIFT-NNN — <one-line goal: which promise becomes true> {#root}

<status stage="spec" state="done" comment="B2 2026-07-24: template in force since Phase A (DRIFT-001..003 written from it); the task's own document marker is the fenced example below"/>

```
<status stage="impl" state="plan" ref="DRIFT-NNN"/>
```

##status-legend **Status:** queued | in-progress | review | done | returned @spec/done
##EXECUTOR-SPLIT **Executor:** Opus. **Reviewer:** Fable, against §6 verbatim. @spec/done
##cluster-field **Cluster:** <registry | workspace | resolver | cli | common> @spec/done
##UNIT-STABILITY **Unit-stability check (release precondition):** every anchor cited in §2 has
no open obligation in the findings ledger and no `unknown` marker. @spec/done

## 1. Goal {#goal}

##goal-format One sentence. What promise, made where, becomes true. @spec/done

## 2. Contract {#contract}

##CONTRACT-VERBATIM Verbatim quotes of the governing spec units — never paraphrase: @spec/done

```
> <quoted unit text>
> — `spec://vibevm/…#anchor`
```

- ##CONTRACT-LIST-ALL List every anchor this task realises. @spec/done
- ##PREMATURE-RETURN If units disagree, the task is premature — return it to stitching. @spec/done

## 3. Current state {#current}

##current-evidence From campaign verification evidence (do not re-discover): @spec/done

- ##cs-what-exists `crates/<…>/src/<file>.rs:<line>` — what exists; @spec/done
- ##cs-what-missing what is missing / broken, with the verification verdict refs. @spec/done

## 4. Required behavior {#behavior}

##BEHAVIOR-EXHAUSTIVE Step by step, exhaustively — inputs, outputs, error paths, edge cases. Spell
out everything the spec left implied; nothing here may require judgment: @spec/done

```
1. …
2. …
```

##behavior-edges Edge cases: … Error paths (exact error types/codes): … @spec/done

## 5. Boundaries {#boundaries}

- ##bd-not-touch Files/subsystems NOT to touch: … @spec/done
- ##bd-construction-sites Sanctioned construction sites to respect (e.g. conform R-001): … @spec/done
- ##BD-NEVER-EDIT-SPEC Never edit spec text or golden tests. Spec doubts → §8, not improvisation. @spec/done

## 6. Acceptance {#acceptance}

##ACCEPTANCE-EXECUTABLE Executable, complete — the review runs exactly this: @spec/done

```bash
cargo test -p <crate> <filter>        # new tests listed below must pass
bash tools/self-check.sh              # floor stays green
```

- ##acc-new-tests New tests to write: `<test_name>` asserts <what>; … @spec/done
- ##acc-cli-scenario CLI scenario (when applicable): `vibe <…>` → expected output verbatim. @spec/done
- ##ACC-DISCIPLINE Discipline: `#[spec(implements = "spec://…#anchor")]` on new items (closes
  the specmap orphan), `cargo fmt --all`, clippy clean, atomic commits per
  the repo rules, no AI attribution anywhere. @spec/done
- ##ACC-UPDATE-MARKERS On completion update the unit markers: `impl/work → impl/done` (+ next
  marker per plan, typically `test/plan`), same commit discipline. @spec/done

## 7. Analogies {#analogies}

##analogies-format "Do it like X": `crates/<…>` — the closest existing shape to imitate. @spec/done

## 8. Stop rule {#stop}

- ##STOP-RULE If the spec is silent or ambiguous on a point you need: STOP, mark
  `<!-- REVIEW: <question> -->` at the code point, record the question in the
  task file under this section, set status `returned`. Do not invent semantics. @spec/done
- ##BUDGET-SIGNAL Budget signal: if the change grows past <N> files / <M> lines, stop and
  return with findings — the task was mis-scoped. @spec/done

## 9. Log {#log}

##log-format Appended by executor/reviewer: claimed <ts>, returned/review notes, done <ts>. @spec/done
