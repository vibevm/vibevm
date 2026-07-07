You are working in a small TypeScript project that follows a strict typing discipline — read AGENTS.md first and follow it.

Task: create a new cell `src/cells/announce/` with its public seam `index.ts` exporting `announce(name: GuestName): string` that returns `Attention: <greeting>` where `<greeting>` is the result of the greeting cell's `greet(name)`.

Requirements: cells may import other cells ONLY through their `index.ts` seam; `./node_modules/.bin/tsc --noEmit` must stay clean; do not use `any`, cross-type `as` casts, non-null `!`, or `@ts-ignore`; add a `node:test` test in `src/cells/announce/index.test.ts`. Verify with `./node_modules/.bin/tsc --noEmit` and `node --test src/cells/announce/index.test.ts` before finishing.
