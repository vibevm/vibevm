# F07 — Code-span quotations do not define fact anchors

## Meaning and source judgment

The prototype's narrow correction8a4c096f removes a real inconsistency in the existing main parser: after canonical parsing masked inline code, a second raw scan counted definition-looking quotations again. This could reject legitimate documents, conversions and fact scans. Historical B-092 deliberately introduced that diagnostic convention; the current PROP-043 code-opacity/first-token grammar says otherwise. The history is retained with a dated compatibility correction, not erased or described as accidental.

Duplicate checking now takes definitions only from parsed heading/fact units. The raw scan, unused token helper and raw-text plumbing are removed. The existing lexer, fence/inline masking, typed-fact and swallowed/missing-anchor checks remain. Real duplicate addresses still fail. The single documentation tutorial exception caused by the false diagnostic is removed; unrelated corpus counts, versions and exceptions are unchanged.

## Ordinary cached users

The first green candidate tested CLI with --no-cache only. Root found that unchanged source could still retrieve a schema3 ParsedDoc carrying the old false error. A real old-main scan reproduced that gap: corrected parser with old payload gave warm1/fresh0; genuine duplicates gave1/1. The final change advances only erasable PAYLOAD_SCHEMA3→4 through the existing foreign-schema-as-miss loader. Normal warm/fresh checks now agree. Read-only checks preserve source, sidecar and tracked cache bytes; ordinary scan regenerates4 while preserving CACHE_SCHEMA2 and verdict/evidence maps. No authored migration or manual cache cleanup is needed.

## Evidence and acceptance

Final ten-file candidate has233distinct selected test functions passing, with actual parser, conversion, facts and CLI RED→GREEN plus the warm-cache amendment. Root reviewed every production change and focused assertion, verified full changed-path/SHA256 sets and archived original/final evidence and the genuine old payload fixture. The invalid zero-selected scan filter and probe-harness mistakes are retained separately from product REDs.

Integrated main:7new library-consumer cases,5actual CLI cases (including warm checks/refresh), and1existing sidecar CLI control pass. Fmt/diff, four scoped conform gates and specmap --check pass:0new conform findings (CLI67existing observations),9067units/4765tagged items/4005edges,0suspects/0gated orphans/0unresolved host edges,43known warnings. Exact final ten source hashes match the reviewed candidate. Linux execution only; no full platform/workspace or ordinary-document classification acceptance is implied. FMT-013 records the diagnostic and derived-cache format change.
