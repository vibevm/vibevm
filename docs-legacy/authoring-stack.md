# Authoring a `stack` package

A **stack** is a *language / framework target* — the runtime surface a feat compiles against. Stacks describe how feats become real software: which language, which framework, which build system, which deployment target, which capabilities the runtime provides.

Examples (planned for M1.5):
- `stack:rust-cli` — a binary CLI built with `clap`, `tracing`, standard Rust tooling.
- `stack:rust-axum` — a web service in Axum, with `sqlx`, `tokio`, structured logging.
- `stack:typescript-next` — a Next.js application with TypeScript, Tailwind, Prisma.

A stack is **feat-agnostic at authoring time** — the same `stack:rust-cli` should compile any feat that declares it needs the capabilities `stack:rust-cli` provides. Stacks supply infrastructure; feats supply business logic. Together they produce code.

> **Status note.** Today (M1.1-revision) `vibe install stack:<name>` works and lays the spec content into the consumer project. The actual code-generation step (`vibe build feat:<name> --stack rust-cli`) is M1.5 ([`VIBEVM-SPEC.md` §11.3](../VIBEVM-SPEC.md)). This guide covers stack authoring; the build pipeline is documented when M1.5 ships.

## Anatomy of a stack package

```
stack-<name>/
├── vibe.toml                        # manifest, carries a [package] table
├── README.md
├── boot/
│   └── stack-<name>.md               # optional — surfaces the active stack at boot
└── spec/
    └── stacks/
        └── <name>/
            ├── STACK.md               # canonical "what does this stack provide"
            ├── conventions.md         # naming / layout / code-style decisions
            ├── capabilities/
            │   ├── ui-host.md         # one file per capability the stack provides
            │   ├── session-store.md
            │   └── …
            ├── tooling.md             # build / test / lint / format invocations
            └── deployment.md          # how to ship a build of this stack
```

After `vibe install stack:<name>`, the package's whole published tree is materialised verbatim into a slot under the workspace-root `vibedeps/` tree:

```
<workspace-root>/
└── vibedeps/
    └── stack-<name>/
        └── <version>/                  # the stack's published tree, verbatim
            ├── vibe.toml
            ├── boot/stack-<name>.md
            └── spec/stacks/<name>/
```

A materialised package *is* its verbatim subtree under its `vibedeps/` slot — `vibe install` never writes into a consuming node's authored `spec/` ([the loading model](loading-model.md)).

## What goes in `STACK.md`

The stack's spec is the build-time contract: a build LLM reading STACK.md alongside a feat's SPEC.md must have everything it needs to generate real code.

Concrete sections:

1. **Language + version.** "Rust 1.93+", "TypeScript 5.4+", "Python 3.12+".
2. **Framework.** Which web framework / CLI library / UI toolkit.
3. **Project layout.** Where source files live, where tests live, naming conventions for modules / files / types.
4. **Build / test / lint commands.** Concrete shell invocations a CI can run as-is.
5. **Capabilities provided.** Each one in its own file under `capabilities/`, with the contract spelled out: function/type signature, semantics, error model, any state expectations.
6. **External deps.** Crates / packages / images this stack pulls in. Why each one was chosen (the design notes that prevent yak-shaving when a build LLM is tempted to "improve" a deliberate choice).
7. **Anti-patterns.** What NOT to do — common Rust / TypeScript / Python smells the stack rejects.

A consumer who installs your stack and reads STACK.md should know how to add a hand-written file that fits the stack's conventions, even before the build LLM ships.

## Manifest: `vibe.toml`

A publishable package carries a `vibe.toml` with a `[package]` table.

```toml
[package]
name = "rust-cli"
kind = "stack"
version = "0.1.0"
authors = ["You <you@example.com>"]
license = "EULA"
description = "Rust CLI stack: clap + tracing + thiserror + standard cargo workspace."
keywords = ["rust", "cli", "stack"]

[compatibility]
min_vibe_version = "0.1.0"

# Optional — surface "you are building against rust-cli" at session start.
# `category` sets the band in the computed boot sequence; `source` is the
# path to the boot file inside the package. No `filename`, no `[writes]`.
[boot_snippet]
category = "stack"
source = "boot/stack-rust-cli.md"

# Stacks are providers — this is where most of the value lands.
[provides]
capabilities = [
    "cli:entrypoint@0.1.0",
    "log:structured@0.1.0",
    "config:env-file@0.1.0",
    "test:runner@0.1.0",
]

# Stacks rarely require packages directly. They might require a shared
# flow (e.g. a coding-discipline flow) every project using this stack
# is expected to follow.
[requires]
capabilities = []

# [requires.packages] is a table: pkgref → constraint string.
[requires.packages]
"flow:atomic-commits" = "^0.1"
```

The `[provides].capabilities` list is the stack's most important manifest entry. Every capability listed here is a contract the stack promises to satisfy at build time, so any feat that requires the same capability can be paired with this stack.

## Capability files

For every entry in `[provides].capabilities`, ship a markdown file under `spec/stacks/<name>/capabilities/<capability-name>.md` that nails down the contract:

- **Capability name.** As declared in the manifest (`cli:entrypoint`, `log:structured`).
- **Version.** Which capability version this file describes; bumps follow the stack's package version.
- **What it provides.** The function / type / API surface a feat can rely on.
- **Semantic guarantees.** Idempotency, ordering, error handling — anything a feat author may need to assume.
- **Limitations.** What this capability does NOT cover. Steers feat authors who'd otherwise reach for it for the wrong reason.
- **Examples.** Code snippets in the stack's language showing how a feat would consume the capability.

A feat that declares `[requires].capabilities = ["cli:entrypoint@^0.1"]` can be confident the stack supplies a stable surface matching this file.

## Choosing capability names

Capability names are the wire vocabulary across packages. Pick conservatively:

- **Namespace by domain**, not by package: `ui:`, `db:`, `auth:`, `log:`, `cli:`, `config:`, `test:`. The same capability namespace can have impls in multiple stacks.
- **Be specific enough to be useful**: `ui:landing-page-host@0.1` is more useful than `ui:host@0.1` because a feat author knows what kind of UI host it's getting.
- **Be loose enough to be portable**: a capability shouldn't bake in a particular language. `db:postgres-pool@0.1` is fine; `db:rust-sqlx-pool@0.1` is too narrow.

When a project standardises on a vocabulary, document it as its own `flow:<project>-capability-vocabulary` package. Then every stack and feat in the project shares the same naming.

## Versioning

Same SemVer rules as flows and feats:

- Patch: STACK.md wording, capability prose tweaks, no contract change.
- Minor: additive — new capabilities, new optional sections in capability files, deps bumped within their own SemVer.
- Major: breaking — capability removed, capability semantics changed, language / framework version bumped beyond compatibility.

Stacks tend to bump majors more often than flows because their dep tree is bigger and language ecosystems move.

## Publishing

```bash
vibe registry publish ./path/to/your/stack-package
```

Same procedure as flows and feats; see [`vibe registry publish`](commands/registry-publish.md).

## Tips

- **Spec the conventions, not the configuration.** Ship the *decisions* (we use `clap` over `structopt`; we use `thiserror` over `anyhow` for library code; we use `tracing` over `log`); leave the *application configuration* (env vars, secret keys, tenant names) to the project that consumes the stack.
- **Each capability is its own file.** Resist the urge to lump them all into STACK.md — when a feat authors something specific, they should be able to read just the capability file they care about.
- **Document anti-patterns.** A list of "what we deliberately don't do" is more useful than a list of "what we do" when you're integrating with an LLM that will otherwise try every option. "We don't use macros for plumbing"; "we don't bring in async-trait if a regular trait works"; "we don't use a global static logger".
- **Match the language's idioms.** A Rust stack should look idiomatic in Rust; a TypeScript stack should look idiomatic in TypeScript. The stack's job is to encode taste; "taste" is dialect-specific.

## Related

- [authoring-feat.md](authoring-feat.md) — the consumer side of the capability contract.
- [authoring-flow.md](authoring-flow.md) — discipline modules; a stack often `[requires]` one or two flows.
- [`VIBEVM-SPEC.md` §4.1](../VIBEVM-SPEC.md) — the four-kind package model.
- [`PROP-002 §2.9`](../spec/modules/vibe-registry/PROP-002-decentralized-registry.md#capability) — capability syntax and the depsolver's resolution rules.
