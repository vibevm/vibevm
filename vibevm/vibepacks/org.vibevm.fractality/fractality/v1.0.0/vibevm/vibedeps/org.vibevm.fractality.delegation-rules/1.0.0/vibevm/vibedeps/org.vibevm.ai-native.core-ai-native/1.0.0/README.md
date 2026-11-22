# The AI-Native Code Discipline — core (flow:org.vibevm.ai-native/core-ai-native) {#root}

<status stage="doc" state="done" audience="user"/>

@fact:core-contents-lead The language-independent core of the Discipline: @status:impl/done

- @fact:CORE-PRINCIPLES principles, @status:impl/done
- @fact:CORE-PATTERN-CARD-FORMAT the pattern-card format, @status:impl/done
- @fact:CORE-SCAFFOLD-CATALOG the executable-scaffold catalog, @status:impl/done
- @fact:CORE-OPERATING-PLAYBOOKS the operating playbooks (raid, sweep, campaign
  form, WAL convention), @status:impl/done
- @fact:CORE-MECHANISM-SPECS and the mechanism specs the shipped checkers
  implement. @status:impl/done

@fact:CODE-OPTIMAL-TARGET Code optimal for **comprehension and safe modification
by AI agents — including weak readers** (small models in swarms maintaining
frontier-authored code). @status:impl/done

@fact:PROMPT-CONTENT-ONLY This package ships prompt content and the neutral engine crates it authors — five **library** crates (`core-ai-native-conform`, `-mcp`, `-specmap`, `-specmark`, `-specmark-grammar`); it ships no binary. @status:impl/done

@fact:RUNNABLE-HALF-IN-STACKS The runnable half — the checkers, the
per-language cards, the guides — ships in each language stack
(`stack:org.vibevm.ai-native/rust-ai-native-lang` first: `rust-ai-native-conform`, `rust-ai-native-specmap`,
`rust-ai-native`, the Rust GUIDE and cards). @status:impl/done

## Reading order (human reviewer / strong author) {#reading-order}

1. @fact:READ-MANIFESTO `spec/00-MANIFESTO.xml` — mission, axioms, the central law, §8 the package map. Start here. @status:impl/done
2. @fact:READ-PATTERN-CARD-FORMAT `spec/01-PATTERN-CARD-FORMAT.xml` — the format every pattern card is written in. @status:impl/done
3. @fact:READ-SCAFFOLDS `spec/02-EXECUTABLE-SCAFFOLDS.xml` — the nine runnable-capital classes. @status:impl/done
4. @fact:READ-STACK-GUIDE The active language stack's GUIDE (e.g. `spec/rust/GUIDE-AI-NATIVE-RUST.md` in the Rust stack). @status:impl/done
5. @fact:READ-PLAYBOOKS `spec/03-RAID-PLAYBOOK.xml` + `spec/04-SWEEP-PLAYBOOK.xml` + `spec/05-CAMPAIGN-FORM.xml` — campaigns and the standing sweep. @status:impl/done
6. @fact:READ-WAL-CONVENTION `spec/06-WAL-CONVENTION.xml` — session-durable project state (optional but preferred). @status:impl/done
7. @fact:READ-MECHANISMS `spec/mechanisms/` — ENGINE-CONFORM, PROP-014 (specmap), BROWNFIELD-PROTOCOL, LEDGER-INTENT, MCP-CORE: the normative mechanism specs; `spec://org.vibevm.ai-native/core-ai-native/mechanisms/…` is what code tags cite. @status:impl/done
8. @fact:READ-APPENDIX `spec/appendix/` — `CONTRADICTION-MAP.xml` (synthesis provenance) and `ATLAS.xml` (findings ledger). @status:impl/done

## The two load-bearing results behind everything {#load-bearing-results}

- @fact:RESULT-CENTRAL-LAW **Central law (Manifesto §3):** idiomatic inside the file, engineered
  around the file. Surface stays in-distribution (OOD syntax collapses
  models); strictness moves to types, contracts, meta, and the verification
  loop. @status:impl/done
- @fact:RESULT-RUNNABLE-CAPITAL **Runnable capital (Manifesto §5):** explanation capital must be
  executable. Weak agents leapt from executable scaffolds, not prose. Hence
  the nine-class catalog — and why every procedure here is backed by a
  shipped tool, not a description of one. @status:impl/done

## Status and honesty {#status-and-honesty}

@fact:beta-status BETA. @status:impl/done

@fact:maturity-tags-lead Maturity is tagged throughout: @status:impl/done

- @fact:MATURITY-E-STRONG [E-strong] (benchmark-backed), @status:impl/done
- @fact:MATURITY-E-MID [E-mid] (adjacent evidence), @status:impl/done
- @fact:MATURITY-E-HYP [E-hyp] (first-principles, pilot-gated). @status:impl/done

@fact:open-question-transfer The central open question — does the
executable-scaffold advantage transfer from *generation* to *modification* —
is unproven and is the pilot's job (see
`spec/appendix/CONTRADICTION-MAP.xml` C-7). @status:spec/done

@fact:names-failure-modes A discipline that names its failure modes is more
trustworthy than one that hides them. @status:spec/done

