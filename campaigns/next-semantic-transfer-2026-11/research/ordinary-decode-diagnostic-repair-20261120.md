# Ordinary pre-classification decode diagnostics — repair candidate

Predecessor: accepted defect-evidence atom `b8d543b82e747222dbb4a49baafc3781f212c2bc`, manifest `59eabd57289b83ae45e8674810c27ce2dcde66e8a588739c4e975dcd7188e202`. That atom is an observed disclosure RED, not a passed security qualification. Its later unexecuted worlds remain NOT RUN in its historical receipts.

Repair contract `NST-ORDINARY-SECRET-DIAGNOSTICS-REPAIR-01`, architect `6266fb59af8cff6f8686f0c5e01451a9c93e3a9c`, integrated `a47533d18`. Architect-directed separate repair: close error propagation from ordinary input decoding before definition/secret classification. Existing strict generated shapes, valid inputs, public controls and semantic public-only refusals remain unchanged. No secret admission, hidden-input channel, persistence or secret delivery is added.

## Existing owners

- `vibe-wire::behaviour::ordinary_configuration_preview::parse_input` decodes CLI input with a closed static invalid-shape class and static field schema. The CLI adapter uses it instead of propagating serde's original diagnostic.
- Existing registered MCP `configuration_preview` already uses behaviour `parse_request`; that decoder now discards original serde value/key diagnostics and returns a static request schema. Explicit null refusal remains static and unchanged.
- Existing ordinary-install envelope behaviour parser likewise discards original decode errors. Protocol rejection remains a static unsupported-protocol class.
- No raw values, unknown keys, caller-supplied digests or chained serde errors are used to construct these diagnostics. No generated file/schema/epoch change.

## Verification scope

Run the original secret matrix to completion plus actual registered MCP variants. Positive public bool controls across all four routes; separate authored public string control. Omitted/well-typed/wrong-type secret bool and string worlds remain refusal subjects, not secret delivery. Raw/digest scanner controls and all synthetic framework state scanning remain in place.

Dedicated actual preview/envelope/MCP tests exercise wrong value, unknown key, digest-shaped unknown key and malformed input shape. MCP is valid JSON-RPC carrying malformed tool argument shape; this is not a claim about arbitrary broken JSON-RPC framing. Wire tests cover malformed serialized bytes, safe Display/Debug and no retained error source chain. Full incumbent regression includes valid input parity and actual registered preview behavior.

First complete secret matrix runtime passed all29 worlds (one test,101 filtered), including all original subjects and registered MCP. Wire runtime2 plus preview-wire2 passed, including safe Display/Debug and absent retained error source. Dedicated safe-error r1 retained RED: eight CLI/envelope worlds passed; the first MCP world had clean scanner/static schema but the test searched lowercase `error` instead of checking actual `isError`. The corrected oracle parses the response's error/isError fields. This is not a disclosure finding or permission to omit MCP checks. Final unfiltered public suite (`secret-repair-public-all-r1.log`) passed102/102 with zero filtered: all29 secret-matrix worlds and all12 dedicated actual safe-error worlds ran. Exact CLI SHA-256 `cc851cfce26e5f5bbfe1a08e31822997140cee64a89dffc174824727a2592ce9`, fixture `07926e301989a3f1d0ceb0fda4540aa5dab03025f3ea31954d7216bdc23ae7b1`. App dispatch4, wire2 and preview-wire2 passed independently in enforced bwrap isolation. Strict fixture and affected wire-test lint plus separate workspace fmt/diff checks passed. Independent acceptance remains pending. Canonical schemas/generated files are unchanged, so no regeneration claim is needed. The submitted-writer publication-fault injection gap remains UNVERIFIED for final integration; this repair does not close it. No NST/parent/USER-PROJECT promotion; REMAINDER/IDE unchanged.
