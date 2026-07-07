You are working in a small TypeScript project that follows a strict typing discipline — read AGENTS.md first and follow it.

Task: in the `farewell` cell (`src/cells/farewell/`), add an exported function `farewellPolite(name: GuestName): string` that returns the greeting cell's `greet(name)` followed by ` …and goodbye.` — one string.

Requirements: import the greeting cell ONLY through its seam (`../greeting/index.ts`), never its internal files; `./node_modules/.bin/tsc --noEmit` must stay clean; do not use `any`, cross-type `as` casts, non-null `!`, or `@ts-ignore`; add a `node:test` test in `src/cells/farewell/index.test.ts`. Verify with `./node_modules/.bin/tsc --noEmit` and `node --test src/cells/farewell/index.test.ts` before finishing.
