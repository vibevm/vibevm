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
| `ENGINE-CONFORM-v0.1.md` | conformance engine: fact store, rules-as-queries, SARIF, ratchet baseline | `stack:org.vibevm/rust-ai-native` (`conform-core`, `conform-frontend-rust`, `conform-cli`), `cargo xtask conform` |
| `LEDGER-INTENT-v0.1.md` | intent ledger: facts vs interpretations, epoch-keyed cache, provenance | `crates/specmap-core::ledger`, `.ledger/` (git-ignored) |

> **ENGINE-CONFORM relocated (PROP-024, code-bearing packages).** Its
> implementing crates moved out of `crates/` and INTO
> `stack:org.vibevm/rust-ai-native`, so the checker ships with the stack a
> consumer installs rather than being a vibevm-only tool: `conform-core` +
> `conform-frontend-rust` + the `conform` binary (`conform-cli`) live in the
> package's own Cargo workspace, which vibevm consumes by external path-dep
> (its `xtask conform` is now a thin shim over `conform-cli`). The **spec
> stays vibevm-hosted** — 28 product / spec / terraform files cite it, so
> moving it would cascade dead `spec://` refs. Its in-source `scope!` edges
> were dropped with the move, so unlike the other three mechanisms above
> ENGINE-CONFORM is an **edge-less spec unit by design** (the specmap orphan
> ratchet records the crates' exemption while they lived in `crates/`; once
> relocated they leave the scan entirely).

The Discipline *product* is not here — its language-neutral core
(manifesto, card format, scaffold catalog, raid playbook) is the
installed package `flow:org.vibevm/discipline-core`, and the concrete
per-language cards ship in each stack (`stack:org.vibevm/rust-ai-native`,
`stack:org.vibevm/typescript-ai-native`); see `spec/neworder/README.md`,
the shim. These four documents are referenced by the product (Guide §7
cites PROP-014) but are hosted by vibevm because vibevm's code carries
their `implements` edges.
