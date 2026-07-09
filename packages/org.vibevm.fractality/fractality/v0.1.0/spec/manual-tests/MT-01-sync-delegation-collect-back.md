# MT-01 — sync delegation with collect-back (Phase 3 exit E2E)

_Proves: the full `fractality run` loop against a live GLM-5.2 worker —
spawn through mission-control, pod supervision, prompt via stdin,
worker-authored deliverables, stream metering, result collection with
provenance, and pod-side acceptance (`cargo test`) — end to end on one
packet. This is the plan's Phase 3 exit criterion and manual-test #1._

**Paid:** one GLM-5.2 (`model = "big"`) worker turn on the z.ai coding
plan. **Isolated:** the run uses a scratch `--home`, so the real
`~/.fractality` (journal, runs, lockfile) is never touched. Profiles
are home-scoped (`<home>/profiles.toml`) — the scratch home gets its
own copy in step 1, and mission-control refuses the run with a loud
400 naming the fix if it is missing.

## Prerequisites

- `~/.fractality/profiles.toml` with the `glm` profile (copy
  `spec/examples/profiles.sample.toml`); `~/.vibevm/zai.api.token`
  present.
- Built binaries: from `fractality/v0.1.0/`, `cargo build --workspace`.
- `cargo` on PATH (the acceptance command uses the pod's own env).

## Steps

1. Make a scratch home, give it the profile, and run the packet:

   ```sh
   cd packages/org.vibevm.fractality/fractality/v0.1.0
   MT01_HOME=$(mktemp -d)/fractality-home
   mkdir -p "$MT01_HOME"
   cp ~/.fractality/profiles.toml "$MT01_HOME/profiles.toml"
   ./target/debug/fractality.exe --home "$MT01_HOME" \
       run --packet spec/manual-tests/mt-01-packet.toml
   echo "EXIT=$?"
   ```

   **Expected:** `run <ulid> spawned (dir <scratch>/runs/<ulid>)` on
   stderr, then the one-screen summary: `state: completed`,
   `exit_code: 0`, a `usage:` line with non-zero `in=`/`out=` and
   `events=`, a `cost_usd:` line, `result: …work\result.md (worker)`
   — provenance `worker`, the model wrote its own report —
   and `acceptance: 1/1 ok` with `  ok   exit=0 cargo test`.
   `EXIT=0`.

2. Inspect the run dir named by the summary:

   ```sh
   ls "<run_dir>" "<run_dir>/work"
   ```

   **Expected:** run dir holds `packet.toml`, `run-spec.toml`,
   `worker-stdout.jsonl`, `worker-stderr.log`, `status.json`,
   `usage.json`, `acceptance.log`, `pod.log`, `cc-config/`; `work/`
   holds `Cargo.toml`, `src/lib.rs`, `result.md`, and a `target/`
   created by the acceptance run.

3. Check the recorded verdicts:

   ```sh
   cat "<run_dir>/status.json"
   ```

   **Expected:** `"state": "completed"`, `"exit_code": 0`,
   `"result_source": "worker"`, and an `"acceptance"` array with one
   entry: `"command": "cargo test"`, `"ok": true`, `"exit_code": 0`.

4. Confirm the deliverable actually passes outside the harness:

   ```sh
   cd "<run_dir>/work" && cargo test
   ```

   **Expected:** the worker-written unit tests compile and pass —
   the same green the pod recorded in `acceptance.log`.

5. Stop the scratch daemon and clean up:

   ```sh
   ./target/debug/fractality.exe --home "$MT01_HOME" mc stop
   rm -rf "$(dirname "$MT01_HOME")"
   ```

   **Expected:** `mc stopped`; the real `~/.fractality` never changed.

## Recorded run

_(Agent pre-run output is appended below on each execution; the pass is
signed by a human — the pre-run only flags divergence.)_

### Pre-run 2026-07-10 — GREEN, no divergence (human sign-off pending)

Step 1 summary (scratch home under the session temp dir; run
`01KX4JRBNQ774N0G9VYG218TKD`):

```text
state:      completed
exit_code:  0
waited:     36s
usage:      in=15165 out=1772 cache_w=0 cache_r=26880 events=599
cost_usd:   0.133565
result:     …\runs\01KX4JRBNQ774N0G9VYG218TKD\work\result.md (worker)
acceptance: 1/1 ok
  ok   exit=0 cargo test
EXIT=0
```

Step 2: run dir held all nine artifacts (incl. `usage.json`,
`acceptance.log`); `work/` held `Cargo.toml`, `src/lib.rs`,
`result.md`, plus `Cargo.lock`/`target/` created by the acceptance
run. Step 3: `status.json` recorded `"result_source": "worker"` and
`acceptance: [{command: "cargo test", ok: true, exit_code: 0,
duration_ms: 366}]`. Step 4: `acceptance.log` shows the worker's four
unit tests (`fizz…`/`buzz…`/`fizzbuzz…`/`otherwise…`) all green —
`classify` is a correct implementation on inspection. Step 5: scratch
daemon stopped; the real `~/.fractality` untouched.

One divergence found and folded back into this document on the first
attempt: profiles are **home-scoped**, so the scratch home needs its
own `profiles.toml` copy (step 1 now does this); mission-control
refused the profileless run with a 400 citing the spec anchor and the
exact fix — the D14 error contract observed working in the field.
