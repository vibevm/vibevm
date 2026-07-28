# rust-ai-native-mcp — health

_Captured 2026-07-28 against `packages/org.vibevm.ai-native/rust-ai-native-mcp/v0.7.0/`._

```console
$ rust-ai-native health
=== Discipline health (rust-ai-native health) ===
gated: 0 | exempt: 0 | pub-doctest-gated: 0
conform baseline: 0 frozen
file-length: 0 over budget (>600) | 0 in danger band [540,600]
deviation debt: 0 fn-grain #[spec(deviates)] site(s)
pub-doctest promotion candidates: none (no gated crate is at full type coverage)
pub-doctest drain backlog: empty — every gated crate's types are documented
wrote discipline/health/latest.json
conform: NO conform.toml — topology default in force, nothing is gated; run `rust-ai-native init` to write a starting policy.
EXIT=0
```

**Scope:** every fact under `packages/org.vibevm.ai-native/rust-ai-native-mcp/v0.7.0/` that this run bears on. The anchor list is not maintained here — a verdict cites this file in its `ev[]`, and the reverse index is derived from the verdict maps at the phase close (PHASE-C-BATCH-PLAN.md §5).
