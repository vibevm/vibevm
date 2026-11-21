# Shared native settings genesis — Linux first launch

## Actual cold-install discriminator

The clean six-adapter distribution at `8936a0cb2a5dc40f64f69a158f074909fdd534fe` was produced through the normal native distribution owner. The public bundle and installer were used to produce a new kit, run the actual native installer with synthetic HOME/settings and `--configure-ai --unattended --no-path`, then invoke the installed shim for keyring creation and connection add. This was not only a version/help probe. All six vendor-native directories remained absent; explicit Pi native setup refused without creating `.pi`.

Evidence: `/data/research/vibevm/cold-native-setup-8936-5qeonv2e/receipt.json`, SHA-256 `48cca8e82da85dfb19bbbce3a35a5efb51ab316c522b17d29ad4c30b31c96b97`, plus `steps.json`. `installed-payload-identity.json` uses public `self which vibe --json` to identify the installed payload and verifies release SHA-256 `15712d90c6e9b1fbcdf093f63ca942c0964ca39a8940a921d1563e93fb9c5f87`. The receipt's earlier `installed_cli_sha256` names the **shim**, SHA-256 `dc4cc8616f31a32138311067b5a96e3b56f62d12600e82639f80bd8cf9075d1c`, not the payload. These identities are distinct, not a mismatch.

The inspected installer delegates the installed `ai setup` operation rather than creating vendor-native settings directories. The native owner previously required every directory to exist. This confirms a native-target genesis gap rather than a reason to add a second installer creation mechanism.

## One fixed-path owner

`vibe-secrets::references::native_settings::target` observes and creates the fixed directory components for all six existing native writers:

| Agent | Native destination relative to injected HOME |
| --- | --- |
| Pi | `.pi/agent/models.json` |
| Qwen | `.qwen/settings.json` |
| Codex | `.codex/config.toml` |
| OpenCode | `.config/opencode/opencode.json` |
| Hermes | `.hermes/config.yaml` |
| Claude Code | `.claude/settings.json` |

No caller-defined arbitrary destination or six separate mkdir implementations are introduced. Preview walks held no-follow capabilities, validates existing protection and hashes directory identities plus the exact missing suffix. It creates no native directory/file or native lock. Missing-target preview supplies absent config/auth to the incumbent vendor validators.

Confirmed apply rechecks the operation/key/reference revisions and target witness under existing locks before value resolution, prepares validated native bytes, then re-observes the target. Missing components are created exclusively with mode0700 through existing SafeFS ownership primitives. A race winner is refused, not adopted or chmod-repaired. The final held directory identity is compared to the visible fixed path before publication; config/auth are rechecked. An absent native file is published no-replace with mode0600. Existing-directory writes retain their prior private snapshot/atomic replacement behavior and lock through publication.

Existing insecure/foreign directories, symlinks and observed substitutions refuse. If this invocation created an empty private ancestor before a later component creation fails, it remains for inspection; **no full rollback is claimed**, no arbitrary occupant is removed and no secret is published on the refused path. The final recheck is not a filesystem compare-and-swap against hostile same-user concurrent path mutation; this limitation is not hidden.

## Executed candidate gates

Evidence root: `/data/git/vibevm-keyring-storage-20261120T0100Z/`.

- `native-genesis-clean-build-r3/`: isolated full storage library **29/29 PASS**, complete public AI connections **21/21 PASS**, no ignored/filtered. Existing-directory native regressions remain green.
- New library coverage observes all six fixed paths without effects, creates/admit-checks their private components, checks stale directory witness and the **second** Pi component appearing after observation, preserves an occupant's `foreign` file and refuses a symlink without touching its target.
- Durable public first-launch case: preview and invalid confirmation leave `.pi` absent; confirmed apply creates `.pi`, `.pi/agent` at0700 and `models.json` at0600; repeated apply is an unchanged no-op.
- `native-genesis-public-controls-r1/receipt.json`: separate public CLI processes prove first-component and second-component appearance after preview refuse, occupant is preserved and no native secret file is published; an independent fresh HOME positive has the exact private modes and no-op.
- `native-genesis-public-pty-r1/receipt.json`: Escape, safe default Keep and explicit Apply, **3/3 PASS**; cancellation creates no native directories, terminal state restored.

Strict-r1 found one collapsible-if lint in the new final snapshot check. Its condition form was corrected without suppression or behavior changes. `native-genesis-clean-build-r4` recompiles the exact final subject and passes **29/29 library plus 21/21 public tests**. `native-genesis-public-controls-r2` and `native-genesis-public-pty-r2` requalify the final immutable CLI for both path-appearance refusals, private creation/no-op and all three attended journeys. `native-genesis-clean-strict-r2` completed all targets of the three affected crates with warnings denied, **exit 0**. The final immutable CLI SHA-256 is `349646b3a36ce00cc25b8f5b691e4e39145e44a9d5386cbd811a7e1702516591`. No candidate commit or rebuilt delivery is claimed.

## Not claimed

This is Linux with an existing private HOME. It does not implement Windows ACL admission, foreign-native runtime acceptance, arbitrary native roots, complete multi-agent onboarding or complete CORP/NST acceptance. The already delivered distribution at `8936a0cb2` does not contain this fix; a new accepted commit and native distribution build are required before delivery. The accepted clean release clone and local corporate kit were not modified by this task.
