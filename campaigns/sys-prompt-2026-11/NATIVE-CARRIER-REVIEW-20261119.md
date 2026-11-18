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

## Remaining finite proof

Inspect the actual instance-bound carrier and native Pi extension/file path against the existing packet. Require exact source/runtime binding, complete prefix byte/ordering/once-only evidence, preserved base/resources, changed personal-tail behavior, worker/child exclusion and truthful resume behavior. Distinguish constructed payload from attempted SDK/network delivery. No new generic middleware, permission rounds or speculative proof campaign is requested.

The architect reviews resulting code/receipts; builder owns execution and integration. Overall SYS-PROMPT and NST remain open until their actual acceptance is satisfied. Update source plan before task-tree publication; do not convert preparation receipts into implementation PASS.
