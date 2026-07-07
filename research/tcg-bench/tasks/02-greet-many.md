You are working in a small TypeScript project that follows a strict typing discipline — read AGENTS.md first and follow it.

Task: in the `greeting` cell (`src/cells/greeting/`), add an exported function `greetMany(names: GuestName[]): string[]` that returns the greeting for each name, in order.

Requirements: `./node_modules/.bin/tsc --noEmit` must stay clean; do not use `any`, cross-type `as` casts, non-null `!`, or `@ts-ignore`; add a `node:test` test covering the new function (at least the empty list and a two-name list) in `src/cells/greeting/index.test.ts`. Verify with `./node_modules/.bin/tsc --noEmit` and `node --test src/cells/greeting/index.test.ts` before finishing.
