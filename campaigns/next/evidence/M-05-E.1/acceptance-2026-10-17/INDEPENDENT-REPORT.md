# M-05-E.1 third candidate — independent delta review

**The previous positive CLI reachability P2 is resolved. No new P1/P2 was identified in the reviewed delta.** The ordinary command can now query an explicitly trusted embedding host holding the real admitted frozen basis, and the new positive test drives the normal show dispatcher, real handler and stdout emitter. Recommend Root assess this candidate against the existing E1 criteria, keeping the known global formatting failure separate. This report is not WorkAccepted, safety acceptance or payment of judging debt.

The review was read-only. No build/test, source/Git mutation, model, provider, API/process control or cleanup action was performed. Research outputs are outside the checkout.

## Candidate binding

Observed checkout HEAD is `87da7758f7ecddc447cc8da0062443773d57af86`. All **15** current source hashes match the third MANIFEST. The actual `git diff HEAD` plus nonignored untracked path set equals those exact15 paths. Independently reconstructed the full published patch against the base: every resulting file equals the current source. Both published patch/report sizes and SHA256 values match CANDIDATE-RESULT artifacts. The third report embeds the complete second report unchanged, and its prior-report digest matches the second published artifact.

Exactly four source/spec hashes changed since the second candidate: CLI inspect arguments, CLI features handler/tests, M16 catalog and the governing resolver spec. The other eleven are unchanged. Producer satisfied dispositions still have empty evidence arrays; those flags were not used as semantic proof.

## Why the former CLI finding is closed

The actual source path is now:

1. `crates/vibe-cli/src/main.rs:285` maps ordinary parsed `Command::Show` to `commands::show::run`.
2. `commands/show/mod.rs:39` dispatches Features to `run_features`.
3. `commands/show/features.rs:31` checks `discovery_host` before resolving project paths or preparing disk. Its new branch queries the host and returns through ordinary `ctx.emit_json` or human summary.
4. `features.rs:133` connects the explicit Unix endpoint, sends only capture/root, closes its write half, bounds response reading and checks returned capture identity.
5. Public `PreparedConfigurationDiscovery::serve_cli_query` at `document_analysis/catalog.rs:169` is callable by an embedding Rust host on its held typed admitted basis. Its closed request rejects unknown fields; exact capture/root must match before it serializes the immutable query projection.

This is a reachable production composition. The embedding caller owns UnixListener binding, access and lifecycle and invokes the public exchange method per accepted stream. No new daemon, launch engine or persistent endpoint registry is required to compose this API. The normal command is `vibe --json show features --discover --discovery-host <host-socket> --capture <held-id> --path <exact-held-root>`; clap requires discover/capture with host, prevents prepare with host, and keeps cached-manifest paths attached to explicit preparation.

The new `production_cli_admitted_host_output` at `features.rs:331` no longer substitutes the old prepared helper for positive dispatch. Its child test process parses that ordinary command, calls actual `show::run`, reaches `run_features`, and captures actual Context stdout. The parent supplies the caller-owned listener and public held-basis exchange. It compares the entire emitted JSON with an actual prepared `McpTool::run` result on the same basis, twice after corrupting cached manifest bytes. It additionally checks NeedsSelection, declared red default, all three tagged red effects, and exact manifest/lock/cache preservation. Existing helper tests remain auxiliary coverage. Child test harness invocations are observations within one unique test, not additional distinct tests.

The test exercises normal command dispatch and output, rather than the complete application startup binary. Source inspection separately verifies the existing main-to-show mapping. This is sufficient for the specific original reachability gap; no additional whole-binary build or generic host server gate is requested.

## Trust and platform boundary

The endpoint is explicitly **caller-trusted**. Root/capture checks correlate the held basis; they do not authenticate an arbitrary socket peer or prove its OS-user identity. Socket permissions and lifecycle remain with the embedding host. CLI response JSON is presentation only: it never becomes CandidateConfiguration, install authority, default permission or persistent capability state. No package manifest supplies the endpoint and no DTO flag admits a definition.

UnixStream transport is implemented on Unix. Other platforms return an explicit unsupported-route error; they do not acquire authority or silently use a different transport. No cross-platform runtime qualification or cross-user hostile-endpoint guarantee is claimed. `serve_cli_query` accepts a caller-supplied Read+Write connection; it does not listen, spawn, prepare or install. The adapter is a narrow VibeVM context presentation boundary, without Zap task semantics or PodBay process/lifecycle ownership.

The request reader is limited to64KiB; the CLI response reader to64MiB, with30-second stream read/write timeouts. These are local transport bounds, not an authenticated framing or hostile-host availability guarantee. No stronger guarantee is needed or inferred here.

Raw-only CLI behavior remains unavailable without typed admission, correctly preserving the earlier B2/C1 authority boundary. The new route supplies the missing positive path without manufacturing normalized identity from ordinary manifest bytes.

## Original E1 criteria and preserved semantics

The original goal, three steps, positive/negative cases and two criteria remain intact. This delta appends only M05E1-CLI-HOST-QUERY at `PROP-003-dep-evolution.xml:930`; no preexisting B1/B2/C1/C2/C3/D1/E1 law is removed. No schema/epoch, authored grammar, solver, admission or installation behavior was added by this delta.

The previously reviewed production freshness implementation, D1 projection, lock codec, current-authority guard and exact-replay refusals have identical hashes. Its existing10 integration cases still cover choice/feature/same-version-definition/default receipt changes before Fresh, unchanged configured Fresh, missing authority, malformed/unknown records and observer current/retained identity. Exact-replay refusal remains valid E1 changed-intent behavior; this work does not authorize E2 reconfiguration.

The admitted M16 construction and full B2/C1 projection remain the same production model. It retains exact resource/bytes, source/content/definition receipts, domain/instance, groups/guards/defaults, dormant alternatives, typed effects and declarations, validated through incumbent constraints and D1 projection. No production default authorization is created. Queries remain immutable and never prepare again.

Criterion1, fast-path request/definition identity, remains supported by unchanged production source and applicable prior integration execution. Criterion2, no authored/cache mutation by discovery, is supported by the frozen query implementation, current real CLI/MCP parity and byte-preservation tests. Counters describe frozen service operations, not socket writes, endpoint binding, explicit preparation or all process activity. The candidate states this limit correctly.

## Published executions and applicability

New current final executions contain actual named tests and nonzero result lines:

| Family | Log | Final result | Exit |
| --- | --- | --- | ---: |
| CLI | check-04.log | 5 passed,0 failed | 0 |
| M16 | check-06-attempt-2.log | 7 passed,0 failed | 0 |
| MCP | check-05.log | 4 passed,0 failed | 0 |

The initial M16 `check-06.log` is retained:6 passed,1 failed, exit101. Its negative socket test correctly observed host refusal, then incorrectly unwrapped a clean-EOF read where Linux returned ConnectionReset after unread invalid request bytes. Current source accepts only EOF or ConnectionReset while still requiring `serve_cli_query` Err and no reply bytes. This corrects the test assumption without weakening production rejection. The earlier second-candidate E0425 compile101 and retries also remain in carried evidence.

Current final total is16. Applicable unchanged prior cases are17: install10, legacy feature successor1, two distinct prepared controls2, workspace4. Combined total is **33 unique selected tests**, not31+16. The old CLI4/M166/MCP4 executions are superseded for the changed closure and not counted again. Duplicate check slots02/07 still cite one actual prepared root-union run once. The stale A1 root_union binding was not restored.

Carrying install/workspace evidence is justified here: their production/test/registration sources are unchanged, the exact checkout path set contains no manifest/lock modifications, and the new changes are downstream CLI/orchestrator plus additive spec. The CLI production dependency changed only by the newly added host exchange; the later M16 correction is confined to its negative cfg(test) assertion. No entire dependency suite rerun is claimed or requested.

The literal global `cargo fmt --all -- --check` remains the prior **exit1** on the same seven unrelated baseline paths. It was not rerun or repaired, and is not green. Third report retains scoped rustfmt exit0, governing XML parse success, tracked diff check exit0 and added-file no-index checks with literal exit1/no whitespace diagnostics. Root retains baseline interpretation and the existing gate decision.

## Recommendation and limits

Recommend proceeding to Root's original E1 acceptance decision: the former CLI dispatch P2 is resolved, original criteria are supported within their stated boundaries, and no new actionable P1/P2 was found. Preserve all failed attempts and the valuable WIP. Do not add a full suite, proof/kernel gate, new server architecture or persistence requirement to this bounded review.

Unqualified claims would still be wrong: this is Unix transport with caller-owned trust/lifecycle, not a standalone auto-starting discovery host, authenticated cross-user service, all-platform proof, implicit install/default consent, current whole-machine purity guarantee or global-green formatting result. Those are stated limitations, not new acceptance blockers for the scoped E1 MVP.
