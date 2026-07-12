# `flow:redbook` — the AI-native development practices, as a collection

The **redbook** is a curated collection of AI-native development
practices distilled from the book *AI-native development* and from
the practices proven in the vibevm project's own history. Each
practice is a standalone `flow` package — a boot snippet plus
protocol documents, product-agnostic, language-agnostic, and
agnostic of any particular coding agent. This umbrella package names
a **tested set** of them and carries the book itself.

Requiring `flow:redbook` installs the whole collection: the members
arrive through the dependency closure, each contributing its own
boot snippet and its own `spec/flows/<name>/` documents.

## The edition model {#editions}

The umbrella's version is the **edition number**. An edition is a
tested set: every member is pinned exactly (`=X.Y.Z`), so two
projects on the same edition run byte-identical practice text.
Members evolve on their own version lines between editions; a new
edition is a new umbrella version with refreshed pins.

- **Edition 0.1.0** — the book's core: the ten flows for the
  two-process model, the file IPC, and the memory model.
- **Edition 0.2.0** (this edition) — adds eleven project-practice
  flows for running a project over the long haul.

## Members (edition 0.2.0) {#members}

The book's core:

| Flow | One line |
|---|---|
| `two-process-model` @0.1.0 | Human and AI as coprocessors; the human owns coherence; files are the only shared memory. |
| `wal` @0.2.0 | The checkpoint file (WAL) and cold-resume snapshot; session wind-down and resume rituals; the `wal-status` skill. |
| `sync-from-code` @0.1.0 | The sanctioned reverse path: reconcile the spec when code changed first, with human approval. |
| `atomic-commits` @0.1.0 | One commit, one idea; Conventional Commits; pushed history is frozen. |
| `addressable-specs` @0.1.0 | `spec://` URIs, stable anchors, size budgets, and the spec tree layout. |
| `decision-records` @0.1.0 | Decisions, not facts: reason + rejected alternatives + revisit trigger, at the governing anchor. |
| `conflict-protocol` @0.1.0 | Human > Spec > Tests > Code; REVIEW markers; the conservative-default path when the spec is silent. |
| `campaign-plans` @0.1.0 | Cold-executable campaign plans: phase gates, falsifiable predictions, execution and deferral ledgers. |
| `discovery-prompt` @0.1.0 | The DISCOVERY collaborative-research prompt, packaged verbatim with a usage guide. |
| `attribution-policy` @0.1.0 | A deliberate authorship posture: human-authored surface by default, disclosure documented as the alternative. |

The project-practice wave:

| Flow | One line |
|---|---|
| `operating-modes` @0.1.0 | Codeword-triggered work postures; red lines that survive every mode. |
| `health-audit` @0.1.0 | The periodic judgment sweep over what the per-commit gate is blind to; a skill and an append-only trend. |
| `manual-tests` @0.1.0 | Human-runnable walkthroughs for the integration surfaces automation cannot prove. |
| `secrets-hygiene` @0.1.0 | Surface-secrets never printed or persisted; scope discipline; third-party-code consent. |
| `licensing` @0.1.0 | A deliberate licence posture; permissive-only dependencies; the EULA-to-open path; a drafting skill. |
| `source-mirrors` @0.1.0 | Single-writer multi-homing; manifest-driven fail-loud fast-forward-only fan-out. |
| `spec-genres` @0.1.0 | Contract vs lore vs research vs plans — what goes where, who wins, two-way links. |
| `comparative-research` @0.1.0 | Evergreen competitor studies with two-way gap analysis and numbered roadmap deltas. |
| `managed-blocks` @0.1.0 | How a tool writes into files it does not own — one delimited block, deterministic scanning (for tool authors). |
| `qualified-naming` @0.1.0 | Namespaces for package ecosystems: groups, identity tuples, collision vs conflict (for ecosystem designers). |
| `tool-design-lessons` @0.1.0 | Paid-for lessons for self-updating tools and package systems. |

## The book {#book}

The collection takes the general spirit of the process from the
book. The full text ships in this package under `spec/book/ru/` —
currently the Russian manuscript, included as-is. An English edition
will sit alongside it and take priority once it exists; until then
the Russian text is the reference. See `spec/book/README.md`.

The book is reference depth: the member flows carry the operational
rules, the book carries the *why* behind all of them.

## Relation to the AI-Native Discipline {#discipline}

The redbook and the AI-Native Code Discipline
(`flow:org.vibevm.ai-native/core-ai-native` and its language families) are
complementary layers:

- **redbook** is pure method — its value survives with only a git
  repository and a markdown editor. Any product, any language, any
  agent.
- **The Discipline** is code-enforced rigor — pattern cards, gates,
  and runnable checkers shipped per language.

Where the two describe the same practice, **the redbook package is
canonical**: `flow:wal` is the canonical home of the WAL convention
and `flow:campaign-plans` of the campaign-plan format; the
Discipline's internal copies defer to them from their next release.

## Install {#install}

```bash
vibe install flow:redbook
```

## Uninstall {#uninstall}

```bash
vibe uninstall flow:redbook
```

Uninstalling the umbrella removes its own files; member packages are
removed by uninstalling them individually.

## License {#license}

UPL-1.0. See `LICENSE.md`. The book text under `spec/book/` is the
author's manuscript and ships under the same terms.
