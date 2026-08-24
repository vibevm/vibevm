# AI-Native TypeScript (stack:org.vibevm.ai-native/typescript-ai-native-lang) {#root}

<status stage="doc" state="done" audience="user"/>

@fact:TYPESCRIPT-PROJECTION-SHIPS-A-RUNNABLE-TOOLCHAIN The TypeScript projection of the AI-Native Code Discipline — and the
**runnable toolchain** that enforces it (PROP-024 code-bearing packages):
installing this stack yields working checkers and procedures, not
descriptions of them. @status:impl/done

@fact:NEUTRAL-METHOD-COMES-FROM-THE-CORE-DEPENDENCY The language-neutral method (manifesto, playbooks, mechanism specs)
comes from its dependency `flow:org.vibevm.ai-native/core-ai-native`. @status:impl/done

## What ships {#what-ships}

- @fact:SHIPS-FOUR-BINARIES **Four binaries** (this package's own Cargo workspace, `crates/`,
  declared as `[[binary]]` in `vibe.toml` for PROP-025 lockfile
  dispatch): @status:impl/done
- @fact:SHIPS-TYPESCRIPT-AI-NATIVE-UMBRELLA `typescript-ai-native` — the umbrella tool: `init` (bootstrap
    policies + registries), `floor` (the portable verification floor —
    prettier → tsc → tests → eslint → conform → specmap → test-gate, one
    exit code), `conform`, `specmap`, `trace`, `test-gate`, `tripwire`,
    `health`, `fast-loop`, `codemod`. @status:impl/done
- @fact:SHIPS-TYPESCRIPT-AI-NATIVE-CONFORM `typescript-ai-native-conform` — the conformance gate alone (ENGINE-CONFORM). @status:impl/done
- @fact:SHIPS-TYPESCRIPT-AI-NATIVE-SPECMAP `typescript-ai-native-specmap` — the traceability engine alone (PROP-014). @status:impl/done
- @fact:SHIPS-TYPESCRIPT-AI-NATIVE-TCG `typescript-ai-native-tcg` — the agentic type oracle (TCG-ORACLE-v0.1 /
    TCG-PROTOCOL-v0.1): a persistent enriching `serve` relay for MCP
    hosts plus one-shot `validate` / `scope` / `complete` / `type` /
    `bench`, answered by the CONSUMER's own `typescript` install over
    in-memory overlays with the gate's own conform rules merged in.
    **Prerequisite:** node ≥ 22.6 and the project's own `typescript`
    devDependency — the same install the `tsc` floor step needs, so the
    oracle adds no new dependency. The floor stays the truth; the oracle
    exists so the floor stays green on the first try. @status:impl/done
- @fact:SHIPS-GUIDE-AND-CARDS **The TypeScript guide and cards**
  (`spec/typescript/GUIDE-AI-NATIVE-TYPESCRIPT.xml`, `spec/cards/` — the
  nine scaffolds A–I in their TypeScript shape, Band-3 ops blocks for
  weak readers). @status:impl/done
- @fact:SHIPS-TWO-AGENT-SKILLS **Two agent skills** (`vibe skill install` projects them):
  `/typescript-ai-native-terraform` (brownfield adoption per BROWNFIELD-PROTOCOL)
  and `/typescript-ai-native-sweep` (the recurring sweep per the Sweep
  Playbook). @status:impl/done
- @fact:SHIPS-TWO-NODE-SIDE-TOOLS **Two node-side tools** (`tools/`, run directly under node ≥ 22.6
  type-stripping): `ts-extract`, the Compiler-API fact extractor the
  conform frontend and the specmap scanner drive; and `ts-oracle`, the
  long-lived language-service oracle behind `typescript-ai-native-tcg`.
  Both resolve the CONSUMER project's `typescript` at runtime. @status:impl/done
- @fact:SHIPS-MECHANISM-SPECS **The TypeScript mechanism specs and tool briefs** —
  `spec/typescript/mechanisms/TCG-ORACLE-v0.1.xml` and
  `TCG-PROTOCOL-v0.1.xml`, plus `spec/typescript/tools/`. @status:impl/done
- @fact:NEUTRAL-ENGINES-RIDE-ALONG-VENDORED **The neutral engines ride along as vendored copies**
  (`crates/vendor/core-ai-native-{conform,specmap,specmark,specmark-grammar}`),
  so the slot is its own Cargo workspace and builds standalone. @status:impl/done

## Running the tools {#running-the-tools}

@fact:running-forms-lead Three supported forms, from your project root (where `vibedeps/` is): @status:impl/done

```sh
# (a) vibe-native (PROP-025) — build once in the slot, dispatch through
#     the project's lockfile:
vibe bin build            # or straight to:
vibe bin exec typescript-ai-native -- floor

# (b) install once onto PATH — then just `typescript-ai-native …`
cargo install --path vibedeps/<stack-slot>/crates/typescript-ai-native-cli

# (c) zero-install, run in place
cargo run --manifest-path vibedeps/<stack-slot>/Cargo.toml \
    -p typescript-ai-native-cli --bin typescript-ai-native -- floor
```

@fact:STACK-SLOT-IS-THE-MATERIALISED-DIRECTORY `<stack-slot>` is this package's materialised directory (e.g.
`stack-typescript-ai-native-lang/0.6.0` — check your `vibe.lock`). @status:impl/done

@fact:SLOT-BUILD-DROPS-A-TARGET-DIRECTORY Building in the
slot drops a `target/` there; add `vibedeps/**/target/` to your
`.gitignore` (build output is already excluded from the package's content
hash, PROP-024 §2.2). A repo that also carries Rust keeps
`[workspace] exclude = ["vibedeps"]`. @status:impl/done

## The lifecycle {#the-lifecycle}

```sh
vibe install                          # materialise this stack into vibedeps/
npm install -D typescript prettier eslint typescript-eslint
typescript-ai-native init             # policies + registries + external spec resolution
# … write spec units, tag exports (GUIDE §9), adopt cell by cell …
typescript-ai-native floor            # the gate panel, one exit code
/typescript-ai-native-sweep           # the recurring sweep (agent skill)
/typescript-ai-native-terraform       # brownfield adoption (agent skill)
```

@fact:wiring-and-sweep-pointers The wiring recipe — install, binaries, project toolchain, bootstrap,
the generation-time oracle — is GUIDE §15; the sweep idioms are GUIDE
§16. @status:impl/done

@fact:POLICIES-STAY-WITH-THE-CONSUMER-PROJECT The policies (`conform.toml`, `specmap.toml`) stay with YOUR project:
this package ships engines, never policy (PROP-024 §2.2). @status:impl/done

