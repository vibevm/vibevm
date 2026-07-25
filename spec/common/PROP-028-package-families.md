# PROP-028 — Package families: `<family>` / `-lang` / `-mcp` {#root}

<status stage="impl" state="done" comment="B0 2026-07-24: IMPLEMENTED 2026-07-07; the three families in force; fact grain 2026-07-24"/>

##status-line **Status:** IMPLEMENTED 2026-07-07 — owner-directed (the package-family
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
`INDEX.md` through the transitive BFS closure of `[requires]`). @impl/done

---

## 1. Context {#context}

- ##context-delivery-shape The AI-Native discipline reaches a consumer as several installable packages per
  language: the language STACK (guide, cards, runnable toolchain), the MCP SERVER
  that serves that toolchain over the wire (PROP-027), and — beneath both — the
  language-neutral FLOW core the stack projects. @impl/done
- ##context-before Before this convention a single name (`rust-ai-native`) meant the stack alone,
  and the server borrowed an unrelated name (`discipline-rust`). @impl/done
- ##context-costs Two costs followed: @impl/done
  - ##context-cost-pinning a consumer who wanted "the Rust discipline, whole" had to know and hand-pin
    three loosely-related packages; @impl/done
  - ##context-cost-name the server's name hid which stack it served (so the exact-pin law
    of PROP-027 §2.3 read as an accident rather than a family tie). @impl/done

## 2. Decision {#decision}

### 2.1 Three roles, one family stem {#roles}

##req-roles `req r1` @impl/done

##FAMILY-DEF A **package family** is a set of packages sharing a `<family>` stem and
delivering one coherent capability across three roles: @impl/done

- ##ROLE-AGGREGATOR **`<family>`** — the *aggregator*. `kind = "stack"`, content-minimal: a
  `vibe.toml` and a `README.md`, and nothing else — no code, no boot snippet,
  no `specmap.toml` / `conform.toml`. Its whole job is to name the family's
  members at one resolved version set through exact `=X.Y.Z` pins in
  `[requires]`. Requiring the aggregator installs the family. @impl/done
- ##ROLE-LANG **`<family>-lang`** — the *language stack* (`kind = "stack"`, PROP-024): the
  guide, the cards, the boot snippet, and the runnable toolchain. It requires
  the flow foundation it projects. @impl/done
- ##ROLE-MCP **`<family>-mcp`** — the *MCP server* (`kind = "mcp"`, PROP-027): that same
  toolchain served over MCP. It exact-pins its `-lang` stack (PROP-027 §2.3)
  and version-mirrors it, so one engine set answers both the CLI floor and the
  agent's tools. @impl/done

##AGGREGATOR-PINS-DELIBERATE The aggregator's exact pins are deliberate, not kind-mandated: a stack may pin
its dependencies however it likes, but a family is a *tested version set*, so
the aggregator holds its members equal. @impl/done

### 2.2 The version line follows the name, and the family moves in unison {#versioning}

##req-versioning `req r1` @impl/done

- ##VERSION-FOLLOWS-NAME A family member's version line is continuous with its NAME. @impl/done
- ##RENAME-NEW-IDENTITY Per PROP-008 §2.2 a
  renamed package is a new identity, so versions do not transfer across a rename:
  when a name is minted it continues past the highest version that name previously
  carried, and no `<name>@X.Y.Z` coordinate is ever reused for a different
  artifact. @impl/done
- ##AGGREGATOR-ABOVE-HISTORY In particular an aggregator name that reuses a stem the old stack
  used (e.g. `rust-ai-native`, once the 0.5.0 stack, now the aggregator) begins
  its aggregator line ABOVE that history, never at or below it. @impl/done

- ##UNISON-LAW Within a family the members move in **unison**: a content change to any member
  bumps EVERY member of that family to one shared version, and the aggregator's
  version IS that family version. @impl/done
- ##NO-MIXED-NUMBERS A family is a *tested set*, so its members never
  carry mixed numbers — reading `rust-ai-native 0.7.0` tells you
  `rust-ai-native-lang`, `rust-ai-native-mcp`, and the aggregator are all 0.7.0. @impl/done
- ##MIRROR-PAIRWISE The `-mcp` member's version-mirroring of `-lang` (PROP-027 §2.3) is the pairwise
  case of this whole-family law. @impl/done
- ##family-standings The families currently stand at: rust **0.7.0**, typescript
  **0.6.0**, go **0.1.0** (the aggregator plus its `-lang` and `-mcp` members), and
  the shared foundation core-ai-native **0.8.0** — a foundation is not in any one
  family's unison, it bumps on its own content and every family widens its `^`
  floor to meet it. @impl/done

### 2.3 The families in force {#families}

##req-families `req r1` @impl/done

- ##FAM-CORE **`core-ai-native`** (flow) — the language-neutral discipline core. It stands
  ALONE: it is the shared foundation every language family requires, not itself
  an aggregator (there is nothing to aggregate beneath a foundation), so no
  `core-ai-native-lang` / `-mcp` exist. Each `<family>-lang` requires it. @impl/done
- ##FAM-RUST **`rust-ai-native`** — aggregator over `rust-ai-native-lang` (the stack) and
  `rust-ai-native-mcp` (the server). @impl/done
- ##FAM-TYPESCRIPT **`typescript-ai-native`** — aggregator over `typescript-ai-native-lang` and
  `typescript-ai-native-mcp`. @impl/done

### 2.4 Naming below the package: crates, binaries, skills, servers {#surface-naming}

##req-surface-naming `req r1` @impl/done

##SURFACE-NAMING-LAW The family stem is language-FIRST and reaches every named surface a family
ships, not only the package identities. Each crate, binary, agent skill, and
MCP server carries the `<family>` prefix: @impl/done

- ##NAME-UMBRELLA-BINARY **The umbrella binary is the family name.** A `-lang` stack's driver binary —
  the tool a consumer puts on PATH — is named `<family>` itself
  (`rust-ai-native`, `typescript-ai-native`): `init` / `floor` / `conform` /
  `specmap` / `trace` / … all hang off it. Its crate is `<family>-cli`. @impl/done
- ##NAME-ROLE-BINARIES **Every other binary and its crate share the `<family>-<role>` form.** The
  standalone gates and the oracle: `<family>-conform`, `<family>-specmap`,
  `<family>-tcg`; their crates match name-for-name, so
  `cargo … -p <family>-conform --bin <family>-conform` and
  `vibe bin exec <family>-conform` read the same token. Library-only crates
  take the same law (`<family>-conform-frontend`, `<family>-tcg-bridge`,
  `<family>-env-audit`). @impl/done
- ##NAME-MCP-SURFACE **`<family>-mcp` is the family's MCP surface.** For a language family it is
  the server package, its single authored crate, AND its binary — all three the
  one name (maximal coherence: the package a consumer pins, the crate that
  builds, and the artifact that serves are indistinguishable). Its
  `[[mcp_server]].name` — the agent-visible key written into `.mcp.json` — is
  the FAMILY name (`rust-ai-native`), so the tool namespace an agent sees is the
  family, not an internal binary. For the flow foundation, `core-ai-native-mcp`
  is the neutral MCP transport crate the servers vendor. @impl/done
- ##NAME-SKILLS **Skills carry the stem too**: `<family>-sweep`, `<family>-terraform`. @impl/done
- ##NAME-NEUTRAL-ENGINES **The neutral engine crates the core authors** take the CORE stem —
  `core-ai-native-conform`, `core-ai-native-specmap`, `core-ai-native-specmark`,
  `core-ai-native-specmark-grammar`, `core-ai-native-mcp` — because they belong
  to no single language; each `-lang` / `-mcp` package vendors them
  byte-identically (PROP-024; `cargo xtask sync-engines`). @impl/done

- ##D13-SUPERSEDED **Supersession of the `-rust` suffix policy (D13).** The earlier owner policy —
  «every artifact with a cross-language analog ends in `-rust` / `-typescript`»,
  recorded as the standing rule in GUIDE-AI-NATIVE-RUST §2 and referenced in
  PROP-026 and the WAL history — is SUPERSEDED by this language-FIRST family
  prefix. @impl/done
- ##D13-LANGUAGE-LEADS `conform-rust` becomes `rust-ai-native-conform`, not a suffixed
  `conform-rust`; the language LEADS the name so every artifact of one family
  sorts and reads together, and the aggregator name is the common prefix of its
  whole surface. @impl/done
- ##D13-GOAL-PRESERVED The suffix scheme's goal (a cross-language pair differs
  consistently, never sometimes) is preserved and strengthened — the whole name,
  not just its tail, now carries the family. @impl/done
- ##D13-NEUTRAL-OUTSIDE Language-NEUTRAL artifacts stay outside any family stem: vibevm's own generic
  `vibe-*` crates, the `vibe-tcg` product cell. @impl/done

## 3. Rejected alternatives {#rejected}

- ##REJ-FEATURE-FLAGS **One package with feature flags** instead of a family: a package is a whole
  project of ONE kind (PROP-024); a stack and an MCP server are different kinds
  with different delivery machinery, and the flow core is language-neutral. One
  package cannot be three kinds. @impl/done
- ##REJ-ALIAS-WITH-CONTENT **The aggregator ships the stack's content** (an alias that also carries
  code): then `<family>` and `<family>-lang` would duplicate content and drift.
  The aggregator is deliberately empty so there is exactly one home for each
  artifact and the family is a pure naming/pinning layer. @impl/done
- ##REJ-CARET-PINS **Caret pins in the aggregator**: a caret would let members skew within a
  single install, dissolving the "one engine, one truth" the `-mcp` exact pin
  exists to guarantee. @impl/done

## 4. Open questions {#open}

<status stage="spec" state="work" comment="B1 2026-07-24: two questions still open, no owner ruling yet"/>

1. ##open-family-app A future `<family>-app` role (the anticipated `app` kind, VIBEVM-SPEC §4.1)
   would join the aggregator's `[requires]` under the same exact-pin rule. @spec/work
2. ##open-member-opt-out Whether `vibe install <family>` should offer per-member opt-out (the mirror
   of PROP-027 §4's multi-server question); v1 is all-or-nothing per family. @spec/work
