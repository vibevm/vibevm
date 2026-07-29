# AI-Native TypeScript — The Guide {#root}

<status stage="spec" state="done"/>

##status-line **Discipline v0.2 · status: BETA · T2 · TypeScript only (JavaScript gets a separate guide) · supersedes GUIDE-TYPESCRIPT-v0.1** @impl/done

##projection-onto-typescript *The projection of the Discipline onto TypeScript.* @impl/done

##GUIDE-COVERS-TYPESCRIPT-AS-A-TYPED-LANGUAGE *This guide covers TypeScript as a typed language in its own right — not JavaScript-with-types-bolted-on; the JS guide is separate and will address the untyped substrate.* @impl/done

##READ-THE-T1-CORE-FIRST *Read `00-MANIFESTO.md` and `02-EXECUTABLE-SCAFFOLDS.md` (the T1 core) first; this guide assumes the central law and the nine scaffold classes.* @impl/done

##A-HUMAN-CAN-READ-AI-NATIVE-TYPESCRIPT *A human CAN read and modify AI-Native TypeScript; it may be less comfortable to write by hand than ordinary TypeScript, but it remains ordinary idiomatic TypeScript at the token level.* @spec/done

##what-differs-is-the-envelope-lead *What differs is the envelope:* @impl/done

- ##ENVELOPE-MAXED-COMPILER-CONFIGURATION *the maxed compiler configuration,* @impl/done
- ##ENVELOPE-BRANDED-CONTRACT-BEARING-TYPES *branded contract-bearing types,* @impl/done
- ##ENVELOPE-RUNTIME-VALIDATION-AT-THE-ERASURE-BOUNDARY *runtime validation at the erasure boundary,* @impl/done
- ##ENVELOPE-EXECUTABLE-SCAFFOLDS *executable scaffolds,* @impl/done
- ##ENVELOPE-FAST-PER-CELL-LOOP *and a fast per-cell verification loop.* @impl/done

##STRUCTURALLY-PARALLEL-TO-THE-RUST-GUIDE *Structurally parallel to `rust/GUIDE-AI-NATIVE-RUST.md` so the two projections stay comparable — that comparison is how the language-independent T1 layer gets validated.* @impl/done

##CROSS-REFERENCE-NOTATION *Section cross-references to the Rust guide are marked `(≈ Rust §N)`.* @impl/done

##TS-SPECIFIC-NOTATION *Where TypeScript has no Rust analogue (the configurable compiler, the erasure boundary, type-level testing), the section is marked `[TS-specific]`; those are the levers and the hazards that make the TypeScript projection heavier on bans and boundary validation than the Rust one, by design (§0).* @impl/done

---

## 0. Why TypeScript is special — and the law applied to TS {#law}

> ##LAW-IDIOMATIC-INSIDE-ENGINEERED-AROUND **Idiomatic inside the file; engineered around the file.** *(≈ Rust §0)* @impl/done

##TYPESCRIPT-IS-DEEPLY-IN-DISTRIBUTION TypeScript is deeply in-distribution (among the most common languages on GitHub), so ordinary typed application code and standard generic APIs are safe to be strict in. @spec/done

##TOOLING-CUTS-THREE-WAYS But TypeScript has, arguably, the most powerful *and most tractable* tooling of any mainstream language (its only rival for analyzability is C++ via the clang/LLVM backend), and that cuts three ways for the Discipline — two advantages and one hazard that has no Rust analogue. @spec/done

##ADVANTAGE-1-TCD-EXISTS-FOR-TYPESCRIPT **Advantage 1 — generation-time type-constrained decoding EXISTS for TypeScript.** The one result behind much of this work — type-constrained decoding cutting compile errors ~74.8% — was measured *on TypeScript* (Mündler et al., PLDI'25; the only language with a real implementation, R2C-005 / DR1-014). @spec/done

##RUST-ORACLE-IS-A-MULTI-YEAR-FUTURE-BET For Rust, that oracle is a multi-year future bet (`vibe-tcg` Stage 3). @spec/done

##TYPESCRIPT-COMPILER-AS-ORACLE-IS-AVAILABLE-TODAY For TypeScript it is available *today*: the compiler-as-oracle can run **during** generation, not only after. @spec/done

##tooling-story-flipped-in-typescripts-favor This flips the tooling story in TypeScript's favor (see §14). @spec/done

##ADVANTAGE-2-MATURE-CODEMOD-ECOSYSTEM **Advantage 2 — the most mature codemod/AST ecosystem of any language.** `ts-morph`, the TypeScript Compiler API, `jscodeshift`, and typed ESLint autofix make Classes A (generators), F (structured diagnostics), and especially I (codemods) far more achievable than in Rust — where Class I is [E-hyp] partly *because* the tooling is immature. @spec/done

##SCAFFOLDED-EDIT-OPERATIONS-ARE-NEAR-TERM In TypeScript, scaffolded edit operations are a near-term reality, not a research gamble. @spec/done

##HAZARD-TYPES-ARE-ERASED-AND-CAN-BE-LIED-TO **The hazard with no Rust analogue — types are erased and can be lied to.** Rust's types are load-bearing at runtime; TypeScript's are **erased**. @spec/done

##COMPILER-BELIEVES-AN-UNCHECKED-ASSERTION The compiler believes a `value as Foo` assertion with no runtime check behind it. @spec/done

##TYPE-SYSTEM-IS-A-DEFEATABLE-VERIFIER This means TypeScript's type system — unlike Rust's borrow checker — is a verifier you can *defeat by writing the right two words* (`as`, `any`, `!`, `@ts-ignore`). @spec/done

##THE-DEFEATING-WORDS-ARE-STATISTICALLY-COMMON For an AI reader this is dangerous precisely because those words are statistically common in training data. @spec/done

##CENTRAL-TYPESCRIPT-SPECIFIC-JOB The Discipline's central TypeScript-specific job is therefore to **forbid the moves that defeat the type system** (§8) and to **regenerate trust at runtime boundaries** (§2). @impl/done

##LAW-PROJECTED-ORDINARY-IDIOMATIC-TYPESCRIPT **The law, projected.** TypeScript source under this discipline reads as *ordinary idiomatic TypeScript*. @impl/done

##NO-INVENTED-SYNTAX No invented syntax — that would incur the out-of-distribution penalty (EsoLang-Bench: 0–11% on unfamiliar surface; in-context learning cannot teach it). @spec/done

##TYPESCRIPT-SECOND-OOD-EDGE-IS-TYPE-LEVEL-METAPROGRAMMING But TypeScript has a second OOD edge Rust lacks: its *type-level metaprogramming* (deep conditional types, recursive template-literal types, HKT emulation) is a sparse long tail that models handle far worse. @spec/done

##LAW-CLAUSE-EXPRESSIVENESS-UP-TO-IDIOMATIC So the law has a TypeScript-specific clause: **use the type system's expressiveness up to the point where the types stay idiomatic, and not one step beyond.** @impl/done

##TYPE-LEVEL-WIZARDRY-IS-TREATED-LIKE-UNSAFE Type-level wizardry is OOD and is treated like `unsafe` — isolated, justified, deviation-marked (§8). @impl/done

##STRICTNESS-LIVES-IN-THE-ENVELOPE-NOT-IN-CLEVER-TYPES The strictness we add lives in the *compiler configuration, the runtime boundary, the metadata, and the verification loop* — not in clever types. @impl/done

##TYPESCRIPT-STRICTNESS-IS-CONFIGURABLE Unlike Rust, where the borrow/type checker's strictness is fixed and free, in TypeScript that strictness is **configurable**, so the first move of the discipline is to turn it all on (§1). @impl/done

## 1. Compiler configuration is discipline (the biggest free lever) `[TS-specific]` {#compiler-config}

##RUST-STRICTNESS-IS-FIXED Rust's strictness is fixed; you get the borrow checker whether you ask or not. @spec/done

##NON-STRICT-CONFIG-GIVES-ALMOST-NO-SAFETY TypeScript's strictness is **configurable** — a non-strict config gives "the syntax of static typing with almost none of the safety." @spec/done

##MANDATE-THE-STRICTEST-PRACTICAL-CONFIGURATION AI-Native TypeScript therefore **mandates the strictest practical configuration**, because every flag turned on is intent moved from prose into the compiler (A3). @impl/done

##COMPILER-IS-A-FREE-HALLUCINATION-DETECTOR The compiler is a free hallucination detector; we give it the maximum to check. @impl/done

##typescript-analogue-of-the-rust-guides-line This is the TypeScript analogue of the Rust guide's "the compiler is a free hallucination detector" (Rust §0) — except here we must *opt in* to it, flag by flag. @spec/done

##mandatory-tsconfig-floor-lead Mandatory `tsconfig` floor: @impl/done
- ##TSCONFIG-STRICT-TRUE `"strict": true` — bundles the eight base flags (`strictNullChecks`, `strictFunctionTypes`, `strictBindCallApply`, `strictPropertyInitialization`, `noImplicitAny`, `noImplicitThis`, `alwaysStrict`, `useUnknownInCatchVariables`). @impl/done
- ##TSCONFIG-BEYOND-STRICT **Beyond `strict` (NOT included, all mandatory here):** `noUncheckedIndexedAccess` (array/index access yields `T | undefined` — catches a whole class of "it's always there" hallucinations), `exactOptionalPropertyTypes` (distinguishes absent from `undefined`), `noPropertyAccessFromIndexSignature`, `noImplicitOverride`. @impl/done
- ##TSCONFIG-DEFECT-CATCHERS **Defect-catchers:** `noUnusedLocals`, `noUnusedParameters`, `noFallthroughCasesInSwitch`, `noImplicitReturns`, `allowUnreachableCode: false`. Mandated here and enforced nowhere yet: no shipped rule reads `tsconfig.json`, and the stack's own oracle fixture sets none of the five. @spec/done
- ##TSCONFIG-ERASABLE-SYNTAX-ONLY **Forward-looking erasure flag:** `erasableSyntaxOnly` — restricts to syntax that erases cleanly (forbids runtime `enum`/`namespace`), keeping TypeScript a thin typed layer over JavaScript. This matters now that the native compiler (TS 7 / "Corsa") and Node's type-stripping treat types as erasable annotations. It is also AI-native: it removes constructs whose runtime behavior diverges from their syntax. @impl/done

##TSCONFIG-IS-A-VERSIONED-DISCIPLINE-ARTIFACT The `tsconfig` is a **versioned artifact of the discipline** (a card-checked file), not a per-developer preference. @impl/done

##LOOSENING-A-MANDATORY-FLAG-REQUIRES-DEVIATES Loosening any mandatory flag requires `deviates` + reason. @impl/done

##RULE-STRICT-FLOOR-IS-SET-BY-CONFIG *Rule:* the strict floor is set by config and maxed out; "we use strict TypeScript" while disabling individual flags or bypassing them with `@ts-ignore` is a discipline violation. @impl/done

## 2. The erasure boundary: regenerate trust at runtime edges `[TS-specific]` {#erasure-boundary}

##ERASURE-BOUNDARY-IS-WHERE-GUARANTEES-END Because types vanish at runtime, the boundary between the typed interior and the untyped exterior (network, JSON, `process.env`, user input, third-party `any`) is where the type system's guarantees end and a model's false confidence begins. @spec/done

##BOUNDARY-REQUIRES-SINGLE-SOURCE-RUNTIME-VALIDATION At that boundary the Discipline requires **runtime validation from a single source that is simultaneously the static type and the runtime checker** — a schema library (Zod, Valibot, ArkType, TypeBox). @impl/done

##ONE-DECLARATION-YIELDS-TYPE-AND-PARSER One declaration yields both `type User = z.infer<typeof UserSchema>` and `UserSchema.parse(input)`. @spec/done

##SCHEMA-FUSES-THREE-SCAFFOLDS This fuses three scaffolds (A generator + B typed surface + C runnable contract) into one TypeScript-shaped artifact — the densest single move in the projection. @impl/done

##RULE-EXTERNAL-DATA-ENTERS-AS-UNKNOWN *Rule:* untyped external data enters as `unknown` (never `any`) and is narrowed only through a runtime validator or an assertion function (§5, §8); a bare `as` on external data is forbidden (§8). @impl/done

##SCHEMA-IS-THE-BOUNDARYS-CONTRACT The schema is the boundary's contract; inside the boundary the compiler is trusted, outside it is not. @impl/done

##re-establishing-the-verifiers-guarantee This is the TypeScript form of "the compiler is a verifier we maximize" — we re-establish the verifier's guarantee at exactly the points where erasure would otherwise silently void it. @impl/done

## 3. Cells, closure, ownership *(≈ Rust §1)* {#cells}

##cell-is-the-unit-of-modification-lead The **cell** is the unit of modification, closed under paging (R3-001): it declares its full semantic dependency set so a pager can assemble sufficient context mechanically. @impl/done
- ##CELL-GRANULARITY Granularity: a module (file) or a small directory with a single public entry (`index.ts` as the seam), with promotion criteria to a larger cell when cohesion demands. @impl/done
- ##EXPLICIT-IMPORTS-ONLY **Explicit imports only; no barrel-file ambient re-export sprawl** that hides the dependency graph. Cells import seams + core, never sibling cells' internals (R-002). @impl/done
- ##OWNERSHIP-ALIGNS-WITH-FILE-BOUNDARIES **Ownership aligns with file boundaries** (R3-013): one cell = one file-set with one public entry. God-modules and giant barrel files serialize the swarm and obscure closure — an anti-pattern. Shared facts go to append-only ledgers, not shared mutable modules. @impl/done
- ##AMBIENT-COUPLING-IS-FORBIDDEN Ambient coupling — module-level mutable singletons, global augmentation, ambient `declare global`, config read outside the composition root — breaks closure and is forbidden outside the composition root (R3-001). @impl/done

## 4. Surface form: naming, position, and the structural-typing trap *(≈ Rust §2)* {#surface}

- ##NAMES-ARE-TOKEN-PROGRAMS **Names are token programs** (R3-004, R-020): one name = one referent across the contract surface; no shadowing, no synonym pairs; structural tokens from a closed vocabulary. Length is free; ambiguity is not. (Short closure-local bindings are exempt — scope the rule to contract surfaces.) @impl/done
- ##FAMILY-PREFIX-RULE **The family-prefix rule (owner policy, 2026-07-07; supersedes the `-typescript` suffix rule).** Every named surface of the TypeScript discipline is language-FIRST: it carries the family stem `typescript-ai-native` as a *prefix*, not a `-typescript` suffix (PROP-028 §2.4). The umbrella binary is the family name itself (`typescript-ai-native`, over `init` / `floor` / …; its crate `typescript-ai-native-cli`); the standalone tools and their crates share `typescript-ai-native-<role>` (`typescript-ai-native-conform`, `typescript-ai-native-specmap`, `typescript-ai-native-tcg`, and the libraries `typescript-ai-native-conform-frontend`, `typescript-ai-native-specmap-scan`, `typescript-ai-native-tcg-bridge`, `typescript-ai-native-extract-bridge`); the server package/crate/binary is `typescript-ai-native-mcp` and the agent-visible server name is the family (`typescript-ai-native`); the skills are `typescript-ai-native-sweep` / `typescript-ai-native-terraform`; the token brief is `typescript-ai-native-tcg.md` beside `rust-ai-native-tcg.md`. Language-NEUTRAL artifacts stay outside the stem (the shared engine crates take the core stem `core-ai-native-*`). @impl/done
- ##CONTRACT-FIRST-ORDERING **Contract-first ordering within an item** (R3-002): the exported type/signature, then its invariants, then its error contract, then one canonical example precede the implementation. Autoregression makes reading order conditioning order; intent goes first. @impl/done
- ##POSITION-IS-A-RESOURCE **Position is a resource** (R3-003): module-level invariants and the public surface live at the top; prefer more, smaller, single-purpose modules over long files at equal token mass. A conform check warns on files over a length threshold and on invariant-bearing comments in the diluted middle third (for `.ts` that structural gate runs through the `typescript-ai-native-conform-frontend` crate — `typescript/tools/conform-frontend-typescript.md` — feeding the same language-neutral engine the Rust stack ships, now that PROP-024 made conform a code-bearing, relocatable package). @impl/done
- ##UNIFORMITY-IS-LOAD-BEARING **Uniformity is load-bearing** (R3-006, H6): one idiom per operation. The codebase is the few-shot prompt; a second coexisting idiom becomes false training signal and propagates. Legitimate exceptions are MARKED (`deviates`) so they do not propagate as imitation. @impl/done
- ##STRUCTURAL-TYPING-TRAP **The structural-typing trap (TypeScript-specific).** TypeScript is *structurally* typed: two types with the same shape are interchangeable, so a model can silently pass a `UserId` where an `OrderId` is expected if both are `string`. Rust gets nominal safety free via newtypes; **TypeScript must recover it manually through branding**. *Rule:* identifiers and other meaning-bearing primitives crossing a seam are **branded** (`type UserId = string & { readonly __brand: 'UserId' }`, or a branding helper) so the wrong same-shaped value fails `tsc`. This is the single most important TypeScript-specific safety move and is the basis of scaffold card B (§5). It is the manual recovery of the nominal safety Rust's newtypes give for free. @impl/done

## 5. The nine scaffolds in TypeScript *(≈ Rust §3)* {#scaffolds}

##scaffold-cards-lead Each is a card in this package's `cards/` (the TypeScript projection of the language-neutral scaffold catalog `02-EXECUTABLE-SCAFFOLDS.md`); here is the TypeScript shape and the rule. @impl/done

- ##SCAFFOLD-A-GENERATORS **A — Generators / codegen** (`scaffold-a-generators`). `ts-morph` / Compiler API generators; types generated from a single schema source (Zod→`infer`, OpenAPI/GraphQL/Prisma→types); `satisfies` + `as const` for checked literal tables; template-literal types as bounded type-level generation. Committed output is plain idiomatic TS; the generator carries the structural decision. TypeScript's codegen is mature — favor it. *Rule:* where an artifact is mechanically derivable from a smaller spec, ship generator + committed output + determinism check, not hand-maintained output (A3). @impl/done
- ##SCAFFOLD-B-TYPED-SURFACES **B — Typed surfaces / branding / typestate** (`scaffold-b-typed-builders`). **Branded types** for nominal safety over structural typing (§4 — the key TS move); discriminated unions; phantom-type-parameter builders for call-order protocols; `satisfies` for exhaustiveness; sealed unions; no boolean/positional argument soups. Make the statistically-likely wrong same-shaped call fail `tsc`, not a runtime assert. *Rule:* seam protocols are encoded in types, not docstrings (R3-008; ~94% of compile errors are type-level). @impl/done
- ##SCAFFOLD-C-RUNNABLE-CONTRACTS **C — Runnable contracts** (`scaffold-c-runnable-contracts`). **Assertion functions** with `asserts` predicates (`function assertIsUser(x: unknown): asserts x is User`) — uniquely TypeScript: one function that BOTH checks at runtime AND narrows the static type; `tiny-invariant`; Zod/Valibot schemas as executable contracts at boundaries; invariants restated at use sites (R3-009). *Rule:* every load-bearing invariant is witnessed by a runnable assertion where it is relied upon, not only documented at definition. @impl/done
- ##SCAFFOLD-D-DIFFERENTIAL-ORACLES **D — Differential / characterization oracles** (`scaffold-d-differential-oracle`). `fast-check` property-based differential harnesses (old-vs-new); `vitest`/`jest` snapshot tests for opaque legacy behavior (must fail loudly when stale, never auto-update). *Rule:* no replacement of a non-trivial cell merges without a differential or characterization oracle against prior behavior (R-040). The modification-specific safety net (§11). @impl/done
- ##SCAFFOLD-E-PER-CELL-FAST-LOOP **E — Per-cell fast loop** (`scaffold-e-fast-loop`). `tsc --noEmit` per project (project references for isolation) + `vitest` for the cell; the native compiler (TS 7 / "Corsa", ~10× faster checking) makes per-cell first-signal sub-second — a strong substrate. The agent loop is edit → `tsc --noEmit -p <cell>` + `vitest run <cell>` → read structured diagnostic → edit; first signal < ~60s (R3-007). *Rule:* whole-repo CI is not an agent loop; the per-cell loop is the substrate that makes every other scaffold's signal fast enough. @impl/done
- ##SCAFFOLD-F-STRUCTURED-DIAGNOSTICS **F — Structured, REQ-citing diagnostics** (`scaffold-f-structured-diagnostics`). Custom `@typescript-eslint` rules whose messages cite the violated `spec://` REQ and the fix surface; the Compiler API's diagnostics are already coded (TS2322 etc.) — wrap them with REQ context. *Rule:* every custom check emits "violates REQ <uri>: <why>; fix surface: <where>", never bare free text (R3-011). Error text is the agent's percept. @impl/done
- ##SCAFFOLD-G-EXECUTABLE-EXAMPLES **G — Executable examples** (`scaffold-g-doctests`). **Twoslash** (type-checks code in documentation — the TypeScript doctest equivalent); `@example` JSDoc blocks validated by tooling; `expectTypeOf`/`tsd` for type-level examples; `examples/` cells built in CI. *Rule:* every public seam carries ≥1 type-checked example of canonical use; an example that lies fails the build; a prose snippet that lies ships (R2C-004, H4). @impl/done
- ##SCAFFOLD-H-LOCAL-SIMULATORS **H — Local simulators / reference models** (`scaffold-h-simulators`). In-memory fakes (MSW for network, fake implementations of seams); `.d.ts` declaration files as shape models; runnable reference implementations of protocols/state-machines the reader can step through. *Rule:* subsystems with non-obvious dynamics ship a runnable model or fake, not a prose description (execution-prediction is where weak models are weakest — DR2-019, CRUXEval ~63% even for strong models). @impl/done
- ##SCAFFOLD-I-CODEMODS **I — Scaffolded edit operations / codemods** (`scaffold-i-codemods`). `ts-morph` / `jscodeshift` codemods for "add a cell," "register a variant," "rename across the seam"; typed ESLint autofix as constrained one-shot transforms. **TypeScript's biggest scaffold advantage** — mature codemod tooling makes Class I far more achievable here than in Rust. *Rule (provisional, [E-hyp]):* a capability-demanding multi-file edit is offered as one parameterized checked operation. The tooling-immaturity half of Rust's [E-hyp] does not apply (the ecosystem is mature); the *weak-agent-can-parameterize* half remains open — validate in pilot. @impl/done

## 6. Errors as contract surface (TypeScript has no checked exceptions) *(≈ Rust §4)* {#errors}

##THROW-IS-UNTYPED `throw` in TypeScript is **untyped** — you can throw anything, and the type system is blind to it. @spec/done

##a-thrown-error-is-invisible So a thrown error is invisible to a reader and to the compiler. @spec/done

##FAILURE-IS-A-VALUE-ON-THE-CONTRACT-SURFACE The Discipline therefore makes failure a **value**, not a throw, on the contract surface: a discriminated union `Result<T, E> = { ok: true; value: T } | { ok: false; error: E }` (or `neverthrow`/`Effect`), with `E` a discriminated union of named error variants carrying `spec://` REQ references. @impl/done

##EXHAUSTIVENESS-OVER-E-IS-ENFORCED Exhaustiveness over `E` is enforced by a `satisfies never` / `assertNever` check in the default branch. @impl/done
- ##THROW-IS-FOR-UNRECOVERABLE-DEFECTS `throw` is reserved for truly unrecoverable defects (the panic analogue), at the binary edge. @impl/done
- ##FALLIBLE-SEAMS-RETURN-RESULT Fallible seams return `Result`, never a `Promise<T>` that rejects with an untyped error. @impl/done
- ##TYPESCRIPT-PROJECTION-OF-THISERROR This is the TypeScript projection of Rust's "one `thiserror` enum per layer; variants carry REQ edges; panics are defects" — the discriminated-union `E` is the `thiserror` enum, and the untyped `throw` is the panic. @impl/done

##RULE-FAILURE-ON-A-SEAM-IS-A-TYPED-VALUE *Rule:* failure on a seam is a typed value with REQ-citing variants; the exhaustive `switch` over the error union is checked at compile time (R-010, projected). @impl/done

## 7. Registry, flags & the composition root *(≈ Rust §5)* {#registry}

##SAME-RULE-AS-RUST-SHARPENED-BY-ERASURE The Rust guide forbids `if flag` in domain logic; the same rule holds in TypeScript, and the erasure boundary (§2) sharpens it. @impl/done

##FLAGS-READ-ONCE-AT-THE-COMPOSITION-ROOT Flags and external configuration are read **once, at the composition root** (the app/entry cell), narrowed there through a schema (so `process.env` — pure untyped exterior — is validated and typed exactly once), and a **registry** (a typed `as const` map, or a discriminated-union selector) chooses the cell/strategy. @impl/done

##NO-IF-FLAG-IN-DOMAIN-CELLS **No `if (flag)` scattered through domain cells** (R-001 — defined in the shared conform engine, not yet mounted on the TypeScript gate). @spec/done

##EXPLICIT-SWITCH-BEATS-DYNAMIC-LOOKUP An explicit `switch` over a discriminated config union at the composition root, exhaustiveness-checked, beats string-keyed dynamic lookup and module-load side effects — "one switch is the system's table of contents." @impl/done

##two-tiers-lead Two tiers, mirroring Rust's cargo-features-vs-runtime-flags split: @impl/done
- ##TIER-BUILD-TIME **Build-time:** bundler `define` / dead-code elimination / env-gated conditional compilation (code physically absent from the bundle). The TypeScript analogue of cargo features. @impl/done
- ##TIER-RUNTIME **Runtime:** a registry object selects a cell/implementation at run time (code present, cell chosen). The TypeScript analogue of runtime flags. @impl/done

##FLAG-REGISTRY-IS-TYPED-DATA-WITH-PROVENANCE The flag/registry is **typed data with provenance, birth, and sunset** — a branded or `as const` table, not stringly-typed ambient lookup, and never a module-level mutable singleton (which would breach §3 closure). @impl/done

##RULE-FLAGS-READ-AT-THE-ROOT-AND-DISPATCHED *Rule:* flags are read at the composition root and dispatched through a typed registry; `if (flag)` in a domain cell, or reading config outside the root, requires `deviates` + reason. Unchecked today — R-001 is unmounted here, so the `deviates` clause has nothing to fire against. @spec/done

## 8. Bans and their escape hatches — the TypeScript `unsafe` set *(≈ Rust §6)* {#bans}

##BANS-ARE-THE-MOVES-THAT-DEFEAT-THE-TYPE-SYSTEM These are the moves that *defeat the erased type system* — the TypeScript analogue of Rust's `unwrap`/inline-asm/`unsafe` ban set. @impl/done

##forbidden-by-default-lead Forbidden by default in domain code; legal only with the escape hatch shown and a recorded reason (`deviates`): @impl/done
- ##BAN-ANY **`any`** — disables checking and **propagates transitively** (one `any` poisons everything it touches). Banned. *Escape hatch:* `unknown` + runtime narrowing; or, at a genuine third-party boundary, a localized `// eslint-disable-next-line @typescript-eslint/no-explicit-any -- reason` confined to one line. @impl/done
- ##BAN-UNCHECKED-AS **Unchecked `as` assertions** — the erasure hazard's sharp edge: `data as User` makes the compiler believe a lie. Banned on untrusted/domain data. *Escape hatch:* `as` only *after* a runtime check, or the always-safe `as const`; cross-type assertions require `deviates` + reason. (Note: the common "fix" `key as keyof typeof obj` to silence `noUncheckedIndexedAccess` is exactly this hazard — narrow instead.) @impl/done
- ##BAN-NON-NULL-ASSERTION **Non-null assertion `!`** — claims non-null without proof. Banned. *Escape hatch:* narrowing, or an assertion function `function assertDefined<T>(x: T): asserts x is NonNullable<T>`. @impl/done
- ##BAN-TS-IGNORE **`@ts-ignore`** — silences the compiler invisibly and stays silent even after the error is gone. Banned outright. *Escape hatch:* `@ts-expect-error -- reason`, which *fails* if the error disappears (it cannot rot silently) — the only acceptable form. @impl/done
- ##BAN-TYPE-LEVEL-METAPROGRAMMING **Type-level metaprogramming beyond the idiomatic** (deep recursive/conditional types) — OOD tail (§0). *Escape hatch:* isolate genuinely needed type-level code behind a documented boundary with `deviates` + reason; never spread it through domain types. @impl/done
- ##BAN-RUNTIME-ENUM-AND-NAMESPACE **Runtime `enum` and `namespace`** — don't erase cleanly; forbidden under `erasableSyntaxOnly` (§1). *Replacement:* `as const` union objects and ES modules. @impl/done

##BAN-WITHOUT-HATCH-IS-A-DISCIPLINE-BUG A ban with no escape hatch is a discipline bug; a deviation with no reason is a code bug. @impl/done

## 9. Metadata layer (specmap in TypeScript) *(≈ Rust §7)* {#specmap}

##SPEC-URIS-CARRIED-BY-JSDOC-OR-DECORATORS `spec://` URIs carried by JSDoc tags (`/** @implements spec://... */`), TC39 decorators (stage-3 ES decorators) on classes/methods, or a sidecar mapping; `.d.ts` files as a natural meta/shape layer. @impl/done

##EDGE-KINDS-MIRROR-PROP-014 The edge kinds mirror PROP-014 (`implements | verifies | documents | deviates | informs`, ≤3 edges per item, the specmark budget); two-tier revisions (author-asserted semantic revision + content hash) with **asymmetric invalidation** (spec bump → edges suspect; code change → edges stay valid); a derived deterministic committed index; an orphan ratchet; `deviates` requires a reason. @impl/done

##METADATA-IS-THE-AUTHORED-RETRIEVAL-INDEX The metadata is the authored retrieval index (R3-012): stable anchors + a uniform one-line what/why per exported symbol, in a fixed grammar the pager consumes. @impl/done

##prefer-jsdoc-tags-over-decorators (Decorators have runtime cost and partial erasure — prefer JSDoc tags for inert metadata to stay erasure-clean under §1.) @impl/done

## 10. Prose discipline (the asymmetric hazard) *(≈ Rust §8)* {#prose}

##WRONG-PROSE-IS-WORSE-THAN-NO-PROSE Wrong prose is worse than no prose (R2C-004, H4): a model conditions on in-repo text with high trust, so a lying comment is adversarial input, and the harm exceeds that of absence. @spec/done

##DRIFTED-JSDOC-IS-TRUSTED-OVER-THE-TYPES TypeScript-specific sharp edge: a JSDoc `@param`/`@returns` that has drifted from the signature is a lie the model trusts *over* the (correct) types. @spec/done

##RULE-BEHAVIORAL-CLAIMS-ARE-MACHINE-CHECKED *Rule:* behavioral claims near code are **machine-checked** — backed by Twoslash/`@example` (type-checked) or `expectTypeOf` (type-level checked) — or **explicitly trust-labeled** (verified / unverified / aspirational); JSDoc that merely restates the types is duplication (a defect) — let the types speak. @impl/done

##MISLEADING-STRINGS-COUNT-TOO Misleading `console.log`/error strings count too (the harm is the false claim, not the syntax). @impl/done

##TSDOC-IS-THE-HUMAN-DETAIL-LAYER TSDoc remains the human detail layer; duplication with the spec is a spec defect. @impl/done

## 11. Replacement protocol *(≈ Rust §9)* {#replacement}

##REPLACEMENT-SHIPS-A-DIFFERENTIAL-ORACLE Replacing a cell ships a **differential oracle** (Class D, §5) against the old cell — `fast-check` feeding identical generated inputs to `old` and `new` and asserting equal outputs — plus the `@verifies spec://…` edge (§9). @impl/done

##CHARACTERIZATION-GOLDENS-PIN-LEGACY-BEHAVIOR Characterization goldens (`vitest` snapshots) pin opaque legacy behavior; goldens must fail loudly when stale (run under `--ci`; never `--update` silently auto-rewriting). @impl/done

##CHARACTERIZATION-ENSHRINES-BUGS-TOO The characterization variant enshrines current behavior including its bugs — pair it with a spec edge marking which behaviors are intentional vs incidental. @impl/done

##BYTE-FOR-BYTE-THE-RUST-PROTOCOL This is byte-for-byte the Rust replacement protocol (R-040) with TypeScript tools; it is the one place the modification-time safety net is mandatory rather than advisory. @impl/done

## 12. Test matrices and type-level testing *(≈ Rust §10 + a TS-unique scaffold)* {#matrices}

##TEST-MATRICES-ARE-DECLARED **Test matrices.** Declared test matrices, never an implicit `2^n`. @impl/done

##MATRIX-TOOLING `vitest`/`jest` `test.each` / `it.each` over a **named, bounded case table** (`as const` so the table is typed and exhaustiveness is visible); `fast-check` for behavioral surfaces; the differential oracle (§11) covers replacement; per-cell `vitest` runs in the fast loop (§5). @impl/done

##MATRIX-IS-AUTHORED-DATA The matrix is authored data, not a combinatorial explosion the reader must hold in their head (R-060, projected). @impl/done

##TYPE-LEVEL-TESTING-IS-TYPESCRIPT-UNIQUE **Type-level testing (a TypeScript-unique scaffold).** TypeScript can assert **type relationships at compile time** — a class of runnable contract no mainstream language has so readily, and a place where TypeScript's expressiveness pays the discipline back instead of costing it. @spec/done

##TYPE-LEVEL-TEST-TOOLING `expectTypeOf<X>().toEqualTypeOf<Y>()` (vitest), `tsd`'s `expectType`, and `@ts-expect-error` as a negative assertion let you test that a generic, a branded type (§4), or a discriminated union (§6) behaves as intended *before any code runs*. @impl/done

##RULE-PUBLIC-SURFACES-CARRY-TYPE-LEVEL-TESTS *Rule:* public generic/branded/union surfaces carry type-level tests asserting their key relationships; these run in the Class E loop (a type-level test that regresses fails `tsc`). @impl/done

##type-level-testing-is-additive-over-rust This is Class C/D applied to the types themselves — Rust has no comparable readily-available form, so it is additive over the Rust projection, not a mirror. @spec/done

## 13. How a weak reader actually uses this guide *(≈ Rust §11)* {#weak-reader}

##WEAK-SWARM-DOES-NOT-READ-THIS-GUIDE The weak swarm does **not** read this guide. @impl/done

##WEAK-READER-RECEIVES-BAND-3-OPS-PER-EDIT It receives, per edit, the Band-3 ops extract of whichever cards' triggers fire — a small, activation-matched set (lazy-push, R3-014; minimal sufficiency, AGENTbench). @impl/done

##GUIDE-IS-THE-AUTHORING-AND-REVIEW-ARTIFACT This guide and the cards are the authoring/review artifact for the strong author and the human; the runtime surface for the weak reader is "the right TypeScript card's routine + checker, when its trigger fires" — and for `.ts` edits that is a card from *this package's* `cards/`, never the Rust core's. @impl/done

##RAIDS-SWEEP-CROSS-CUTTING-CONCERNS Cross-cutting concerns the per-edit loop cannot hold are swept by raids (`03-RAID-PLAYBOOK.md`). @impl/done

## 14. Tooling roadmap pointer (the tcg line) *(≈ Rust §12)* {#tooling}

##two-tcg-briefs-lead The tcg line has TWO briefs, split by where the intervention happens: @impl/done

- ##TCG-AGENTIC-BRIEF-SHIPPED **[`typescript/tools/vibe-agentic-tcg-ts.md`](tools/vibe-agentic-tcg-ts.md) — SHIPPED (the agentic oracle):** a long-lived language-service oracle the agent CONSULTS — validate-an-overlay / scope / type-valid completions / quick info at millisecond latency, discipline-enriched by the same conform engine as the gate — delivered as `tcg_*` MCP tools and one-shot `typescript-ai-native-tcg` CLI forms. Mechanisms: `mechanisms/TCG-ORACLE-v0.1.md`, `mechanisms/TCG-PROTOCOL-v0.1.md`. It is the generation-time complement to the post-generation `tsc --noEmit` loop (Class E): the loop stays the GUARANTEE; the oracle removes red iterations before they happen. @impl/done
- ##TCG-TOKEN-LEVEL-BRIEF-VERY-FAR-FUTURE **[`typescript/tools/typescript-ai-native-tcg.md`](tools/typescript-ai-native-tcg.md) — VERY-FAR-FUTURE (token-level):** logit masking to type-checker-validated, discipline-conformant continuations, by construction. It waits, owner-dispositioned, on an inference substrate (`vibe-llm` is a stub; hosted agent APIs never expose logits) and will reuse the SAME oracle as its completability answer when it comes. @spec/done

##TOOLING-ASYMMETRY-STATED-HONESTLY **The TypeScript tooling asymmetry, stated honestly.** The PLDI'25 result proves type-constrained decoding works for TypeScript (~74.8% compile-error reduction; ~94% of TS compile errors are type-level) — but its repository is inspiration-only under the clean-room rule (never a code source; the algorithm would be reimplemented from the paper in structurally different code). @spec/done

##TYPESCRIPT-EXPOSES-ITS-CHECKER-PROGRAMMATICALLY What makes TypeScript first anyway is the compiler itself: unlike Rust at decode time, TypeScript exposes its checker programmatically (Compiler API / language service), so our oracle stands on the REAL checker rather than a rebuilt subset. @spec/done

##TYPESCRIPT-IS-THE-STRONGEST-NEAR-TERM-PILOT Combined with the mature codemod ecosystem (Class I), that makes TypeScript the strongest near-term pilot for the swarm story, second only to where the algorithmic core lives (Rust/vibevm). @spec/done

##COUNTERWEIGHT-DEFAULT-SAFETY-IS-LOWER **The honest counterweight.** TypeScript's erasure and structural typing mean its *default* safety is *lower* than Rust's — more must be done manually (branding, runtime validation, banning `as`/`any`) to reach the same floor. @spec/done

##MORE-LEVERAGE-AND-MORE-ROPE TypeScript gives more tooling leverage and more expressiveness, but it also gives more rope. @spec/done

##PROJECTION-IS-HEAVIER-ON-BANS-BY-DESIGN The Discipline's TypeScript projection is heavier on *bans and boundary validation* (§2, §8) than the Rust projection precisely for this reason — and that asymmetry, not a failure of mirroring, is the genuine T2 content. @impl/done

##STANDING-OPEN-QUESTION **The standing open question (shared with Rust).** The 74.8% / type-constrained-decoding result is a *generation*-time result; the Discipline's central unproven bet is whether scaffolds help *comprehension and modification* of in-distribution code, not just generation. @spec/done

##MODIFICATION-SAFETY-IS-STILL-THE-PILOTS-JOB A type oracle makes a weak agent *write* well-typed TypeScript (by construction at token level; by cheap consultation agentically); whether it then *modifies* existing TypeScript safely is still the pilot's job — and erasure means well-typed code can still lie at runtime if an `as` slipped through, which is why the §8 ban on `as` matters even with the type oracle on. @spec/done

##AGENTIC-BATTERY-IS-THE-FIRST-MEASUREMENT The agentic battery — two arms, weak model, mechanical verification; the sibling brief `vibe-agentic-tcg-ts` names it in §2 (the weak population it measures) and §4 (the stage it measures) — is the first standing measurement of exactly this question. @impl/done

## 15. Wiring a consumer (the shipped toolchain) *(≈ Rust §13)* {#wiring}

##wires-it-in-five-moves-lead The stack ships the toolchain as runnable code (PROP-024); a consumer wires it in five moves: @impl/done

1. ##WIRE-1-INSTALL-THE-STACK **Install the stack** — `vibe install` with `stack:org.vibevm.ai-native/typescript-ai-native-lang` in `[requires].packages` materialises the slot under `vibedeps/` (the neutral engines ride along as vendored copies; the slot is its own Cargo workspace and builds standalone). @impl/done
2. ##WIRE-2-GET-THE-BINARIES **Get the binaries** — `cargo install --path vibedeps/<stack-slot>/crates/typescript-ai-native-cli` (plus `typescript-ai-native-conform` / `typescript-ai-native-specmap` if you want the narrow engines on PATH), or run in place: `cargo run --manifest-path vibedeps/<stack-slot>/Cargo.toml -p typescript-ai-native-cli --bin typescript-ai-native -- <args>`. @impl/done
3. ##WIRE-3-PROJECT-TOOLCHAIN **Project toolchain** — node ≥ 22.6 (strip-types runs `.ts` directly; `node --test` is the default runner) and `npm install -D typescript prettier eslint typescript-eslint`. The structural gate parses through the PROJECT's own `typescript` — the same install the `tsc` floor step uses, so the gate adds no new dependency. @impl/done
4. ##WIRE-4-BOOTSTRAP **Bootstrap** — `typescript-ai-native init` writes conform.toml (`[typescript]`: roots, `cells_dir`, seam), specmap.toml (namespace + discovered `[[external_specs]]`), both ratchet baselines, and the BROWNFIELD registries; then `typescript-ai-native specmap` mints the index and `typescript-ai-native floor` runs the seven steps. Adoption on a brownfield tree: the `/typescript-ai-native-terraform` skill. @impl/done
5. ##WIRE-5-GENERATION-TIME-ORACLE **The generation-time oracle (optional but cheap)** — the stack's 4th binary, `typescript-ai-native-tcg`, answers validate/scope/complete/type over in-memory overlays (§14). One-shot from anywhere: `vibe bin exec typescript-ai-native-tcg -- validate src/cells/<cell>/index.ts --json`. Warm, inside an agent session: the `tcg_*` MCP tools (`vibe mcp serve`; vibevm PROP-026) hold a persistent oracle per language, so consulting the type checker before an edit costs milliseconds. The floor stays the truth; the oracle exists so the floor stays green on the first try. @impl/done

##gotchas-the-fresh-walks-caught-lead Gotchas the fresh walks caught: @impl/done

- ##GOTCHA-WORKSPACE-EXCLUDE-VIBEDEPS a repo that also carries Rust keeps `[workspace] exclude = ["vibedeps"]`; @impl/done
- ##GOTCHA-NODE-MODULES-AND-LOCKFILES `node_modules/` is gitignored but lockfiles are committed; @impl/done
- ##GOTCHA-EXTRACTOR-REMATERIALISES the extractor materialises content-addressed under `target/conform/ts-extract/` — clean builds re-materialise it automatically. @impl/done

## 16. Sweep idioms *(≈ Rust §14)* {#sweep}

##sweep-idioms-lead The recurring posture is the shipped Sweep Playbook driven by `/typescript-ai-native-sweep`; the TypeScript-specific idioms: @impl/done

- ##SWEEP-DANGER-BAND-SPLITS **Danger-band splits** keep traceability: the new module gets its own file-level `@scope` (or carries the moved exports' `@implements` tags) so the orphan ratchet never regresses on a refactor. @impl/done
- ##SWEEP-SUPPRESSION-DRAINS **Suppression drains**: `@ts-ignore` → `@ts-expect-error -- reason` is always a strict improvement (it fails when the error goes); an unreasoned `@ts-expect-error` in the health census is unrecorded testimony — reason it or fix it. @impl/done
- ##SWEEP-UNSAFE-SET-DRAINS **Unsafe-set drains** go type-first: `any` → `unknown` + one narrowing helper reused everywhere (uniformity is load-bearing); a cross-type `as` at an erasure boundary becomes a schema parse; `!` becomes an `asserts` function. @impl/done
- ##SWEEP-FLOOR-DISABLEMENT-IS-DEBT **Floor disablement is debt**: every `[[typescript.floor_disable]]` entry prints on every run — re-question the reasons weekly; an empty list is the exit criterion the terraform aims at. @impl/done
