# Discipline v0.2 (BETA) — boot snippet {#root}

<status stage="impl" state="done"/>

@fact:FOLLOWS-DISCIPLINE This project follows the AI-Native Code Discipline. @status:impl/done

@fact:corpus-lives-here-lead The language-neutral corpus lives in this package: @status:impl/done

- @fact:CORPUS-GUIDING-LAYER the guiding layer (`spec/00-MANIFESTO.md`,
  `spec/01-PATTERN-CARD-FORMAT.md`, `spec/02-EXECUTABLE-SCAFFOLDS.md`), @status:impl/done
- @fact:CORPUS-OPERATING-PLAYBOOKS the operating playbooks (`spec/03-RAID-PLAYBOOK.md`
  campaigns, `spec/04-SWEEP-PLAYBOOK.md` the standing sweep, `spec/05-CAMPAIGN-FORM.md`
  the campaign paper trail, `spec/06-WAL-CONVENTION.md` session-durable state —
  optional but preferred), @status:impl/done
- @fact:CORPUS-MECHANISM-SPECS the mechanism specs under `spec/mechanisms/`
  (ENGINE-CONFORM, PROP-014 specmap, BROWNFIELD-PROTOCOL, LEDGER-INTENT — the
  units `spec://org.vibevm.ai-native/core-ai-native/…` tags cite), @status:impl/done
- @fact:CORPUS-APPENDIX and `spec/appendix/`. @status:impl/done

@fact:CARDS-AND-CHECKERS-PER-STACK The concrete per-language `cards/` and the
runnable checkers ship in each language stack, not here. @status:impl/done

@fact:DO-NOT-READ-ALL-AT-BOOT **Do not read it all at boot** — the Discipline's
own delivery rule is minimal sufficiency: @status:impl/done

- @fact:BAND-3-ON-TRIGGER load a card's Band-3 ops block only when its trigger
  fires; @status:impl/done
- @fact:PLAYBOOK-ON-PROCEDURE open a playbook when you run its procedure. @status:impl/done

@fact:two-laws-lead The two laws that always apply: @status:impl/done

1. @fact:LAW-IDIOMATIC-INSIDE-ENGINEERED-AROUND **Idiomatic inside the file, engineered around the file.** Code
   surface stays ordinary and in-distribution; all added strictness
   lives in types, contracts, metadata, and the verification loop. @status:impl/done
2. @fact:LAW-EXPLANATION-IS-RUNNABLE-CAPITAL **Explanation capital must be runnable capital.** Prose that could
   be a checker, doctest, or typed API is a WISH until it becomes one. @status:impl/done

@fact:CARD-REGISTRY Card registry: the active language stack's `spec/cards/INDEX.md` (trigger →
card; the nine executable scaffolds A–I in their per-language shape). @status:impl/done

@fact:CROSS-CUTTING-SWEEPS Cross-cutting sweeps follow `03-RAID-PLAYBOOK.md`. @status:impl/done

@fact:RULE-WITH-NO-CHECKER-IS-WISH A rule with no checker is a WISH; a deviation with no reason is a
defect (`#[spec(deviates, reason)]`). @status:impl/done
