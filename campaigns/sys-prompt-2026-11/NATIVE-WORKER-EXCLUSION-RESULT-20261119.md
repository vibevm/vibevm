# Actual native worker exclusion — bounded Hermes/Pi result

2026-11-19; base `d83a1e5ada444ba497b4d058d6004eaf735e17cd`. One native main→one child journey each; not all worker modes/platforms or SYS closure.

## Hermes actual delegate tool

Actual Vibe query enters the accepted native supervisor/owned main carrier. Synthetic loopback transport returns a real `delegate_task` tool call; pinned Hermes tool dispatch creates/runs the child via `tools/delegate_tool.py` and `_ChildRun`, then sends the valid child response back in the parent's tool-result transcript. The child makes one captured streaming request with its task and without main-prefix; parent requests carry main-prefix. Parent finishes normally with synthetic completion. This is actual tool execution, not direct child factory construction or the earlier environment-filter test.

Native source `4ae451d4126327c871fd084e60c045dec0c85911`; delegate_tool SHA256 `8a11dcfbf92af95cdd6df259913c6607c7670ad04d8e33b332c1fc0785bd8c0e`; child_run `0d8cbb6c607dbab6bd5c13afd9c78dafd8956cb2a6424db979c21c326c095c54`.

Private retained `control/hermes-native-worker-deps.log`: PASS; binary `bcf981fbb27087785f49ad54874933ea277e1d4007db417e6e5c36ca595c93c7`; childRequests1, mainPrefixPresent, childPrefixAbsent, validChildResponseReturned. First `hermes-native-worker.log` and diagnostic retry were RED: missing native snowballstemmer, repeated native errors. Upstream pyproject/uv.lock pin3.1.1 wheel SHA256 `7e207fa178741da09cdee59d3ecec3827ad5f92b1fc5c9ff3755b639f71f5752` was downloaded into a new private input directory, verified, extracted with path/type checks and mounted read-only as a private site. No host dependency installation, credential or provider substitution. Hermes was not repeated for Pi repairs.

## Pi explicitly loaded shipped example, not builtin/default

Published `@earendil-works/pi-coding-agent@1.5.2`, Node24.18.0; explicit native example `examples/extensions/subagent/index.ts` SHA256 `3b738fd10c1aaadfb5f96d0c683481faad916cd88544677efd7280eb38f425fe`; agents.ts `cc85ecd33b89aa575bea3d2a9a9da7654a6305da4de02e5041aa5342a1de29b4`. This proves that explicitly loaded pinned example only, not a default native subagent feature and not equivalence with the separate repository pin.

Actual Vibe main loads the example; synthetic native provider event stream returns a real `subagent` tool call. Native example discovers a synthetic user agent and spawns native plain Pi `--mode json -p --no-session`. Child runs in a distinct PID, produces one valid captured request with its own agent identity and no main-prefix. Its valid response returns through actual subagent tool result into the parent's next request; parent completes normally.

This exposed a product executable-identity defect: example `getPiInvocation` uses existing `process.argv[1]`, which was the Vibe-private adapter. Child then invoked that adapter with native argv and failed opening `--mode/package.json`. Product fix decodes private bootstrap arguments once, then sets argv to selected installed package `dist/cli.js` plus exact caller args before native imports/resources. process.execPath remains native runtime; no native method/class/config override and no private carrier/prefix args enter child invocation. Native config.js and package-manager-cli also use argv[1] to locate their package; inspected within the same runtime.

`control/pi-native-worker-identity-fixed.log` PASS on binary `62fc96af2b5f9b4b4c6ec54d0151299c27e0d89a6aa8ae88bf5cb73bbfe6b5da`; final fixture additionally asserts realpath(argv[1]) is actual admitted cli.js and no private bootstrap/content-addressed carrier/prefix argument is present. Final logs/gates retained as `pi-native-worker-final.log`, `native-worker-cli-gates.log`, `native-worker-conform.log`.

Pi failed fixtures remain: initial provider `.mjs` not auto-discovered (corrected to native `.js`); context.systemPrompt was undefined because actual native transcript uses system messages/sections (oracle corrected); then actual child tool result showed the product argv bug. None is relabelled green.

## Isolation, identity and limits

All native execution is under existing bwrap unshare-all/process/fs/net isolation with allowlisted read-only native runtimes/control/binary, synthetic writable HOME/settings/project and no host auth/config/socket mounts. Hermes uses Python `-I`; Pi uses selected Node and native extension loader. Synthetic provider boundaries only, no real provider/cost/cache measurements. Parent/child prefix isolation is not separate child filesystem sandboxing: native child remains inside the same outer sandbox and may see its synthetic files. No claim that marker absence proves universal privacy or that fixture tool identities are an authorization mechanism.

Private exact runtime/factory/fixture hashes: `/data/git/vibevm-native-carrier-20261119/NATIVE-WORKER-INPUTS.json`. No fixture scripts, downloads, credentials or raw captures are shipped as product. Product delta is only Pi argv identity; focused regression/conformance and independent exact review required before acceptance. Existing oneshot-bootstrap/classic-TUI/shared-gateway/platform/parser limitations remain open.
