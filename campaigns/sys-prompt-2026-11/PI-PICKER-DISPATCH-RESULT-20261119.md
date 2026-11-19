# Pi native picker/cwd/command admission — bounded candidate

2026-11-19; base `5c90cee41d4cf341b3a568413a27207b1bbf9137`. Existing H4 repair, not SYS/NST closure.

## Concrete defects / existing owners

Published Pi1.5.2 native main selects its SessionManager, then starts cwd-bound runtime services at `sessionManager.getCwd()`. Vibe's adapter previously supplied original admitted cwd without checking the selected manager's cwd. A picker/explicit session could therefore load wrong-project resources with another history. Immediately after native `createSessionManager`, the adapter now resolves both paths and uses native `utils/paths.pathsEqual` identity (Windows case comparison retained) before services, resource loading, carrier attachment or provider. Mismatch refuses with relaunch/re-admission remedy; missing paths refuse as well. Saved cwd is never rewritten. Full cross-project pre-prefix selection is not implemented by this guard.

Native auth/package/config/MCP commands dispatch before native chat parse. Adapter now returns to native main for exact first-argument families `auth`, `config`, `mcp`, `install`, `remove`, `uninstall`, `update`, `list`, copied from the admitted main/package-parser grammar rather than an independent command implementation. `uninstall` is native remove alias. Options/delimiter not in first position do not become command-family dispatch; native parse owns literal prompts. Main argv identity repair from a5bb remains intact. No new auth/config/package implementation, installed-runtime modification, boot solver or provider loop.

## Native qualification

Private retained root `/data/git/vibevm-native-carrier-20261119/control/`:

- `pi-h4-native-stdin-fixed.log`: actual Vibe → native PTY `--resume` picker with same-project saved history, select real entry, native provider request includes prior answer + new user text + admitted project prefix; valid response, `/quit`, normal exit. Foreign-cwd explicit saved session refuses before test extension's top-level resource sentinel and before any provider capture. Native config/list/auth/MCP `--help` succeed with no provider resource loading. Published Pi1.5.2 + existing Node24.18.0, not research-pin equivalence.
- `pi-h4-native-final.log`: same finite fixture adds `-- list` literal-prompt control (normal native provider response, not package command), then resets only fixture-owned sentinels before foreign negative. Binary identity is recorded by the driver.
- First `pi-h4-native.log` RED timed out waiting for headless inherited stdin EOF; corrected fixture uses DEVNULL. This is not a successful cwd refusal and remains retained.
- `pi-h4-cli-after-oom.log`:16 focused launcher tests PASS. `pi-h4-conform-after-oom.log`:0 new/141 existing; format/diff checks PASS. Native assets are embedded JS, no wire/schema/codegen changes.

Each native run uses existing unshare-all bwrap isolation, read-only allowlisted Pi runtime/Node/control/binary, synthetic writable HOME/settings/project and Python `-I` supervisor. Synthetic native event stream only; no real provider, credentials, host auth/config mutation or external package operation. Informational native command proof does not qualify destructive/auth mutation behavior. No paid/cache/fullsize/worker rerun.

## Platform admission inventory / limits

- Common program owner resolves explicit paths and Windows PATH/PATHEXT, preserves `.cmd/.bat` argument escaping through the standard process owner and refuses inferred PowerShell execution. Existing injected-path unit tests are not Windows native runtime proof.
- Pi admission requires selected package name/version1.5.2 and selected Node installation layout or explicit env-node launcher delegation. Actual Linux npm-style native entry qualified here; Windows `.cmd`/macOS/Bun standalone and other layouts are not newly qualified.
- Hermes admission remains direct absolute interpreter shebang plus exact native module hashes; shell wrappers/Windows launchers are not admitted by that owner. This inventory does not invent support or install another runtime.
- Same-project picker is now actual native evidence, not inferred from saved-file selectors. Cross-cwd runtime replacement remains safe relaunch refusal, not full support. Informational/package/native commands retain upstream first-argument grammar; full native command outcomes and all parser combinations are not claimed tested.

## OOM recovery / gate status

Owner reported OOM termination during integration baseline. Retained self-check log shows step1 fmt PASS, interruption at step2 workspace tests with no terminal result; it is **INTERRUPTED**, neither product FAIL nor green. No cargo/rustc remained on resumption. `/fast/git/vibevm` resolved to the same canonical `/data/git/vibevm` checkout; dirty work preserved. Focused build/tests resumed with process-local CARGO_BUILD_JOBS=2, existing target and incremental unchanged; no global config/toolchain/cleanup/process termination changes. Full integration gate remains owed before final closure.

H1 unattended Hermes bootstrap, H2 classic/TUI handoff and H3 shared gateway remain unresolved native-owner boundaries. This candidate repairs the two demonstrated Pi defects and qualifies a finite picker/command/platform surface only.
