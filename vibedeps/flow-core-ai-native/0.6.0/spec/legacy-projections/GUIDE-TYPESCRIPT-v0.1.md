# GUIDE — TypeScript/JavaScript under the Discipline, v0.1

**Status.** Beta; second T2 guide. Section structure is deliberately isomorphic to `GUIDE-RUST-v0.1.md` — cross-language diffing of the guides is a feature of the discipline, not a coincidence. Scope: only what the axioms require; formatting and generic style live in tooling.

Framing note — the inverse of Rust's. Rust *enforces*; TypeScript *permits everything*, because underneath it is JavaScript: mutable, dynamic, side-effectful, with an erased type system. The primary instrument of this guide is therefore **subset selection**, enforced by compiler flags and type-aware lints (the C++ situation, with a far better enforcement toolkit). Classical patterns dissolve here too, differently: Strategy = interface + composition root; Visitor = discriminated union + exhaustive `switch`; Decorator = wrapper object; Observer = an explicit event seam, never `EventEmitter` inheritance; Singleton = forbidden, pass it.

---

## 0. Language baseline {#baseline}

- **Cells are TypeScript (`.ts`).** Plain JS is tolerated only in scripts/config under `// @ts-check` with JSDoc types. Seam declarations are always TS.
- **tsconfig (load-bearing flags):** `strict`, `noUncheckedIndexedAccess` (indexing can miss), `exactOptionalPropertyTypes`, `noImplicitOverride`, `noFallthroughCasesInSwitch`, `noPropertyAccessFromIndexSignature`, `isolatedModules`, `verbatimModuleSyntax`; `module: NodeNext` (server / sidecar cells) or `bundler` (front-end cells).
- **ESM-only:** `"type": "module"`; `require`/CJS interop is quarantined in boundary modules.
- **Lint baseline:** typescript-eslint `strict-type-checked` + `stylistic-type-checked`; ESLint runs as a conform **evidence provider** via SARIF (ENGINE §2) — we cite its checks, we do not reimplement them.
- **Type-suppression policy:** `@ts-expect-error` (with a reason comment) is the **only** permitted suppression — it is xfail-strict by construction: the directive itself errors when the underlying error disappears, forcing promotion (BROWNFIELD §4 at the type level). `@ts-ignore` is banned. `any`, bare `as`, and non-null `!` are banned in cells; they are legal only in **boundary modules** (§4) with a one-line justification.
- **Workspace:** pnpm workspaces; tsc project references (`composite: true`) for incremental type-checking — the facts-class cache at the type level (axiom A2).
- **Tests:** vitest; property testing: fast-check (the proptest analog). All MIT/Apache-class tooling.

## 1. Cells {#cells}

A cell is a directory module behind one seam: single public entry (`index.ts`) exporting the seam implementation and nothing else.

- **Side-effect-free on import.** In JS, importing a module *executes* it — the language's most underrated landmine. Cell modules perform no top-level work (no I/O, no registration, no global mutation); `"sideEffects": false` in `package.json` must be honest. Enforced: lint + bundler audit.
- **No cross-cell barrels.** A barrel that re-exports across cells silently fuses them (and wrecks tree-shaking and circular-import analysis). The cell's `index.ts` is its only surface; sibling-cell imports are forbidden (R-002) — including via path aliases, which is why this check needs T-sem module resolution, not grep.
- **Platform capabilities are injected.** Cells never touch `process.env`, `fs`, `fetch`, or other ambient platform globals directly — those are seams, passed in at construction (ports-and-adapters; the sans-io instinct). Time and randomness SHOULD also be injected (determinism in tests). Consequence: cell tests need no module interception at all.
- **Promotion to a workspace package** when: heavy optional dependencies, independent publish boundary, or ~2 kLoC — mirrors the Rust crate-promotion procedure and is equally pilot-measured.

Cell manifest (JSDoc carrier, §5):

```ts
/**
 * @spec implements spec://vibevm/modules/vibe-resolver/PROP-003#solver-upgrade r2
 * @cell seam=DepSolver variant=sat replaces=naive flag=solver
 */
export class SatDepSolver implements DepSolver { /* … */ }
```

## 2. Seams {#seams}

- A seam is a TS `interface` living in core/seams — never inside a cell.
- **Composition over inheritance is a MUST at seams:** no abstract base classes with behavior (template methods are hidden control flow, R-021). A seam exposes functions; state stays inside the cell.
- Structural typing caveat: TS interfaces match by shape, so accidental conformance is possible. For identity-critical seams, a phantom brand (`readonly __seam?: "DepSolver"`) MAY be used; the registry remains the real gatekeeper.
- Seam methods that can fail in expected ways return `Result` (§4) — the failure surface is part of the interface text, visible to the compiler.

## 3. Registry and flags {#flags}

R-001 binding — flag at the seam, never in the veins:

```ts
// src/registry.ts — the only module reading selection flags
// and the only legal site of computed dynamic import().
export async function depSolver(flags: Flags, provider: DepProvider): Promise<DepSolver> {
  switch (flags.get("solver")) {              // provenance: default | env | cli | lockfile
    case "sat":  return new (await import("./cells/sat-dep-solver/index.js")).SatDepSolver(provider);
    default:     return new NaiveDepSolver(provider);   // eager: imported statically above
  }
}
```

- **Two tiers, never confused:** bundle-time constants (`define` / `import.meta.env` → dead-code elimination) answer *"is the code in the bundle"* — the cargo-feature analog; runtime flags answer *"is the cell selected"*. A runtime flag must not change the type surface; a bundle constant must not encode product choice.
- Eager vs lazy cell loading (static import vs `await import()` in the registry) is the code-level mirror of vibevm's delivery modes; lazy is a registry decision, never a cell's own.
- **No DI containers.** Inversify/tsyringe/Nest-style `reflect-metadata` injection is link-time magic: it violates R-021 and blinds T-syn analysis. Explicit constructor injection + the hand-written registry `switch` is the system's table of contents.

## 4. Errors as contract {#errors}

TypeScript cannot type `throws`; an exception is invisible in the seam's signature — an A1 hole the language will never close. Therefore:

- **Expected failures are values.** Seams return a discriminated Result — the minimal core type, no framework dependency (`neverthrow` MAY be adopted later; the shape is what matters):

  ```ts
  type Result<T, E> = { ok: true; value: T } | { ok: false; error: E };
  ```

  Error objects carry `code` and the violated REQ URI (`static spec = "spec://…#req-…" as const` on error classes/factories) — conform checks coverage via the index; user-facing rendering appends the URI (PROP-014 §2.6).
- **`throw` is for invariant violations only** — the panic analog. Never throw strings or bare `Error` in cells; chain with `cause` (ES2022).
- **Async hygiene (MUST, type-aware lint):** no floating promises; no unhandled rejections; no `async` functions passed where `void`-returns are expected.
- **Exhaustiveness:** closed sets are discriminated unions with compiler-checked exhaustive `switch` (`assertNever` in `default`, or `satisfies`-based patterns); the type-aware switch-exhaustiveness lint gates it.
- **Source maps are the release map.** Here the discipline's founding analogy becomes literal: production builds ship (or retain server-side) `.map` files keyed by build id; the A1 chain *minified frame → original source → item → REQ* must resolve end-to-end. An error report that dies at `chunk-7f3a.js:1:48211` is an explainability defect, not a cosmetic one.

## 5. specmark carrier {#specmark}

JSDoc tags — zero runtime cost, valid in `.ts` and `.js`, parseable by both tree-sitter and the TS compiler API, and (a pleasant simplification over Rust) the carrier is already the hover-doc surface, so no doc-injection machinery is needed:

```
/** @spec implements <uri> r<N> */            one edge per line; tags repeat
/** @spec deviates <uri> r<N> reason="…" */   reason mandatory
/** @verifies <uri> r<N> */                   on tests
/** @specScope <uri> r<N> */                  file-top block: module-level inheritance
```

Standard (TC39) decorators were considered and rejected as carrier: runtime weight, `.js` incompatibility, and no analytical gain over JSDoc. ≤3 edges per item or split (same lint as Rust).

## 6. Naming (R-020/R-021 bindings) {#naming}

- Canonical cell type name is computed from the manifest: `{Variant}{Seam}` → `SatDepSolver`. Hand-written names are linted against the computation. Length free, ambiguity not — `MultiRegistryResolverWithRedirectFollowing` is fine *iff* every token is manifest-backed.
- The TS ecosystem loves both verbose names and hidden magic; this guide keeps the first and bans the second. **Forbidden in cells regardless of elegance:** Proxy-based domain APIs; getters/setters with side effects; prototype or builtin patching; behavior-bearing decorators; `Symbol.toPrimitive`/`valueOf` coercion tricks; module-graph interception in tests (`vi.mock`/`jest.mock` — inject capabilities instead, §1); computed dynamic `import()` outside the registry.

## 7. Replacement protocol (R-040 binding) {#replacement}

A cell with `replaces=…` ships a differential oracle: fast-check property tests asserting agreement with the old cell across the seam (documented-divergence list otherwise), `@verifies`-tagged. **Snapshot/golden discipline (brownfield binding):** snapshots are characterization artifacts; CI never updates them; local updates require the promotion protocol — a debt/intent reference in the commit body. Blanket `--update` is the snapshot ecosystem's graveyard mode and is banned.

## 8. Risk table (what conform must cover for TS/JS) {#risks}

| Footgun | Rule | Tier |
|---|---|---|
| `any` / `as` / `!` / `@ts-ignore` outside boundary modules | §0 | T-syn |
| floating promise / unhandled rejection | §4 | T-sem |
| top-level side effects in a cell module | §1 | T-sem |
| cell importing a sibling cell (incl. via aliases/barrels) | R-002 | T-syn + T-sem |
| expected failure thrown across a seam / thrown string | §4 | T-sem |
| non-exhaustive switch on a discriminated union | §4 | T-sem |
| Proxy / prototype patching / behavior decorators in cells | §6 | T-syn |
| direct `process.env` / `fs` / `fetch` in cells | §1 | T-syn |
| computed dynamic `import()` outside the registry | §3, §6 | T-syn |
| `require` / CJS interop inside cells | §0 | T-syn |
| flag read outside the registry | R-001 | T-syn |
| snapshot updated without a promotion reference | §7 | process check |
| public export without own/inherited spec edge | PROP-014 §3.2-6 | T-syn + index |

## 9. Doc layer {#docs}

TSDoc on every tagged export states the practically-important behavior: error codes and their REQ URIs, async semantics (cancellation, ordering, retry expectations), edge cases, performance traps. TSDoc is the human-facing detail layer; the spec stays thin; the ledger renders machine explanations from both. Duplication between TSDoc and spec is a defect on the spec side.

---

**First carrier note.** vibevm has no TypeScript today; the first TS cell under this guide is already designated by the architecture itself — the conform engine's TypeScript sidecar (ENGINE §2), the process that asks the TS compiler about its own language. Self-hosting, again.

*Any rule binding here without a corresponding conform check (or explicit `WISH` mark in the Charter rule record) by the first TS carrier milestone is removed rather than carried as aspiration.*
