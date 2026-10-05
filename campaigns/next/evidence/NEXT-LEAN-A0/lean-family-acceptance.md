# A0.2 accepted: minimal AI-Native Lean family

The source integration is `c6698c64f` followed by `3f1139b84` on the
current NEXT branch. The supporting host validator correction is `7e9ff7c91`:
content-minimal stack aggregators and optional-boot MCP packages no longer
fail `vibe check` for omitting an empty boot directory. The old flow-directory
negative case remains required and passed.

The family release line is `0.1.0`: `stack:org.vibevm.ai-native/lean-ai-native`
exact-pins its `lang` and `mcp` members; the MCP member exact-pins the lang
package. The lang package supplies a pinned Lean 4.34.1 checker, theorem
carrier/scanner, axiom inspection, model-proof witness, CLI, guide and cards.
The MCP package serves the same engine with typed reply parity. The stack
aggregator carries no code or boot snippet. The neutral core vendor copies
retain byte-identity checks. Package version, Lean toolchain version and
carrier/witness schema versions are distinct.

The reviewed implementation receipt is
`/fast/git/v/research/2026-10-05-lean-a0-2-implementation-receipt.json`
(SHA-256 `b0a781410f045c992df7e3ecd39e649b2a4c592d6b83b64bf78a2156e1f3f37f`).
It records 10 focused engine tests, nine MCP tests plus a doctest, actual
CLI/MCP G1 reply parity, package formatting/vendor checks and `vibe check`
exit 0 for all three new packages. An independent CLI check of the shipped
fixture returned `ok=true` for its exact method; an unknown method returned
the typed `unknown_subject` refusal with exit 1. Independent code review
found one saved-witness replacement race. `3f1139b84` now re-reads exact
witness bytes after the fresh checker and returns their digest; its
deterministic replacement and snapshot-digest tests passed, and the final
review found no remaining concrete P1/P2 in that fix.

This acceptance is **G1 model checking only**. It certifies neither the
meaning of a human task model nor correctness of Rust/TypeScript product
code. G3 implementation refinement requests refuse. The selected local Lean
modules and configured pinned distribution are trusted inputs, not a
sandbox. The first release admits ASCII theorem selectors and declared local
modules; larger Lake dependency graphs and extraction are later work.
`verify-witness` describes a checked file snapshot, not a promise that an
external writer cannot modify the path after the response.

A0.3 remains open: VibeVM must compose the Rust and Lean scanners/checkers
for one exact spec method, preserve both results through its host projections,
and join fresh proof evidence without treating a trace edge as a verdict.
No full product gate, mutation campaign, public release or historical-36
recheck was performed.
