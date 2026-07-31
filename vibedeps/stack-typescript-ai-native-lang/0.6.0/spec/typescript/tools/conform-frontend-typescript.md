# Tool Spec (high-level): `typescript-ai-native-conform-frontend` — the TypeScript frontend for the language-neutral conform engine {#root}

<status stage="spec" state="done"/>

##status-line *Status: **SHIPPED** (0.3.0, the deferrals-closeout campaign) — this package's `crates/typescript-ai-native-conform-frontend` (`id = "ts-tsc"`) + `crates/typescript-ai-native-conform` (binary **`typescript-ai-native-conform`**), fed by the Compiler-API extractor at `tools/ts-extract/` through the `typescript-ai-native-extract-bridge` NDJSON protocol.* @impl/done

##TYPESCRIPT-COUNTERPART-OF-THE-RUST-FRONTEND *The TypeScript counterpart to `rust-ai-native-conform-frontend`: it gives `.ts` code the SAME structural discipline gate that `.rs` code gets from `rust-ai-native-conform check` — by feeding TypeScript facts into the language-neutral conform engine, not by re-implementing the rules in ESLint.* @impl/done

##SECTION-THREE-QUESTION-WAS-ANSWERED *The §3 question below was answered by the engine consolidation: option (a) happened — `conform-core` now lives in `flow:org.vibevm.ai-native/core-ai-native`, both stacks vendor it (`cargo xtask sync-engines`).* @impl/done

##brief-remains-as-design-rationale *This brief remains as the design rationale.* @impl/done

> ##WHY-A-TS-FRONTEND-IS-NOW-POSSIBLE **Why this is now possible (PROP-024).** Until the code-bearing-packages refactor, the conform engine was hardcoded inside the vibevm workspace — a Rust-only, vibevm-only tool. PROP-024 made a package a project that ships runnable code, and relocated `conform-core` + `rust-ai-native-conform-frontend` into `stack:org.vibevm.ai-native/rust-ai-native-lang`. `conform-core` is **language-neutral** by construction: it owns the `Fact` model, the rules-as-queries, the SARIF renderer, and the ratchet baseline, and it consumes facts through a `Frontend` trait that carries nothing Rust-specific (`rust-ai-native-conform-frontend` is just one implementation, over `syn`). So adding TypeScript to the discipline's *structural* gate is a new frontend, not a second engine. @impl/done

## 1. The division of labour with the native TS tooling {#division}

##TS-TOOLING-CARRIES-THE-TYPE-GENERATION-HALF The TypeScript cards already specify a rich, mature **type / generation** checker set — `@typescript-eslint`, `tsc --noEmit`, `tsd` / `expectTypeOf`, Twoslash, `fast-check` (GUIDE §14; the cards' Band-3 ops). @spec/done

##THAT-HALF-IS-WELL-TYPED-AND-EXAMPLES-TYPE-CHECK Those answer *"is this well-typed, and do the examples type-check?"* — the half TypeScript's compiler does natively and does superbly. @spec/done

##frontend-answers-the-structural-half-lead `typescript-ai-native-conform-frontend` answers the **other** half — the *structural / architectural* rules no type checker expresses, the ones `conform check` already enforces for Rust: @impl/done

- ##RULE-FILE-LENGTH-BUDGET the file-length budget (position is a resource); @impl/done
- ##RULE-CELL-ISOLATION cell isolation (a cell imports seams + core only, never sibling cells); @impl/done
- ##RULE-BANS-AS-FACTS the bans-as-facts (`any` / unchecked `as` / `!` / `@ts-ignore` in domain code — the §8 set) surfaced as conform findings in the Class-F `violates REQ …; fix surface: …` grammar, navigable back to the governing card; @impl/done
- ##RULE-DEVIATION-ESCAPE-HATCH the deviation escape hatch (`@ts-expect-error -- reason`, a recorded `deviates`), honoured the way `#[spec(deviates)]` is for Rust. @impl/done

##eslint-could-express-some-of-these ESLint *could* express some of these as custom rules. @spec/done

##ONE-ENGINE-ONE-GRAMMAR-ONE-BASELINE Routing them through conform instead keeps **one rule engine, one finding grammar, one ratchet baseline** across both languages, with the rules defined once in `conform-core` and fed by either frontend — so a rule cannot drift between the Rust and TypeScript projections. @impl/done

## 2. What it is {#what-it-is}

##TS-FRONTEND-IS-A-FACT-PRODUCER A fact producer: parse a `.ts` / `.tsx` file and emit the language-neutral `conform_core::Fact` stream the rules consume. @impl/done

- ##PARSER-IS-THE-COMPILER-API **Parser:** the TypeScript Compiler API (or `ts-morph` over it) — the most analyzable AST of any mainstream language (GUIDE §0, Advantage 2). Items with their decorators / attribute text, imports, construction sites, the `unsafe`-set tokens, whole-file metrics — the same fact shapes `rust-ai-native-conform-frontend` extracts from `syn`. @impl/done
- ##IMPLEMENTS-THE-FRONTEND-TRAIT **Implements `Frontend`:** an `id()` such as `"ts-tsc"`, a `version()` that bumps when the fact schema grows (retiring cache slots wholesale, exactly as the Rust frontend does), and `extract(file, crate, module, text) -> Vec<Fact>`. @impl/done
- ##UNPARSEABLE-FILE-YIELDS-ZERO-FACTS **Unparseable file → zero facts, never an error** (the B5 rule), so one malformed file does not blind the gate to the rest of the tree. @impl/done

##ENGINE-PARTS-ARE-REUSED-UNCHANGED The rules, the content-addressed store, the SARIF, the baseline — all reused unchanged from `conform-core`. @impl/done

##only-the-fact-source-is-new Only the fact source is new. @impl/done

## 3. The architectural question this brief leaves open {#open-question}

##CONFORM-CORE-HOMES-IN-THE-RUST-STACK `conform-core` is language-neutral but currently **homes in `stack:org.vibevm.ai-native/rust-ai-native-lang`** — it moved there with `rust-ai-native-conform-frontend`, its first and only consumer at the time (PROP-024 phase 4: the clean move set was conform-core + rust-ai-native-conform-frontend + rust-ai-native-env-audit). @spec/done

##A-TS-FRONTEND-NEEDS-CONFORM-CORE-REACHABLE A TypeScript frontend that reuses it needs `conform-core` reachable from a *different* package. @spec/done

##two-clean-resolutions-lead Two clean resolutions, deferred to when this tool is built: @spec/done

- ##RESOLUTION-PROMOTE-CONFORM-CORE **(a) Promote `conform-core` to `flow:org.vibevm.ai-native/core-ai-native`** (the language-neutral package): `rust-ai-native-conform-frontend` stays in the Rust stack, `typescript-ai-native-conform-frontend` ships in this one, both depend on the neutral core. This is the principled end-state — the engine is language-neutral, so it belongs in the language-neutral package. @spec/done
- ##RESOLUTION-CROSS-PACKAGE-DEPENDENCY **(b) Cross-package dependency:** this package depends on the Rust stack's `conform-core`. Simpler, but couples the TypeScript stack to the Rust one for no reason beyond where the code happens to sit today. @spec/done

##option-a-is-the-better-shape (a) is the better shape. @spec/done

##a-follow-up-move-not-a-blocker It is a follow-up move, not a blocker for the cards — and it is exactly the kind of relocation PROP-024 made cheap. @spec/done

## 4. The code-root {#code-root}

##FRONTEND-SHIPS-IN-THIS-PACKAGE-AS-ITS-CODE-ROOT When implemented, `typescript-ai-native-conform-frontend` ships **in this package** — `stack:org.vibevm.ai-native/typescript-ai-native-lang` — as the package's own code-root: the TypeScript mirror of how `rust-ai-native-lang` now carries `crates/rust-ai-native-conform-frontend`. @impl/done

##INSTALLING-THE-STACK-YIELDS-A-WORKING-GATE Installing the stack would then yield a working `.ts` structural gate, not a description of one (the PROP-024 promise, applied to TypeScript). @impl/done

##SHIPPABLE-TREE-EXCLUDES-BUILD-OUTPUT The shippable tree excludes build output (`node_modules/`, `dist/`, build caches) the same way it excludes `target/` for Rust (PROP-024 §2.2). @impl/done

##BINARY-IS-LANGUAGE-SUFFIXED Its binary is **`typescript-ai-native-conform`** — language-suffixed like the Rust stack's `rust-ai-native-conform`, so several discipline checkers can share a `PATH` without shadowing one another. @impl/done

## 5. The honest note {#honesty}

##SPECIFIED-NOT-BUILT This is **specified, not built** — like the TypeScript checker cards, status `specified`, awaiting the forthcoming VibeVM TypeScript surface as its pilot. @unknown

##NATIVE-TYPE-TOOLING-IS-REAL-TODAY The native type tooling (the Class-E `tsc` loop, the type-level test tools) is real and usable today; what waits is the *structural* gate — the language-neutral rules applied to TypeScript through this frontend. @unknown

##NOT-ON-THE-CRITICAL-PATH-FOR-TYPE-CHECKS-ONLY Nothing here is on the critical path for a TypeScript consumer who only wants the type checks; it is the path to giving TypeScript the *same architectural discipline* Rust has, through the *same engine*, once there is TypeScript code to hold to it. @spec/done
