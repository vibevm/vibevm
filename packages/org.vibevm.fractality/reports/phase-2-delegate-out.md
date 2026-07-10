# Phase 2 report — delegate-out (retrospective)

_Campaign: FRACTALITY-IGNITION v0.1 · Phase 2 · code landed 2026-07-10
(prior session), exit E2E fired 2026-07-10 · written retroactively at
campaign close from the §14 ledger (commits `b15bd02`, `10bc4b9`,
`38d78bc`, `9996f74`); the ledger stays canonical._

## What the phase built

The fire-a-worker path end to end: profiles (D6) with validation;
the D5 clean-slate env constructor as a pure function over an os-env
snapshot (the I1 poisoned-parent test is a plain unit test); the
headless invocation builder (flags pinned live on CC 2.1.202); RunSpec
+ BackendSecrets (Debug-redacted) + the widened WorkerBackend seam;
pod `--run-spec` product mode (the token is read pod-side at spawn,
never transits specs); the MC spawn path (validation → D8 workspaces
incl. git worktrees → run spec → detached pod launch); `fractality run
--packet` as the sync loop.

## The exit proof

Run `01KX4H4KESV9ADN6S0AJMWQHFW`: a live GLM worker, exit 0 in 29 s,
`hello.txt` byte-exact, a worker-authored `result.md`, the transcript
carrying usage fields — **P2 CONFIRMED on the product path** (and R5
re-confirmed: a fresh config dir onboards headless inside the product
flow).

## Strange things / paid-for lessons

- **F14 — the Windows spawn seam, three defects found by ONE E2E
  firing** (the first run, kept on disk as the autopsy):
  (a) CreateProcess resolves bare names to `.exe` only — npm ships
  `claude.cmd`, so the pod resolves PATHEXT-style against the WORKER's
  PATH; (b) the prompt cannot ride argv (cmd.exe rejects newline
  arguments; 32 KiB cap — fatal to big one-shot goals) — it rides
  `WorkerSpec::stdin`; (c) the D5 whitelist matched case-sensitively
  while stock Windows spells `Path`/`ComSpec` — a pod launched outside
  bash handed its worker no PATH.
- **F15 — arbitration is our own domain:** a running daemon holds the
  .exe lock against cargo rebuilds (Windows denies
  write-while-execute). The dev law "stop the daemon before builds"
  was born here — and kept firing for the rest of the campaign.
- The worktree-manager integration tests were delegated to GLM
  (scenario 1, cwd pinned) and landed green first try — the
  delegation-law rhythm proving itself before the fabric could.
