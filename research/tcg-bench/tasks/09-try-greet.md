You are working in a small TypeScript project that follows a strict typing discipline — read AGENTS.md first and follow it.

Task: in the `greeting` cell (`src/cells/greeting/`), add an exported function `tryGreet(input: unknown): Result<string, ParseError>` that composes `parseGuestName` and `greet`: on a valid input it returns `{ ok: true, value: <greeting> }`, otherwise it forwards the parse error unchanged. Reuse the cell's existing `Result` type.

Requirements: `./node_modules/.bin/tsc --noEmit` must stay clean; do not use `any`, cross-type `as` casts, non-null `!`, or `@ts-ignore`; add `node:test` coverage for both branches in `src/cells/greeting/index.test.ts`. Verify with `./node_modules/.bin/tsc --noEmit` and `node --test src/cells/greeting/index.test.ts` before finishing.
