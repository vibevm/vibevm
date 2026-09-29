# `flow:redbook` — the AI-native development practices, as a collection {#root}

<status stage="doc" state="done" audience="user"/>

@fact:REDBOOK-IS-A-CURATED-COLLECTION-OF-PRACTICES The **redbook** is a curated collection of AI-native development
practices distilled from the book *AI-native development* and from
the practices proven in the vibevm project's own history. @status:impl/done

@fact:EACH-PRACTICE-IS-A-STANDALONE-FLOW-PACKAGE Each
practice is a standalone `flow` package — a boot snippet plus
protocol documents, product-agnostic, language-agnostic, and
agnostic of any particular coding agent. @status:impl/done

@fact:THE-UMBRELLA-NAMES-A-TESTED-SET-AND-CARRIES-THE-BOOK This umbrella package names
a **tested set** of them and carries the book itself. @status:impl/done

@fact:REQUIRING-THE-UMBRELLA-INSTALLS-THE-WHOLE-COLLECTION Requiring `flow:redbook` installs the whole collection: the members
arrive through the dependency closure, each contributing its own
boot snippet and its own `spec/flows/<name>/` documents. @status:impl/done

## The edition model {#editions}

@fact:THE-UMBRELLAS-VERSION-IS-THE-EDITION-NUMBER The umbrella's version is the **edition number**. @status:impl/done

@fact:AN-EDITION-IS-A-TESTED-SET-OF-EXACT-PINS An edition is a
tested set: every member is pinned exactly (`=X.Y.Z`), so no member's
version can skew inside an edition. The *roster* is a second question —
while a wave settles, members are accumulated in place and the edition
number moves once at the end (see the manifest's own note above the
cultural-extraction wave), so two projects on the same edition run
byte-identical text of every member they share. @status:impl/done

@fact:MEMBERS-EVOLVE-BETWEEN-EDITIONS Members evolve on their own version lines between editions; a new
edition is a new umbrella version with refreshed pins. @status:impl/done

- @fact:EDITION-0-1-0 **Edition 0.1.0** — the book's core: the ten flows for the
  two-process model, the file IPC, and the memory model. @status:impl/done
- @fact:EDITION-0-2-0 **Edition 0.2.0** (the second edition) — added eleven project-practice
  flows for running a project over the long haul. @status:impl/done
- @fact:EDITION-1-0-0 **Edition 1.0.0** (this edition) — remints the tested
  collection as the stable 1.0 world. @status:impl/done

## Members (edition 1.0.0) {#members}

@fact:books-core-table-lead The book's core: @status:impl/done

| Flow | One line |
| --- | --- |
| @fact:ROW-TWO-PROCESS-MODEL `two-process-model` @1.0.0 @status:impl/done | @fact:ROW-TWO-PROCESS-MODEL-ONE-LINE  Human and AI as coprocessors; the human owns coherence; files are the only shared memory. @status:impl/done |
| @fact:ROW-WAL `wal` @1.0.0 @status:impl/done | @fact:ROW-WAL-ONE-LINE  The checkpoint file (WAL) and cold-resume snapshot; session wind-down and resume rituals; the `wal-status` skill. @status:impl/done |
| @fact:ROW-SYNC-FROM-CODE `sync-from-code` @1.0.0 @status:impl/done | @fact:ROW-SYNC-FROM-CODE-ONE-LINE  The sanctioned reverse path: reconcile the spec when code changed first, with human approval. @status:impl/done |
| @fact:ROW-ATOMIC-COMMITS `git-atomic-commits` @1.0.0 @status:impl/done | @fact:ROW-ATOMIC-COMMITS-ONE-LINE  One commit, one idea; Conventional Commits; pushed history is frozen. @status:impl/done |
| @fact:ROW-ADDRESSABLE-SPECS `addressable-specs` @1.0.0 @status:impl/done | @fact:ROW-ADDRESSABLE-SPECS-ONE-LINE  `spec://` URIs, stable anchors, size budgets, and the spec tree layout. @status:impl/done |
| @fact:ROW-DECISION-RECORDS `decision-records` @1.0.0 @status:impl/done | @fact:ROW-DECISION-RECORDS-ONE-LINE  Decisions, not facts: reason + rejected alternatives + revisit trigger, at the governing anchor. @status:impl/done |
| @fact:ROW-CONFLICT-PROTOCOL `conflict-protocol` @1.0.0 @status:impl/done | @fact:ROW-CONFLICT-PROTOCOL-ONE-LINE  Human > Spec > Tests > Code; REVIEW markers; the conservative-default path when the spec is silent. @status:impl/done |
| @fact:ROW-CAMPAIGN-PLANS `campaign-plans` @1.0.0 @status:impl/done | @fact:ROW-CAMPAIGN-PLANS-ONE-LINE  Cold-executable campaign plans: phase gates, falsifiable predictions, execution and deferral ledgers. @status:impl/done |
| @fact:ROW-DISCOVERY-PROMPT `discovery-prompt` @1.0.0 @status:impl/done | @fact:ROW-DISCOVERY-PROMPT-ONE-LINE  The DISCOVERY collaborative-research prompt, packaged verbatim with a usage guide. @status:impl/done |
| @fact:ROW-ATTRIBUTION-POLICY `git-attribution-policy` @1.0.0 @status:impl/done | @fact:ROW-ATTRIBUTION-POLICY-ONE-LINE  A deliberate authorship posture: human-authored surface by default, disclosure documented as the alternative. @status:impl/done |

@fact:project-practice-table-lead The project-practice wave: @status:impl/done

| Flow | One line |
| --- | --- |
| @fact:ROW-OPERATING-MODES `operating-modes` @1.0.0 @status:impl/done | @fact:ROW-OPERATING-MODES-ONE-LINE  Codeword-triggered work postures; red lines that survive every mode. @status:impl/done |
| @fact:ROW-HEALTH-AUDIT `health-audit` @1.0.0 @status:impl/done | @fact:ROW-HEALTH-AUDIT-ONE-LINE  The periodic judgment sweep over what the per-commit gate is blind to; a skill and an append-only trend. @status:impl/done |
| @fact:ROW-MANUAL-TESTS `manual-tests` @1.0.0 @status:impl/done | @fact:ROW-MANUAL-TESTS-ONE-LINE  Human-runnable walkthroughs for the integration surfaces automation cannot prove. @status:impl/done |
| @fact:ROW-SECRETS-HYGIENE `secrets-hygiene` @1.0.0 @status:impl/done | @fact:ROW-SECRETS-HYGIENE-ONE-LINE  Surface-secrets never printed or persisted; scope discipline; third-party-code consent. @status:impl/done |
| @fact:ROW-LICENSING `licensing` @1.0.0 @status:impl/done | @fact:ROW-LICENSING-ONE-LINE  A deliberate licence posture; permissive-only dependencies; the EULA-to-open path; a drafting skill. @status:impl/done |
| @fact:ROW-SOURCE-MIRRORS `source-mirrors` @1.0.0 @status:impl/done | @fact:ROW-SOURCE-MIRRORS-ONE-LINE  Single-writer multi-homing; manifest-driven fail-loud fast-forward-only fan-out. @status:impl/done |
| @fact:ROW-SPEC-GENRES `spec-genres` @1.0.0 @status:impl/done | @fact:ROW-SPEC-GENRES-ONE-LINE  Contract vs lore vs research vs plans — what goes where, who wins, two-way links. @status:impl/done |
| @fact:ROW-COMPARATIVE-RESEARCH `comparative-research` @1.0.0 @status:impl/done | @fact:ROW-COMPARATIVE-RESEARCH-ONE-LINE  Evergreen competitor studies with two-way gap analysis and numbered roadmap deltas. @status:impl/done |
| @fact:ROW-MANAGED-BLOCKS `managed-blocks` @1.0.0 @status:impl/done | @fact:ROW-MANAGED-BLOCKS-ONE-LINE  How a tool writes into files it does not own — one delimited block, deterministic scanning (for tool authors). @status:impl/done |
| @fact:ROW-QUALIFIED-NAMING `qualified-naming` @1.0.0 @status:impl/done | @fact:ROW-QUALIFIED-NAMING-ONE-LINE  Namespaces for package ecosystems: groups, identity tuples, collision vs conflict (for ecosystem designers). @status:impl/done |
| @fact:ROW-TOOL-DESIGN-LESSONS `tool-design-lessons` @1.0.0 @status:impl/done | @fact:ROW-TOOL-DESIGN-LESSONS-ONE-LINE  Paid-for lessons for self-updating tools and package systems. @status:impl/done |

## The book {#book}

@fact:the-collection-takes-its-spirit-from-the-book The collection takes the general spirit of the process from the
book. @status:spec/done

@fact:THE-FULL-TEXT-SHIPS-UNDER-SPEC-BOOK-RU The full text ships in this package under `spec/book/ru/` —
currently the Russian manuscript, included as-is. @status:impl/done

@fact:AN-ENGLISH-EDITION-WILL-TAKE-PRIORITY-ONCE-IT-EXISTS An English edition
will sit alongside it and take priority once it exists; until then
the Russian text is the reference. @status:spec/done

@fact:book-readme-pointer See `spec/book/README.md`. @status:impl/done

@fact:THE-BOOK-IS-REFERENCE-DEPTH The book is reference depth: the member flows carry the operational
rules, the book carries the *why* behind all of them. @status:spec/done

## Relation to the AI-Native Discipline {#discipline}

@fact:complementary-layers-lead The redbook and the AI-Native Code Discipline
(`flow:org.vibevm.ai-native/core-ai-native` and its language families) are
complementary layers: @status:spec/done

- @fact:LAYER-REDBOOK-IS-PURE-METHOD **redbook** is pure method — its value survives with only a git
  repository and a markdown editor. Any product, any language, any
  agent. @status:spec/done
- @fact:LAYER-DISCIPLINE-IS-CODE-ENFORCED-RIGOR **The Discipline** is code-enforced rigor — pattern cards, gates,
  and runnable checkers shipped per language. @status:impl/done

@fact:THE-REDBOOK-PACKAGE-IS-CANONICAL-WHERE-THE-TWO-OVERLAP Where the two describe the same practice, **the redbook package is
canonical**: `flow:wal` is the canonical home of the WAL convention
and `flow:campaign-plans` of the campaign-plan format. That is this
package's position, and the Discipline has not recorded it: as of
core-ai-native 0.8.0 its `05-CAMPAIGN-FORM.xml` and
`06-WAL-CONVENTION.xml` carry no deferral, so the two remain parallel
copies until one lands. @status:spec/done

## Install {#install}

```bash
vibe install flow:redbook
```

## Uninstall {#uninstall}

```bash
vibe uninstall flow:redbook
```

@fact:UNINSTALLING-THE-UMBRELLA-REMOVES-ONLY-ITS-OWN-FILES Uninstalling the umbrella removes its own files; member packages are
removed by uninstalling them individually. @status:impl/done

## License {#license}

@fact:license-line UPL-1.0. See `LICENSE.md`. @status:impl/done

@fact:THE-BOOK-TEXT-SHIPS-UNDER-THE-SAME-TERMS The book text under `spec/book/` is the
author's manuscript and ships under the same terms. @status:impl/done

