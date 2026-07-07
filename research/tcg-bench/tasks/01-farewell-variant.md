You are working in a small TypeScript project that follows a strict typing discipline — read AGENTS.md first and follow it.

Task: in the `farewell` cell (`src/cells/farewell/`), add an exported function `farewellFor(name: GuestName): string` that returns `Goodbye, <name>!`. Import `GuestName` only through the greeting cell's seam (`../greeting/index.ts`).

Requirements: `./node_modules/.bin/tsc --noEmit` must stay clean; do not use `any`, cross-type `as` casts, non-null `!`, or `@ts-ignore`; add a `node:test` test for the new function in `src/cells/farewell/index.test.ts`. Verify with `./node_modules/.bin/tsc --noEmit` and `node --test src/cells/farewell/index.test.ts` before finishing.
