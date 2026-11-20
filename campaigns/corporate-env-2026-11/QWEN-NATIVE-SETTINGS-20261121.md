# Qwen 0.28.0 native settings candidate

## Concrete boundary

The existing explicit public native setup route admits `--agent qwen`, using the same confirmation/result owner as Pi. The fixed native destination is `HOME/.qwen/settings.json`. Explicit native home/system-config override variables are refused instead of being silently ignored. The protected transaction reuses only the existing Pi/Qwen operation witness, inventory/store/native lock order, revision/target checks and atomic private publication. Vendor-specific merge semantics remain separate; there is no public secret getter, arbitrary destination, callback, service, plugin, broker or alternate transport.

Qwen's actual pinned settings schema supports `security.auth.selectedType`, `apiKey`, `baseUrl` and `model.name`/`baseUrl`. The adapter writes the native `openai` selection. Nonempty `modelProviders`, incompatible enforced authentication and external auth are refused rather than silently overriding a stronger route. Native OAuth files are not modified. Unknown fields and comments outside selected value ranges remain byte-preserved.

The bounded JSONC helper only validates syntax and locates/patches object-property ranges. It never resolves a keyring secret. Duplicate keys, including equivalent Unicode-escaped names, unsupported syntax, invalid object shapes, excessive size/depth/nodes and ambiguous targets refuse. The early native operator-policy reader rejects trailing commas even though other settings code accepts comments: Qwen therefore refuses that native-unsupported subject before key resolution, preserving its original bytes instead of normalizing it. The general helper's ability to locate trailing commas is not a claim that Qwen accepts them.

Qwen's native environment interpolation has no admitted literal escape for `$VAR`/`${VAR}`. Interpolation-bearing values refuse rather than changing the key, injecting environment or claiming arbitrary-literal support. Actual standalone request evidence is required in addition to the source/schema observations.

## Retained execution evidence

Evidence root: `/data/git/vibevm-keyring-storage-20261120T0100Z/`.

- `qwen-jsonc-build-r1/runtime.log`: first focused editor test passed; editor-only, not native acceptance.
- `qwen-native-sink-build-r1/runtime.log`: 22/22 library tests passed, including the existing Pi regression and initial Qwen merge tests. This predates the final native trailing-comma refusal subject.
- `qwen028-public-native-e2e-r1`/`r2`: retained native exit 52, no requests. Captured native error identifies unsupported trailing commas. No false native PASS.
- `qwen028-public-native-e2e-r3`: native exit 0, successful synthetic response and correct synthetic authorization; the separate post-native byte-equality assertion failed. This is not a false request failure.
- `qwen028-public-native-e2e-r4`: actual public CLI preview/apply/no-op, direct unmodified official Qwen 0.28.0, successful synthetic loopback requests, comments/unknown fields/OAuth preservation and all three unchanged-file refusals (trailing comma, shadowing, interpolation). Independent equivalent native HOME never configured by Vibe reproduces the same post-native data. The original post-Vibe snapshot is restored before negative controls. Architect independently reviewed this control. Snapshot files contain only synthetic credentials but remain outside Git.
- `pi160-public-native-after-qwen-r1`: actual direct native Pi regression on the Qwen-capable CLI passed. Final qualification must bind both agents to the exact same executable SHA-256, not merely a mutable `target/debug/vibe` path.
- Strict r1/r2 failures are retained: local collapsible-if/needless-borrow issues and then CLI too-many-arguments. Fixed without suppressions by local code cleanup and grouping native destination/override policy.

Final clean test compilation `qwen-native-clean-final-build-r3` succeeded. Its isolated runtime records **22/22 library PASS** and **2/2 durable public Pi/Qwen CLI regression PASS, 14 filtered**. `pi-qwen-final-native-pair-r1/receipt.json` binds both actual native scenarios to the same immutable clean CLI snapshot SHA-256 `64e59f1525e6d6e0d6cb2869c81eff35b8c2928186066cf89e078dd73ba01a60`; both exit 0. The final independent Qwen control identifies the native-added field path as `/$version`, with comments and neighbors retained. Final strict `qwen-native-clean-strict-r3` completed all targets of the three affected crates with warnings denied, **exit 0**. No Qwen candidate commit or aggregate acceptance is claimed.

## Remaining scope

Existing protected Unix native directories only; fresh-directory genesis, Windows ACL/native qualification, alternate selected model-provider routes and the other four native adapters remain separate unfinished work. Vibe's lock coordinates Vibe writers, not arbitrary native editors: the last snapshot check before atomic rename is not a compare-and-swap against an independent concurrent writer. No credential/provider substitution or external publication occurred.
