# PROP-001 — the demo contract {#root}

The walking skeleton's one spec document: what greeting means, what a
guest name is, and where the validation boundary sits. The Rust twin
of ts-demo's PROP-001 — same contract, one language-specific twist
(the newtype's privacy does compiler-side what the TS brand does
type-side).

## A guest name is a validated newtype {#req-guest-name}

`req r1`

A guest name entering the system crosses a validation boundary: it
arrives as untrusted text and becomes a `GuestName` ONLY through the
validator, which enforces the invariant (non-empty after
normalisation, ≤ 80 chars, no control characters). The newtype's
inner field is PRIVATE: `parse_guest_name` is the only constructor,
so the COMPILER refuses a raw construction from outside the cell
(E0603) — what TypeScript recovers with a brand, Rust enforces with
visibility. Failure is a value: a `thiserror` enum whose every
message cites this REQ.

## Greeting is total over valid names {#req-greet}

`req r1`

`greet` MUST return `"hello, <name>"` for every valid `GuestName` —
no panic, no unwrap. Failure cannot reach it: the validator's
`Result` carries the failure as a value with a `spec://`-citing
reason, and the type system admits no unvalidated name.

## The greeting cell {#cell-greeting}

`req r1`

The `greeting` cell owns naming + greeting. Its seam is the module's
public surface (`cells::greeting`); siblings import the seam, never
internals — and the private inner makes "internals" unreachable by
construction.

## The farewell cell {#cell-farewell}

`req r1`

The `farewell` cell demonstrates seam-only composition: it consumes
`greeting` strictly through the seam types.

## Core text utilities {#cell-text}

`req r1`

Normalisation is core — shared, cell-free, dependency-light:
whitespace runs collapse to single spaces and the ends are trimmed.
