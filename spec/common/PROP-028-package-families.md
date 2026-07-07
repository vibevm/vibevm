# PROP-028 — Package families: `<family>` / `-lang` / `-mcp` {#root}

**Status:** IMPLEMENTED 2026-07-07 — owner-directed (the package-family
rename). The three families in force ship under this convention: requiring one
aggregator installs its whole family at a single resolved version set, and the
served engines can never skew from the consumer's gates. Units typed at REQ
grain.
**Related:** [PROP-008 §2.2](../modules/vibe-registry/PROP-008-qualified-naming.md)
(a renamed package is a NEW identity — a family member is not an alias of the
name it replaced), [PROP-027](../modules/vibe-mcp/PROP-027-mcp-packages.md)
(the `mcp` kind and the exact-pin law the `-mcp` member obeys),
[PROP-024](PROP-024-code-bearing-packages.md) (code-bearing packages — every
`-lang` / `-mcp` member is one), [PROP-009](../modules/vibe-workspace/PROP-009-loading-model.md)
(boot loading — the aggregator carries no snippet; members' snippets reach
`INDEX.md` through the transitive BFS closure of `[requires]`).

---

## 1. Context {#context}

The AI-Native discipline reaches a consumer as several installable packages per
language: the language STACK (guide, cards, runnable toolchain), the MCP SERVER
that serves that toolchain over the wire (PROP-027), and — beneath both — the
language-neutral FLOW core the stack projects. Before this convention a single
name (`rust-ai-native`) meant the stack alone, and the server borrowed an
unrelated name (`discipline-rust`). Two costs followed: a consumer who wanted
"the Rust discipline, whole" had to know and hand-pin three loosely-related
packages, and the server's name hid which stack it served (so the exact-pin law
of PROP-027 §2.3 read as an accident rather than a family tie).

## 2. Decision {#decision}

### 2.1 Three roles, one family stem {#roles}

`req r1`

A **package family** is a set of packages sharing a `<family>` stem and
delivering one coherent capability across three roles:

- **`<family>`** — the *aggregator*. `kind = "stack"`, content-minimal: a
  `vibe.toml` and a `README.md`, and nothing else — no code, no boot snippet,
  no `specmap.toml` / `conform.toml`. Its whole job is to name the family's
  members at one resolved version set through exact `=X.Y.Z` pins in
  `[requires]`. Requiring the aggregator installs the family.
- **`<family>-lang`** — the *language stack* (`kind = "stack"`, PROP-024): the
  guide, the cards, the boot snippet, and the runnable toolchain. It requires
  the flow foundation it projects.
- **`<family>-mcp`** — the *MCP server* (`kind = "mcp"`, PROP-027): that same
  toolchain served over MCP. It exact-pins its `-lang` stack (PROP-027 §2.3)
  and version-mirrors it, so one engine set answers both the CLI floor and the
  agent's tools.

The aggregator's exact pins are deliberate, not kind-mandated: a stack may pin
its dependencies however it likes, but a family is a *tested version set*, so
the aggregator holds its members equal.

### 2.2 The version line follows the name, not the role {#versioning}

`req r1`

A family member's version line is continuous with its NAME. Per PROP-008 §2.2 a
renamed package is a new identity, so versions do not transfer across a rename:
when a name is minted it continues past the highest version that name previously
carried, and no `<name>@X.Y.Z` coordinate is ever reused for a different
artifact. In particular an aggregator name that reuses a stem the old stack
used (e.g. `rust-ai-native`, once the 0.5.0 stack, now the aggregator) begins
its aggregator line ABOVE that history (0.6.0), never at or below it. The
`-mcp` member mirrors its `-lang` member's version (PROP-027 §2.3); the
aggregator moves when any member it pins moves.

### 2.3 The families in force {#families}

`req r1`

- **`core-ai-native`** (flow) — the language-neutral discipline core. It stands
  ALONE: it is the shared foundation every language family requires, not itself
  an aggregator (there is nothing to aggregate beneath a foundation), so no
  `core-ai-native-lang` / `-mcp` exist. Each `<family>-lang` requires it.
- **`rust-ai-native`** — aggregator over `rust-ai-native-lang` (the stack) and
  `rust-ai-native-mcp` (the server).
- **`typescript-ai-native`** — aggregator over `typescript-ai-native-lang` and
  `typescript-ai-native-mcp`.

## 3. Rejected alternatives {#rejected}

- **One package with feature flags** instead of a family: a package is a whole
  project of ONE kind (PROP-024); a stack and an MCP server are different kinds
  with different delivery machinery, and the flow core is language-neutral. One
  package cannot be three kinds.
- **The aggregator ships the stack's content** (an alias that also carries
  code): then `<family>` and `<family>-lang` would duplicate content and drift.
  The aggregator is deliberately empty so there is exactly one home for each
  artifact and the family is a pure naming/pinning layer.
- **Caret pins in the aggregator**: a caret would let members skew within a
  single install, dissolving the "one engine, one truth" the `-mcp` exact pin
  exists to guarantee.

## 4. Open questions {#open}

1. A future `<family>-app` role (the anticipated `app` kind, VIBEVM-SPEC §4.1)
   would join the aggregator's `[requires]` under the same exact-pin rule.
2. Whether `vibe install <family>` should offer per-member opt-out (the mirror
   of PROP-027 §4's multi-server question); v1 is all-or-nothing per family.
