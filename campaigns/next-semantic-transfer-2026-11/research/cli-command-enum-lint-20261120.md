# CLI command representation: remove the broad strict lint blocker

Candidate baseline `def5eca0f18af5a223797cfd3565e58d820d05d2`. Architect-assigned bounded repair, not USER-PROJECT or platform acceptance.

## Reproduced first owner and minimal repair

Strict `cargo clippy -p vibe-cli --bin vibe --no-deps -- -D warnings` on the accepted source reproduced `clippy::large_enum_variant` at `cli/command.rs:Command`: Install carried at least880 bytes, next-largest Init at least520. Exact baseline log retained as `cli-enum-strict-baseline-r1.log` under `/data/git/vibevm-keyring-storage-20261120T0100Z/`. No other strict defect was reported.

Change only `Command::Install(InstallArgs)` to `Command::Install(Box<InstallArgs>)`, with explicit owned dereference at the actual main dispatch and the existing global-MCP unit harness dispatch. Clap's existing `CleanChain::Install(Box<InstallArgs>)` already uses this representation. Borrowed routing/progress/cancellation consumers retain field access through normal dereference. InstallArgs, flags, defaults, JSON report schemas, operation owners and serialization are unchanged. No suppression, new parser, extra command boxing or public protocol change.

Corrected strict **bin lint PASS without any category exception**, log `cli-enum-strict-candidate-r1.log`. Earlier member-set review's category-except results remain honestly historical; they are not retroactively strict PASS.

## Verification status

Actual accepted-binary root and install help were captured under isolated namespaces before the edit; rebuilt root and install help are byte-identical, both exit0/empty stderr (`cli-enum-help-comparison-r1.json`). Existing grammar units14/14, application grammar7/7, global-MCP install dispatch17/17 and full actual public common-package/install target125/125 passed on rebuilt artifacts. Complete existing synthetic `cli_app_install` passed7/7, including shell and zip/tar install/public launch/repeat/update/uninstall controls. No supplier binary or real account was used.

The initial app-install run used the common-package runner, which did not mount sibling `vibe-app-proxy`:1PASS/6RED, all six with actual `proxy_image_missing`. Retained `cli-enum-app-install-r1.log` is not erased or called PASS. The same fixture/CLI artifact repeated with the existing explicit-artifact proxy runner and required read-only proxy mount passed7/7 (`cli-enum-app-install-r2.log`), without product or test-oracle changes. Runtime uses filesystem/PID/network namespaces, cleared environment, synthetic HOME/settings and read-only system/artifact mounts, not host auth/config/session sockets.

Affected formatting/diff checks passed. Exact four-file source/artifact/evidence identities are sealed in `cli-enum-candidate-r1.json` under the private evidence root; baseline/candidate lint logs and all grammar/dispatch/common/app runtime logs remain alongside it. This is strict CLI-bin lint closure and scoped regression, not a full host/platform gate or independent acceptance by itself.

No runner/credential/toolchain/global configuration changes, provider calls, publication, parent-state promotion, REMAINDER or IDE work. USER-PROJECT remains RUNNER-BLOCKED/UNRUN. This repair is not NST completion.
