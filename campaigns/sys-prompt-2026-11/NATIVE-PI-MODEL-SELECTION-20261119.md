# Pi scoped/default/saved model selection

2026-11-19; base `ff9a52e00`. Bounded follow-up, not full native parity.

The embedded entrypoint now maps explicit `--models` or native enabled-model settings through native `resolveModelScope`, then supplies those models to the native session factory. Fresh sessions select the configured default when in scope, otherwise the first scoped model. Existing session history prevents this fresh-session fallback; native SDK owns model restoration. Explicit model resolution errors remain fatal; no invalid CLI-model fallback or separate Vibe model-state store. Explicit thinking keeps precedence over scoped thinking.

Actual native request proof `b20idwsfx` exit0: first scoped model=`one`; default-in-scope=`two`; default-only=`two`; explicit model=`one`; saved-session model with real native persisted history=`two` despite scope starting with `one`. Synthetic provider extension captures the chosen model and intentionally throws; each independent capture is checked. Fixtures use two synthetic models and native SessionManager APIs; no real provider/key, no network, fresh tmpfs HOME per arm, read-only runtime/candidate under unshare-all bwrap. Runtime published Pi1.5.2/Node24.18.0. Direct asset import, not rebuilt production Vibe dispatch.

Asset SHA256 `0fc620a016abd57e88093ebc5079cf72188e9e1744568c0af8d910b83e358e1c`. Private evidence root `/data/git/vibevm-native-carrier-20261119/control/`: `pi-scoped-model-proof.mjs`, `pi-scoped-saved-model-proof.log` SHA256 `d8e7abb4e38e1c885809cab1f7655454c5fe6c0fed70ab514f2fc927dcb0bea7`. Initial four-arm proof also passed; later fifth arm adds native persisted-history restoration. `git diff --check` passes.

This does not prove thinking-level cycling, malformed model-pattern diagnostics, native interactive session picker, fork, worker or precompaction admission. Temporary refused parser families remain resume/fork/replacing-systemPrompt. Same-cwd native replacement/resource lifecycle and cross-cwd boot re-admission remain separate obligations; Hermes executable bootstrap still open.
