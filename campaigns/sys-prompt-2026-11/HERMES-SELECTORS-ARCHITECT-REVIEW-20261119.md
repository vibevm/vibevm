# Hermes query selectors — bounded architectural acceptance

2026-11-19. Base `cda62ffbbaeea51f1402622d9d2c332a79b87005`.

Accepted frozen manifest `/data/git/vibevm-native-carrier-20261119/HERMES-SELECTORS-CANDIDATE.json`, SHA256 `11c278d0309e3fc741139d94f3772303e059b96ed2e8d98496f7a77bf333390e`. Independently matched its digest and all 10 current file hashes (six product, four documents). Committed-blob verification and source publication are subsequent steps, not established by candidate acceptance.

## Reviewed design and concrete rework

Trusted launcher resolution precedes native selection. One owned supervisor holds native profile/runner across ready/ack; the existing Rust project select/build/opt-in/composition owners operate at the selected cwd. Exact session/cwd is checked again before attachment. Existing native history/resume policy and worktree lifecycle owners remain authoritative. No native identity fabrication, global/class patch, installed-runtime modification or copied startup loop is admitted.

Two concrete cancellation findings were repaired and inspected:

1. Cancellation of `to_thread(create)` could lose its eventual owned worktree result. The same creation task is now retained/shielded/settled and its exact result passed to native finish once. Side-effectful restore and finish retain their tasks as well. The synthetic repeated-cancellation create/restore test was inspected; its PASS log is unit ownership proof, not native hard-kill coverage.
2. Pre-ack cancellation was conditional on prefix presence, so read/opted-out fallback could acknowledge frontend start after cancellation. Signal polling is now unconditional; the existing atomic acknowledgement body is a private helper that vetoes before writes. The focused None+cancel empty-ack test and noncancelled None control were inspected.

Rust teardown has a 240s outer leash and explicit residual-state warning before killing/reaping its supervisor. This is bounded cleanup, not guaranteed native descendant/worktree cleanup. Unknown retained paths are not deleted. Native finish may preserve a tree containing generated dirty boot artifacts according to its own policy.

## Evidence inspected

- `control/hermes-selectors-exact-history.log`: actual resume/continue history and invalid-selector zero additional requests, binary `8561f8367b1f414fcebb1bf815c1cc586bcc04a65f991d21ad7507ec75b609a8`.
- `control/hermes-selectors-terminal-worktree.log`: actual selected worktree cwd/boot request, binary `f0a4ade3e79cf900a98c9b3eb5f0f5c5815543bc7035b666965c935925289a2e`. Producer fixture additionally checks one retained native tree and its generated boot artifacts.
- `control/hermes-selectors-cancel-owners.log`: repeated-cancellation owned create/restore PASS; test source inspected.
- Final `control/hermes-selectors-cancel-ack-cli.log`: 16 PASS; `hermes-selectors-cancel-ack-conform.log`: 0 new / 141 existing. Earlier accept CLI log has 15 tests and is not the final repaired gate. Formatting/check are producer evidence; scoped read-only diff check was independently clean before the final guard change.

Native logs bind their stated earlier binaries. Final short-option and pre-ack guard edits are covered by focused tests, not falsely claimed as the exact same native-run binary. Architect reviewed peer sources/logs without building or writing in the peer checkout.

## Scope and continuation

Accept the bounded native query resume/continue, explicit worktree and cwd selection slice. Native oneshot remains BLOCKED/RED at the source-bound runtime readiness/ticket prerequisite. Query is not a replacement for unattended oneshot. Native classic-versus-gateway flag dispatch, config-driven worktree, shared/external gateway, actual workers, platforms and final parity audit remain open. No SYS-PROMPT/CORP-ENV/NST closure follows.

Builder was instructed to commit exactly the accepted 10 files, send OID, update source plan before dashboard, then address finite native parser/frontend and config-driven worktree mapping through existing native owners. Upstream/bootstrap changes require a separately admitted route; the blocked seam proposal is not authority to patch native security or waive parity.
