# typescript-ai-native-mcp — specmap-gate

_Captured 2026-07-28 against `vibevm/vibepacks/org.vibevm.ai-native/typescript-ai-native-mcp/v0.6.0/`._

```console
$ typescript-ai-native specmap --gate
Error: violates REQ discipline://typescript-ai-native-lang/guide#tooling: the project cannot resolve `typescript` ((node:48768) [MODULE_TYPELESS_PACKAGE_JSON] Warning: Module type of file:///C:/Users/olegc/git/v/vibevm/packages/org.vibevm.ai-native/typescript-ai-native-mcp/v0.6.0/target/conform/ts-extract/extract-f678dc7748b4054a.ts is not specified and it doesn't parse as CommonJS.
Reparsing as ES module because module syntax was detected. This incurs a performance overhead.
To eliminate this warning, add "type": "module" to \\?\C:\Users\olegc\package.json.
(Use `node --trace-warnings ...` to show where the warning was created)
ts-extract: cannot resolve `typescript` from `C:/Users/olegc/git/v/vibevm/vibevm/vibepacks/org.vibevm.ai-native/typescript-ai-native-mcp/v0.6.0`. The structural gate parses with the project's own compiler — run `npm install -D typescript` (the tsc floor step needs it too).); fix surface: `npm install -D typescript` in the project root — the tsc floor step needs the same install
EXIT=1
```

**Scope:** every fact under `vibevm/vibepacks/org.vibevm.ai-native/typescript-ai-native-mcp/v0.6.0/` that this run bears on. The anchor list is not maintained here — a verdict cites this file in its `ev[]`, and the reverse index is derived from the verdict maps at the phase close (PHASE-C-BATCH-PLAN.md §5).
