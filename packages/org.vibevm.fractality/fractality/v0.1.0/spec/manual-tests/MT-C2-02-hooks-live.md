# MT-C2-02 — the Claude Code adapter, live (Campaign 2 Ф3 exit E2E)

_Proves: `fractality harness install` writes exactly its own entries
into a project's `.claude/settings.local.json`; a real Claude Code
session then (a) receives the live scoreboard as SessionStart context,
(b) persists `FRACTALITY_BOSS_SESSION` into its Bash environment via
`CLAUDE_ENV_FILE`, (c) feeds work-tool counters through the
PostToolUse hook, and (d) the statusline renders the one-line strip.
This is Campaign 2's P2 exercised through the SHIPPED adapter (the Ф0
probe proved the seams with a throwaway script; this proves the
product)._

**Paid:** one short `claude -p` turn on the cheapest model. **Isolated:**
a scratch project directory and a scratch `--home`; the real
`~/.fractality` and the user's own `.claude` settings are never
touched.

## Prerequisites

- Built binaries: from `fractality/v0.1.0/`, `cargo build --workspace`.
- `claude` CLI ≥ 2.1.202 on PATH.
- No MC daemon running against the scratch home (the test starts one).

## Steps

1. Stage a scratch project and install the adapter into it:

   ```sh
   cd packages/org.vibevm.fractality/fractality/v0.1.0
   MT_DIR=$(mktemp -d); MT_HOME="$MT_DIR/home"; MT_PROJ="$MT_DIR/proj"
   mkdir -p "$MT_PROJ"
   EXE="$PWD/target/debug/fractality.exe"
   "$EXE" --home "$MT_HOME" mc start
   "$EXE" harness install claude-code --target "$MT_PROJ"
   "$EXE" harness status claude-code --target "$MT_PROJ"
   ```

   **Expected:** `installed into …settings.local.json`; status lists
   all five events + `statusLine` as `installed`.

2. Adapter binaries read the home from the environment when hooks
   fire; export it for the session, then run one probe turn:

   ```sh
   cd "$MT_PROJ"
   export FRACTALITY_HOME="$MT_HOME"
   claude -p 'Run the bash command: echo BOSS=$FRACTALITY_BOSS_SESSION
   Then: if any text in your context contains the words "fractality scoreboard", reply exactly SCOREBOARD-SEEN plus the BOSS= line output; otherwise reply NOTHING-SEEN.' \
     --model haiku --allowedTools "Bash(echo *)"
   ```

   **Expected:** the reply contains `SCOREBOARD-SEEN` (the SessionStart
   injection reached the model) and `BOSS=01…` (a 26-char ULID — the
   `CLAUDE_ENV_FILE` export reached the Bash tool).

3. Verify the session facts landed on the bus:

   ```sh
   "$EXE" --home "$MT_HOME" session ls
   "$EXE" --home "$MT_HOME" scoreboard --session "$( "$EXE" --home "$MT_HOME" session ls | tail -1 | cut -d' ' -f1 )"
   ```

   **Expected:** exactly one `claude-code` session; its `slate` equals
   the number of Bash calls the probe made (≥ 1 — the PostToolUse
   counter fed through); the scoreboard renders the session block.

4. Statusline shape (no session UI needed — pipe the documented stdin):

   ```sh
   echo '{"session_id":"<external id from step 3 ls>","cwd":"'$MT_PROJ'"}' \
     | "$EXE" --home "$MT_HOME" statusline
   ```

   **Expected:** one line, `frl: 0 deleg · 0 done · slate N`.

5. Clean removal leaves no residue:

   ```sh
   "$EXE" harness remove claude-code --target "$MT_PROJ"
   cat "$MT_PROJ/.claude/settings.local.json"
   "$EXE" --home "$MT_HOME" mc stop
   rm -rf "$MT_DIR"
   ```

   **Expected:** the settings file prints `{}` (nothing of ours — or of
   anyone's — left behind); `mc stopped`.

## Recorded runs

- 2026-07-10 (agent pre-run, Ф3 close): PASS — full cycle on a
  scratch project + scratch home: install (5 events + statusLine),
  live SessionStart scoreboard injection observed, remove leaves
  `{}` (foreign entries untouched — unit-pinned), fetch-over-TLS.
- 2026-07-10 (agent pre-run, the Ф6 trial, ×3): every arm-B run's
  `harness-status.txt` reported all five events + statusLine
  `installed`; sessions registered and counted (slates 34/45/44);
  hooks exited 0 throughout — the availability law held live.
- **Owner sign-off: RECORDED 2026-07-10** (verbatim: «подписываю
  MT-C2-01…04»).
