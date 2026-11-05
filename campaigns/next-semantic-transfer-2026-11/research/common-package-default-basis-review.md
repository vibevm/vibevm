# Package default basis: bounded source review and explicit root policy

Status: read-only source evidence, no tests/builds, source/Git mutations, network or new workers. Prototype reads use only Git blobs at `7fe6376ccef7f321302a8f18610fcb0c9cc2686f` in `/fast/git/v/vibevm-next`; dirty files were not read. Current core and named standing files were read in MAIN. Root accepted the recommendation below as a **NEW explicit policy extension**, not a recovered or runtime-complete prototype behavior. The explicit/no-auto first implementation remains independent and continues under its existing writer.

## Conclusion

Do not bind fresh automatic consent to the highest version before feasibility. That policy was proposed in `/tmp/nst-common-positive-design.md:68,147`; its amendment at151 onward explicitly withheld approval. Neither the prototype's ordinary authored route nor its real shown-default binding caller implements it.

There are two different authorities:

1. **Genuinely fresh, unbound authored permission:** `use-declared-defaults` permits candidate-local interpretation of omitted choice answers. It does not assert that the consumer saw one particular candidate's defaults. Root now authorizes binding its receipt only to the final feasible selected candidate.
2. **Already shown/retained basis:** this is a decision about particular reviewed meaning. Reuse cannot silently renew it. A semantic mismatch requires rediscovery before default-induced effects. Re-reading the same manifest or passing `-y` does not create fresh permission.

## What current ordinary input authorizes

`crates/vibe-core/src/manifest/configuration/intent.rs:18–40` requires the choice policy and separately retains profile, feature additions, `default_features` (default true), selections and public parameters. It contains no version-selection rule or historical default receipt.

`normalize_input`, same file120–142,154–174,199–229, first validates the actual definition/input. It applies only an explicitly named profile, then replaces profile group entries with authored entries as supplied. `Some([])` remains explicit empty; `None` becomes omitted. Feature defaults are independent of choice policy: named profile features win as base; otherwise `default_features=true` selects the declared ordinary feature base. The default-profile marker does not automatically select a profile. Parameters are supplied/profile values here; this function does not fill parameter defaults or bind a candidate (`:43–57,175–198`). Thus `require-selection` alone does not mean “no automatic reliance”; feature/profile/parameter origins still matter.

`crates/vibe-core/src/configuration/choices/evaluate.rs:260–299` chooses explicit members first, including explicit empty. Only omitted answers under `UseDeclaredDefaults` take that group's declared defaults. Absent defaults remain unanswered. Answer strength stays independent: an absent entry is additive; omitted/exact retains exact strength when defaults exist. One consumer's policy does not authorize another consumer.

Core `ChoicePolicy` explicitly grants no effect authority (`choices/types.rs:76–87`). `DefaultUse` is provisional observation, **not a receipt** (`:118–132`); only active groups report these uses (`evaluate.rs:110–121`). Underflow produces questions, conflicting cardinality produces conflicts, and only a fully resolved outcome reaches feature closure (`:123–164`). Consequently a list of observed default uses cannot by itself establish reviewed/historical meaning or cover dormant/absent defaults.

Normative anchors: current PROP-069 lines21–28, especially AUTHORED-INTENT, INPUT-NORMALIZATION, DEFAULT-CONSENT and CANDIDATE-BOUNDARY. The app-scoped precedent at57, APP-CONFIGURATION-BASIS, already allows fresh policy to bind the actual selected definition while retained reuse cannot renew changed meaning. Applying that distinction to package candidate solving is this newly accepted extension, not proof that app code already implemented package solving.

## What the pinned prototype actually does

All paths below refer to the immutable pin, not current MAIN.

| Actual body/caller | Source evidence |
| --- | --- |
| Ordinary authored declaration cannot express this defaults permission | `crates/vibe-core/src/manifest/consumer_configuration.rs:5–17` is strict and has no policy field. `crates/vibe-install/src/authored_configuration.rs:32–46` hardcodes `require_selection`; `:71–75` starts `defaults=[]`. Ordinary planner really calls this at `crates/vibe-install/src/plan.rs:360–375`, before transaction capture. This is not merely a prepared API observation. |
| The receipt identifies a particular decision | `crates/vibe-resolver/src/resolvo_engine/configuration.rs:149–178`: `DefaultBinding::shown` stores edge, actual candidate identity, full shown definition and request. It does not choose a version. |
| Real production binding is an explicit shown-answer operation | `crates/vibe-orchestrator/src/machine_operation/configuration_choice.rs:245–256` rejects unshown consent in the initial request. `:331–373` verifies shown identity, requires explicit `allow_defaults`, requires exactly one matching admitted candidate, then constructs the binding. It never silently selects the highest of multiple matches. |
| Its discovery basis is admitted, not highest-version selection | `crates/vibe-orchestrator/src/document_analysis/catalog.rs:54–118,149–168` checks explicit admissions against captured bytes/identity and retains those admitted candidates. No solver or default-selection rule is run there. |
| Actual candidate enforcement precedes automatic implications | `resolvo_engine/configuration.rs:511–547` compares binding identity, full definition and request. If default-governed input is unbound/stale, it removes default authority before evaluating the candidate. `:572–612` marks Rediscovery/NeedsSelection rather than manufacturing an answer. |
| Only selected unresolved state surfaces | Same file689–779 solves the graph, collects needs/rediscovery from selected nodes, and returns an error before a ConfiguredResolution with effects can escape. Rejected candidates do not become selected receipts. |
| Historical test demonstrates a bound decision, not automatic highest policy | `resolvo_engine/m_05_c_2_cases.rs:375–425` explicitly constructs a binding for definitions[1], observes older selected version needs rediscovery, then explicitly binds definitions[0]. That caller-supplied choice must not be reinterpreted as automatic “highest first.” These assertions were read, not executed here. |

Pinned search for `DefaultBinding::shown` across `crates` found the above production machine-operation caller; other occurrences were test fixtures. This, plus the constructor/private fields and ordinary authored path, is the decisive boundary. Generic solver newest-version preference is not semantic consent and cannot supply a missing shown-definition decision.

## Newly accepted policy: exact behavior

**Fresh unbound operation.** Evaluate each candidate against the same original consumer inputs, retaining automatic contributions only within that candidate. Ordinary solver feasibility may reject a candidate using its own declared default-induced dependencies. No provisional candidate observation is installed as a global/current historical basis. After final feasible selection and complete admission, persist only the selected candidate's original input, typed automatic reliance and corresponding semantic basis. This is initial authorization, not renewal of a previously shown answer.

**Shown, retained or split-client operation.** Preserve the existing reviewed basis per real consumer. Compare relevant semantic meaning before using automatic selections/parameters/profile meaning. Mismatch produces rediscovery; do not let the candidate's new default supply internal implications and then claim the resulting graph was authorized. Presentation-only changes and fully explicit/no-auto compatible answers remain distinct from automatic reliance; conservative whole-semantic-definition comparison is allowed by the current contract. Expected candidate identity from split interaction must match its actual use. A failure/retry does not promote provisional evaluation into history or erase real history. Unchanged lexical authored policy, a new process and `-y` are not fresh setters/review.

**Malformed, unanswered and incompatible are different.** Invalid source definitions/local references are validation errors, not ordinary SAT reasons to try older broken packages until one parses. A mandatory omitted answer with no authorized default yields NeedsSelection/questions, no executable effects; do not label that candidate “incompatible” solely to search for a version that chooses for the user. Legitimate candidate incompatibility, such as an unsatisfiable authorized dependency requirement or an explicit stable selection incompatible with that candidate, may cause backtracking while preserving that exact original intent. Independently impossible candidates need not force questions from rejected branches: only the selected unresolved candidate asks. Defaults policy is permission to use declared defaults, not permission to suppress required questions or ignore conflicting exact input.

### Discriminating highest-before-feasibility counterexample

A fresh authored edge allows suite1 or2, has `use-declared-defaults`, and no shown/history basis. Suite2's declared default is `fast`, activating an unavailable dependency; suite1's declared default is `small`, with a satisfiable graph. The newly accepted policy allows suite1/small, retaining only suite1's receipt. Binding suite2 first would invent a “shown fast” decision from an infeasible candidate and force a rediscovery the consumer never caused. That is precisely why semver preference cannot mint consent.

Repeat with a real earlier shown/retained suite2/fast basis: suite1/small must require rediscovery, not silently become a successful answer. Repeat with exact explicit `fast`, no automatic inputs: preserve that hard answer and fail/conflict if no candidate can satisfy it; never substitute small. Repeat with suite2 merely missing a mandatory answer but otherwise feasible: return questions rather than selecting suite1 solely to avoid asking. These are proposed acceptance discriminators; none was run during this source review.

This decision does not complete automatic package runtime support, defaults persistence/replay, cross-consumer forwarding or generic effect delivery. It removes one policy ambiguity while the authorized explicit/no-auto first path proceeds.

## Source binding

Prototype SHA values below are computed from `git show <pin>:<path>`. MAIN values capture the reviewed files at report generation; no claim of a runtime gate accompanies them.

| Source | Path | SHA256 |
| --- | --- | --- |
| MAIN | `vibevm/vibespecs/boot/90-user.xml` | `aed268136df3c0f22eea34c864529397915e520af7bbd64c00a03f7b79ae54ec` |
| MAIN | `vibevm/vibespecs/common/PROP-069-common-configuration.xml` | `6b417a62f0c93de3da90ad77b8ef0b910064e8c38fe66423765841b4f0f9c148` |
| MAIN | `campaigns/next-semantic-transfer-2026-11/research/common-package-positive-decisions.md` | `6882785be426da6c3c144aed6c25d816aa7642c2ff6f13a2f6509e7f475ee907` |
| MAIN | `crates/vibe-core/src/manifest/configuration/intent.rs` | `0ea07758d744116c755a5ac78d60b2788da17160c9e2e997dadf9795461b6c83` |
| MAIN | `crates/vibe-core/src/configuration/choices/types.rs` | `4d3af0bcfa5700edf4444bae002c1bcc5682bb943226da629f9170f9dd772a80` |
| MAIN | `crates/vibe-core/src/configuration/choices/evaluate.rs` | `5572c229891593d9290cce7f22f07651fd2e1282270469b2eab8a15fb6f5ca1b` |
| PIN | `crates/vibe-core/src/manifest/consumer_configuration.rs` | `fc0540919e48c92eef15aae957420b6eccc820ade940092a3faea540d44105ce` |
| PIN | `crates/vibe-install/src/authored_configuration.rs` | `2e4bfa7858f77b3fb4f30c5363d39b2c540cb9a5c5554833dfd43687980ecb51` |
| PIN | `crates/vibe-install/src/plan.rs` | `1495b9b494c3f1c491d9aec3a408fbbed5a9c72884df24e108b014a39a143fea` |
| PIN | `crates/vibe-resolver/src/resolvo_engine/configuration.rs` | `b13c7516929498a3e0d314b4e3648865b89301ca757d91eb71b5b1e294ea6ab7` |
| PIN | `crates/vibe-resolver/src/resolvo_engine/m_05_c_2_cases.rs` | `7833dca3211e4de0c016842cdad5688e50d264f3a70dc34b027182fb5d36e477` |
| PIN | `crates/vibe-orchestrator/src/machine_operation/configuration_choice.rs` | `9fa0f7631e2f123af99b70c950360c18ae73456e2c70dd5bd9edc5dc8fe00e22` |
| PIN | `crates/vibe-orchestrator/src/document_analysis/catalog.rs` | `da955bf662e4e4419483d18ef697feea10408ff8aa110e78a1912cf7760a2506` |
