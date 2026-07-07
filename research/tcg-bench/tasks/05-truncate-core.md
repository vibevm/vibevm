You are working in a small TypeScript project that follows a strict typing discipline — read AGENTS.md first and follow it.

Task: add an exported function `truncate(s: string, max: number): string` to `src/core/text.ts` that returns `s` unchanged when its length is <= `max`, and otherwise the first `max` characters followed by `…`. Then use it in the `farewell` cell so farewell lines never render a name longer than 20 characters.

Requirements: `./node_modules/.bin/tsc --noEmit` must stay clean; do not use `any`, cross-type `as` casts, non-null `!`, or `@ts-ignore`; add `node:test` tests for `truncate` (boundary: exactly `max`) and for the farewell behaviour. Verify with `./node_modules/.bin/tsc --noEmit` and `node --test src/cells/farewell/index.test.ts` before finishing.
