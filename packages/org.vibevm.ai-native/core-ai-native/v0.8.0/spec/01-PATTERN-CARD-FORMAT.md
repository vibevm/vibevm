# The AI-Native Pattern Card Format {#root}

<status stage="spec" state="done"/>

##status-line **Discipline v0.2 · status: BETA · T1 · supersedes part of GUIDE-SPEC-AUTHORING-v0.1** @impl/done

##EVERY-PATTERN-IS-A-CARD *Every pattern, scaffold, rule, and anti-pattern in the Discipline is authored as a card in this shape.* @impl/done

##FORMAT-SYNTHESIZES-THREE-PARENTS *The format is a synthesis of three parents: the GoF design-pattern template (teaches WHEN to apply), the OpenJDK JEP template (teaches WHAT COUNTS AS DONE and at what cost), and a third operational layer with no precedent in either — because both predate AI readers.* @impl/done

## 0. Why three parents {#why-three-parents}

- ##PARENT-GOF-RECOGNITION **GoF → recognition & navigation.** Its load-bearing sections for us are *Applicability* (how to RECOGNIZE the situation from the code) and *Related Patterns* (where to go if this feels wrong). @impl/done
- ##PARENT-JEP-ACCOUNTABILITY **JEP → accountability & tradeoffs.** Its load-bearing sections are *Goals/Non-Goals* (scope discipline — the most-violated boundary in AI work), *Risks & Assumptions*, *Alternatives*. @impl/done
- ##PARENT-OPERATIONAL-EXECUTION **Operational layer → execution.** GoF and JEP assume a human applying judgment once, at design time, with unlimited attention. We know three things they did not: the card is a **runtime trigger**, **raid fuel**, and will be read by a **weak model that cannot hold all cards at once**. So we add *Trigger, Routine, Checker, Raid-role, Budget* — and we make that layer **machine-extractable**, so the harness can deliver just the operational extract to a weak reader (the §6 minimal-sufficiency rule of the Manifesto). @impl/done

## 1. The three bands {#three-bands}

##BANDS-1-2-ARE-AUTHORING-PROSE Bands 1–2 are prose for the strong author and the human reviewer (full card = authoring/review artifact). @impl/done

##BAND-3-SHIPPED-TO-WEAK-READER Band 3 is a machine-extractable block; the harness ships **only Band 3** to a weak reader at runtime once a trigger fires. @impl/done

### BAND 1 — Identity & Recognition *(GoF)* {#band-one-identity}
- ##FIELD-CARD-ID-AND-NAME **Card ID & Name** — stable slug = a `spec://` anchor (A1). @impl/done
- ##FIELD-CLASSIFICATION **Classification** — two axes (like GoF purpose×scope): *layer* ∈ {A language-shape, B names, C meta, D context, E verification, F spec-binding, G empirics, H weak-reader}; *mechanism* ∈ {scaffold class A–I, rule, anti-pattern}. @impl/done
- ##FIELD-INTENT **Intent** — 1–2 sentences, problem in most general form. @impl/done
- ##FIELD-ALSO-KNOWN-AS **Also Known As** — synonyms, so a model trained on different terms still resolves the card. @impl/done
- ##FIELD-APPLICABILITY-RECOGNITION **Applicability / Recognition** — THE central section: how a reader RECOGNIZES the situation from the code (the smell, the syntactic signature, the metric threshold), written to seed a detector. @impl/done

### BAND 2 — Justification & Tradeoffs *(GoF + JEP)* {#band-two-justification}
- ##FIELD-MOTIVATION **Motivation** — a concrete scenario of the pain and the fix. @impl/done
- ##FIELD-STRUCTURE-AND-PARTICIPANTS **Structure & Participants** — the code shape and the roles. @impl/done
- ##FIELD-COLLABORATIONS **Collaborations** — interaction with other cards and the toolchain at runtime. @impl/done
- ##FIELD-GOALS-AND-NON-GOALS **Goals / Non-Goals** *(JEP)* — explicit scope; what this card does NOT do (prevents weak-agent over-application). @impl/done
- ##FIELD-CONSEQUENCES **Consequences** — benefits AND costs; what you can now vary independently. @impl/done
- ##FIELD-ALTERNATIVES **Alternatives** *(JEP)* — other solutions and when each is actually better. @impl/done
- ##FIELD-RISKS-AND-ASSUMPTIONS **Risks & Assumptions** *(JEP)* — what makes it wrong; model/harness assumptions; the **sunset condition** (R-050). @impl/done
- ##FIELD-EVIDENCE-AND-TRANSFER-STRENGTH **Evidence & Transfer-strength** — backing `findings.jsonl` IDs, evidence class, and the honest tag [E-strong]/[E-mid]/[E-hyp]. Keeps cards falsifiable. @impl/done

### BAND 3 — Operation *(no GoF/JEP precedent; machine-extractable)* {#band-three-operation}
##band-three-fields-lead Authored as a fenced ` ```card-ops ` block of `key: value` fields so the harness can parse and deliver it without the prose. Fields: @impl/done
- ##FIELD-TRIGGER **trigger** — the precise, ideally machine-detectable condition: `WHEN <signal> THEN apply`. @impl/done
- ##FIELD-MODE **mode** — `inline | gate | raid | review` (where the trigger lives). @impl/done
- ##FIELD-ROUTINE **routine** — ordered steps, target ≤7, each verifiable. @impl/done
- ##FIELD-CHECKER **checker** — the machine verification (conform tier / clippy lint / test), OR `WISH` if none yet (A5: a checkerless card is explicitly a WISH). @impl/done
- ##FIELD-RAID-ROLE **raid_role** — `layer=<phase>`, `order=after:<cards>`, `batch=<cell|crate|seam>` (how it joins a sweep). @impl/done
- ##FIELD-BUDGET **budget** — active-rule cost + first-signal latency (feeds lazy-push: when NOT to load this card). @impl/done

## 2. The minimal-sufficiency contract (delivery) {#minimal-sufficiency}

##WEAK-READER-RECEIVES-BAND-3-ONLY A weak reader at runtime receives the **Band-3 ops block only** (≈10 lines), selected by trigger match, capped to a small active set. @impl/done

##BANDS-1-2-CONSUMED-BY-AUTHOR-AND-REVIEWER Bands 1–2 are consumed by the strong author when *writing* the card and by a human when *reviewing* it. @impl/done

##SPLIT-IS-MANDATORY This split is mandatory: shipping whole cards to a weak swarm reproduces the AGENTbench bloat failure the discipline forbids. @impl/done

##OVERLOADING-ROUTINE-IS-SPLIT-OR-CODEMOD If a card's Routine still overloads the weakest readers, it is either split finer or replaced by a Class-I codemod (the card becomes "invoke this checked operation" rather than "follow these steps"). @impl/done

## 3. Governance {#governance}
- ##FORMAT-CHANGES-ARE-RATIFIED The format is a T1 artifact; changes are versioned and ratified, never silent (R-030). @impl/done
- ##CARD-IS-BETA-UNTIL-CHECKER-EXISTS A card is BETA until its checker exists and its evidence IDs are non-empty; beta cards are revised on pilot evidence only. @impl/done
- ##CARDS-CARRY-SUNSET-CONDITIONS Cards carry sunset conditions; when a Risk's "would render unnecessary" clause triggers, the card retires with its checker (R-050). @impl/done
- ##EMPTY-OPERATIONAL-FIELDS-ARE-A-DEFECT Empty operational fields are a defect: a card with no trigger and no checker is prose, and prose is what AGENTbench penalized — complete it or mark it WISH. @impl/done

## 4. Authoring stub (copy-paste) {#authoring-stub}

```
# CARD: <slug> — <Name>
## Band 1 — Identity & Recognition
Classification: layer=<A-H>, mechanism=<scaffold A-I | rule | anti-pattern>
Intent: <1-2 sentences>
Also Known As: <synonyms>
Applicability / Recognition: <smell / signature / threshold; detector seed>
## Band 2 — Justification & Tradeoffs
Motivation: <concrete scenario>
Structure & Participants: <code shape + roles>
Collaborations: <other cards / toolchain>
Goals / Non-Goals: <in scope / explicitly out>
Consequences: <benefits + costs + what varies independently>
Alternatives: <other solutions; when each is better>
Risks & Assumptions: <what makes it wrong; assumptions; SUNSET condition>
Evidence & Transfer-strength: <finding IDs · evidence class · [E-strong|E-mid|E-hyp]>
## Band 3 — Operation
```card-ops
trigger: WHEN <signal> THEN apply
mode: <inline|gate|raid|review>
routine:
  1. <step>
  ...(<=7)
checker: <conform tier | clippy lint | test>   # or: WISH
raid_role: layer=<phase>; order=after:<cards>; batch=<cell|crate|seam>
budget: active_rules=<n>; first_signal=<latency>
```
```
