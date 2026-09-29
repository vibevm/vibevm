# `flow:addressable-specs` — correct an agent in twenty tokens {#root}

<status stage="doc" state="done" audience="user"/>

@fact:PACKAGE-INSTALLS-THE-ADDRESSABLE-SPECIFICATIONS-PRACTICE A vibevm `flow` package that installs the **addressable
specifications** practice into a project: `spec://` URIs, stable
`{#anchor}`s on every heading that decides something, and the spec
tree layout that makes both resolvable with zero tooling. @status:impl/done

@fact:SPEC-FILES-ARE-THE-IPC-CHANNEL-NOT-DOCUMENTATION Spec files in a human-agent team are not documentation — they are
the IPC channel between two processes, and the first requirement on
that channel is addressability. @status:spec/done

@fact:paraphrase-costs-hundreds-of-tokens-an-anchor-costs-twenty "You did the verification wrong"
costs the agent hundreds of tokens of guessing; "you are violating
`spec://com.example.shop/PROP-001#verification.timeout` — 600 s, not
300 s" costs about twenty and hits exactly. @status:spec/done

@fact:PACKAGE-IS-THAT-DIFFERENCE-AS-A-STANDING-CONTRACT This package is that
difference, made into a standing contract. @status:impl/done

@fact:package-contents-lead This package ships three pieces of content plus a boot snippet: @status:impl/done

- @fact:CONTENT-THE-PROTOCOL `spec/flows/addressable-specs/ADDRESSABLE-SPECS-PROTOCOL.xml` —
  full protocol: why addressability is IPC requirement #1, the URI
  scheme and anchor grammar, reverse-DNS module names, the
  single-source and placement rules, the bidirectional graph that
  `Implements:` markers and `Test:` lines create, and a re-derive
  prompt for adapting the practice to a concrete project. @status:impl/done
- @fact:CONTENT-THE-AUTHORING-RULES `spec/flows/addressable-specs/authoring-rules.xml` — how to write
  units that stay addressable: one unit = one decision, normativity
  marked with RFC-2119 verbs, deviations recorded honestly, size
  budgets, changelog lines, and the rule that anchors are immutable
  once cited. @status:impl/done
- @fact:CONTENT-THE-SPEC-TREE-LAYOUT `spec/flows/addressable-specs/spec-tree-layout.xml` — the reference
  tree: PROP vs FEAT, the what-goes-where decision table, and the
  `.human/` private buffer enforced by ignore-file invisibility. @status:impl/done
- @fact:CONTENT-THE-BOOT-SNIPPET `spec/boot/15-flow-addressable-specs.xml` — boot snippet loaded at
  session start: the correction contract, the single-source and
  placement rules, and the never-do list. @status:impl/done

## Install {#install}

```bash
vibe install flow:addressable-specs
```

## Uninstall {#uninstall}

```bash
vibe uninstall flow:addressable-specs
```

@fact:UNINSTALL-REMOVES-EVERY-FILE-THE-PACKAGE-WROTE Uninstalling removes every file the package wrote, including the
boot snippet. @status:impl/done

@fact:USER-OWNED-FILES-ARE-NEVER-TOUCHED User-owned files are never touched. @status:impl/done

## Composition {#composition}

- @fact:COMPOSES-TWO-PROCESS-MODEL `flow:two-process-model` — establishes the four IPC requirements
  on shared files; addressability is the first of them, and this
  package is its full elaboration. @status:impl/done
- @fact:COMPOSES-ATOMIC-COMMITS `flow:git-atomic-commits` — one commit, one logical idea, so a spec
  change and the code that satisfies it land as one citable unit. The
  rule that commit bodies cite `spec://` URIs is the sibling
  `flow:git-conventional-commits`', which owns the message *format*;
  this package defines what those URIs resolve to. @status:impl/done
- @fact:COMPOSES-CONFLICT-PROTOCOL `flow:conflict-protocol` — corrections and REVIEW markers cite the
  violated anchor rather than paraphrasing it. @status:impl/done
- @fact:COMPOSES-WAL `flow:wal` — WAL Constraints and next-step pointers cite anchors,
  so a resumed session lands on the exact unit. @status:impl/done
- @fact:COMPOSES-DECISION-RECORDS `flow:decision-records` — records live at the anchors they govern;
  a decision without an address cannot be cited or superseded
  cleanly. @status:impl/done

## Philosophical background {#background}

@fact:practice-extracted-from-the-book The practice is extracted from *AI-native development*, chapter 2
(*"Shared state: файлы как IPC"*, subsections on addressability, the
Lost-in-the-Middle placement rule, control-plane size budgets, and
the practical file structure). @status:spec/done

@fact:CHAPTER-SHIPS-IN-RUSSIAN-INSIDE-REDBOOK The chapter ships in Russian inside
`flow:redbook` at `spec/book/ru/`. @status:impl/done

@fact:BOTTLENECK-IS-TELLING-THE-MACHINE Short version: the human knows
instantly what the agent got wrong; the bottleneck is telling the
machine — so make every fact in the project pointable in one URI. @status:spec/done

## License {#license}

@fact:license-line UPL-1.0. See `LICENSE.md`. @status:impl/done

