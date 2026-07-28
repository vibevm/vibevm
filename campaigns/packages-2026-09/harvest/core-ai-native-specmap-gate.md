# core-ai-native — specmap gate

_Captured 2026-07-28 against `packages/org.vibevm.ai-native/core-ai-native/v0.8.0/`._

The package has no umbrella CLI, so this is not a `floor` step. The command is
the one **this package's own `specmap.toml` names**: the gate is engine code and
lives here, while the binary that runs it ships in `rust-ai-native-lang`.

```console
$ rust-ai-native-specmap --gate --path packages/org.vibevm.ai-native/core-ai-native/v0.8.0
specmap: ratchet gate — 0 gated orphan(s), 0 dispositioned (2 crate(s) exempt).
EXIT=0
```

**Scope:** every fact under `packages/org.vibevm.ai-native/core-ai-native/v0.8.0/` that this run bears on. The anchor list is not maintained here — a verdict cites this file in its `ev[]`, and the reverse index is derived from the verdict maps at the phase close (PHASE-C-BATCH-PLAN.md §5).
