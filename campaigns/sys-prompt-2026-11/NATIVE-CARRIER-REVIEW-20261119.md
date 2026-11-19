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

## Remaining finite proof

Inspect the actual instance-bound carrier and native Pi extension/file path against the existing packet. Require exact source/runtime binding, complete prefix byte/ordering/once-only evidence, preserved base/resources, changed personal-tail behavior, worker/child exclusion and truthful resume behavior. Distinguish constructed payload from attempted SDK/network delivery. No new generic middleware, permission rounds or speculative proof campaign is requested.

The architect reviews resulting code/receipts; builder owns execution and integration. Overall SYS-PROMPT and NST remain open until their actual acceptance is satisfied. Update source plan before task-tree publication; do not convert preparation receipts into implementation PASS.
