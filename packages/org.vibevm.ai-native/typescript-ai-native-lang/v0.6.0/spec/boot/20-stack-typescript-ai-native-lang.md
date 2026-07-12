# AI-Native TypeScript (Discipline v0.2) — boot snippet

TypeScript code in this project follows the AI-Native TypeScript guide
(`typescript/GUIDE-AI-NATIVE-TYPESCRIPT.md` in this package). Read the
guide when authoring or reviewing structure; per-edit work needs only
the card whose trigger fires.

Card registry for TypeScript: `cards/INDEX.md` in this package (trigger →
card; the nine executable scaffolds A–I in their TypeScript shape). This
stack ships its own `cards/` projection — the weak-reader runtime surface
for `.ts` edits is a TypeScript card's Band-3 ops block, never the Rust
one. (The core `cards/` are the Rust pilot's reference set; a future
symmetry pass may unify both languages' Band-3 in the core — see the
guide's tooling note.)

Standing rules at the surface level:

- Ordinary idiomatic TypeScript at the token level — no invented dialect,
  no type-level wizardry in domain code (deep conditional/recursive types
  are the OOD tail, treated like `unsafe`). Strictness lives in the
  envelope: the maxed `tsconfig`, branded types at seams, runtime
  validators at the erasure boundary, `spec://` metadata, per-cell fast
  verification (`tsc --noEmit -p <cell>` + `vitest run <cell>`, < ~60s).
- The compiler is a configurable verifier: the project `tsconfig` is a
  versioned discipline artifact at the strictest practical floor
  (`strict` + `noUncheckedIndexedAccess` + `exactOptionalPropertyTypes` +
  `erasableSyntaxOnly`); loosening a mandatory flag needs
  `deviates` + reason.
- Cells: one cell = one file-set with a single public entry (`index.ts`
  as the seam); cells import seams + core only, never sibling cells'
  internals. No barrel-file ambient re-export sprawl; ambient coupling
  (module-level mutable singletons, `declare global`) is forbidden.
- Types are erased and can be lied to. The `unsafe` set —
  `any` / unchecked `as` / `!` / `@ts-ignore` — is forbidden in domain
  code; escape hatches are `unknown` + a runtime validator, `as` only
  after a check, an `asserts` function, and `@ts-expect-error -- reason`
  (never `@ts-ignore`). Each deviation carries a recorded reason.
- Structural typing is recovered to nominal by branding: a meaning-bearing
  primitive crossing a seam is a branded type, so the wrong same-shaped
  value fails `tsc`.
- Failure on a seam is a typed value (`Result<T, E>` / discriminated
  union), never an untyped `throw`; the `E` union cites `spec://` REQs and
  its exhaustive `switch` is compiler-checked.
- Untyped external data (network, JSON, `process.env`, user input) enters
  as `unknown` and is narrowed only through a single-source schema
  (Zod/Valibot) that is both the static type and the runtime check.
- Every public seam carries one type-checked example (Twoslash) of
  canonical use, and public generic/branded/union surfaces carry a
  type-level test (`expectTypeOf`/`tsd`). Replacing a non-trivial cell
  requires a differential oracle (`fast-check`).
- Uniformity is load-bearing: one idiom per operation; exceptions are
  marked, or they propagate as false training signal.

The shipped toolchain (this stack materialises it; no dev tree needed):
`typescript-ai-native` — `init` (bootstrap policies + registries),
`floor` (prettier→tsc→tests→eslint→conform→specmap→test-gate, one exit
code), `health` (the sweep's fact collector), `test-gate` (xfail-strict
over node's TAP) / `tripwire` / `trace` / `fast-loop` / `codemod`; plus
the narrow `typescript-ai-native-conform` and `typescript-ai-native-specmap` engines, and
the agentic type oracle `typescript-ai-native-tcg`
(also served over MCP by `mcp:org.vibevm.ai-native/typescript-ai-native-mcp` —
PROP-027; persistent enriching `serve`
relay + one-shot `validate`/`scope`/`complete`/`type`/`bench`: check an
edit against in-memory overlays BEFORE writing it, with the SAME
conform rules as the gate — GUIDE §14, §15 move 5). The
structural gate parses through the PROJECT's own `typescript` install
(node ≥ 22.6; the same devDependency the tsc floor step needs). Run
vibe-natively (`vibe bin exec typescript-ai-native -- <args>` —
PROP-025 lockfile dispatch; `vibe bin build` pre-builds), from PATH
(`cargo install --path
vibedeps/<stack-slot>/crates/typescript-ai-native-cli`), or in place via
`cargo run --manifest-path vibedeps/<stack-slot>/Cargo.toml -p
typescript-ai-native-cli --bin typescript-ai-native -- <args>`.
Procedures as agent skills: `/typescript-ai-native-sweep` (recurring),
`/typescript-ai-native-terraform` (brownfield adoption) — `vibe skill install`
projects them.
