You are working in a small TypeScript project that follows a strict typing discipline — read AGENTS.md first and follow it.

Task: in the `farewell` cell (`src/cells/farewell/`), add an exported function `farewellCount(names: GuestName[]): string` that says farewell to every name (reusing the cell's existing single-name logic) and returns the summary line `Goodbyes sent: <n>` where `<n>` is the number of names.

Requirements: `./node_modules/.bin/tsc --noEmit` must stay clean; do not use `any`, cross-type `as` casts, non-null `!`, or `@ts-ignore`; add a `node:test` test in `src/cells/farewell/index.test.ts`. Verify with `./node_modules/.bin/tsc --noEmit` and `node --test src/cells/farewell/index.test.ts` before finishing.
