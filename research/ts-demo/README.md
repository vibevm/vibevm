# ts-demo — the TypeScript discipline walking skeleton

The pilot-lite for `stack:org.vibevm.ai-native/typescript-ai-native` (NOT the
forthcoming VibeVM TypeScript surface): a real consumer project that
exercises the whole shipped toolchain — branded seams, Result-shaped
errors, a validated erasure boundary, seam-only cell composition, JSDoc
spec markers, and the full seven-step floor.

## Bootstrap (from a fresh clone)

```sh
cd research/ts-demo
# 1. Materialise the discipline packages from the in-repo registry.
cargo run --manifest-path ../../Cargo.toml -p vibe-cli -- \
    install --path . --registry ../../packages --assume-yes
# 2. The npm toolchain (node >= 22.6).
npm install
# 3. The floor, via the materialised slot.
cargo run --manifest-path vibedeps/stack-typescript-ai-native-lang/0.6.0/Cargo.toml \
    -p typescript-ai-native-cli --bin typescript-ai-native -- floor
```

## What green means here

All seven floor steps: prettier → tsc → `node --test` (strip-types runs
the `.ts` tests directly) → eslint → `typescript-ai-native-conform`
(`ts-unsafe-in-domain` + `ts-cell-isolation`) → `typescript-ai-native-specmap
--check` (the committed `specmap.json` byte-reproduces; 0 orphans) →
the xfail-strict test-gate.

The conform baseline deliberately carries ONE frozen finding: the
`cleaned as GuestName` cast inside `parseGuestName` — the brand
constructor. A brand is a compile-time fiction, so its single
constructor site is exactly the irreducible `as` the ratchet exists to
hold at one; anything new fails the gate.

## Layout

- `spec/PROP-001.md` — the demo contract (anchored spec units).
- `src/core/` — cell-free shared code (`@scope`-tagged).
- `src/cells/greeting/` — the branded-name cell: `GuestName`,
  `parseGuestName` (the erasure boundary), `greet`.
- `src/cells/farewell/` — seam-only composition over `greeting`.
- `specmap.json` — the committed traceability index (`--check` gates it).
- `typescript-ai-native-conform-baseline.json` — the ratchet (see above).
