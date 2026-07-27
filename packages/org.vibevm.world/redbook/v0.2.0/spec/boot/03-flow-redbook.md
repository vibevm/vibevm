# Flow: redbook {#root}

<status stage="impl" state="done"/>

##PROJECT-FOLLOWS-THE-REDBOOK This project follows the **redbook** — a collection of AI-native
development practices, each installed as its own flow with its own
boot snippet and protocol documents. @impl/done

##THIS-SNIPPET-ONLY-NAMES-THE-COLLECTION This snippet only names the
collection; the members carry the actual rules. @impl/done

## The source of the spirit {#spirit-source}

##practices-distilled-from-the-book The practices are distilled from the book *AI-native development*. @spec/done

##THE-BOOK-SHIPS-IN-THIS-PACKAGE The book itself ships in this package at `spec/book/ru/` (currently
in Russian — see `spec/book/README.md` for the edition plan). @impl/done

##spirit-of-the-process-lead **The
general spirit of the process comes from the book**: @spec/done

- ##SPIRIT-TWO-PROCESSES-ONE-TASK two processes
  working one task, @spec/done
- ##SPIRIT-FILES-AS-THE-ONLY-SHARED-MEMORY files as the only shared memory, @spec/done
- ##SPIRIT-DECISIONS-RECORDED-WITH-THEIR-REASONS decisions
  recorded with their reasons, @spec/done
- ##SPIRIT-ONE-COMMIT-PER-THOUGHT one commit per thought. @spec/done

##DO-NOT-READ-THE-BOOK-AT-SESSION-BOOT Do **not** read the book at session boot — it is reference depth,
not standing instructions. @impl/done

##OPEN-A-CHAPTER-WHEN-A-PHILOSOPHY-QUESTION-ARISES Open a chapter when a philosophy question
actually arises; the members' boot snippets carry everything a
session needs. @impl/done

## The members (edition 0.2.0) {#member-list}

##books-core-list-lead The book's core — the four IPC requirements and the memory model: @impl/done

- ##MEMBER-TWO-PROCESS-MODEL `two-process-model` — the foundation: human and AI as
  coprocessors; the human owns coherence; files are the IPC. @impl/done
- ##MEMBER-WAL `wal` — the checkpoint file and cold-resume snapshot; session
  wind-down and resume rituals. @impl/done
- ##MEMBER-SYNC-FROM-CODE `sync-from-code` — the sanctioned reverse path when code changed
  before the spec. @impl/done
- ##MEMBER-ATOMIC-COMMITS `atomic-commits` — one commit, one idea; Conventional Commits;
  pushed history is frozen. @impl/done
- ##MEMBER-ADDRESSABLE-SPECS `addressable-specs` — `spec://` URIs and stable anchors; correct
  the agent in twenty tokens. @impl/done
- ##MEMBER-DECISION-RECORDS `decision-records` — record decisions with reasons and revisit
  triggers, at the anchor they govern. @impl/done
- ##MEMBER-CONFLICT-PROTOCOL `conflict-protocol` — Human > Spec > Tests > Code; REVIEW markers;
  the conservative-default uncertainty path. @impl/done
- ##MEMBER-CAMPAIGN-PLANS `campaign-plans` — cold-executable plans with phase gates,
  falsifiable predictions, and an execution ledger. @impl/done
- ##MEMBER-DISCOVERY-PROMPT `discovery-prompt` — the structured collaborative-research prompt
  for open-ended sessions. @impl/done
- ##MEMBER-ATTRIBUTION-POLICY `attribution-policy` — the deliberate authorship posture
  (human-authored surface by default). @impl/done

##project-practice-list-lead The project-practice wave — running a project over the long haul: @impl/done

- ##MEMBER-OPERATING-MODES `operating-modes` — codeword-triggered postures; red lines that
  survive every mode. @impl/done
- ##MEMBER-HEALTH-AUDIT `health-audit` — the periodic judgment sweep over what the gate
  cannot see. @impl/done
- ##MEMBER-MANUAL-TESTS `manual-tests` — human-runnable walkthroughs for the integration
  surfaces automation cannot prove. @impl/done
- ##MEMBER-SECRETS-HYGIENE `secrets-hygiene` — surface-secrets never printed or persisted;
  scope discipline; third-party-code consent. @impl/done
- ##MEMBER-LICENSING `licensing` — a deliberate licence posture; permissive-only
  dependencies; the EULA-to-open path. @impl/done
- ##MEMBER-SOURCE-MIRRORS `source-mirrors` — single-writer multi-homing; fail-loud
  fast-forward-only fan-out. @impl/done
- ##MEMBER-SPEC-GENRES `spec-genres` — contract vs lore vs research vs plans; what goes
  where and who wins. @impl/done
- ##MEMBER-COMPARATIVE-RESEARCH `comparative-research` — evergreen competitor studies with two-way
  gap analysis and roadmap deltas. @impl/done
- ##MEMBER-MANAGED-BLOCKS `managed-blocks` — how a tool writes into files it does not own
  (for tool authors). @impl/done
- ##MEMBER-QUALIFIED-NAMING `qualified-naming` — namespaces for package ecosystems (for
  ecosystem designers). @impl/done
- ##MEMBER-TOOL-DESIGN-LESSONS `tool-design-lessons` — paid-for lessons for self-updating tools
  and package systems. @impl/done

##cultural-extraction-list-lead The cultural-extraction wave — host-scale organisation: @impl/done

- ##MEMBER-DEV-RUNTIME-DOCS `dev-runtime-docs` — the running-notes discipline for a project's
  development-runtime documents. @impl/done
- ##MEMBER-WAL-SPECSPACES `wal-specspaces` — nested projects (specspaces), each carrying its
  own boot contract, WAL, and cold-resume file. @impl/done

##AN-EDITION-IS-A-TESTED-SET An **edition** is a tested set: the umbrella pins every member
exactly, and the umbrella's version is the edition number. @impl/done

##MEMBERS-MAY-MOVE-AHEAD-BETWEEN-EDITIONS Individual
members may move ahead on their own lines between editions. @impl/done
