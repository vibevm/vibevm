# Tool Spec (high-level): `typescript-ai-native-conform-frontend` — the TypeScript frontend for the language-neutral conform engine {#root}

<status stage="spec" state="done"/>

@fact:status-line *Status: **SHIPPED** (0.3.0, the deferrals-closeout campaign) — this package's `crates/typescript-ai-native-conform-frontend` (`id = "ts-tsc"`) + `crates/typescript-ai-native-conform` (binary **`typescript-ai-native-conform`**), fed by the Compiler-API extractor at `tools/ts-extract/` through the `typescript-ai-native-extract-bridge` NDJSON protocol.* @status:impl/done

@fact:TYPESCRIPT-COUNTERPART-OF-THE-RUST-FRONTEND *The TypeScript counterpart to `rust-ai-native-conform-frontend`: it gives `.ts` code the SAME structural discipline gate that `.rs` code gets from `rust-ai-native-conform check` — by feeding TypeScript facts into the language-neutral conform engine, not by re-implementing the rules in ESLint.* @status:impl/done

@fact:SECTION-THREE-QUESTION-WAS-ANSWERED *The §3 question below was answered by the engine consolidation: option (a) happened — `conform-core` now lives in `flow:org.vibevm.ai-native/core-ai-native`, both stacks vendor it (`cargo xtask sync-engines`).* @status:impl/done

@fact:brief-remains-as-design-rationale *This brief remains as the design rationale.* @status:impl/done

> @fact:WHY-A-TS-FRONTEND-IS-NOW-POSSIBLE **Why this is now possible (PROP-024).** Until the code-bearing-packages refactor, the conform engine was hardcoded inside the vibevm workspace — a Rust-only, vibevm-only tool. PROP-024 made a package a project that ships runnable code, and relocated `conform-core` + `rust-ai-native-conform-frontend` into `stack:org.vibevm.ai-native/rust-ai-native-lang`. `conform-core` is **language-neutral** by construction: it owns the `Fact` model, the rules-as-queries, the SARIF renderer, and the ratchet baseline, and it consumes facts through a `Frontend` trait that carries nothing Rust-specific (`rust-ai-native-conform-frontend` is just one implementation, over `syn`). So adding TypeScript to the discipline's *structural* gate is a new frontend, not a second engine. @status:impl/done

## 1. The division of labour with the native TS tooling {#division}

@fact:TS-TOOLING-CARRIES-THE-TYPE-GENERATION-HALF The TypeScript cards already specify a rich, mature **type / generation** checker set — `@typescript-eslint`, `tsc --noEmit`, `tsd` / `expectTypeOf`, Twoslash, `fast-check` (GUIDE §14; the cards' Band-3 ops). @status:spec/done

@fact:THAT-HALF-IS-WELL-TYPED-AND-EXAMPLES-TYPE-CHECK Those answer *"is this well-typed, and do the examples type-check?"* — the half TypeScript's compiler does natively and does superbly. @status:spec/done

@fact:frontend-answers-the-structural-half-lead `typescript-ai-native-conform-frontend` answers the **other** half — the *structural / architectural* rules no type checker expresses, the ones `conform check` already enforces for Rust: @status:impl/done

- @fact:RULE-FILE-LENGTH-BUDGET the file-length budget (position is a resource), carried by the root `conform.toml` key `max_file_lines`, with two sibling root keys feeding `invariant-comment-position` (R3-003) — `invariant_comment_markers` and `invariant_comment_min_file_lines`; a fourth, `sarif_reports`, feeds the SARIF-ingest path described below. All four are root keys, language-neutral, NOT under `[typescript]`, and what they mean and default to is described once, in `ENGINE-CONFORM §6` (the policy file) — this surface names which ones the TS rules read, not their values; @status:impl/done
- @fact:RULE-CELL-ISOLATION cell isolation (a cell imports seams + core only, never sibling cells); @status:impl/done
- @fact:RULE-BANS-AS-FACTS the bans-as-facts (`any` / unchecked `as` / `!` / `@ts-ignore` in domain code — the §8 set) surfaced as conform findings in the Class-F `violates REQ …; fix surface: …` grammar, navigable back to the governing card; @status:impl/done
- @fact:RULE-DEVIATION-ESCAPE-HATCH the deviation escape hatch (`@ts-expect-error -- reason`, a recorded `deviates`), honoured the way `#[spec(deviates)]` is for Rust. @status:impl/done

@fact:THE-SARIF-CITATION-PATH-IS-FED-BY-FOREIGN-LINTERS-NOT-BY-TS-EXTRACT One more rule is mounted in this gate and is deliberately absent from the list above: `lint-suppression-needs-reason`, whose facts come from SARIF **ingest** — eslint and its plugins — rather than from this frontend. It is fed by the fourth root key, `sarif_reports`, and it is the T-sem citation path: a Discipline rule citing a foreign linter's diagnosis as its own evidence. Listing it beside the extractor's own rules would claim a fact source this frontend does not have. @status:impl/done

@fact:eslint-could-express-some-of-these ESLint *could* express some of these as custom rules. @status:spec/done

@fact:ONE-ENGINE-ONE-GRAMMAR-ONE-BASELINE Routing them through conform instead keeps **one rule engine, one finding grammar, one ratchet baseline** across both languages, with the rules defined once in `conform-core` and fed by either frontend — so a rule cannot drift between the Rust and TypeScript projections. The TypeScript gate carries the same coverage invariant as the sibling stacks: every cell the `[typescript]` roots enumerate is either `[typescript] gated` or carries a `[[typescript.exempt]]` entry with its reason (`{unit, reason}` — the cell is TS's gate unit), enforced by the engine in `check` and `freeze` — an unclassified cell, a duplicate, a both-listed cell, a reasonless or a ghost exemption all fail loudly with cell-naming messages. A present config whose TS roots resolve to zero cells warns instead of passing silently, and a gated cell the scan attributed no file to is flagged vacuous. @status:impl/done

## 2. What it is {#what-it-is}

@fact:TS-FRONTEND-IS-A-FACT-PRODUCER A fact producer: parse a `.ts` / `.tsx` file and emit the language-neutral `conform_core::Fact` stream the rules consume. @status:impl/done

- @fact:PARSER-IS-THE-COMPILER-API **Parser:** the TypeScript Compiler API (or `ts-morph` over it) — the most analyzable AST of any mainstream language (GUIDE §0, Advantage 2). Items with their decorators / attribute text, imports, construction sites, the `unsafe`-set tokens, whole-file metrics — the same fact shapes `rust-ai-native-conform-frontend` extracts from `syn`. @status:impl/done
- @fact:IMPLEMENTS-THE-FRONTEND-TRAIT **Implements `Frontend`:** an `id()` such as `"ts-tsc"`, a `version()` that bumps when the fact schema grows (retiring cache slots wholesale, exactly as the Rust frontend does), and `extract(file, crate, module, text) -> Vec<Fact>`. @status:impl/done
- @fact:UNPARSEABLE-FILE-YIELDS-ZERO-FACTS **Unparseable file → zero facts, never an error** (the B5 rule), so one malformed file does not blind the gate to the rest of the tree. @status:impl/done

@fact:ENGINE-PARTS-ARE-REUSED-UNCHANGED The rules, the content-addressed store, the SARIF, the baseline — all reused unchanged from `conform-core`. @status:impl/done

@fact:only-the-fact-source-is-new Only the fact source is new. @status:impl/done

## 3. The architectural question this brief leaves open {#open-question}

@fact:CONFORM-CORE-HOMES-IN-THE-RUST-STACK `conform-core` is language-neutral but currently **homes in `stack:org.vibevm.ai-native/rust-ai-native-lang`** — it moved there with `rust-ai-native-conform-frontend`, its first and only consumer at the time (PROP-024 phase 4: the clean move set was conform-core + rust-ai-native-conform-frontend + rust-ai-native-env-audit). @status:spec/done

@fact:A-TS-FRONTEND-NEEDS-CONFORM-CORE-REACHABLE A TypeScript frontend that reuses it needs `conform-core` reachable from a *different* package. @status:spec/done

@fact:two-clean-resolutions-lead Two clean resolutions, deferred to when this tool is built: @status:spec/done

- @fact:RESOLUTION-PROMOTE-CONFORM-CORE **(a) Promote `conform-core` to `flow:org.vibevm.ai-native/core-ai-native`** (the language-neutral package): `rust-ai-native-conform-frontend` stays in the Rust stack, `typescript-ai-native-conform-frontend` ships in this one, both depend on the neutral core. This is the principled end-state — the engine is language-neutral, so it belongs in the language-neutral package. @status:spec/done
- @fact:RESOLUTION-CROSS-PACKAGE-DEPENDENCY **(b) Cross-package dependency:** this package depends on the Rust stack's `conform-core`. Simpler, but couples the TypeScript stack to the Rust one for no reason beyond where the code happens to sit today. @status:spec/done

@fact:option-a-is-the-better-shape (a) is the better shape. @status:spec/done

@fact:a-follow-up-move-not-a-blocker It is a follow-up move, not a blocker for the cards — and it is exactly the kind of relocation PROP-024 made cheap. @status:spec/done

## 4. The code-root {#code-root}

@fact:FRONTEND-SHIPS-IN-THIS-PACKAGE-AS-ITS-CODE-ROOT When implemented, `typescript-ai-native-conform-frontend` ships **in this package** — `stack:org.vibevm.ai-native/typescript-ai-native-lang` — as the package's own code-root: the TypeScript mirror of how `rust-ai-native-lang` now carries `crates/rust-ai-native-conform-frontend`. @status:impl/done

@fact:INSTALLING-THE-STACK-YIELDS-A-WORKING-GATE Installing the stack would then yield a working `.ts` structural gate, not a description of one (the PROP-024 promise, applied to TypeScript). @status:impl/done

@fact:SHIPPABLE-TREE-EXCLUDES-BUILD-OUTPUT The shippable tree excludes build output (`node_modules/`, `dist/`, build caches) the same way it excludes `target/` for Rust (PROP-024 §2.2). @status:impl/done

@fact:BINARY-IS-LANGUAGE-SUFFIXED Its binary is **`typescript-ai-native-conform`** — language-suffixed like the Rust stack's `rust-ai-native-conform`, so several discipline checkers can share a `PATH` without shadowing one another. @status:impl/done

## 5. The honest note {#honesty}

@fact:SPECIFIED-NOT-BUILT This is **specified, not built** — like the TypeScript checker cards, status `specified`, awaiting the forthcoming VibeVM TypeScript surface as its pilot. @status:unknown

@fact:NATIVE-TYPE-TOOLING-IS-REAL-TODAY The native type tooling (the Class-E `tsc` loop, the type-level test tools) is real and usable today; what waits is the *structural* gate — the language-neutral rules applied to TypeScript through this frontend. @status:unknown

@fact:NOT-ON-THE-CRITICAL-PATH-FOR-TYPE-CHECKS-ONLY Nothing here is on the critical path for a TypeScript consumer who only wants the type checks; it is the path to giving TypeScript the *same architectural discipline* Rust has, through the *same engine*, once there is TypeScript code to hold to it. @status:spec/done
