# MT-02 — a three-worker swarm on disjoint modules (Phase 4, P4)

_Proves: admission + async verbs run a real swarm — three GLM workers
spawned asynchronously against one git repository, each in its own
worktree on its own branch, zero conflicts, and swarm wall-clock below
1.6× the slowest single run (prediction P4). Manual-test #2._

**Paid:** three GLM (`model = "small"`) worker turns on the z.ai coding
plan. **Isolated:** scratch `--home` and a scratch git repository; the
real `~/.fractality` is never touched. Profiles are home-scoped (F16) —
step 1 copies them in.

## Prerequisites

- `~/.fractality/profiles.toml` with the `glm` profile;
  `~/.vibevm/zai.api.token` present.
- Built binaries: from `fractality/v0.1.0/`, `cargo build --workspace`.
- `git` on PATH.

## Steps

1. Scratch home, scratch repo, three disjoint-module packets:

   ```sh
   cd packages/org.vibevm.fractality/fractality/v0.1.0
   # cygpath -m: the path crosses into Windows processes (mission-control
   # spawns `git -C <repo>`), so it must be a Windows form, not /tmp/....
   MT02_BASE=$(cygpath -m "$(mktemp -d)")
   MT02_HOME="$MT02_BASE/fractality-home"
   MT02_REPO="$MT02_BASE/swarm-repo"
   mkdir -p "$MT02_HOME" "$MT02_REPO"
   cp ~/.fractality/profiles.toml "$MT02_HOME/profiles.toml"
   git -C "$MT02_REPO" init -b main -q
   echo "swarm target repo" > "$MT02_REPO/README.md"
   git -C "$MT02_REPO" add README.md
   git -C "$MT02_REPO" -c user.email=mt02@local -c user.name=mt02 commit -qm "init"
   for name in alpha beta gamma; do
     cat > "$MT02_BASE/$name.toml" <<EOF
   schema = 1
   [task]
   title = "swarm-$name"
   goal = """Create the file \`$name.md\` in the current working directory (the root of your git worktree) containing exactly one line: \`$name: done by the swarm.\` Do not modify any other file. Do not run any commands."""
   acceptance = ["type $name.md"]
   [workspace]
   mode = "worktree"
   repo = '$MT02_REPO'
   base = "main"
   [budget]
   wall_secs = 300
   max_turns = 10
   [routing]
   profile = "glm"
   model = "small"
   EOF
   done
   ```

   **Expected:** three packet files; `git -C "$MT02_REPO" log --oneline`
   shows one `init` commit on `main`.

2. Spawn the swarm asynchronously and wait for all three:

   ```sh
   T0=$(date +%s)
   A=$(./target/debug/fractality.exe --home "$MT02_HOME" spawn --packet "$MT02_BASE/alpha.toml")
   B=$(./target/debug/fractality.exe --home "$MT02_HOME" spawn --packet "$MT02_BASE/beta.toml")
   C=$(./target/debug/fractality.exe --home "$MT02_HOME" spawn --packet "$MT02_BASE/gamma.toml")
   echo "spawned: $A $B $C"
   ./target/debug/fractality.exe --home "$MT02_HOME" wait "$A" "$B" "$C"
   echo "WAIT_EXIT=$?"; T1=$(date +%s); echo "SWARM_WALL=$((T1-T0))s"
   ```

   **Expected:** three ULIDs print instantly (spawn returns before the
   workers finish — the async property); `wait` prints one
   `<id> completed exit=0` line per run; `WAIT_EXIT=0`.

3. The swarm is visible as three trees / three worktrees:

   ```sh
   ./target/debug/fractality.exe --home "$MT02_HOME" ps
   ./target/debug/fractality.exe --home "$MT02_HOME" tree
   git -C "$MT02_REPO" worktree list
   git -C "$MT02_REPO" branch --list 'fractality/*'
   ```

   **Expected:** `ps` shows the three runs `completed`; `tree` prints
   three single-node trees; `git worktree list` shows the main checkout
   plus three `runs/<id>/wt` worktrees; three `fractality/<id>` branches.

4. Zero conflicts: every module landed, and the three branches merge
   into `main` cleanly (workers write files; committing the deliverable
   is the boss's review step — permission posture keeps workers away
   from git):

   ```sh
   # Commit ONLY each worker's intended module. Every worker also writes
   # result.md (its report, per the output contract) into its worktree —
   # three branches carrying different result.md at one path conflict at
   # merge. The report belongs to the run dir review, not the repo.
   for pair in "$A:alpha" "$B:beta" "$C:gamma"; do
     id=${pair%%:*}; name=${pair##*:}
     run_dir=$(./target/debug/fractality.exe --home "$MT02_HOME" show $id --json \
       | grep '"run_dir"' | cut -d'"' -f4)
     git -C "$run_dir/wt" add -- "$name.md"
     git -C "$run_dir/wt" -c user.email=mt02@local -c user.name=mt02 \
       commit -qm "swarm deliverable $id"
   done
   git -C "$MT02_REPO" merge --no-ff -m "merge alpha" "fractality/$A" \
     && git -C "$MT02_REPO" merge --no-ff -m "merge beta" "fractality/$B" \
     && git -C "$MT02_REPO" merge --no-ff -m "merge gamma" "fractality/$C"
   ls "$MT02_REPO"
   echo "MERGES_EXIT=$?"
   ```

   **Expected:** three merges succeed with no conflict output;
   `alpha.md`, `beta.md`, `gamma.md` all present in the repo root;
   `MERGES_EXIT=0`.

5. P4 arithmetic — swarm wall vs slowest single (bus timestamps):

   ```sh
   for id in $A $B $C; do
     ./target/debug/fractality.exe --home "$MT02_HOME" show $id --json \
       | grep -E '"(started_ts_ms|updated_ts_ms)"'
   done
   ```

   **Expected:** per run, `single = updated_ts_ms - started_ts_ms`;
   `swarm_wall = max(updated) - min(started)`. Record the three singles
   and the ratio `swarm_wall / max(single)` — **P4 passes when the
   ratio < 1.6** (admission ran the three genuinely in parallel).

6. Teardown:

   ```sh
   ./target/debug/fractality.exe --home "$MT02_HOME" mc stop
   rm -rf "$MT02_BASE"
   ```

   **Expected:** `mc stopped`; the real `~/.fractality` untouched.

## Recorded run

_(Agent pre-run output is appended below on each execution; the pass is
signed by a human — the pre-run only flags divergence.)_

### Pre-run 2026-07-10 — GREEN on firing #3; two findings folded back
(human sign-off pending)

Firing #1 hung in step 2: `A=$(fractality spawn …)` never returned —
the first CLI call auto-started the daemon, and on Windows the detached
daemon inherited the command substitution pipe's write end, so the
shell waited for an EOF that could not come (**F17**; fixed in
`fractality-mc-client` by stripping `HANDLE_FLAG_INHERIT` from the
caller's std handles around the detached spawn, pinned by the
`autostart_does_not_capture_the_callers_pipes` CLI test). The one
spawned worker (alpha) completed normally — the swarm itself was never
the problem.

Firing #2 ran the swarm green (three workers parallel, 23 s wall,
P4 ratio 1.00) but step 4's merges conflicted: every worker also
writes `result.md` (its report, per the output contract) into its
worktree, and `git add -A` had swept three different reports at one
path into three branches (**F18**). Step 4 now commits only each
worker's intended module; the report stays a run-dir artifact.

Firing #3 — all steps green:

```text
MT02-SPAWNED: A=01KX4Q3AGV7HTT1FAQ00SSF99N B=01KX4Q3AKM6FFBSKX8EEVSC9CM C=01KX4Q3AP1CKG97YE0FVK8C6DC
<three x> completed exit=0        (wait exit 0)
SWARM_WALL=16s
MERGES_OK=1  (alpha.md beta.md gamma.md all on main, no conflicts)
P4: swarm_ms=15268 slowest_single_ms=15268 ratio=1.00  (< 1.6 - PASS)
```

Spawns returned instantly (the async property), `ps`/`tree` showed the
three runs, `git worktree list` the three worktrees, and the three
`fractality/<id>` branches merged into `main` cleanly. **P4 confirmed:
the swarm's wall clock equals its slowest member.**
