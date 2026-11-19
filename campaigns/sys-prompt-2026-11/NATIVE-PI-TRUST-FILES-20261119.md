# Pi native trust, built-ins and file inputs — bounded follow-up

2026-11-19; base `7050eb66b`. This follow-up does not close native Pi/Hermes parity.

## Implemented boundary

`crates/vibe-cli/src/commands/agent/native/pi_entry.mjs` now supplies Pi's actual `builtInExtensions` alongside the Vibe inline carrier. The previous request captures established prompt/base/tools placement, NOT complete built-in extension/MCP resource parity. This restores a missing native startup input; it is not a retrospective resource-proof claim.

Native `resolveProjectTrusted`, `createProjectTrustContext`, `ProjectTrustStore`, runtime `SettingsManager` and `resourceLoaderReloadOptions` own project trust. Initial resources stay untrusted until the native resolver decides. Native explicit `--approve`/`--no-approve` are passed as user choices, not invented defaults. Native startup UI handles interactive consent; noninteractive absence of consent excludes project resources. Same-cwd runtime replacements use native trust context when supplied. Cross-cwd replacement still refuses pending selected-project boot re-admission. No custom trust UI, automatic grant or persistent Vibe trust database.

Native `processFileArguments(..., {autoResizeImages:false})` feeds native `buildInitialMessage`; file input is no longer a temporary refused family. Native SDK retains request-model image resizing. Provider-without-model now refuses as native main does. Native extension-load errors also refuse before session creation rather than continuing with an incomplete resource set.

## Actual finite proof

Final task `bx5j7k568` exit0, exact entry asset SHA256 `3b95d60c1b5227f6d63aafb28ba2639b5144787db3fbaa8056ec7e3d82c3e199`. Actual published Pi1.5.2, Node24.18.0, fresh credential-free HOME and unshare-all network-isolated bwrap subjects; runtime/candidate read-only. No host auth/config copied, provider requests or real credentials.

- Untrusted project, non-TTY: project extension excluded.
- Explicit approve: project extension loaded.
- Explicit deny: project extension excluded.
- Native remembered trust: project extension loaded.
- Native `@input.txt` plus user prompt reached the real native request context with the Vibe prefix. Supported synthetic provider extension captures locally and intentionally throws `OFFLINE_FILE_CAPTURE`; proof independently checks capture contents, not just exit status.

Private root `/data/git/vibevm-native-carrier-20261119/control/`: `pi-trust-resource-proof.mjs`, `pi-file-input-proof.mjs`, `pi-trust-files-final.log`. Final log SHA256 `3ca23c4f6bd0fefb57276baf7d35de76087ea5e0d10eb67281606b24e7145c61`. `git diff --check` passed.

First trust fixture used `.mjs` under project extensions; native directory discovery supports `.js`/`.ts`, so approve/remembered assertions failed. The failed log remains `pi-trust-resource-proof.log`; corrected `.js` fixture passes. This was a fixture discovery defect, not evidence that project trust had passed earlier.

## Remaining / not claimed

Native interactive trust selection and real command-loop exit remain unrun. Built-in extension input restoration is source-bound, not a full six-agent resource/worker/MCP matrix. `resume`, `fork`, scoped `models`, replacing `systemPrompt` remain explicit pending families. Saved/default model restoration needs its finite fixture; native resolver without cliModel returns undefined, so its unconditional call was NOT a confirmed override defect. Native main requires an explicitly resolved session-option model before applying `--api-key`; scoped-model mapping remains pending separately. No second model-state store introduced.

Existing-history precompaction, native replacement/worker admission, full parser/subcommands and Hermes executable bootstrap remain open. Rust embedding gate `bx7rwmm69` subsequently passed fmt and all3 native publisher/runtime tests, compiling the changed asset into the test binary. Log: private `control/pi-trust-files-embedding-gate.log`. This proves embedding compatibility, not rebuilt production Vibe dispatch: native behavioral proof directly imports the current embedded-asset source in the selected runtime.
