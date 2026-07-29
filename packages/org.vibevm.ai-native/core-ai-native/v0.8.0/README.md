# The AI-Native Code Discipline — core (flow:org.vibevm.ai-native/core-ai-native) {#root}

<status stage="doc" state="done" audience="user"/>

##core-contents-lead The language-independent core of the Discipline: @impl/done

- ##CORE-PRINCIPLES principles, @impl/done
- ##CORE-PATTERN-CARD-FORMAT the pattern-card format, @impl/done
- ##CORE-SCAFFOLD-CATALOG the executable-scaffold catalog, @impl/done
- ##CORE-OPERATING-PLAYBOOKS the operating playbooks (raid, sweep, campaign
  form, WAL convention), @impl/done
- ##CORE-MECHANISM-SPECS and the mechanism specs the shipped checkers
  implement. @impl/done

##CODE-OPTIMAL-TARGET Code optimal for **comprehension and safe modification
by AI agents — including weak readers** (small models in swarms maintaining
frontier-authored code). @impl/done

##PROMPT-CONTENT-ONLY This package is prompt content plus the five neutral
engine crates it authors — `core-ai-native-conform`, `-mcp`, `-specmap`,
`-specmark`, `-specmark-grammar`. All five are libraries: the package carries
no binary, no `[[bin]]` target and no CLI of its own. @impl/done

##RUNNABLE-HALF-IN-STACKS The runnable half — the checkers, the
per-language cards, the guides — ships in each language stack
(`stack:org.vibevm.ai-native/rust-ai-native-lang` first: `rust-ai-native-conform`, `rust-ai-native-specmap`,
`rust-ai-native`, the Rust GUIDE and cards). @impl/done

## Reading order (human reviewer / strong author) {#reading-order}

1. ##READ-MANIFESTO `spec/00-MANIFESTO.md` — mission, axioms, the central law, §8 the package map. Start here. @impl/done
2. ##READ-PATTERN-CARD-FORMAT `spec/01-PATTERN-CARD-FORMAT.md` — the format every pattern card is written in. @impl/done
3. ##READ-SCAFFOLDS `spec/02-EXECUTABLE-SCAFFOLDS.md` — the nine runnable-capital classes. @impl/done
4. ##READ-STACK-GUIDE The active language stack's GUIDE (e.g. `rust/GUIDE-AI-NATIVE-RUST.md` in the Rust stack). @impl/done
5. ##READ-PLAYBOOKS `spec/03-RAID-PLAYBOOK.md` + `spec/04-SWEEP-PLAYBOOK.md` + `spec/05-CAMPAIGN-FORM.md` — campaigns and the standing sweep. @impl/done
6. ##READ-WAL-CONVENTION `spec/06-WAL-CONVENTION.md` — session-durable project state (optional but preferred). @impl/done
7. ##READ-MECHANISMS `spec/mechanisms/` — ENGINE-CONFORM, PROP-014 (specmap), BROWNFIELD-PROTOCOL, LEDGER-INTENT, MCP-CORE: the normative mechanism specs; `spec://org.vibevm.ai-native/core-ai-native/mechanisms/…` is what code tags cite. @impl/done
8. ##READ-APPENDIX `spec/appendix/` — `CONTRADICTION-MAP.md` (synthesis provenance) and `ATLAS.md` (findings ledger). @impl/done

## The two load-bearing results behind everything {#load-bearing-results}

- ##RESULT-CENTRAL-LAW **Central law (Manifesto §3):** idiomatic inside the file, engineered
  around the file. Surface stays in-distribution (OOD syntax collapses
  models); strictness moves to types, contracts, meta, and the verification
  loop. @impl/done
- ##RESULT-RUNNABLE-CAPITAL **Runnable capital (Manifesto §5):** explanation capital must be
  executable. Weak agents leapt from executable scaffolds, not prose. Hence
  the nine-class catalog — and why every procedure here is backed by a
  shipped tool, not a description of one. @impl/done

## Status and honesty {#status-and-honesty}

##beta-status BETA. @impl/done

##maturity-tags-lead Maturity is tagged throughout: @impl/done

- ##MATURITY-E-STRONG [E-strong] (benchmark-backed), @impl/done
- ##MATURITY-E-MID [E-mid] (adjacent evidence), @impl/done
- ##MATURITY-E-HYP [E-hyp] (first-principles, pilot-gated). @impl/done

##open-question-transfer The central open question — does the
executable-scaffold advantage transfer from *generation* to *modification* —
is unproven and is the pilot's job (see
`spec/appendix/CONTRADICTION-MAP.md` C-7). @spec/done

##names-failure-modes A discipline that names its failure modes is more
trustworthy than one that hides them. @spec/done
