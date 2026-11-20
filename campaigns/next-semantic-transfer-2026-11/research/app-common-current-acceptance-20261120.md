# Existing application acceptance gates — current-product rerun

Product/evidence pin `cfc891a801bcbadff249a635283af51b29ce71b6`. This executes the existing PROP-069#ACCEPTANCE application compatibility baseline after bounded ordinary configuration deliveries. No positive test, product, schema, expected output or acceptance criterion changed.

Build-only emitted exact integration artifacts; existing explicit-artifact bwrap runner executed them with filesystem/PID/network namespaces, cleared environment, synthetic HOME/settings and read-only CLI/proxy/system mounts. Synthetic wizard acquisition uses isolated loopback only; no provider/model, real credential or host configuration is available. Tests signal only their own spawned Vibe/recipe processes.

| Existing target | Actual result | Scope |
|---|---|---|
| `cli_app_common_journey` | 7/7, zero filtered | Actual describe/plan/install/modify/update/rollback/force repair preserves original typed paths and dormant choice input; separate unanswered/invalid/secrecy boundary and automatic-input-removal review controls |
| `cli_app_wizard` | 1/1, zero filtered | Existing tool-driven wizard describe/plan/install/events/run diagnostics; actual SIGTERM interrupts selected step without installation success; retained work records interrupted and later resume succeeds |
| `cli_app_interrupt` | 1/1, zero filtered | Actual SIGINT reaches recipe and waited-on foreground child; owned process group does not survive |

Architect independently reproduced all three exact target results. The first target's dormant maintenance evidence and the other targets' cancellation evidence remain **separate scenarios**. No claim is made that interrupt plus dormant common intent was proved in one combined operation. Architect explicitly determined no additional Cartesian-combination test is needed for this bounded baseline gate.

Affected strict fixture lint passed. Source was unchanged, so no new product rebuild or full host/platform gate is inferred. Exact artifact/source/evidence hashes are in `app-common-current-candidate-r1.json`. Private root `/data/git/vibevm-keyring-storage-20261120T0100Z/`: `app-common-current-build-r1.log`, `app-common-current-runtime-r1.log`, `app-common-current-cancel-build-r1.log`, separate `app-common-current-cli_app_wizard-runtime-r1.log` and `app-common-current-cli_app_interrupt-runtime-r1.log`, and `app-common-current-clippy-r1.log`.

This is a current rerun of accepted application owners, not reopening their implementation or declaring all configuration complete. Ordinary richer semantics, applicable source/history/platform/CORP obligations, mandatory blind real-agent USER-PROJECT adoption, later consolidation and ordered final repair remain separate. UP-ADOPTION stays RUNNER-BLOCKED/UNRUN: no isolated authenticated runner grant appeared. No parent-state promotion, external push/publication, credential change, REMAINDER or IDE work.
