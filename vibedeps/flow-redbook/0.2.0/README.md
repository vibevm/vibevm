# `flow:redbook` — the AI-native development practices, as a collection {#root}

<status stage="doc" state="done" audience="user"/>

##REDBOOK-IS-A-CURATED-COLLECTION-OF-PRACTICES The **redbook** is a curated collection of AI-native development
practices distilled from the book *AI-native development* and from
the practices proven in the vibevm project's own history. @impl/done

##EACH-PRACTICE-IS-A-STANDALONE-FLOW-PACKAGE Each
practice is a standalone `flow` package — a boot snippet plus
protocol documents, product-agnostic, language-agnostic, and
agnostic of any particular coding agent. @impl/done

##THE-UMBRELLA-NAMES-A-TESTED-SET-AND-CARRIES-THE-BOOK This umbrella package names
a **tested set** of them and carries the book itself. @impl/done

##REQUIRING-THE-UMBRELLA-INSTALLS-THE-WHOLE-COLLECTION Requiring `flow:redbook` installs the whole collection: the members
arrive through the dependency closure, each contributing its own
boot snippet and its own `spec/flows/<name>/` documents. @impl/done

## The edition model {#editions}

##THE-UMBRELLAS-VERSION-IS-THE-EDITION-NUMBER The umbrella's version is the **edition number**. @impl/done

##AN-EDITION-IS-A-TESTED-SET-OF-EXACT-PINS An edition is a
tested set: every member is pinned exactly (`=X.Y.Z`), so two
projects on the same edition run byte-identical practice text. @impl/done

##MEMBERS-EVOLVE-BETWEEN-EDITIONS Members evolve on their own version lines between editions; a new
edition is a new umbrella version with refreshed pins. @impl/done

- ##EDITION-0-1-0 **Edition 0.1.0** — the book's core: the ten flows for the
  two-process model, the file IPC, and the memory model. @impl/done
- ##EDITION-0-2-0 **Edition 0.2.0** (this edition) — adds eleven project-practice
  flows for running a project over the long haul. @impl/done

## Members (edition 0.2.0) {#members}

##books-core-table-lead The book's core: @impl/done

| Flow | One line |
|---|---|
| ##ROW-TWO-PROCESS-MODEL `two-process-model` @0.1.0 @impl/done | Human and AI as coprocessors; the human owns coherence; files are the only shared memory. @impl/done |
| ##ROW-WAL `wal` @0.2.0 @impl/done | The checkpoint file (WAL) and cold-resume snapshot; session wind-down and resume rituals; the `wal-status` skill. @impl/done |
| ##ROW-SYNC-FROM-CODE `sync-from-code` @0.1.0 @impl/done | The sanctioned reverse path: reconcile the spec when code changed first, with human approval. @impl/done |
| ##ROW-ATOMIC-COMMITS `git-atomic-commits` @0.1.0 @impl/done | One commit, one idea; Conventional Commits; pushed history is frozen. @impl/done |
| ##ROW-ADDRESSABLE-SPECS `addressable-specs` @0.1.0 @impl/done | `spec://` URIs, stable anchors, size budgets, and the spec tree layout. @impl/done |
| ##ROW-DECISION-RECORDS `decision-records` @0.1.0 @impl/done | Decisions, not facts: reason + rejected alternatives + revisit trigger, at the governing anchor. @impl/done |
| ##ROW-CONFLICT-PROTOCOL `conflict-protocol` @0.1.0 @impl/done | Human > Spec > Tests > Code; REVIEW markers; the conservative-default path when the spec is silent. @impl/done |
| ##ROW-CAMPAIGN-PLANS `campaign-plans` @0.1.0 @impl/done | Cold-executable campaign plans: phase gates, falsifiable predictions, execution and deferral ledgers. @impl/done |
| ##ROW-DISCOVERY-PROMPT `discovery-prompt` @0.1.0 @impl/done | The DISCOVERY collaborative-research prompt, packaged verbatim with a usage guide. @impl/done |
| ##ROW-ATTRIBUTION-POLICY `git-attribution-policy` @0.1.0 @impl/done | A deliberate authorship posture: human-authored surface by default, disclosure documented as the alternative. @impl/done |

##project-practice-table-lead The project-practice wave: @impl/done

| Flow | One line |
|---|---|
| ##ROW-OPERATING-MODES `operating-modes` @0.1.0 @impl/done | Codeword-triggered work postures; red lines that survive every mode. @impl/done |
| ##ROW-HEALTH-AUDIT `health-audit` @0.1.0 @impl/done | The periodic judgment sweep over what the per-commit gate is blind to; a skill and an append-only trend. @impl/done |
| ##ROW-MANUAL-TESTS `manual-tests` @0.1.0 @impl/done | Human-runnable walkthroughs for the integration surfaces automation cannot prove. @impl/done |
| ##ROW-SECRETS-HYGIENE `secrets-hygiene` @0.1.0 @impl/done | Surface-secrets never printed or persisted; scope discipline; third-party-code consent. @impl/done |
| ##ROW-LICENSING `licensing` @0.1.0 @impl/done | A deliberate licence posture; permissive-only dependencies; the EULA-to-open path; a drafting skill. @impl/done |
| ##ROW-SOURCE-MIRRORS `source-mirrors` @0.1.0 @impl/done | Single-writer multi-homing; manifest-driven fail-loud fast-forward-only fan-out. @impl/done |
| ##ROW-SPEC-GENRES `spec-genres` @0.1.0 @impl/done | Contract vs lore vs research vs plans — what goes where, who wins, two-way links. @impl/done |
| ##ROW-COMPARATIVE-RESEARCH `comparative-research` @0.1.0 @impl/done | Evergreen competitor studies with two-way gap analysis and numbered roadmap deltas. @impl/done |
| ##ROW-MANAGED-BLOCKS `managed-blocks` @0.1.0 @impl/done | How a tool writes into files it does not own — one delimited block, deterministic scanning (for tool authors). @impl/done |
| ##ROW-QUALIFIED-NAMING `qualified-naming` @0.1.0 @impl/done | Namespaces for package ecosystems: groups, identity tuples, collision vs conflict (for ecosystem designers). @impl/done |
| ##ROW-TOOL-DESIGN-LESSONS `tool-design-lessons` @0.1.0 @impl/done | Paid-for lessons for self-updating tools and package systems. @impl/done |

## The book {#book}

##the-collection-takes-its-spirit-from-the-book The collection takes the general spirit of the process from the
book. @spec/done

##THE-FULL-TEXT-SHIPS-UNDER-SPEC-BOOK-RU The full text ships in this package under `spec/book/ru/` —
currently the Russian manuscript, included as-is. @impl/done

##AN-ENGLISH-EDITION-WILL-TAKE-PRIORITY-ONCE-IT-EXISTS An English edition
will sit alongside it and take priority once it exists; until then
the Russian text is the reference. @spec/done

##book-readme-pointer See `spec/book/README.md`. @impl/done

##THE-BOOK-IS-REFERENCE-DEPTH The book is reference depth: the member flows carry the operational
rules, the book carries the *why* behind all of them. @spec/done

## Relation to the AI-Native Discipline {#discipline}

##complementary-layers-lead The redbook and the AI-Native Code Discipline
(`flow:org.vibevm.ai-native/core-ai-native` and its language families) are
complementary layers: @spec/done

- ##LAYER-REDBOOK-IS-PURE-METHOD **redbook** is pure method — its value survives with only a git
  repository and a markdown editor. Any product, any language, any
  agent. @spec/done
- ##LAYER-DISCIPLINE-IS-CODE-ENFORCED-RIGOR **The Discipline** is code-enforced rigor — pattern cards, gates,
  and runnable checkers shipped per language. @impl/done

##THE-REDBOOK-PACKAGE-IS-CANONICAL-WHERE-THE-TWO-OVERLAP Where the two describe the same practice, **the redbook package is
canonical**: `flow:wal` is the canonical home of the WAL convention
and `flow:campaign-plans` of the campaign-plan format. That is this
package's position, and the Discipline has not recorded it: as of
core-ai-native 0.8.0 its `05-CAMPAIGN-FORM.md` and
`06-WAL-CONVENTION.md` carry no deferral, so the two remain parallel
copies until one lands. @spec/done

## Install {#install}

```bash
vibe install flow:redbook
```

## Uninstall {#uninstall}

```bash
vibe uninstall flow:redbook
```

##UNINSTALLING-THE-UMBRELLA-REMOVES-ONLY-ITS-OWN-FILES Uninstalling the umbrella removes its own files; member packages are
removed by uninstalling them individually. @impl/done

## License {#license}

##license-line UPL-1.0. See `LICENSE.md`. @impl/done

##THE-BOOK-TEXT-SHIPS-UNDER-THE-SAME-TERMS The book text under `spec/book/` is the
author's manuscript and ships under the same terms. @impl/done
