# Default AI connection selection — metadata-only candidate

Baseline first consumer `c8734a9ef`; selection contracts integrated `10d5a5d5f` and `4ced522e1`. CORP-FMT-053 records minimum epoch before activation. No harness, provider, value resolver or network capability.

Existing reference index is the sole atomic selection owner. AI index schema1 remains read/write-compatible when no selection is present. First real set upgrades to schema2 with a strict required tagged state: none or selected existing connection. Clear retains schema2/explicit none. Read, clear-none and ordinary writes do not silently upgrade v1. Unknown/missing selection and dangling selected ID refuse; older strict clients refuse v2 rather than discard it. Generic reader understands selection only as reference metadata, never AI payload.

Set/clear uses inventory then store lock, validates expected AI store revision before no-op decisions and publishes once. Same-ID set and clear-none do not write/increment. Unknown ID and stale revision refuse unchanged. Selected connection unbinding refuses until explicit clear; no alternate connection is chosen. Other connections, keyring references and secret bytes remain intact. Default show validates metadata and returns selection from one snapshot, not two independently loaded observations.

Public CLI adds only `ai connections default show|set ID --expect-store N|clear --expect-store N --yes`. A missing default is metadata absence, not native/managed harness state. No agent is opted in by this operation.

## Evidence

Private root `/data/git/vibevm-keyring-storage-20261120T0100Z`. `default-compat-r1.log` verifies exact retained pre-change497621fbee8665ecb25551093b7c44f69d4619671abc5159894ab4480c6dc2f7 binary refuses schema1 unknown default field and schema2 on list/add, preserving index; restored schema1 control succeeds. Architect independently exercised list/add/remove before approving epoch2.

`default-selection-tests-r2.log`4 public tests PASS includes two connections A→B, fresh-process show, stale/no-op, clear-none v1 byte preservation, selected connection removal refusal, missing consent, explicit clear then unbind, other connection/key intact. Final tests additionally require missing/unknown/dangling v2 selection to refuse without rewrite; their final log/manifest is the acceptance binding.

`default-emitted-compat-r3.log` uses the actual newly emitted schema2 index after public set. Pre-change client list/add/remove all exit1 with both AI/key indexes unchanged; new clear keeps v2 explicit none. No artificial epoch edit substitutes for this gate. Initial strict lint found a collapsible if and was corrected without suppression; final core/connections lint and CLI lint are separate gate logs.

No real credentials were read, no provider/model request occurred, and no Windows DACL claim exists. Final source/binary manifest and independent acceptance remain required. Agent bindings, native delivery, setup/onboarding and distribution remain separate; this atom does not complete CORP-AI-CONNECTIONS or NST.
