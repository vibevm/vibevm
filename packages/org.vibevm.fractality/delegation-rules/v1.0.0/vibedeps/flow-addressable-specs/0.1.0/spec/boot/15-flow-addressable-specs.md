# Flow: Addressable Specs {#root}

Every normative statement in this project's spec tree is
**addressable**: it lives under a stable `{#kebab-anchor}` and is
cited by URI, never by paraphrase.

```
spec://<module>/<doc>#<section>[.<sub>]
```

## The correction contract {#correction-contract}

When the human corrects the agent, the correction cites the violated
anchor: "you are violating `spec://…#verification.timeout` — the
spec says 600 s, you wrote 300 s". Resolve the URI, read the unit,
compare, fix. Twenty tokens, exact hit — no guessing what
"verification" means or which part is "wrong".

The same contract binds the agent: when citing the spec — in chat,
commit bodies, code markers, review notes — cite the anchor, never a
paraphrase and never a line number.

## Single source of truth {#single-source}

Each fact has exactly one authoritative anchor. Never copy a
normative value into a second file — cite the anchor instead. Two
copies *will* diverge, and a later session cannot tell which one
binds.

## Placement {#placement}

Critical constraints live at the START or END of a file, never
buried mid-document. Models attend to the edges of context ("Lost in
the Middle", Liu et al. 2023/2024); a mid-file invariant is an
invariant the reader statistically skipped.

## Where the full rules live {#pointers}

- Why addressability is IPC requirement #1, the URI scheme, the
  token economics:
  [`spec/flows/addressable-specs/ADDRESSABLE-SPECS-PROTOCOL.md`](../flows/addressable-specs/ADDRESSABLE-SPECS-PROTOCOL.md)
- Unit of meaning, normativity marking, deviations, size budgets,
  anchor stability:
  [`spec/flows/addressable-specs/authoring-rules.md`](../flows/addressable-specs/authoring-rules.md)
- PROP vs FEAT, what goes where, the `.human/` buffer:
  [`spec/flows/addressable-specs/spec-tree-layout.md`](../flows/addressable-specs/spec-tree-layout.md)

## Never {#never}

- Never cite a spec section by paraphrase when an anchor exists.
- Never duplicate a normative value into a second file — cite its
  anchor.
- Never bury an invariant in the middle of a file.
- Never rename or delete an anchor that has ever been cited —
  anchors are immutable; retire with a tombstone instead.
- Never invent an anchor — resolve the URI and read the unit before
  acting on it.
