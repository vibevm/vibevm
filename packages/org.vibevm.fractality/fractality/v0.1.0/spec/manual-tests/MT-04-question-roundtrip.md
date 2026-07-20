# MT-04 — a live worker parks on ask_boss and resumes (Phase 4b, D18)

_Proves: the interaction layer end to end — a headless GLM worker calls
the `ask_boss` MCP tool, the run parks (`waiting_on_boss`, question on
the bus and on the plane), the boss triages with `fractality questions`
and replies with `fractality answer`, the worker receives the text as
its tool result and finishes the task with it. Manual-test #4._

**Paid:** one GLM (`model = "small"`) worker turn, most of it idle
waiting on the answer. **Isolated:** scratch `--home`; a dedicated
`mt4` profile switches the broker on (`ask_boss = true`).

## Prerequisites

- `~/.fractality/profiles.toml` with the `glm` profile;
  `~/.vibe/zai.api.token` present.
- Built binaries: from `fractality/v0.1.0/`, `cargo build --workspace`.

## Steps

1. Scratch home with the `mt4` profile and the packet:

   ```sh
   cd packages/org.vibevm.fractality/fractality/v0.1.0
   MT04_BASE=$(cygpath -m "$(mktemp -d)")
   MT04_HOME="$MT04_BASE/fractality-home"
   mkdir -p "$MT04_HOME"
   cp ~/.fractality/profiles.toml "$MT04_HOME/profiles.toml"
   cat >> "$MT04_HOME/profiles.toml" <<'EOF'

   [profile.mt4]
   backend = "claude-code"
   base_url = "https://api.z.ai/api/anthropic"
   token_file = "~/.vibe/zai.api.token"
   [profile.mt4.models]
   big = "glm-5.2[1m]"
   small = "glm-5-turbo"
   haiku_slot = "glm-5-turbo"
   [profile.mt4.env]
   API_TIMEOUT_MS = "3000000"
   CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC = "1"
   [profile.mt4.permissions]
   mode = "acceptEdits"
   ask_boss = true
   EOF
   cat > "$MT04_BASE/ask.toml" <<'EOF'
   schema = 1
   [task]
   title = "mt4-ask-roundtrip"
   goal = """You must NOT decide the greeting yourself. Step 1: call the ask_boss tool with exactly this question: `Greeting A or greeting B?` Step 2: create the file `greeting.txt` containing ONLY the answer text you received, nothing else. Step 3: stop."""
   [workspace]
   mode = "dir"
   [budget]
   wall_secs = 600
   [routing]
   profile = "mt4"
   model = "small"
   EOF
   ```

   **Expected:** profile block and packet in place.

2. Spawn, wait for the park, read the question:

   ```sh
   R=$(./target/debug/fractality.exe --home "$MT04_HOME" spawn --packet "$MT04_BASE/ask.toml")
   echo "run=$R"
   while ! ./target/debug/fractality.exe --home "$MT04_HOME" questions | grep -q "$R"; do
     sleep 3
   done
   ./target/debug/fractality.exe --home "$MT04_HOME" questions
   ./target/debug/fractality.exe --home "$MT04_HOME" ps --state waiting_on_boss
   cat "$MT04_HOME"/runs/$R/question.md
   ```

   **Expected:** `questions` lists the run with age and the question
   text (`Greeting A or greeting B?`); `ps --state waiting_on_boss`
   shows exactly this run; `question.md` holds the same text (I2: the
   bus carried it, the plane recorded it).

3. Answer; the worker resumes and finishes:

   ```sh
   ./target/debug/fractality.exe --home "$MT04_HOME" answer "$R" "Greeting B - and mention the fractal."
   ./target/debug/fractality.exe --home "$MT04_HOME" wait "$R"
   echo "WAIT_EXIT=$?"
   WORK=$("$MT04_HOME"/../ 2>/dev/null; ./target/debug/fractality.exe --home "$MT04_HOME" show $R --json | grep '"run_dir"' | cut -d'"' -f4)
   cat "$WORK/work/greeting.txt"
   cat "$MT04_HOME"/runs/$R/answer.md
   ```

   **Expected:** `answer` prints `<id> running`; `wait` settles
   `completed exit=0` (`WAIT_EXIT=0`); `greeting.txt` contains the
   answer text (the worker used the tool result, not its own guess);
   `answer.md` persisted the reply.

4. Teardown:

   ```sh
   ./target/debug/fractality.exe --home "$MT04_HOME" mc stop
   rm -rf "$MT04_BASE"
   ```

   **Expected:** `mc stopped`; the real `~/.fractality` untouched.

## Recorded run

_(Agent pre-run output is appended below on each execution; the pass is
signed by a human — the pre-run only flags divergence.)_

### Pre-run 2026-07-10 — GREEN first firing, no divergence

**PASS — signed off by the owner, 2026-07-10** (covers the 11-minute
park addendum below).

```text
MT04-RUN=01KX4QEE308PBK98RHHHWJRVHY
questions: 01KX4QEE308PBK98RHHHWJRVHY 6s  Greeting A or greeting B?
ps --state waiting_on_boss: exactly this run
question.md: Greeting A or greeting B?
answer "Greeting B - and mention the fractal."  ->  <id> running
wait: completed exit=0
greeting.txt: Greeting B - and mention the fractal.   (verbatim tool result)
answer.md:    Greeting B - and mention the fractal.
status.json:  "state": "completed"
```

The worker parked ~6 s after boot, resumed the moment the answer
landed, and wrote the reply verbatim — it decided nothing itself. The
phase prediction (a parked worker survives >= 10 minutes idle and
resumes cleanly) was checked by a separate firing with a 30-minute
wall budget and a deliberate 11-minute silence before the answer; see
the addendum below.

### Addendum — the 11-minute park (same day): prediction CONFIRMED

Run `01KX4QG4PSNXENAS5HPEK7XP9R` (wall budget 1800 s): parked at ~6 s,
left in `waiting_on_boss` for 11 minutes of deliberate silence
(SILENCE-OVER at 03:51:40), answered, resumed, `completed exit=0`,
`greeting.txt` verbatim. A parked worker burns no tokens — the CC
process blocks on the one MCP tool result. **The phase prediction
holds with a minute to spare over its 10-minute bar.**
