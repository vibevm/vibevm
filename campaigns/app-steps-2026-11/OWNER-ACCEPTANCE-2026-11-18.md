# PRIOR-APP-STEPS — owner acceptance with explicit limits

On 2026-11-18 the owner answered the builder's actual question dialog by selecting
**«Принять с указанными пределами»**. This accepts the completed PRIOR scope with
its disclosed verification limits; it is not inferred from a goal/stop hook or peer
approval.

## Accepted implementation and evidence

- Full contribution logs and shared presentation: `ddc7e6863`.
- Native authority and cross-thread terminal ownership repairs: `f8dcdbe72`.
- Test-only conformance correction: `d0632efdbc85cdc4c3491c73a819b543107d9187`.
- Final complete eight-stage canonical run on d063: **7/8 stages PASS, exit101**.
  The sole failing test is
  `cli_hook_rerun::direct_install_quiet_keeps_the_existing_slot_stream_policy`.
  Its legacy output-preservation requirement was explicitly superseded by the
  owner. The test was not skipped, hidden or changed to manufacture a green gate.
  This run is **not all-green**.
- Corrected source-f8 Windows and macOS builds and actual ordinary install/full-log
  proofs pass, including full2400032-byte logs independent of bounded previews.
  The f8→d063 changes are test-only; that relationship was independently reviewed.
- Actual macOS PTY stop journey: exit130, interrupted record, partial log retained,
  terminal flags restored. This does not prove a human visual GUI observation.

Final receipt: `/fast/tmp/prior-final-canonical-receipt-20261118.json`.
Full log SHA256: `12a8fc120ce0afd9ce718724512baa9222e0309b5bd5a358cfbc264c1860da98`.
Native receipts: `/fast/tmp/prior-{windows,macos}-f8-NATIVE-LOGGING-RESULTS.json`.
Mac terminal receipt: `/fast/tmp/prior-macos-terminal-f8-RESULTS.json`.
Earlier native failures, aborted gates and the cancellation hang remain retained;
no old result was retroactively relabeled successful.

## Disclosed unperformed checks

Windows physical-machine/manual terminal checks were already made nonblocking by
an earlier explicit owner decision. The final acceptance additionally covers the
remaining unperformed macOS GUI, physical external-media and current attended
real-agent MT-06 checks. These remain **UNVERIFIED / NOT RUN, never PASS**.
Mechanical agent-hook tests and native/PTY evidence keep their exact narrower
proof boundaries. No live provider call, credential change or arbitrary download
was performed to manufacture completion evidence.

## Scope and stop boundary

No legacy compatibility shim was delivered. The owner moved broad legacy removal
to the ordered global-repair stage after initial ideal synchronization; it is not
an outstanding PRIOR implementation task. Its retained ledger and future plan
remain distinct from current acceptance.

The owner explicitly required stopping after PRIOR completion for subsequent
server maintenance. Do not automatically start Hermes/Pi, NST global repair or
maintenance, and do not infer external publication authority. No external push or
release is implied by this acceptance.
