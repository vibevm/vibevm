# spec/discipline — relocated into the Discipline packages

The four mechanism specs that lived here — the Discipline's normative
mechanism layer — now **ship with the Discipline itself**, in
`flow:org.vibevm.ai-native/core-ai-native` under `spec/mechanisms/`
(SELF-SUFFICIENCY-PLAN Phase 2, 2026-07-07): a consumer of the discipline
stacks receives the documents its code tags cite, instead of needing
vibevm's dev tree.

| Mechanism | Shipped spec unit (cite these) | Implemented by |
|---|---|---|
| spec↔code traceability (anchors, revisions, tags, index, queries) | `spec://org.vibevm.ai-native/core-ai-native/mechanisms/PROP-014#…` | `stack:org.vibevm.ai-native/rust-ai-native-lang` (`core-ai-native-specmap`, bin `rust-ai-native-specmap`) |
| brownfield terraforming (registries, xfail-strict gate, tripwires) | `spec://org.vibevm.ai-native/core-ai-native/mechanisms/BROWNFIELD-PROTOCOL-v0.1#…` | `core-ai-native-specmap::{testgate,tripwire}` + the `rust-ai-native-terraform` skill |
| conformance engine (fact store, rules-as-queries, SARIF, ratchet) | `spec://org.vibevm.ai-native/core-ai-native/mechanisms/ENGINE-CONFORM-v0.1#…` | `stack:org.vibevm.ai-native/rust-ai-native-lang` (`core-ai-native-conform`, bin `rust-ai-native-conform`) |
| intent ledger (facts vs interpretations, epoch-keyed cache) | `spec://org.vibevm.ai-native/core-ai-native/mechanisms/LEDGER-INTENT-v0.1#…` | `core-ai-native-specmap::ledger`, `.ledger/` (git-ignored) |

In this repository the shipped copies live at
`vibedeps/flow-core-ai-native/<version>/spec/mechanisms/` (and the editable
sources at `packages/org.vibevm.ai-native/core-ai-native/<version>/spec/mechanisms/` —
vibevm is the packages' dev repo). vibevm's `specmap.toml` resolves the
`spec://org.vibevm.ai-native/core-ai-native/…` URIs through its `[[external_specs]]` entry, so
the in-repo tags into these units are fully resolved, not dangling.

Historical note: vibevm-hosted URIs of the form
`spec://vibevm/discipline/<DOC>#<anchor>` map 1:1 to
`spec://org.vibevm.ai-native/core-ai-native/mechanisms/<DOC>#<anchor>` (anchors unchanged).
Occurrences in historical documents (the WAL's prior tail, past terraform
plans and reports, old commit bodies) are records of their time and were
deliberately not rewritten. All live code and living documents cite the
shipped units. (An earlier revision of this README claimed ENGINE-CONFORM
was an edge-less spec unit by design — that was true only between the Ф4a
tag strip and the Phase 4 re-tag of the traceability relocation; its
implementing crates carry `scope!` edges into it again.)

The Discipline *product* is not here and never was: the language-neutral
core (manifesto, card format, scaffold catalog, the raid/sweep/campaign/WAL
playbooks) is the installed package `flow:org.vibevm.ai-native/core-ai-native`, and
the concrete per-language cards + checkers ship in each stack.
