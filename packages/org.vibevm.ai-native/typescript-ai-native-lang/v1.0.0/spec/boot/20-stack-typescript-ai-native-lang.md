# AI-Native TypeScript (Discipline v0.2) — boot snippet {#root}

<status stage="impl" state="done"/>

@fact:TYPESCRIPT-CODE-FOLLOWS-THE-TYPESCRIPT-GUIDE TypeScript code in this
project follows the AI-Native TypeScript guide
(`spec/typescript/GUIDE-AI-NATIVE-TYPESCRIPT.md` in this package). @status:impl/done

@fact:READ-THE-GUIDE-FOR-STRUCTURE Read the
guide when authoring or reviewing structure; per-edit work needs only
the card whose trigger fires. @status:impl/done

@fact:CARD-REGISTRY-FOR-TYPESCRIPT Card registry for TypeScript: `spec/cards/INDEX.md` in this package (trigger →
card; the nine executable scaffolds A–I in their TypeScript shape). @status:impl/done

@fact:STACK-SHIPS-ITS-OWN-CARDS-PROJECTION This
stack ships its own `spec/cards/` projection — the weak-reader runtime surface
for `.ts` edits is a TypeScript card's Band-3 ops block, never the Rust
one. @status:impl/done

@fact:core-cards-are-the-rust-pilot-reference-set (The core `cards/` are the Rust pilot's reference set; a future
symmetry pass may unify both languages' Band-3 in the core — see the
guide's tooling note.) @status:spec/done

@fact:standing-rules-lead Standing rules at the surface level: @status:impl/done

- @fact:RULE-ORDINARY-IDIOMATIC-TYPESCRIPT-SURFACE Ordinary idiomatic TypeScript at the token level — no invented dialect,
  no type-level wizardry in domain code (deep conditional/recursive types
  are the OOD tail, treated like `unsafe`). Strictness lives in the
  envelope: the maxed `tsconfig`, branded types at seams, runtime
  validators at the erasure boundary, `spec://` metadata, per-cell fast
  verification (`tsc --noEmit -p <cell>` + `vitest run <cell>`, < ~60s). @status:impl/done
- @fact:RULE-COMPILER-IS-A-CONFIGURABLE-VERIFIER The compiler is a configurable verifier: the project `tsconfig` is a
  versioned discipline artifact at the strictest practical floor
  (`strict` + `noUncheckedIndexedAccess` + `exactOptionalPropertyTypes` +
  `erasableSyntaxOnly`); loosening a mandatory flag needs
  `deviates` + reason. @status:impl/done
- @fact:RULE-CELLS Cells: one cell = one file-set with a single public entry (`index.ts`
  as the seam); cells import seams + core only, never sibling cells'
  internals. No barrel-file ambient re-export sprawl; ambient coupling
  (module-level mutable singletons, `declare global`) is forbidden. @status:impl/done
- @fact:RULE-UNSAFE-SET-IS-FORBIDDEN-IN-DOMAIN-CODE Types are erased and can be lied to. The `unsafe` set —
  `any` / unchecked `as` / `!` / `@ts-ignore` — is forbidden in domain
  code; escape hatches are `unknown` + a runtime validator, `as` only
  after a check, an `asserts` function, and `@ts-expect-error -- reason`
  (never `@ts-ignore`). Each deviation carries a recorded reason. @status:impl/done
- @fact:RULE-BRANDING-RECOVERS-NOMINAL-TYPING Structural typing is recovered to nominal by branding: a meaning-bearing
  primitive crossing a seam is a branded type, so the wrong same-shaped
  value fails `tsc`. @status:impl/done
- @fact:RULE-TYPED-SEAM-FAILURES Failure on a seam is a typed value (`Result<T, E>` / discriminated
  union), never an untyped `throw`; the `E` union cites `spec://` REQs and
  its exhaustive `switch` is compiler-checked. @status:impl/done
- @fact:RULE-EXTERNAL-DATA-ENTERS-AS-UNKNOWN Untyped external data (network, JSON, `process.env`, user input) enters
  as `unknown` and is narrowed only through a single-source schema
  (Zod/Valibot) that is both the static type and the runtime check. @status:impl/done
- @fact:RULE-EXAMPLE-PER-SEAM Every public seam carries one type-checked example (Twoslash) of
  canonical use, and public generic/branded/union surfaces carry a
  type-level test (`expectTypeOf`/`tsd`). Replacing a non-trivial cell
  requires a differential oracle (`fast-check`). @status:impl/done
- @fact:RULE-UNIFORMITY-IS-LOAD-BEARING Uniformity is load-bearing: one idiom per operation; exceptions are
  marked, or they propagate as false training signal. @status:impl/done

@fact:shipped-toolchain-lead The shipped toolchain (this stack materialises it; no dev tree needed): @status:impl/done

- @fact:TOOLCHAIN-TYPESCRIPT-AI-NATIVE-UMBRELLA `typescript-ai-native` — `init` (bootstrap policies + registries),
  `floor` (prettier→tsc→tests→eslint→conform→specmap→test-gate, one exit
  code), `health` (the sweep's fact collector), `test-gate` (xfail-strict
  over node's TAP) / `tripwire` / `trace` / `fast-loop` / `codemod`; @status:impl/done
- @fact:TOOLCHAIN-NARROW-ENGINES plus
  the narrow `typescript-ai-native-conform` and `typescript-ai-native-specmap` engines, @status:impl/done
- @fact:TOOLCHAIN-AGENTIC-TYPE-ORACLE and
  the agentic type oracle `typescript-ai-native-tcg`
  (also served over MCP by `mcp:org.vibevm.ai-native/typescript-ai-native-mcp` —
  PROP-027; persistent enriching `serve`
  relay + one-shot `validate`/`scope`/`complete`/`type`/`bench`: check an
  edit against in-memory overlays BEFORE writing it, with the SAME
  conform rules as the gate — GUIDE §14, §15 move 5). @status:impl/done

@fact:STRUCTURAL-GATE-PARSES-THROUGH-THE-PROJECT-TYPESCRIPT The
structural gate parses through the PROJECT's own `typescript` install
(node ≥ 22.6; the same devDependency the tsc floor step needs). @status:impl/done

@fact:RUN-VIBE-NATIVELY-FROM-PATH-OR-IN-PLACE Run
vibe-natively (`vibe bin exec typescript-ai-native -- <args>` —
PROP-025 lockfile dispatch; `vibe bin build` pre-builds), from PATH
(`cargo install --path
vibedeps/<stack-slot>/crates/typescript-ai-native-cli`), or in place via
`cargo run --manifest-path vibedeps/<stack-slot>/Cargo.toml -p
typescript-ai-native-cli --bin typescript-ai-native -- <args>`. @status:impl/done

@fact:PROCEDURES-AS-AGENT-SKILLS Procedures as agent skills: `/typescript-ai-native-sweep` (recurring),
`/typescript-ai-native-terraform` (brownfield adoption) — `vibe skill install`
projects them. @status:impl/done
