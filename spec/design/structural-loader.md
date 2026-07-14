# The structural loader — honouring directives without the inline compiler {#root}

**Status:** DESIGN — provisional (PROP-035 §13). These instructions are authored but **not yet wired into any live boot**: migration is demo-corpus-first, `org.vibevm.world` next, vibevm's own boot last (PROP-035 §15). This document is the reference text; a package that adopts the spec-compiler format will load it (or its successor) first.

---

## What this is {#what}

The **first-loaded instructions** that make an agent honour the spec-compiler directives (`#use` / `#embed` / `#source` / `@spec`) in **structural mode** — reading on demand — the lazy counterpart to the algorithmic inline compiler (PROP-035 §2). Until a hard algorithmic agent exists (§14), an LLM executes them.

A project or package that manages its content this way MUST load these instructions before anything else; without them the directives are inert prose. A managed project or package that omits them is **broken** (§13), and the project/package-creation tools must inject them.

The one hard contract: your structural reading MUST reach the **same effective spec** the inline compiler would produce. The inline compiler is the reference semantics; you are its lazy executor (§2).

---

## `#use spec://…` and `@spec://…` — dependency edges {#use}

When you meet `#use spec://X` — or the in-place `@spec://X` — treat X as a dependency:

- **Read X before using anything defined in it.** Not eagerly on sight, but **before first use** of any entity X names.
- **Reads cascade.** If X itself `#use`s Y, read Y too, and so on. This is how a large package is entered through one file and expands only along what is actually used (tree-shaking) rather than loaded whole.
- **`@spec://` (with the `@`) is mandatory** — always read it on first encounter. A **bare `spec://`** (no `@`) is discretionary: read it only if you need what it names.

---

## `#embed spec://…` — the macro splice {#embed}

`#embed` is a materialization-time macro (§7.1). In a properly installed package it is **already expanded** — you will normally see the spliced text, not the directive. If you do meet an unexpanded `#embed spec://X`, read X's section and treat its text as spliced in place.

---

## `#source spec://…` — contract → implementation {#source}

A short `contract` section names its heavy implementation via `#source spec://X`. When you need the **full behaviour** behind a contract section (not just its summary), read the `#source` target and combine it with the contract text by the marker on the **source** heading:

- `:replace` — the source text is canonical; ignore the contract text.
- `:add` (the default) — use **both**, contract first then source.

A contract section with no `#source`, or whose behaviour you do not need, is read as-is.

---

## The read-set — read once, survive compaction {#read-set}

To avoid re-reading the same target endlessly, keep a persistent **read-set** at `.vibe/session/read-set.json`. Before reading an `@spec` target, consult it; after reading, append `{ specpath, content_hash }`. Reuse specmap's `content_hash`, so a **changed** target is re-read.

This survives context compaction because **these instructions are re-read at boot**, so the habit of consulting the read-set is restored even after the conversation is summarized.

Crucially, the read-set records **what exists and where, not what is currently in your context**. Compaction evicts the *text* but not the *fact*. So the rule is: read an `@spec` target if **(a)** it is not in the read-set, **or (b)** it is in the read-set but its content is no longer in your context. Re-reading is cheap — the file sits in `vibedeps/`. Think of the read-set as a linker symbol table, but for what you have read.

---

## Never {#never}

- Never use an entity from a `#use` / `@spec` target without reading the target first.
- Never re-read an `@spec` target that is both already in the read-set **and** still in your context.
- Never treat a bare `spec://` as mandatory — only `@spec://` compels a read.
- Never let structural reading diverge from what the inline compiler would produce (§2).
