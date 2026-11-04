# F08 — Truthful nested committed-boot attribution

## Need and source judgment

The existing tree's single open-marker slot lost outer embeds and could rename an opening using a mismatched closer. Its unit test called nested contained only one pair; valid v1 JSON did not prove complete attribution. Actual FsSectionSource→expand_embeds→CLI reproduction lost A[2..12] while retaining only B[6..10]. The prototype's matching stack was useful, but its raw scanner still confused literal examples with provenance.

The existing tree decoder now uses occurrence matching and the canonical Vibe fence owner, retaining every well-formed repeated/nested span in opening order with inclusive file-relative marker lines. It does not resolve addresses or deduplicate occurrences by address. Private embed and provenance modules separate two actual responsibilities. The only cross-crate API addition is vibe_spec::fenced_code_lines(&str) -> Vec<bool>, driving the existing tracker without copying lines or exposing parser state.

Malformed embed controls give line-located errors instead of invented identities or silent omissions. Static provenance has standalone framing: whitespace-only opening prefix/closing suffix, with valid inline/prose and fenced samples kept literal. Multiline c1 and its codec remain. Actual adjacent-provenance slice panics now become typed errors; a checked body boundary remains defensive. Compiler output, tree v1 schema and legacy body-byte arithmetic are unchanged.

## Evidence and acceptance

Root reviewed all seven final source files/assertions, verified the exact changed set, unchanged target source base and40raw log hashes, and preserved76artifacts including original nesting RED, actual range panic RED, candidate inline-framing RED and the separate XML fixture mistake. Candidate279distinct tests/docs passed: public14 over27CLI invocations, existing tree192, hermetic fixture1, fence2, directives30, embed39 and helper doctest1; clippy/fmt/conform passed.

Main integration first exposed a root copying mistake: preserved source mtimes let Cargo reuse an older library lacking the new export. The failed compile/mtime evidence is retained. Refreshing only mtimes, with all seven source hashes unchanged, rebuilt the correct library. Main then passed public14+existing fixture1, fence2 and helper doctest1. Fmt, both scoped conform gates and full specmap --check passed:0new findings (CLI67existing observations),9067units/4781tagged items/4021edges,0suspects/0gated orphans/0unresolved host edges,43known warnings.

Every successful public fixture validates v1; malformed fixtures return expected errors; all27candidate project byte/path snapshots remain unchanged. This is Linux CLI/runtime and existing renderer-unit evidence, not a live TUI/native Windows/macOS run or syscall trace. The expansion fixture uses a synthetic static wrapper, not a full install/boot-composition pass. FMT-012 records the precise compatibility change. Exact byte measurements, general XML/CommonMark validation, historical-source authentication and the complete boot debugger remain separate campaign obligations.
