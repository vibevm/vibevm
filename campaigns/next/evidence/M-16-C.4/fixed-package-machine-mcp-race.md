# Fixed package machine-frame and MCP concurrent submit

Zap Current revision 11, strategy `strategy.vibevm-next.coarse-c4.1`, names
`next.m16c4.parent`. This is a bounded M-16-C.4 process proof after
`d3d5d3441`; C.4 remains active.

A machine-frame process issues one fixed package grant, then reattaches while a
separate CLI-launched MCP process reaches `tools/list` readiness. A barrier
releases both surfaces to submit the same scoped key. Both may receive the
same retained completed reply, or the lease loser may receive a typed
`submission_refused` with `effect_observation=unavailable` and no claimed run.
After both finish, retry on the losing surface binds to the completed owner.
The test requires one C1 namespace/key, one package intent/capsule/result set,
the same output bytes and an unchanged grant pointer.

## Exact checks

- `cargo test -p vibe-cli --test machine_session_package machine_frame_and_mcp_race_one_fixed_package_key -- --exact`: 1/1, exit 0 after correction, then four further exact 1/1 runs, all exit 0.
- `cargo test -p vibe-cli --test machine_session_package`: 5/5, exit 0.
- `rustfmt --edition 2024` on the changed test file and `git diff --check`: exit 0.
- These Cargo commands used the campaign two-slot wrapper with a temporary `/tmp` lock root and `RUSTC_WRAPPER=` because this sandbox cannot write the shared lock directory or start sccache. Final logs: `/tmp/vibevm-next-c4-package-race-final-exact.log`, `/tmp/vibevm-next-c4-package-race-final-repeat-{1,2,3,4}.log`, `/tmp/vibevm-next-c4-package-race-final-family.log`.

The first readiness-enabled run failed its test oracle: it expected the lease
loser to report `no_effects`, while the machine-frame owner correctly reported
`uncertain/unavailable` because another owner could be moving effects. The
oracle now requires that conservative typed state and same-run retry; no
production defect was observed. The barrier aligns request dispatch but does
not instrument exact overlap inside the domain owner. This proof does not
cover distinct intents or complete C.4 equivalence. Historical accepted tasks
were not revalidated; no full gate panel, routine mutation testing or public
1.1.0 release ran.
