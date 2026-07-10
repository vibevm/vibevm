# MT-03 — recursive kill of a depth-2 tree (Phase 4, P5)

_Proves: the fractal property and its safety valve — a worker spawns a
child through its own `fractality` CLI (the call tree becomes real),
and `fractality kill --tree` terminates the whole live tree in under
two seconds with zero orphans (prediction P5). Manual-test #3._

**Paid:** one GLM `big` turn (the parent — it must follow a three-step
instruction) and one `small` turn (the child sleeper); both are killed
mid-sleep, so the token spend is the boot overhead. **Isolated:**
scratch `--home`; a dedicated `mt3` profile pre-approves the Bash tool
(the static-allowlist layer of D18) so the headless parent may invoke
`fractality spawn` — the pod injects `FRACTALITY_HOME`, `FRACTALITY_RUN_ID`,
and its own directory onto the worker PATH, so the child lands on the
same scratch mission-control and attaches to the parent automatically.

## Prerequisites

- `~/.fractality/profiles.toml` with the `glm` profile;
  `~/.vibevm/zai.api.token` present.
- Built binaries: from `fractality/v0.1.0/`, `cargo build --workspace`.

## Steps

1. Scratch home with the `mt3` profile (Bash pre-approved), and the two
   packets:

   ```sh
   cd packages/org.vibevm.fractality/fractality/v0.1.0
   # cygpath -m: the path lands inside packets read by Windows processes.
   MT03_BASE=$(cygpath -m "$(mktemp -d)")
   MT03_HOME="$MT03_BASE/fractality-home"
   mkdir -p "$MT03_HOME"
   cp ~/.fractality/profiles.toml "$MT03_HOME/profiles.toml"
   cat >> "$MT03_HOME/profiles.toml" <<'EOF'

   [profile.mt3]
   backend = "claude-code"
   base_url = "https://api.z.ai/api/anthropic"
   token_file = "~/.vibevm/zai.api.token"
   [profile.mt3.models]
   big = "glm-5.2[1m]"
   small = "glm-5-turbo"
   haiku_slot = "glm-5-turbo"
   [profile.mt3.env]
   API_TIMEOUT_MS = "3000000"
   CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC = "1"
   [profile.mt3.permissions]
   mode = "acceptEdits"
   allow_tools = ["Bash"]
   EOF
   cat > "$MT03_BASE/child.toml" <<'EOF'
   schema = 1
   [task]
   title = "mt3-child-sleeper"
   goal = """Using the Bash tool, run exactly this command and wait for it to finish: `ping -n 240 127.0.0.1 -w 1000`. When it finishes, create `done.md` containing `child finished`. Do nothing else."""
   [workspace]
   mode = "dir"
   [budget]
   wall_secs = 600
   [routing]
   profile = "mt3"
   model = "small"
   EOF
   cat > "$MT03_BASE/parent.toml" <<EOF
   schema = 1
   [task]
   title = "mt3-parent-spawner"
   goal = """Do exactly these three steps, in order, using the Bash tool.
   Step 1: run \`fractality spawn --packet $MT03_BASE/child.toml\` — it prints a run id (a 26-character ULID) on stdout.
   Step 2: create the file \`child-id.txt\` in your working directory containing that run id.
   Step 3: run \`ping -n 240 127.0.0.1 -w 1000\` and wait for it to finish.
   Do nothing else. Do not kill or wait for the child run."""
   [workspace]
   mode = "dir"
   [budget]
   wall_secs = 600
   [routing]
   profile = "mt3"
   model = "big"
   EOF
   ```

   **Expected:** the profile block and both packets in place.

2. Spawn the parent; watch the tree until the child appears and both
   workers are running:

   ```sh
   P=$(./target/debug/fractality.exe --home "$MT03_HOME" spawn --packet "$MT03_BASE/parent.toml")
   echo "parent=$P"
   # Poll until the tree is two deep (the parent has booted GLM and
   # called `fractality spawn`; expect ~30-90 s):
   while true; do
     ./target/debug/fractality.exe --home "$MT03_HOME" tree "$P"
     C=$(./target/debug/fractality.exe --home "$MT03_HOME" ps --quiet | grep -v "$P" | head -1)
     [ -n "$C" ] && break
     sleep 5
   done
   echo "child=$C"
   ./target/debug/fractality.exe --home "$MT03_HOME" wait "$C" --timeout 0 &
   sleep 20   # give the child worker time to reach running
   ./target/debug/fractality.exe --home "$MT03_HOME" tree "$P"
   ```

   **Expected:** `tree $P` prints two lines — the parent and, indented
   beneath it, the child (`depth` edge real: the parent's own CLI call
   registered it). Both eventually show `running`. The child's record
   carries `parent = $P` (`show $C` to confirm).

3. Snapshot the worker pids, then kill the tree and time the deaths:

   ```sh
   PPID_W=$(./target/debug/fractality.exe --home "$MT03_HOME" show $P --json | grep '"worker_pid"' | grep -o '[0-9]*')
   CPID_W=$(./target/debug/fractality.exe --home "$MT03_HOME" show $C --json | grep '"worker_pid"' | grep -o '[0-9]*')
   echo "worker pids: parent=$PPID_W child=$CPID_W"
   T0=$(date +%s%3N)
   ./target/debug/fractality.exe --home "$MT03_HOME" kill "$P" --tree
   while tasklist /FI "PID eq $PPID_W" 2>nul | grep -q "$PPID_W" \
      || tasklist /FI "PID eq $CPID_W" 2>nul | grep -q "$CPID_W"; do :; done
   T1=$(date +%s%3N)
   echo "TREE_DEAD_IN=$((T1-T0))ms"
   ```

   **Expected:** `kill` prints `<parent> killed` and `<child> killed`
   (root first); both worker pids vanish; **P5 passes when
   `TREE_DEAD_IN` < 2000 ms** (worst case is one 1 s heartbeat till the
   pod claims the command, plus the Job Object close).

4. Zero orphans and honest records:

   ```sh
   ./target/debug/fractality.exe --home "$MT03_HOME" ps
   ./target/debug/fractality.exe --home "$MT03_HOME" show $P | grep killed
   grep -E "kill command received|orphan sweep" \
     "$MT03_HOME"/runs/*/pod.log
   ```

   **Expected:** both runs `killed` (reason `manual`); each pod.log
   carries `kill command received; terminating worker tree` and
   `orphan sweep clean: worker tree is gone`; `status.json` in both run
   dirs says `"state": "killed"`.

5. Teardown:

   ```sh
   ./target/debug/fractality.exe --home "$MT03_HOME" mc stop
   rm -rf "$MT03_BASE"
   ```

   **Expected:** `mc stopped`; no `claude`/`node` strays from these two
   trees in Task Manager; the real `~/.fractality` untouched.

## Recorded run

_(Agent pre-run output is appended below on each execution; the pass is
signed by a human — the pre-run only flags divergence.)_

### Pre-run 2026-07-10 — GREEN first firing, no divergence
(human sign-off pending)

```text
MT03-PARENT=01KX4Q5FV7B5A1HQFKNDY7V3PD
MT03-CHILD=01KX4Q5R757WWEGP8P2MQQEA76     (parent's own `fractality spawn`)
tree: parent + child, both running (depth-2, parent edge on the record)
MT03-PIDS: parent_worker=44776 child_worker=45664
kill --tree: both `killed` (root first)
TREE_DEAD_IN=1025ms                        (< 2000 - P5 PASS)
records: both killed / manual; status.json both "killed"
pod.log x2: "kill command received; terminating worker tree"
           + "orphan sweep clean: worker tree is gone"
```

The fractal property held live: the GLM parent executed `fractality
spawn` through its pre-approved Bash tool; the pod-injected
`FRACTALITY_HOME`/`FRACTALITY_RUN_ID` routed the child onto the same
scratch mission-control with the parent edge attached automatically.
The 1 s heartbeat delivered both kills on the next beat; the Job
Object close reaped each worker tree with zero orphans. **P5
confirmed: a depth-2 live tree dies in ~1 s, nothing survives.**
