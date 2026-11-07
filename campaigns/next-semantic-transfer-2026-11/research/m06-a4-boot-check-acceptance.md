# A4 bound boot graph checking — accepted adapter

Accepted in `04e7ccad8ead2002fd49d936e6917b8631e7006a`; root verified10committed blobs. Original A4 remains partial and counts stay12/31/31/39.

The real check command now supplies current project/operator materialized format to an additive bound verifier. It reuses durable selection, source bindings, mapped unit paths and format-aware plan fingerprints with the existing stale/pending owner. The old unbound wrapper preserves its contract; no format is inferred from a record or suffix.

Missing lock remains non-applicable and does not load irrelevant configuration. Stale/missing unit INDEX remains Warning and warning-only exit0. Applicable load/configuration/lock/binding failures now yield BootGraphIntegrity Error, JSON ok=false and error exit through normal report emission. The prior actual CLI silently returned ok:true/findings:[] for corrupt declared INDEX and format drift; the RED→GREEN regression proves this behavior change. Check remains read-only with no lifecycle lease, regeneration or new schema/option.

CLI6, direct2, legacy/activated2 and corrected native7 pass; snapshots cover project directories/bytes and operator config. The earlier native filter ran0 and is not coverage. Strict workspace/targeted CLI Clippy,3Conform scopes, fmt/diff pass. Root map/check:9162units/5539items/4688edges, zero orphan/unresolvedhost gates,44existing warnings; all9160prior occurrences retained. One warning simply moved with the unchanged legacy verifier pin into its new module. Wire-diff passes; no codec generation required.

Compile-only helper split/import/ContentHash mistakes and a test-only import warning were corrected without weakening gates. FMT-035 records new visible error/exit behavior. This checks incumbent INDEX fingerprints and pending semantics, not arbitrary STATIC-body authenticity. Broader host checker support and other facts/docs/skills/publish/pattern consumers remain A4 work.
