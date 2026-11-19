# Native carrier architectural review — 2026-11-19

Status: proof in progress, not native parity acceptance.

## Accepted architecture

Full cache-stable prefix capability parity for Hermes and Pi is required by the owner. Reduced read-only behavior cannot close the task. Exact architecture correction f181067b3e451ee4af48bd6ae8d4803916ebff05; builder integration f79617c7139436f74ae1e3c5da24572b08f37850. Necessary runtime provisioning is explicitly authorized locally and on flash/base; isolation and actual harness boundaries remain binding.

Builder packet `NATIVE-CARRIER-PROOF-PACKET-20261119.md`, SHA25635ca60a0cae867ea7f4f9c1591f1834106158a8bfd4ffeea846c2ab3cf49f00e, was read and approved for bounded provider-free execution. It covers actual native prompt assembly, full file prefix, stable/volatile placement, preserved resources, fresh personal layers, worker/child exclusion and resume. No extra architectural approval is needed for that already decided scope.

## Independently inspected preliminary receipt

Read `/data/git/vibevm-native-carrier-20261119/HERMES-ASSEMBLY-PRELIMINARY.json`.

- Hermes upstream4ae451d4126327c871fd084e60c045dec0c85911, archive SHA25685dff02520dd8d71e01f1313d4b0dcb111c4404c69ea058e53bfab2bb6ff0601; assembly-source SHA256d9b4289c45144ad193fd179cc8c2b9f8053b41edfdc657e1768c061c3593e891.
- Actual `agent.system_prompt.build_system_prompt_parts` import/construction executed with synthetic agent state and empty toolset. Stable1476/context24/volatile791 characters; caller context in the context tier.
- Receipt records bwrap unshare-all, clear environment, synthetic HOME and read-only upstream/dependency inputs; no provider construction, SDK request or network-call attempt. This review read the receipt, not an independent rerun or a complete isolation audit.
- Failed dependency/import attempts are retained; no host privilege/security change is claimed.

Verdict: a real native composition entry point is established, so carrier execution can continue. This is NOT native CLI, complete tool/resource preservation, request transport, worker/resume or cache-stable prefix parity proof.

## Pi preliminary native entry

Independently read `/data/git/vibevm-native-carrier-20261119/PI-ASSEMBLY-PRELIMINARY.json`: official `@earendil-works/pi-coding-agent@1.5.2`, Node24.18.0, npm exit0 with lifecycle scripts disabled; package-lock SHA2562be44cffc03d2982915ac01f581cd78c1858e9348507fcd9626bc9f5b199364a and installed prompt-module SHA2565f68e89c31ba3d4d1af8ad2bfd0c7c548879d47ee6ab4aabee50a826100da32c. Actual native buildSystemPrompt returned a string with additive canary and four ordinary tools under offline bwrap, with no provider/SDK/network attempt. This is helper-level evidence, not extension/CLI/worker/resume parity. Published-package-to-repository-commit equivalence is explicitly unproven; bind the actual installed runtime/module rather than inventing equivalence or blocking prototype work on unrelated reproducible-build work.

Both native composition entries now run. Runtime absence is resolved; actual bounded carrier coding/execution is the next producer step, with no additional architecture readiness cycle required.

## Actual coding dispatch and isolation receipt

Builder reports background task bxbt6q1nw launched through the approved claude-ds lane against the admitted full-boot worker snapshot7874cad21, with writes limited to prototype/proof artifacts. A first pre-worker attempt was refused because /data/git was not an admissible trusted ancestor; builder relocated only runtime staging below /data without changing shared directory permissions or weakening the observer. No worker or carrier PASS is inferred from process startup.

Independently read `/data/git/vibevm-native-carrier-20261119/ISOLATION-GATE.json`: exit0, SENTINEL_ENV_PID_GATE_OK, stated fs/env/pid sentinel scope and unchanged approved relay. Reading this receipt is not an independent audit of every network/credential boundary. Await actual worker terminal output and independent offline carrier proof.

## First carrier candidate rejected

Builder reported the first carrier run timed out after1800seconds with signal9; a runner exit0 only meant the control receipt was emitted, not worker success. Initial candidate review found a module-global replacement of build_system_prompt_parts despite the instance-scoped contract, with a recursive path back into the patched function. A manually reproduced prompt tail was not actual native CLI integration, Pi prototype/proof remained absent. Architect agrees these are blocking contract violations; the candidate was not executed or accepted as parity proof. The failed run and partial artifacts are retained. One bounded rework through the same approved lane was started; behavioral failure is not a reason to switch provider/model until a passing result appears. Await exact corrected candidate and independent native proof; do not add speculative acceptance criteria.

## Accepted native seams and actual CLI boundary

Independently read HERMES-ACTUAL-INSTANCE-SEAM-RESULT.json and the hash-matching wrapper86e6a60965f36e7cdbd53c01f8640a1d0cdb6e362adedcb0f09eed701fa7a131. Actual native AIAgent constructor and original bound build executed netless,310040byte prefix once before caller context, unrelated actual instance excluded, class/module unchanged and detach restores. Scope accepted: real constructor/instance seam, not full resources, worker creation, resume or CLI. An unused synthetic-substrate helper remains in the proof file and must not ship as product support.

Independently read PI-NATIVE-EXTENSION-SEAM-RESULT.json and actual factory761f18c295f43d221df52be2abd4ad5b4786b7de25d36be7e552d6b70d94fdb3: native loader, SessionManager, ExtensionRunner and prompt construction consume310024bytes once with explicit session binding, independent of tool count. PI-ACTUAL-SDK-ATTACH-RESULT.json further establishes real ModelRuntime/DefaultResourceLoader/createAgentSession attachment, no provider request. These narrow outcomes are accepted. Nonempty append ordering was actually RED; a finite prefix-before-tail and exact options-owned WeakMap fix is approved, not yet accepted as completed.

Actual Hermes CLI at the pinned version uses GatewayAgent transport, not the local AIAgent construction path. The accepted instance proof is not CLI parity. Next bounded feasibility target is a genuinely invocation-owned native gateway/session context whose supported factory constructs a real native instance and attaches the carrier for that exact session. No shared-gateway/global class/factory patch, second agent loop or reduced UI is accepted. Return a precise native API gap if per-session injection cannot be established; do not manufacture parity from the earlier SDK proof.

## Actual native gateway/client ingress milestone

Independently read `/data/git/vibevm-native-carrier-20261119/HERMES-NATIVE-CLIENT-INGRESS-RESULT.json` and verified all ten referenced log SHA256 values against their actual bytes. Reviewed receipt SHA256: `439923c04a00dcdb4bfe0f12f8805cab225c1406cb7a432fb0db82daea464ad3` (subsequent append changes this witness).

Actual native `GatewayRunner`, session authority, API listener, lazy web application initialization, own-process URL builder and `connect_gateway` execute in the synthetic isolated runtime. Successful logs establish session.list, session.create/info, then actual native threaded `GatewayConnection` and `GatewayAgent` binding to the created CLI session; positive markers and exit0 independently checked. Private synthetic URL credentials were not printed/persisted according to the receipt; this review does not reproduce them or independently audit all sandbox capabilities.

Accept this as native client/session/CLI-adapter feasibility only. It is NOT whole interactive CLI invocation, prefixed agent construction, worker/fork/resume, complete resource parity or a provider turn. Native missing Nous auth and absent snowballstemmer warnings remain recorded; do not claim tool-search/resource completeness. Preserve failed provisioning/auth-mode attempts separately: a missing-fastapi403 is not an authentication-negative result. Builder additionally supplied a distinct complete-runtime missing/invalid-credential403 negative receipt; its referenced log hash was verified, without broadening the positive milestone.

Next finite verification: actual supported CLI entrypoint through that same invocation-owned endpoint, without prompt/model submission or admission changes. Session-specific gateway factory/carrier implementation still needs the approved coding lane. Builder explicitly confirmed that the coding-lane timeout blocker is unresolved; no successful coding receipt or new substitute-lane authorization exists. Independent runtime verification does not authorize product patches.

## Actual Hermes CLI ingress and normal exit

Read HERMES-ACTUAL-CLI-RESULT.json for task `bgbbehj3l`; independently verified frozen executed script SHA256 `150ee0b1723f67e034accef6653fb7f52833b4fd02fd8bcbdda644f5ad0fe361` and actual log SHA256 `bc4289a269a28ad6dcc4c9c37149ea707a3b99e900c8fc9a5b324d92c6024fd5`. Actual `hermes_cli.main.main chat` ran in the child PTY, declined native telemetry, reached a gateway session, received `/exit` and exited0 without timeout or forced termination; final authority session count0. Accept native CLI ingress and normal-exit feasibility only.

The driver readiness check uses a prompt glyph in cumulative output, so this is not proof of every interactive widget being ready. Missing truststore/SSL bundle, missing snowballstemmer and native SQLite DELETE fallback remain disclosed. No provider turn, prefixed gateway instance, full resources, workers or resume/cache parity is established. Earlier timeout/terminated0 attempts remain unsuccessful; the current frozen script differs from the older version that did not print diagnostics on timeout0. Do not retroactively infer those older missing tails.

## Pi ordering candidate and proof review

Read revised carrier `/data/git/vibevm-native-carrier-20261119/pi-order-exact-retry/carrier.mjs`, verified SHA256 `f8697d8b7521dbc33233b1f3e8102dd52c1a1ccdf11ba4d1d5c581a405fe84b9`, corresponding proof SHA256 `f2bee973f5c8dbe5698c5a9f7aa1e115e8bfc07de55438fc4fbb0588697763a4`, and all three logs in PI-ORDERING-RESULT.json. Actual source uses selected-session admission before file IO, validates options, freezes prefix on first admitted event, preserves caller tail after the prefix and uses a private WeakMap rather than text heuristics. No blocking design defect found in this ordering candidate; this is not full adapter acceptance.

Proof review found an insufficient native unrelated-session assertion: optional chaining plus an expected-value fallback could pass if the result/options were absent. Builder was asked to assert actual native output presence and exact unchanged append, and preserve native extension-error observations. Pure-handler missing-file exclusion and native full-tool-session binding are distinct evidence; do not merge them into an uninstrumented native file-read claim.

Actual installed ExtensionRunner catches handler exceptions and emits errors while continuing. Consequently a handler throw establishes neither native turn abortion nor zero provider submissions. Ordering/native diagnostic proof and production fail-closed adapter proof remain separate. A replacing system prompt, global native patch or custom agent loop is not an acceptable shortcut.

## Native runner and UI integration feasibility

Hermes owned-runner candidate SHA256 `f3b06d75f4c5da058365f9d273a70be5c3f15ad1052fe3ed2339186f72dd5c9a` and strict driver SHA256 `75fc36f0b6caa146fc5c74a9cdfcb065b8e8f53f2d34156343cf28d4d32aba2b` were independently read/hash-verified. Receipt HERMES-OWNED-RUNNER-RESULT.json distinguishes actual CLI-admitted identity followed by direct native turn construction/cache calls from a CLI-submitted model turn. Native constructor/prefix placement, cached same-instance reuse and unrelated-session exclusion are feasibility evidence. Production findings remain: cache lookup must use the native lock, every turn must validate source/route/platform against authority, and foreign runner instance shadows/double attachment must refuse. Strict session-info proof does not replace runtime admission checks. No full resource/worker/cache-hit acceptance follows.

Pi actual native UI script SHA256 `8be339e6aea977b90ed08d96518191089526680034686f5c45daeeb76940002b` and log SHA256 `3971bf69805e1383edb9f624323a578667efa8d1846ed861e067536941dd054c` were independently read/hash-verified. Task bakr6hddt exercises exported runtime/services, real InteractiveMode.init, actual reload, stop and dispose, with prefix-before-tail, fresh reload tail and two owned listener attachments. Accept native UI/API lifecycle feasibility only: read-only tool selection is not full resource proof, stop is an API call not a user quit command, and full argv/print/RPC/fork/resume/product wiring remain open.

## Remaining finite proof

Inspect the actual instance-bound carrier and native Pi extension/file path against the existing packet. Require exact source/runtime binding, complete prefix byte/ordering/once-only evidence, preserved base/resources, changed personal-tail behavior, worker/child exclusion and truthful resume behavior. Distinguish constructed payload from attempted SDK/network delivery. No new generic middleware, permission rounds or speculative proof campaign is requested.

The architect reviews resulting code/receipts; builder owns execution and integration. Overall SYS-PROMPT and NST remain open until their actual acceptance is satisfied. Update source plan before task-tree publication; do not convert preparation receipts into implementation PASS.
