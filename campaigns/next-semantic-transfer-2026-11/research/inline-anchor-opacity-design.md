# Inline-code anchor opacity through the existing parsed-definition owner

Root decision,2026-11-04. Current PROP-043 PARSE-MARKDOWN-FIRST and PARSE-FACT-ANCHORS explicitly make code spans/fences opaque. Main's B-092 diagnostic deliberately overrides that by comparing raw and inline-masked tokens and inventing additional definitions inside code. The prototype commit8a4c096f305479f9c56e86db6c7421956174ba24 removes that second scan. The history and actual consumer chain are recorded in history-ordinary-documents-review.md, D6.

## Actual need and prior policy

Root ran the existing main CLI's `progress check --no-cache --json --offline --unattended` against four isolated synthetic projects. A real `@fact:SAME` definition plus `@fact:OTHER see ` followed by a code-span quotation of `@fact:SAME` exits1 with DuplicateId. A quoted legacy `##SAME` reference passes; two real SAME definitions exit1; a fenced SAME example passes. All four project path/byte snapshots remain unchanged. Raw input/output/argv, source hashes and CLI hash are retained in local evidence nst-inline-anchor-cli-reproduction.json. This is a focused current-binary observation; no full-HEAD rebuild, syscall trace or native-platform claim is made.

The first probe harness incorrectly assumed stdout was JSON. This command prints a diagnostic there and an error envelope on stderr. The corrected probe preserves both streams; that output contract was not changed or adjudicated by this work.

B-092 remains historical evidence: an August20 lexical duplicate concern led to counting definition-looking quotations as definitions, although no wrong-address consumer had been demonstrated in that record. That convention conflicts with the current permanent first-token/opacity grammar and later makes legitimate tutorial conversion fail. Root chooses the current grammar; the behavioral change and historical policy remain explicit.

## Change and compatibility

The parser's real unit and fact anchors are the sole definition set for duplicate checking. Remove only the additional raw qualified-token scan and its now-unused helper/plumbing. Keep the existing canonical anchor lexer and inline/fence masking algorithms unchanged. Do not introduce CommonMark/source-descriptor dependencies or another lexer, and do not hide all text after unmatched backticks.

Actual parsed definitions across headings/facts/typed facts share the same namespace and still reject duplicate IDs with useful line diagnostics. Marked units still require anchors; swallowed-anchor and typed-fence adjacency checks remain. A definition-looking token inside a code span or fence remains ordinary literal content and cannot add an address or create DuplicateId. Legacy literal citations remain compatible.

No authored syntax, vibe.toml key, storage epoch, dependency or public Rust signature changes are needed. The diagnostic behavior changes: documents formerly rejected solely for quoting an existing anchor now pass. No document rewrite or migration is required. Record this change in FMT-013 and, after acceptance, add a dated supersession note to the existing B-092 record without erasing its historical rationale. Do not transplant prototype documentation version/count changes or unrelated corpus exceptions.

## Required proof and acceptance

Use focused meaningful tests through actual parse_document, public from_markdown/conversion and an existing public fact/CLI consumer. Positive cases include inline and typed-looking quotations, table-cell quotations and existing fences; negative controls retain genuine duplicate IDs, heading/fact collisions, marked missing anchors and unmatched/mixed-width backtick behavior. Assert parsed anchors/counts and preserved literal content, not merely absence of an error. Repeat the actual progress-check discriminator against the candidate-built CLI with read-only project snapshots.

Review current main corpus expectations rather than importing prototype counts; change only the expectation actually invalidated by this grammar correction. Run affected parser/SpecDoc/fact tests, the bounded public CLI target, scoped clippy/fmt/conform and root's final map check. No new public API means no boilerplate wrapper solely for traceability. Root reviews exact changed bodies and assertions, archives raw before/after evidence, integrates atomically and removes the isolated task copy at close. The broader ordinary-document/descriptor feature remains separate and required.
