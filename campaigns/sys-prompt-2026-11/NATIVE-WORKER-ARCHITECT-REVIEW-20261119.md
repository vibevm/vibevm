# Native worker exclusion and Pi executable identity — bounded acceptance

2026-11-19. Base `d83a1e5ada444ba497b4d058d6004eaf735e17cd`.

Accepted manifest `/data/git/vibevm-native-carrier-20261119/NATIVE-WORKER-CANDIDATE.json`, SHA256 `a9d7f44cb49666806d7cce2cfdb01c5c5521756e6f1a26b8410e3899e69e320e`. Independently verified digest and both current file hashes: Pi entry and producer result. Commit binding/publication are separate follow-up steps.

## Product review

Actual Pi example child execution exposed an executable-identity defect: the child reused process.argv[1], previously pointing to Vibe's private adapter with an incompatible argument grammar. The three-line repair sets native argv to the selected installed package's dist/cli.js plus exact caller args after package admission and before native dynamic imports. Private carrier/prefix/bootstrap arguments do not become the native identity. The main carrier remains instance-owned; native child uses the ordinary native entry. No native method/class patch, alternate loop or adapter refactor.

## Evidence inspected

- Hermes actual tool journey uses delegate_task and native child owner; synthetic transport supplies the tool call, and valid child response returns in the parent's tool transcript. `control/hermes-native-worker-deps.log` binds binary `bcf981fbb27087785f49ad54874933ea277e1d4007db417e6e5c36ca595c93c7`: main prefix present, child prefix absent, one captured child request. Fixture assertions inspected. Missing-dependency attempts remain RED; the exact pinned wheel/private read-only site is not a host install.
- Pi explicitly loads the published 1.5.2 shipped subagent example, not a builtin/default worker feature. Fixture/provider assertions check realpath native cli.js, absence of private argv, distinct child PID/identity, one child request, parent prefix exclusion and valid child response returned through the actual tool result. `control/pi-native-worker-final.log` binds binary `62fc96af2b5f9b4b4c6ec54d0151299c27e0d89a6aa8ae88bf5cb73bbfe6b5da`. Failed fixture and actual product-bug attempts remain RED.
- Native runtime/example/factory/fixture identities are retained in private NATIVE-WORKER-INPUTS.json. Inspected Pi sandbox launcher: unshare-all, clear environment, synthetic HOME, read-only runtime/control/binary mounts and no host home/auth/socket mount.
- Focused CLI 16 PASS and conform 0 new / 141 existing logs inspected. Architect inspected peer sources/evidence; did not build or write in peer checkout.

## Limits and next boundary

Accept these actual parent-to-child prefix-exclusion journeys and the Pi identity repair only. Children remain within the same outer sandbox; this is not separate child filesystem isolation, universal privacy, all worker modes/platforms or measured cache savings. Synthetic provider usage does not establish live provider behavior. Existing oneshot/classic-TUI/shared-gateway/platform gaps remain open.

Builder instructed to commit exactly the two accepted paths and send OID; update authoritative source plan before dashboard. Next is the finite six-harness parity audit using existing capability table and accepted receipts. Consolidate remaining native prerequisites into one owner-facing decision set rather than repeating unsupported waivers or research loops. Full SYS-PROMPT/CORP-ENV/NST completion is not implied.
