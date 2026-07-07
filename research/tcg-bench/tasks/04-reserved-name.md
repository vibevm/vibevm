You are working in a small TypeScript project that follows a strict typing discipline — read AGENTS.md first and follow it.

Task: extend `parseGuestName` in `src/cells/greeting/index.ts` so that the name `admin` (case-insensitive, after normalisation) is rejected with a NEW `ParseError` kind `"reserved"` and a human-readable reason. Extend the `ParseError` type accordingly.

Requirements: `./node_modules/.bin/tsc --noEmit` must stay clean (update any exhaustive handling the new kind breaks); do not use `any`, cross-type `as` casts, non-null `!`, or `@ts-ignore`; keep `parseGuestName` the only constructor of `GuestName`; add `node:test` coverage for the accepted/rejected cases in `src/cells/greeting/index.test.ts`. Verify with `./node_modules/.bin/tsc --noEmit` and `node --test src/cells/greeting/index.test.ts` before finishing.
