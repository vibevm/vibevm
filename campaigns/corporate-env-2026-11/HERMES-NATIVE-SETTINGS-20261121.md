# Hermes 0.23.3 native YAML settings

## Fixed supported native operation

The explicit public native setup route admits `--agent hermes`, reusing the existing preview/confirmation/result owner. The fixed first-slice destination is `HOME/.hermes/config.yaml` under an existing protected Unix directory; native config/home overrides refuse rather than silently target a different file. The connection must explicitly use Chat Completions.

The sink selects native `model.provider = custom:vibevm`, `model.default`, and `providers.vibevm.base_url/api_key/api_mode`. It never exports a key, invokes a credential command, changes a pool, patches Hermes, adds a service/broker, injects credential environment or installs dependencies on the recipient. Existing selected-provider command/env/header overrides and ambiguous legacy custom routing refuse. Protected `auth.json` is read only for an opaque witness; selected-provider credentials and any nonempty credential pool currently refuse conservatively. Adjacent OAuth/providers are retained, not rewritten.

## YAML admission and preservation

The range editor admits a deliberately bounded subset: complete documents consisting of two-space block mappings, simple unquoted keys and validated single-line scalar forms. Every noncomment line is validated. It does not skip an opaque section by indentation heuristics. Duplicate keys, anchors, aliases, merge keys, directives/multidocument input, flow/sequence collections, multiline scalars, ambiguous indentation, excessive bounds and unsupported syntax refuse before key resolution. Unsupported input is not automatically repaired or replaced with JSON.

Only the selected scalar ranges or required mapping insertions change. Comments, inline comments and all other bytes remain intact; semantic equality of selected scalars gives a byte-identical no-op. Proposed interpolation-bearing literal values refuse instead of credential substitution. Vibe's inventory/store/native locks, revision/target/auth/content witnesses and private atomic publication are reused; the native lock is not a compare-and-swap against independent editors.

## Native runtime prerequisite and separate archive correction

The first exact 0.23.3 runtime attempt failed before provider requests: gateway child Python lacked the already-packaged `ruamel` dependency. The parent launch inserted packaged site into its own `sys.path`, but the native child did not inherit that path. This was not a key/API configuration failure.

A separate staging payload uses Hermes's supported sealed manifest/dependency activation layout. The next retained attempt failed earlier because its launcher escaped the sealed root; containment was not weakened. The corrected contained staging payload starts the native gateway and produces the synthetic reply without modifying upstream Python source, global Python/PATH or system packages.

The corrected Linux x64 CPython 3.14 archive is independently packaged and verified from a new extraction: 191,294,120 bytes, SHA-256 `06d45b6950855ccf9eda0f77f89a51b34f16d7965145023ba1585bd23a761ee1`. Catalog source/placement text is separately committed as `b1f99614ecf53b35fa24d9999a972685a9d1f759`; no archive is in Git. This native-runtime check is not the owner-excluded installation trial. Foreign native producers remain blocked on qualified native isolation, not falsely qualified by archive staging.

## Executed evidence

Evidence root: `/data/git/vibevm-keyring-storage-20261120T0100Z/`.

- `hermes0233-native-config-r1`: retained child import RED. `r2`: retained containment refusal. `r3`: native exit 0 and correct chat request; fixture aggregate RED was an overbroad assertion requiring model on `/api/show` probes, not a chat failure.
- `hermes0233-public-native-r1/receipt.json`: public preview/apply/no-op followed by direct native Hermes from the corrected archive's read-only extraction. Exit 0; `/v1/chat/completions` matches model and credential. Four separate `/api/show` probes match credential and are not claimed to contain the chat model. YAML comments/neighbor provider and adjacent OAuth are preserved by the writer; ambiguous YAML, command and pool-shadow controls refuse unchanged, no command marker appears. No provider/key/model arguments or credential environment; no target pip or installation trial. Architect independently accepted the behavior.
- Public/native CLI snapshot SHA-256 `b4b7b7434f2b048b7282685e902ed6242d4516772308d7d94de5553c57bfad73`; receipt SHA-256 `9d9c71ea134130d9a3173245539c7697c3150dd3d71ab76b83abaa686318751b`.
- `hermes-native-clean-build-r1/vibe_secrets-runtime.log`: complete library **26/26 PASS**, no ignored/filtered.
- `hermes-native-clean-final-build-r2/runtime.log`: complete public AI connections target **19/19 PASS**, no ignored/filtered.
- `hermes-native-public-pty-r1/receipt.json`: Escape, default Keep and explicit Apply **3/3 PASS**, terminal state restored, no provider requests or secret output.

Self-review found a semantic no-op edge: unquoted YAML `true` is a boolean, not the selected string `"true"`. The writer now quotes implicit scalar spellings; repeated application remains byte-identical. `hermes0233-public-native-r2` verifies this through the exact payload's native `hermes_yaml` reader and a direct native chat request with model string `true`; exit 0. `hermes-native-clean-final-build-r3` reruns **26/26 library and 19/19 public PASS** on the corrected subject. Architect independently closed the type finding. Final strict lint for these latest bytes and exact source/blob acceptance remain pending; earlier strict-r1 belongs to the earlier subject. An additional exact native ruamel check confirmed equivalent implicit keys `true`/`True` are duplicates. Admission now permits only ordinary string-key spellings and refuses implicit-type keys before writing, without expanding the grammar; the model string `"true"` remains a supported value. `hermes-native-clean-final-build-r4` passes **26/26 library and 19/19 public tests**, and `hermes0233-public-native-r3` passes the final public/native scenario including unchanged refusal for equivalent implicit keys and native string-model verification. `hermes-native-clean-strict-r3` completed all targets of the three affected crates with warnings denied, **exit 0**. Final public/native CLI SHA-256 is `63b7bd5dc688f62bd80b9bec9245bc50c5ece781e095e82c1e84680d3cf155e4`; native receipt SHA-256 is `8798941f7f7e4c06667c9d178230c6117bbd0b38108ea060f87d492092c13a9f`. The YAML-adapter sources are not committed yet.

## Remaining

Fresh-directory genesis, larger YAML syntax coverage, alternate native scopes, full onboarding/inspect UI and Windows/macOS native security/runtime qualification remain separate. Conservative pool/unsupported-form refusal is not full native coexistence acceptance. Claude Code is the next separate adapter, with native supported auth only and explicit OpenAI incompatibility. Aggregate CORP/NST and final rebuilt Vibe bundle delivery are not established by this slice.
