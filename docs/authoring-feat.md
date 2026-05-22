# Authoring a `feat` package

A **feat** is a *functional feature* — a self-contained capability of an application, expressed entirely as specification. Where flows describe how the team works (process), feats describe what the project does (product). Examples (planned in M1.5):

- `feat:welcome-page` — a landing page with a single call-to-action.
- `feat:user-authentication` — sign-in / sign-out / password reset flow.
- `feat:billing-portal` — view invoices, change plan, manage payment methods.

A feat is **stack-agnostic at authoring time**. The same `feat:welcome-page` is meant to compile against `stack:rust-cli`, `stack:rust-axum`, `stack:typescript-next`, etc. The feat carries the *what*; the stack carries the *how*. This separation is the heart of vibevm's "spec-driven, multi-stack" pitch.

> **Status note.** Today (M1.1-revision) `vibe install feat:<name>` works and lays the spec content into the consumer project. `vibe build feat:<name> --stack <stack-name>` — which actually generates code — is M1.5 ([`VIBEVM-SPEC.md` §11.3](../VIBEVM-SPEC.md)). This guide covers the authoring side; the build flow is documented separately when M1.5 ships.

## Anatomy of a feat package

```
feat-<name>/
├── vibe.toml                       # manifest, carries a [package] table
├── README.md
├── boot/
│   └── feat-<name>.md              # optional — only if the feat needs front-page mention
└── spec/
    └── feats/
        └── <name>/
            ├── SPEC.md             # canonical "what does this feat do"
            ├── acceptance.md       # observable acceptance criteria
            ├── ui-flows.md         # screen / interaction flows (UI feats)
            ├── data-model.md       # entities and relationships
            ├── api.md              # REST / RPC surface, when applicable
            └── failure-modes.md    # what should happen when things go wrong
```

After `vibe install feat:<name>`, the package's whole published tree is materialised verbatim into a slot under the workspace-root `vibedeps/` tree:

```
<workspace-root>/
└── vibedeps/
    └── feat-<name>/
        └── <version>/                  # the feat's published tree, verbatim
            ├── vibe.toml
            └── spec/feats/<name>/
```

A materialised package *is* its verbatim subtree under its `vibedeps/` slot — `vibe install` never writes into a consuming node's authored `spec/` ([the loading model](loading-model.md)).

## What goes in `SPEC.md`

The feat's spec is what the build LLM reads and turns into code. Be ruthlessly explicit. Avoid handwaving. Concrete sections:

1. **Purpose.** Why does this feat exist? What user problem does it solve? One paragraph.
2. **Inputs.** What data does the feat consume — at runtime (user input, request parameters, config) and at build time (capability requirements from the stack)?
3. **Outputs.** What does the feat produce — UI screens, API responses, side effects (emails sent, records updated)?
4. **Behaviour rules.** Numbered, testable. Each rule is an invariant the implementation must hold.
5. **State transitions.** If the feat is stateful, draw the state machine. Sketch is fine — Markdown table works.
6. **Dependencies on capabilities.** What does the feat assume the stack provides? Examples: a database connection, a session store, a logger, a clock.

Two consumers: a human reading SPEC.md should be able to design tests for the feat; an LLM reading SPEC.md plus a stack's spec should be able to generate a runnable implementation.

## Acceptance criteria

`acceptance.md` is a checklist of observable outcomes that prove the feat works. Frame each as Given / When / Then or as a numbered behavioural rule. Treat it like a test plan written before the test code exists — `vibe build` will use it to generate the test code itself.

## Manifest: `vibe.toml`

A publishable package carries a `vibe.toml` with a `[package]` table.

```toml
[package]
name = "welcome-page"
kind = "feat"
version = "0.1.0"
authors = ["You <you@example.com>"]
license = "EULA"
description = "Landing page with a single call-to-action."
keywords = ["ui", "landing", "marketing"]

[compatibility]
min_vibe_version = "0.1.0"
# This feat needs a stack to compile against. Listing the kind here
# is a hint; the actual constraint is in [requires] / [[requires_any]].
requires_kinds = ["stack"]

# Optional: only ship a boot snippet if you want the feat surfaced at
# session start (e.g. a project's headline feat). `category` sets the
# band in the computed boot sequence; `source` is the path to the boot
# file inside the package. There is no `filename` field, no `[writes]`.
# [boot_snippet]
# category = "flow"
# source = "boot/feat-welcome-page.md"

[provides]
capabilities = []   # feats rarely provide; they consume

[requires]
# No package requirements here — an empty [requires.packages] table is
# simply omitted. Common pattern: declare a generic capability the feat
# needs and let the stack provide it. The depsolver pairs them up at
# install time.
capabilities = ["ui:landing-page-host@^0.1"]

# Use [[requires_any]] when several stacks could host the feat.
# [[requires_any]]
# one_of = ["stack:rust-axum@^0.1", "stack:typescript-next@^0.1"]
```

`requires_kinds = ["stack"]` is a hint to humans and to a future `vibe check` rule that the feat won't fully build without a stack present. The dep-solver doesn't enforce it today; explicit `[requires]` / `[[requires_any]]` entries do the actual work.

## Capability requirements

The cleanest way for a feat to declare what it needs from a stack is via `[requires].capabilities`. Examples:

```toml
[requires]
capabilities = [
    "ui:landing-page-host@^0.1",   # the stack must provide a UI host
    "session:store@^0.1",           # the stack must provide a session store
    "log:structured@>=0.1",         # the stack must provide a structured logger
]
```

A stack package's `[provides].capabilities` declares what it can satisfy. The depsolver matches them up — `feat:welcome-page` can be installed alongside any stack that provides the listed capabilities at compatible versions, regardless of which specific stack package the user picked.

Per [PROP-002 §2.9](../spec/modules/vibe-registry/PROP-002-decentralized-registry.md#capability) capability syntax is `<namespace>:<name>[@<version-or-range>]`. Pick stable namespace names (`ui`, `db`, `auth`, `log`) and document them in your project — capability names are part of your project's wire vocabulary.

## Versioning

Same SemVer rules as flows ([authoring-flow.md §Versioning](authoring-flow.md#versioning)):

- Patch: spec wording, no behaviour change.
- Minor: additive — new optional sections, new acceptance criteria that existing implementations satisfy.
- Major: breaking — behaviour change, capability contract change, removed acceptance criteria.

Pre-1.0 (`0.x`), even minor bumps may break consumers.

## Publishing

```bash
vibe registry publish ./path/to/your/feat-package
```

Same procedure as flows; see [`vibe registry publish`](commands/registry-publish.md).

## Tips

- **Write SPEC.md before code.** The point of a feat is "spec first, code generated from spec". If you find yourself documenting code, you've inverted the flow.
- **Prefer capability requirements over package requirements.** `[requires].capabilities` lets one feat work with many stacks; `[requires].packages` ties it to one. Use packages only when you need a specific implementation, not just any provider.
- **Acceptance criteria are tests in waiting.** Phrase them so a build tool can lift them straight into a test file.
- **Don't fork a stack's API surface.** A feat consumes capabilities; if your feat needs to assume a particular database schema or HTTP framework, that constraint belongs in the *stack* (which provides), not in the *feat* (which consumes).

## Related

- [authoring-stack.md](authoring-stack.md) — the build target side of the contract.
- [authoring-flow.md](authoring-flow.md) — discipline-side packages.
- [`VIBEVM-SPEC.md` §4.1](../VIBEVM-SPEC.md) — the four-kind package model.
- [`vibe install`](commands/install.md) — installing a feat into a project.
