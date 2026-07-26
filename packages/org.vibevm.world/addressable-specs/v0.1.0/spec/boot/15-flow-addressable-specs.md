# Flow: Addressable Specs {#root}

<status stage="impl" state="done"/>

##EVERY-NORMATIVE-STATEMENT-IS-ADDRESSABLE Every normative statement in this project's spec tree is
**addressable**: it lives under a stable `{#kebab-anchor}` and is
cited by URI, never by paraphrase. @impl/done

```
spec://<module>/<doc>#<section>[.<sub>]
```

## The correction contract {#correction-contract}

##CORRECTION-CITES-THE-VIOLATED-ANCHOR When the human corrects the agent, the correction cites the violated
anchor: "you are violating `spec://…#verification.timeout` — the
spec says 600 s, you wrote 300 s". @impl/done

##RESOLVE-READ-COMPARE-FIX Resolve the URI, read the unit,
compare, fix. @impl/done

##twenty-tokens-exact-hit Twenty tokens, exact hit — no guessing what
"verification" means or which part is "wrong". @spec/done

##THE-SAME-CONTRACT-BINDS-THE-AGENT The same contract binds the agent: when citing the spec — in chat,
commit bodies, code markers, review notes — cite the anchor, never a
paraphrase and never a line number. @impl/done

## Single source of truth {#single-source}

##EACH-FACT-HAS-EXACTLY-ONE-AUTHORITATIVE-ANCHOR Each fact has exactly one authoritative anchor. @impl/done

##NEVER-COPY-A-NORMATIVE-VALUE-CITE-THE-ANCHOR Never copy a
normative value into a second file — cite the anchor instead. @impl/done

##two-copies-will-diverge Two
copies *will* diverge, and a later session cannot tell which one
binds. @spec/done

## Placement {#placement}

##CRITICAL-CONSTRAINTS-LIVE-AT-THE-START-OR-END Critical constraints live at the START or END of a file, never
buried mid-document. @impl/done

##models-attend-to-the-edges-of-context Models attend to the edges of context ("Lost in
the Middle", Liu et al. 2023/2024); a mid-file invariant is an
invariant the reader statistically skipped. @spec/done

## Where the full rules live {#pointers}

- ##POINTER-THE-PROTOCOL Why addressability is IPC requirement #1, the URI scheme, the
  token economics:
  [`spec/flows/addressable-specs/ADDRESSABLE-SPECS-PROTOCOL.md`](../flows/addressable-specs/ADDRESSABLE-SPECS-PROTOCOL.md) @impl/done
- ##POINTER-THE-AUTHORING-RULES Unit of meaning, normativity marking, deviations, size budgets,
  anchor stability:
  [`spec/flows/addressable-specs/authoring-rules.md`](../flows/addressable-specs/authoring-rules.md) @impl/done
- ##POINTER-THE-SPEC-TREE-LAYOUT PROP vs FEAT, what goes where, the `.human/` buffer:
  [`spec/flows/addressable-specs/spec-tree-layout.md`](../flows/addressable-specs/spec-tree-layout.md) @impl/done

## Never {#never}

- ##NEVER-CITE-BY-PARAPHRASE-WHEN-AN-ANCHOR-EXISTS Never cite a spec section by paraphrase when an anchor exists. @impl/done
- ##NEVER-DUPLICATE-A-NORMATIVE-VALUE Never duplicate a normative value into a second file — cite its
  anchor. @impl/done
- ##NEVER-BURY-AN-INVARIANT-IN-THE-MIDDLE-OF-A-FILE Never bury an invariant in the middle of a file. @impl/done
- ##NEVER-RENAME-OR-DELETE-A-CITED-ANCHOR Never rename or delete an anchor that has ever been cited —
  anchors are immutable; retire with a tombstone instead. @impl/done
- ##NEVER-INVENT-AN-ANCHOR Never invent an anchor — resolve the URI and read the unit before
  acting on it. @impl/done
