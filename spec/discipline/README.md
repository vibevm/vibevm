# spec/discipline — the retained Discipline mechanisms

The four mechanism specs the Discipline relies on and **this
repository implements**. They moved here from `spec/neworder/` at
v0.3-adoption Phase 0 ("relocate the retained mechanisms under the
Discipline") and keep their content verbatim; only their home — and
therefore their `spec://vibevm/discipline/…` URIs — changed. The
in-source `#[spec]` / `scope!` edges were updated in the same commit,
so the specmap stays suspect-free.

| File | Mechanism | Implemented by |
|---|---|---|
| `PROP-014-specmap-bidirectional-traceability.md` | spec↔code traceability: anchors, revisions, tags, statuses, index, queries | `crates/specmark-grammar`, `crates/specmark`, `crates/specmap-core`, `cargo xtask specmap` |
| `BROWNFIELD-PROTOCOL-v0.1.md` | debt / intent / contradiction as first-class objects; xfail-strict test gate; tripwires | `crates/specmap-core` (`testgate`, `tripwire`), `terraform/registry/` |
| `ENGINE-CONFORM-v0.1.md` | conformance engine: fact store, rules-as-queries, SARIF, ratchet baseline | `crates/conform-core`, `crates/conform-frontend-rust`, `cargo xtask conform` |
| `LEDGER-INTENT-v0.1.md` | intent ledger: facts vs interpretations, epoch-keyed cache, provenance | `crates/specmap-core::ledger`, `.ledger/` (git-ignored) |

The Discipline *product* is not here — its language-neutral core
(manifesto, card format, scaffold catalog, raid playbook) is the
installed package `flow:org.vibevm/discipline-core`, and the concrete
per-language cards ship in each stack (`stack:org.vibevm/rust-ai-native`,
`stack:org.vibevm/typescript-ai-native`); see `spec/neworder/README.md`,
the shim. These four documents are referenced by the product (Guide §7
cites PROP-014) but are hosted by vibevm because vibevm's code carries
their `implements` edges.
