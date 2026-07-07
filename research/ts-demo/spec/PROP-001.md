# PROP-001 — the demo contract {#root}

The walking skeleton's one spec document: what greeting means, what a
guest name is, and where the erasure boundary sits.

## A guest name is branded and validated {#req-guest-name}

`req r1`

A guest name entering the system crosses an erasure boundary: it
arrives as `unknown` and becomes a `GuestName` ONLY through the
validator, which enforces the invariant (non-empty, ≤ 80 chars,
printable). The brand makes a raw `string` unusable where a
`GuestName` is required — the wrong same-shaped value fails `tsc`.

## Greeting is total over valid names {#req-greet}

`req r1`

`greet` MUST return `"hello, <name>"` for every valid `GuestName` —
no throw, no null. Failure cannot reach it: the validator's `Result`
carries the failure as a value with a `spec://`-citing reason.

## The greeting cell {#cell-greeting}

`req r1`

The `greeting` cell owns naming + greeting. Its seam is `index.ts`;
siblings import the seam, never internals.

## The farewell cell {#cell-farewell}

`req r1`

The `farewell` cell demonstrates seam-only composition: it consumes
`greeting` strictly through the seam.

## Core text utilities {#cell-text}

`req r1`

Normalisation is core — shared, cell-free, dependency-light.
