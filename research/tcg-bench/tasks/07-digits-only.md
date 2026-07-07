You are working in a small TypeScript project that follows a strict typing discipline — read AGENTS.md first and follow it.

Task: extend `parseGuestName` in `src/cells/greeting/index.ts` so that a name consisting ONLY of digits (after normalisation) is rejected with the existing `ParseError` kind `"unprintable"` and the reason `digits-only`.

Requirements: `./node_modules/.bin/tsc --noEmit` must stay clean; do not use `any`, cross-type `as` casts, non-null `!`, or `@ts-ignore`; keep `parseGuestName` the only constructor of `GuestName`; add `node:test` coverage (a digits-only rejection and a mixed alphanumeric acceptance) in `src/cells/greeting/index.test.ts`. Verify with `./node_modules/.bin/tsc --noEmit` and `node --test src/cells/greeting/index.test.ts` before finishing.
