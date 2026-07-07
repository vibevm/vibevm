You are working in a small TypeScript project that follows a strict typing discipline — read AGENTS.md first and follow it.

Task: in the `farewell` cell (`src/cells/farewell/`), add an exported function `farewellAll(names: GuestName[]): string` that joins the individual farewell lines with `; `. Import `GuestName` only through the greeting cell's seam.

Requirements: `./node_modules/.bin/tsc --noEmit` must stay clean; do not use `any`, cross-type `as` casts, non-null `!`, or `@ts-ignore`; add a `node:test` test (empty list gives an empty string; two names join correctly) in `src/cells/farewell/index.test.ts`. Verify with `./node_modules/.bin/tsc --noEmit` and `node --test src/cells/farewell/index.test.ts` before finishing.
