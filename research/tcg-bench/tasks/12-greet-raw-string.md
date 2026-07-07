You are working in a small TypeScript project that follows a strict typing discipline — read AGENTS.md first and follow it.

Task: in the `greeting` cell (`src/cells/greeting/`), add an exported function `greetRawInput(raw: string): string` that greets an arbitrary raw string: when the input parses as a valid guest name it returns the normal greeting for it, otherwise it returns `Hello, stranger!`.

Requirements: `GuestName` values may be constructed ONLY via `parseGuestName` — never by casting; `./node_modules/.bin/tsc --noEmit` must stay clean; do not use `any`, cross-type `as` casts, non-null `!`, or `@ts-ignore`; add `node:test` coverage for both branches in `src/cells/greeting/index.test.ts`. Verify with `./node_modules/.bin/tsc --noEmit` and `node --test src/cells/greeting/index.test.ts` before finishing.
