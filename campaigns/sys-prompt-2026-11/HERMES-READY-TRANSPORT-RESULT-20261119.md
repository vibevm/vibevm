# Explicit READY-session transport — actual two-client result

2026-11-19. Implementation candidate, pending independent acceptance. This is **transport/history support, not instruction delivery**. Existing owned Hermes delivery is unchanged. Neither `HERMES-REQUIRED-PROMPT` nor `HERMES-READY-PLACEMENT` is closed.

## User-visible route

`vibe agent <hermes-launcher> --vibe-ready-session=transport --resume <exact-ID>` explicitly attaches to an already-READY local owner of the selected native profile. Optional native `--profile <name>` and an empty trailing delimiter are admitted. Unknown/duplicate/conflicting arguments refuse; Vibe's selector after the native `--` delimiter is not consumed. No title/latest/create/worktree/provider override is inferred.

Before attachment, stderr states that this launch does **not** add Vibe instructions. Existing conversation instructions/history are retained; transport is not a claim that the conversation contains no prior instructions. `--vibe-ready-session=conversation` refuses separately because the admitted structural placement boundary is unavailable. No automatic downgrade to transport, composer call, plugin binding, append/prepend or owner startup/replacement is introduced.

The native installed interpreter and additional discovery/client/view module identities are admitted. The helper discovers READY, connects with `connect_gateway(endpoint=discovered)`, resumes the exact ID, takes the canonical cwd from its native snapshot and retains one full history for native display. Before each send, the same client rechecks the frozen profile/instance/epoch/runtime protocol/origin/control-home and exact session/cwd; compact resumes use `omit_messages=True`. Session-changing slash commands are outside this bounded route and refuse visibly. Other native input/render/response behavior remains `GatewayChatView`'s. Teardown settles only the invocation-owned viewer, never the serving owner. Native discovery retains its documented same-owner lock-mode narrowing behavior; this is not a filesystem-pure query.

## Actual checks

- Initial two formatting-only attempts exited 1 before compilation; retained as such. After bounded formatting/module-order correction: `cargo fmt --all --check` PASS.
- `cargo test -p vibe-cli --bin vibe hermes_ready_session -- --nocapture`: **2 PASS**, terminal exit 0, task `b62lt6n86`.
- `cargo build -p vibe-cli --bin vibe`: terminal exit 0, task `bfyvjjk35`. Process-local jobs=2, existing target and incremental unchanged.
- Python syntax and five pure identity/cwd cases PASS. Static checks establish one full initial history read, compact rechecks, and no plugin/ensure calls in this helper. These are not substitutes for native proof.
- Independent review found two defects before acceptance: importing native `main` could run interrupted-update recovery/relaunch, and mutating slash controls bypassed the transport recheck. Profile selection now uses the narrow native profile APIs without importing `main`; controls reuse the validated compact snapshot, including `/discard`, without a second unvalidated RPC. Pure explicit/sticky/environment-pinned profile checks pass. Prior native passes remain tied to their original bytes; final corrected-source native verification is recorded separately below.
- Final corrected-source verification `b9mc3ebny`: build and the extended actual two-client/native refusal fixture both completed with terminal exit 0. Both review fixes and compact `/discard` snapshot reuse are included in this binary. Earlier passes remain historical rather than being relabeled as final-source results.
- Initial isolated stock-gateway probe `bpjahw3pd`: READY discovery, native ticket, exact returned session ID and expected canonical cwd PASS; no prompt submission.
- Actual Vibe two-client PTY scenario `bwa8mwdvr`, then the extended negative scenario `bsma9ri7t`: terminal exit 0. Client one remains connected while client two opens the same exact native history. Client two sees client one's user turn and synthetic response, submits its own turn and receives a response. Native history contains both turns; the READY owner instance remains unchanged; both clients exit 0 without stopping the owner.
- Extended native negatives: unknown exact ID refuses nonzero; loss of the fixture-owned owner ends its viewer nonzero; a new launch with that owner absent refuses nonzero and no replacement appears.

The private HTTP receiver observed five requests in each positive run; this is **not** classified as five main turns or five passes. Two requested main responses/history checks passed. No real provider/model was contacted. A real replacement-owner restart scenario is not yet executed; pure instance-identity mismatch is covered separately. No Windows/macOS qualification, TUI/Ink support, strict delivery, cache hit/savings, complete worker/compaction parity or SYS completion follows.

## Isolation and subject

Fixture root: `/data/git/vibevm-hermes-ready-transport-20261119T155752Z/`. Control scripts are separate from read-only native source/dependency mounts. Bubblewrap uses `unshare-all`, `clearenv`, isolated proc, tmpfs HOME/TMP, read-only explicit runtime/system mounts and a single writable evidence directory. Native source pin is `4ae451d4126327c871fd084e60c045dec0c85911`. Only fixture-owned gateway/viewer subprocess handles are stopped/reaped. Native configuration and synthetic loopback provider exist only in the sandbox; host credentials/profiles and external network are unavailable.

Candidate source base: `2ebbde8a69ec09d7e80c1bca4ebdda217fa29f80`; unrelated repository dirt remains preserved. Exact candidate manifest (six product files plus this report): `evidence/transport-candidate.json`. Built binary SHA-256: `6257680a56c290cf03b2db35b1067812ef08950d24da6c7bf3dfd14c1b71bbe0`. Positive fixture/logs were retained separately before extending negatives. The manifest/report must be refreshed after any source correction; this report is not architect acceptance or publication.

Next: independent diff/native-proof review, focused affected gates, then the next finite executable SYS scenario. Do not repeat placement research or build a new proof matrix. Final six-harness audit and coherent gate on final source remain owed.
