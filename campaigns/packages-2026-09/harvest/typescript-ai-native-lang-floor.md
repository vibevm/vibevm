# typescript-ai-native-lang — floor

_Captured 2026-07-28 against `packages/org.vibevm.ai-native/typescript-ai-native-lang/v0.6.0/`._

```console
$ typescript-ai-native floor --keep-going
ℹ tests 0
ℹ suites 0
ℹ pass 0
ℹ fail 0
ℹ cancelled 0
ℹ skipped 0
ℹ todo 0
ℹ duration_ms 6.2233

=== prettier --check . ===
floor: `prettier` is not installed in this project — `npm install -D prettier` (or disable the step with a reason in conform.toml [typescript].floor_disable)
floor: `prettier` FAILED

=== tsc --noEmit ===
floor: `tsc` is not installed in this project — `npm install -D typescript` (the structural gate needs it too)
floor: `tsc` FAILED

=== tests (node --test) ===

=== eslint . ===
floor: `eslint` is not installed in this project — `npm install -D eslint typescript-eslint` (or disable the step with a reason in conform.toml [typescript].floor_disable)
floor: `eslint` FAILED

=== typescript-ai-native-conform check ===
typescript-ai-native-conform: NO conform.toml — topology default in force (roots = ["src"], no cells gate); run `typescript-ai-native init` to write a starting policy.
violates REQ discipline://typescript-ai-native-lang/guide#tooling: the project cannot resolve `typescript` ((node:48832) [MODULE_TYPELESS_PACKAGE_JSON] Warning: Module type of file:///C:/Users/olegc/git/v/vibevm/packages/org.vibevm.ai-native/typescript-ai-native-lang/v0.6.0/target/conform/ts-extract/extract-f678dc7748b4054a.ts is not specified and it doesn't parse as CommonJS.
Reparsing as ES module because module syntax was detected. This incurs a performance overhead.
To eliminate this warning, add "type": "module" to \\?\C:\Users\olegc\package.json.
(Use `node --trace-warnings ...` to show where the warning was created)
ts-extract: cannot resolve `typescript` from `C:\Users\olegc\git\v\vibevm\packages\org.vibevm.ai-native\typescript-ai-native-lang\v0.6.0`. The structural gate parses with the project's own compiler — run `npm install -D typescript` (the tsc floor step needs it too).); fix surface: `npm install -D typescript` in the project root — the tsc floor step needs the same install
floor: `conform` FAILED

=== typescript-ai-native-specmap --check ===
typescript-ai-native-specmap: NO specmap.toml — placeholder namespace `project` in force and the orphan gate is off; run `typescript-ai-native init` to write a starting policy.
violates REQ discipline://typescript-ai-native-lang/guide#tooling: the project cannot resolve `typescript` ((node:47656) [MODULE_TYPELESS_PACKAGE_JSON] Warning: Module type of file:///C:/Users/olegc/git/v/vibevm/packages/org.vibevm.ai-native/typescript-ai-native-lang/v0.6.0/target/conform/ts-extract/extract-f678dc7748b4054a.ts is not specified and it doesn't parse as CommonJS.
Reparsing as ES module because module syntax was detected. This incurs a performance overhead.
To eliminate this warning, add "type": "module" to \\?\C:\Users\olegc\package.json.
(Use `node --trace-warnings ...` to show where the warning was created)
ts-extract: cannot resolve `typescript` from `C:\Users\olegc\git\v\vibevm\packages\org.vibevm.ai-native\typescript-ai-native-lang\v0.6.0`. The structural gate parses with the project's own compiler — run `npm install -D typescript` (the tsc floor step needs it too).); fix surface: `npm install -D typescript` in the project root — the tsc floor step needs the same install
floor: `specmap` FAILED

floor: no tests baseline at discipline/registry/tests-baseline.json — the test-gate step arms when `typescript-ai-native init` writes it
Error: floor: 5 step(s) failed: prettier, tsc, eslint, conform, specmap
EXIT=1
```

**Scope:** every fact under `packages/org.vibevm.ai-native/typescript-ai-native-lang/v0.6.0/` that this run bears on. The anchor list is not maintained here — a verdict cites this file in its `ev[]`, and the reverse index is derived from the verdict maps at the phase close (PHASE-C-BATCH-PLAN.md §5).
