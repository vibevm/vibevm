# `flow:addressable-specs` — correct an agent in twenty tokens {#root}

A vibevm `flow` package that installs the **addressable
specifications** practice into a project: `spec://` URIs, stable
`{#anchor}`s on every heading that decides something, and the spec
tree layout that makes both resolvable with zero tooling.

Spec files in a human-agent team are not documentation — they are
the IPC channel between two processes, and the first requirement on
that channel is addressability. "You did the verification wrong"
costs the agent hundreds of tokens of guessing; "you are violating
`spec://com.example.shop/PROP-001#verification.timeout` — 600 s, not
300 s" costs about twenty and hits exactly. This package is that
difference, made into a standing contract.

This package ships three pieces of content plus a boot snippet:

- `spec/flows/addressable-specs/ADDRESSABLE-SPECS-PROTOCOL.md` —
  full protocol: why addressability is IPC requirement #1, the URI
  scheme and anchor grammar, reverse-DNS module names, the
  single-source and placement rules, the bidirectional graph that
  `Implements:` markers and `Test:` lines create, and a re-derive
  prompt for adapting the practice to a concrete project.
- `spec/flows/addressable-specs/authoring-rules.md` — how to write
  units that stay addressable: one unit = one decision, normativity
  marked with RFC-2119 verbs, deviations recorded honestly, size
  budgets, changelog lines, and the rule that anchors are immutable
  once cited.
- `spec/flows/addressable-specs/spec-tree-layout.md` — the reference
  tree: PROP vs FEAT, the what-goes-where decision table, and the
  `.human/` private buffer enforced by ignore-file invisibility.
- `spec/boot/15-flow-addressable-specs.md` — boot snippet loaded at
  session start: the correction contract, the single-source and
  placement rules, and the never-do list.

## Install {#install}

```bash
vibe install flow:addressable-specs
```

## Uninstall {#uninstall}

```bash
vibe uninstall flow:addressable-specs
```

Uninstalling removes every file the package wrote, including the
boot snippet. User-owned files are never touched.

## Composition {#composition}

- `flow:two-process-model` — establishes the four IPC requirements
  on shared files; addressability is the first of them, and this
  package is its full elaboration.
- `flow:atomic-commits` — commit bodies cite `spec://` URIs; this
  package defines what those URIs resolve to.
- `flow:conflict-protocol` — corrections and REVIEW markers cite the
  violated anchor rather than paraphrasing it.
- `flow:wal` — WAL Constraints and next-step pointers cite anchors,
  so a resumed session lands on the exact unit.
- `flow:decision-records` — records live at the anchors they govern;
  a decision without an address cannot be cited or superseded
  cleanly.

## Philosophical background {#background}

The practice is extracted from *AI-native development*, chapter 2
(*"Shared state: файлы как IPC"*, subsections on addressability, the
Lost-in-the-Middle placement rule, control-plane size budgets, and
the practical file structure). The chapter ships in Russian inside
`flow:redbook` at `spec/book/ru/`. Short version: the human knows
instantly what the agent got wrong; the bottleneck is telling the
machine — so make every fact in the project pointable in one URI.

## License {#license}

UPL-1.0. See `LICENSE.md`.
