# MT-C2-03 — nudges and the question push, live (Campaign 2 Ф4 exit E2E)

_Proves: the threshold nudge reaches a real session's next prompt and
goes quiet under its cooldown; a genuinely parked worker interrupts
exactly one Stop with its question and never a second one. The nudge
half is CLI-drivable; the question half needs one real parked worker
(the MT-04 pattern) and is therefore paid._

**Paid:** one GLM worker turn that parks on an ask_boss question (small
slot). **Isolated:** scratch `--home` + scratch project; the real
`~/.fractality` untouched.

## Prerequisites

- Built binaries (`cargo build --workspace`) — **the whole workspace:
  hooks talk to the sibling daemon binary, and a stale
  `fractality-mission-control.exe` folds session events with old rules**
  (caught live in the Ф4 smoke: the cooldown silently never engaged).
- `claude` ≥ 2.1.202; profiles for the paid half (MT-04 pattern).

## Steps — nudge half (unpaid)

1. Scratch home + adapter + seven work-tool events:

   ```sh
   cd packages/org.vibevm.fractality/fractality/v0.1.0
   MT_HOME=$(mktemp -d)/home; EXE="$PWD/target/debug/fractality.exe"
   export FRACTALITY_HOME="$MT_HOME"
   "$EXE" mc start
   for i in 1 2 3 4 5 6 7; do
     echo '{"session_id":"mt-c2-03","cwd":"C:/p","tool_name":"Bash","duration_ms":100}' \
       | "$EXE" hook post-tool-use
   done
   echo '{"session_id":"mt-c2-03","cwd":"C:/p"}' | "$EXE" hook user-prompt-submit
   ```

   **Expected:** one JSON line whose `additionalContext` names the
   count («7 work-tool calls since your last delegation») and cites
   `fractality route`.

2. The cooldown:

   ```sh
   echo '{"session_id":"mt-c2-03","cwd":"C:/p"}' | "$EXE" hook user-prompt-submit
   echo "EXIT=$?"
   ```

   **Expected:** no output, `EXIT=0` (quiet inside the 300 s window;
   `session show mt-c2-03` shows `nudges=1`).

3. The kill switch:

   ```sh
   FRACTALITY_INITIATIVE=off bash -c \
     'echo "{\"session_id\":\"mt-c2-03-b\",\"cwd\":\"C:/p\"}" | "'$EXE'" hook session-start'
   ```

   **Expected:** no output at all (not even the scoreboard).

## Steps — question half (paid, the MT-04 pattern)

4. Start a worker that asks a question (reuse
   `spec/manual-tests/mt-04` packet or equivalent), wait until
   `fractality questions` lists it. In a REAL Claude Code session over
   this home (adapter installed): finish any turn.

   **Expected:** the turn does not stop silently — the Stop feedback
   quotes the worker's question and the exact
   `fractality answer <id> "..."` command. The NEXT turn's stop is
   quiet (the alert acked itself; `session show` counts
   `alerts=1`). `fractality answer` resumes the worker (MT-04
   semantics).

## Recorded runs

- 2026-07-10 (agent pre-run, nudge half, steps 1–3): PASS — threshold
  text exact, cooldown quiet, kill switch silent. Question half:
  pending the Ф6 paid window (RP1).
