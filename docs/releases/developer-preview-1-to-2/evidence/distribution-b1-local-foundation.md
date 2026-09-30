# M-07-B.1 local smoke foundation, still open

The current local path stages exact verified 1.1.0 bundle bytes into a fresh
temporary home, remeasures the executable, and attempts `vibe --version` only
on a compatible native host. It limits child duration and retained output.
Foreign hosts, preflight failures and spawn failures remain `Unavailable`
without a tested digest or runner. A process that actually started may be
`Failed`; only successful exact output becomes `Passed`.

Five target rows have a registered strict `distribution-smoke-report/e1` format
and a pure semantic validator. The local preparation reads committed source
and all bundle/fragment/bootstrap assets under one held output capability,
persists the report by SHA-256 without replacement, derives report-bound smoke
probes, recaptures the final subject and refuses source/asset drift. Report
bytes are reread after final capture. This path has no release-host action.

Focused evidence: stage tests 5/5, Linux system-child harness tests 10/10,
report structural corpus 4/4, semantic validator 6/6, local report store 4/4,
pure five-row join 4/4, and local integration cases covering five honest
`Unavailable` rows, changed bundle refusal and existing capture behavior.
`cargo xtask check-codegen` passed after the report schema was committed.

This does not accept M-07-B.1. Exact Linux GNU and musl bundle smokes are
recorded in `distribution-b1-native-linux-gnu.md` and
`distribution-b1-native-linux-musl.md`. Native Windows, macOS x86_64 and macOS
aarch64 release-bundle receipts remain unobserved on this host. Default apply
readiness still requires all five required probes to pass.
No public Git push, release upload or publication was performed.
