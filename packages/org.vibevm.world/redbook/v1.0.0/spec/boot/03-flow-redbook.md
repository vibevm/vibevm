# Flow: redbook {#root}

<status stage="impl" state="done"/>

@fact:PROJECT-FOLLOWS-THE-REDBOOK This project follows the **redbook** — a collection of AI-native
development practices, each installed as its own flow with its own
boot snippet and protocol documents. @status:impl/done

@fact:THIS-SNIPPET-ONLY-NAMES-THE-COLLECTION This snippet only names the
collection; the members carry the actual rules. @status:impl/done

## The source of the spirit {#spirit-source}

@fact:practices-distilled-from-the-book The practices are distilled from the book *AI-native development*. @status:spec/done

@fact:THE-BOOK-SHIPS-IN-THIS-PACKAGE The book itself ships in this package at `spec/book/ru/` (currently
in Russian — see `spec/book/README.md` for the edition plan). @status:impl/done

@fact:spirit-of-the-process-lead **The
general spirit of the process comes from the book**: @status:spec/done

- @fact:SPIRIT-TWO-PROCESSES-ONE-TASK two processes
  working one task, @status:spec/done
- @fact:SPIRIT-FILES-AS-THE-ONLY-SHARED-MEMORY files as the only shared memory, @status:spec/done
- @fact:SPIRIT-DECISIONS-RECORDED-WITH-THEIR-REASONS decisions
  recorded with their reasons, @status:spec/done
- @fact:SPIRIT-ONE-COMMIT-PER-THOUGHT one commit per thought. @status:spec/done

@fact:DO-NOT-READ-THE-BOOK-AT-SESSION-BOOT Do **not** read the book at session boot — it is reference depth,
not standing instructions. @status:impl/done

@fact:OPEN-A-CHAPTER-WHEN-A-PHILOSOPHY-QUESTION-ARISES Open a chapter when a philosophy question
actually arises; the members' boot snippets carry everything a
session needs. @status:impl/done

## The members (edition 1.0.0) {#member-list}

@fact:books-core-list-lead The book's core — the four IPC requirements and the memory model: @status:impl/done

- @fact:MEMBER-TWO-PROCESS-MODEL `two-process-model` — the foundation: human and AI as
  coprocessors; the human owns coherence; files are the IPC. @status:impl/done
- @fact:MEMBER-WAL `wal` — the checkpoint file and cold-resume snapshot; session
  wind-down and resume rituals. @status:impl/done
- @fact:MEMBER-SYNC-FROM-CODE `sync-from-code` — the sanctioned reverse path when code changed
  before the spec. @status:impl/done
- @fact:MEMBER-ATOMIC-COMMITS `git-atomic-commits` — one commit, one idea; Conventional Commits;
  pushed history is frozen. @status:impl/done
- @fact:MEMBER-ADDRESSABLE-SPECS `addressable-specs` — `spec://` URIs and stable anchors; correct
  the agent in twenty tokens. @status:impl/done
- @fact:MEMBER-DECISION-RECORDS `decision-records` — record decisions with reasons and revisit
  triggers, at the anchor they govern. @status:impl/done
- @fact:MEMBER-CONFLICT-PROTOCOL `conflict-protocol` — Human > Spec > Tests > Code; REVIEW markers;
  the conservative-default uncertainty path. @status:impl/done
- @fact:MEMBER-CAMPAIGN-PLANS `campaign-plans` — cold-executable plans with phase gates,
  falsifiable predictions, and an execution ledger. @status:impl/done
- @fact:MEMBER-DISCOVERY-PROMPT `discovery-prompt` — the structured collaborative-research prompt
  for open-ended sessions. @status:impl/done
- @fact:MEMBER-ATTRIBUTION-POLICY `git-attribution-policy` — the deliberate authorship posture
  (human-authored surface by default). @status:impl/done

@fact:project-practice-list-lead The project-practice wave — running a project over the long haul: @status:impl/done

- @fact:MEMBER-OPERATING-MODES `operating-modes` — codeword-triggered postures; red lines that
  survive every mode. @status:impl/done
- @fact:MEMBER-HEALTH-AUDIT `health-audit` — the periodic judgment sweep over what the gate
  cannot see. @status:impl/done
- @fact:MEMBER-MANUAL-TESTS `manual-tests` — human-runnable walkthroughs for the integration
  surfaces automation cannot prove. @status:impl/done
- @fact:MEMBER-SECRETS-HYGIENE `secrets-hygiene` — surface-secrets never printed or persisted;
  scope discipline; third-party-code consent. @status:impl/done
- @fact:MEMBER-LICENSING `licensing` — a deliberate licence posture; permissive-only
  dependencies; the EULA-to-open path. @status:impl/done
- @fact:MEMBER-SOURCE-MIRRORS `source-mirrors` — single-writer multi-homing; fail-loud
  fast-forward-only fan-out. @status:impl/done
- @fact:MEMBER-SPEC-GENRES `spec-genres` — contract vs lore vs research vs plans; what goes
  where and who wins. @status:impl/done
- @fact:MEMBER-COMPARATIVE-RESEARCH `comparative-research` — evergreen competitor studies with two-way
  gap analysis and roadmap deltas. @status:impl/done
- @fact:MEMBER-MANAGED-BLOCKS `managed-blocks` — how a tool writes into files it does not own
  (for tool authors). @status:impl/done
- @fact:MEMBER-QUALIFIED-NAMING `qualified-naming` — namespaces for package ecosystems (for
  ecosystem designers). @status:impl/done
- @fact:MEMBER-TOOL-DESIGN-LESSONS `tool-design-lessons` — paid-for lessons for self-updating tools
  and package systems. @status:impl/done

@fact:cultural-extraction-list-lead The cultural-extraction wave — host-scale organisation: @status:impl/done

- @fact:MEMBER-DEV-RUNTIME-DOCS `dev-runtime-docs` — the running-notes discipline for a project's
  development-runtime documents. @status:impl/done
- @fact:MEMBER-WAL-SPECSPACES `wal-specspaces` — nested projects (specspaces), each carrying its
  own boot contract, WAL, and cold-resume file. @status:impl/done

@fact:AN-EDITION-IS-A-TESTED-SET An **edition** is a tested set: the umbrella pins every member
exactly, and the umbrella's version is the edition number. @status:impl/done

@fact:MEMBERS-MAY-MOVE-AHEAD-BETWEEN-EDITIONS Individual
members may move ahead on their own lines between editions. @status:impl/done
