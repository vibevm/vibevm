# PROP-024 — Code-bearing packages (a package is a project) {#root}

<status stage="spec" state="done" action="continue" comment="B0 2026-07-24: proposed 2026-06-27, owner-directed; fact grain 2026-07-24"/>

@fact:status-line **Status: IMPLEMENTED** (proposed 2026-06-27, owner-directed; verified against
the tree 2026-07-25 by the spec-actualization campaign — this very session boots on a
vendored toolchain delivered exactly this way). Makes a vibe package able to
ship runnable code, not only prompt content, so the discipline's verification
tools (the conform checker, the specmap/specmark traceability engine) can live
*inside* the discipline packages instead of being hardcoded in the vibevm
workspace. A consumer who installs `stack:org.vibevm.ai-native/rust-ai-native-lang` then has
the working checkers, not a prose description of them. @status:impl/done

@fact:related **Related:** [PROP-002 §2.1](../modules/vibe-registry/PROP-002-decentralized-registry.md#identity)
(content-hash identity — re-scoped here to the shippable tree),
[PROP-007](../modules/vibe-workspace/PROP-007-workspace.md) (workspace +
`vibedeps/`), [PROP-009 §2.1/§2.3](../modules/vibe-workspace/PROP-009-loading-model.md#two-trees)
(the verbatim materialise step + the boot path it emits),
[PROP-011 §2.6](../modules/vibe-workspace/PROP-011-incremental-install.md)
(in-workspace `file://` mutability — the dev loop that re-materialises edited
package source), [PROP-020](../modules/vibe-workspace/PROP-020-install-hooks.md)
(the `post-install` build hook), [PROP-022 §2.2](../modules/vibe-workspace/PROP-022-materialization-modes.md#snapshot)
(the snapshot copy — re-scoped here), the discipline mechanism specs
[ENGINE-CONFORM](../../vibedeps/flow-core-ai-native/0.7.0/spec/mechanisms/ENGINE-CONFORM-v0.1.md) and
[PROP-014](../../vibedeps/flow-core-ai-native/0.7.0/spec/mechanisms/PROP-014-specmap-bidirectional-traceability.md)
(the tools that relocate; the specs themselves now ship in
`flow:org.vibevm.ai-native/core-ai-native` — `spec://org.vibevm.ai-native/core-ai-native/mechanisms/…`). @status:spec/done

@fact:SANCTION-GRANTED **Owner sanction:** PROP-024 reshapes the owner-frozen `VIBEVM-SPEC.md` (§4.2
layout, §7.2 package contents, §7.3 manifest, §7.4 identity, §12 linter, §13.1
package layout). The `VIBEVM-SPEC.md` edits required explicit owner sanction; it
was **granted 2026-06-27** — the same precedent as
[PROP-009 §5 item 8](../modules/vibe-workspace/PROP-009-loading-model.md#open). @status:spec/done

---

## 1. Motivation {#motivation}

### 1.1 The problem — a package can describe a tool but not ship one {#problem}

- @fact:prompt-only-today The discipline packages (`flow:org.vibevm.ai-native/core-ai-native`,
  `stack:org.vibevm.ai-native/rust-ai-native-lang`, `…/typescript-ai-native`) carry the
  manifesto, the guides, and the nine pattern cards — **prompt content**. @status:spec/done
- @fact:machinery-hardcoded But the machinery that makes the discipline *real* — the conform checker
  (Class-F/G rules, the file-length budget, the unwrap ban) and the
  specmap/specmark traceability engine — is **hardcoded as crates inside the
  vibevm workspace** (`crates/conform-core`, `crates/conform-frontend-rust`,
  `crates/specmark`, `crates/specmap-core`, the `cargo xtask conform`/`specmap`
  drivers). @status:spec/done

- @fact:install-gets-description Install `stack:org.vibevm.ai-native/rust-ai-native-lang` today and you get a *description* of
  checkers you do not have. To actually run the discipline you would have to
  re-implement the very tools vibevm already wrote. @status:spec/done
- @fact:not-distributable The discipline is therefore **not distributable**: its strong-author artifacts
  (guide, cards) ship, but its runtime (the checkers) does not. @status:spec/done
- @fact:gap-consequence This is the gap that, left open, makes spec-conformance "fall apart" for any
  consumer that is not vibevm itself. @status:spec/done

- @fact:format-cause The package format is the cause: a package is defined as a bundle of prompt
  files (`VIBEVM-SPEC.md` §7.2 — "vibe.toml, README.md, other content files
  referenced by the manifest"), materialised verbatim. @status:spec/done
- @fact:nowhere-for-code There is nowhere to put code. @status:spec/done
- @fact:identity-fights-code The identity/materialisation machinery (`content_hash` over every file,
  full-tree copy) actively fights it — a Rust crate's `target/` would make
  identity non-deterministic and the copy ruinous. @status:spec/done

### 1.2 The shape — a package is a project {#shape}

@fact:project-shape A vibe *project* already has the right shape: an authored `spec/` corpus
(`VIBEVM-SPEC.md` §4.2 — "`spec/` is *the* spec directory") plus arbitrary code
at the root (`Cargo.toml`, `crates/`, `src/`) plus one `vibe.toml`. A *package*
should be the same object, made installable: @status:spec/done

@fact:PACKAGE-SHAPE **prompt/spec content under `spec/`, arbitrary code at the root, one `vibe.toml`.** @status:spec/done

- @fact:shape-consequences Then a package can ship its tools; an installed package is immediately usable;
  and authoring a package *is* authoring a project — the same layout, the same
  `vibe check`, the same boot computation. @status:spec/done
- @fact:four-layer-fit The discipline's own four-layer model lands cleanly: L1/L2/L3 (manifesto,
  guide, cards — prompts) live under `spec/`, and L4 (the implemented checkers —
  code) lives at the root. @status:spec/done

---

## 2. Decisions {#decisions}

### 2.1 A package is a project — `spec/` for prompts, the root for code {#package-is-project}

@fact:req-package-is-project `req r1` @status:spec/done

@fact:PKG-PROJECT-LAW **Decision.** A package has the identical on-disk shape as a consumer project: @status:spec/done

- @fact:SPEC-SUBTREE **Prompt/spec content lives under the package's `spec/` subtree** — boot
  snippets (`spec/boot/`), cards, guides, manifesto, appendix — laid out exactly
  as an ordinary project's `spec/` (`VIBEVM-SPEC.md` §4.2). `[boot_snippet].source`
  is a `spec/`-relative path (e.g. `spec/boot/20-stack-rust-ai-native.md`). @status:spec/done
- @fact:ROOT-CODE **The package root holds arbitrary code** (e.g. `Cargo.toml` + `crates/`) and
  `vibe.toml`, exactly as a project root does. Code is optional — a prompt-only
  package (e.g. `discipline-core`) simply has no code at its root. @status:spec/done

- @fact:CONS-DEV-IS-DEV **Consequence.** Developing a package is developing a project. @status:spec/done
- @fact:CONS-CHECK-UNCHANGED `vibe check` applies unchanged (its §12 Check 7 — `spec/boot/` exists and holds
  only markdown — is satisfied by the package's own `spec/boot/`). @status:spec/done
- @fact:CONS-BOOT-SAME The package's own boot sequence is computed the same way as any project's. @status:spec/done
- @fact:CONS-NO-CONVENTION There is no package-only directory convention to learn. @status:spec/done

@fact:RETIRES-FLAT-LAYOUT This **retires the flat package layout** (boot snippets and content at the
package root) that the real packages drifted into; it aligns them with — and
extends — `VIBEVM-SPEC.md` §13.1's own canonical example, which already places a
package's content under `spec/`. @status:spec/done

### 2.2 The shippable tree excludes build output {#shippable-tree}

@fact:req-shippable-tree `req r1` @status:spec/done

@fact:SHIPPABLE-TREE-DEF **Decision.** A package's **shippable tree** is its directory minus a
build-output denylist: @status:spec/done

```
.git/        .vibe/        target/        node_modules/
```

- @fact:VIBEIGNORE-EXTENDS plus any glob listed in an optional `.vibeignore` at the package root. @status:spec/done
- @fact:SHIPPABLE-CONSUMERS The `content_hash` (PROP-002 §2.1), the snapshot copy (PROP-022 §2.2), and the
  verbatim materialised slot (PROP-009 §2.1) all operate over the **shippable
  tree**, never the raw directory. @status:spec/done

- @fact:WHY-SOURCE-IDENTITY **Why.** Identity is the *source*, never build artifacts: build output is
  non-deterministic (timestamps, host paths, incremental state) and may be
  gigabytes — hashing or copying it would make identity unstable and
  materialisation ruinous, the exact failure PROP-022 §1.1 names for "big in file
  count". @status:spec/done
- @fact:SOURCE-IS-SHIPPED A package's source — what its author commits — is precisely what is
  hashed, copied, and vendored. @status:spec/done

- @fact:VERBATIM-PRESERVED **"Verbatim" is preserved for the source.** PROP-009 §2.1 / `VIBEVM-SPEC.md`
  §13.1 guarantee no path rewriting and no per-file write list — a human reading
  the package directory sees exactly what materialises. @status:spec/done
- @fact:VERBATIM-HOLDS-SHIPPABLE That guarantee holds unchanged for the shippable tree: build output was never
  part of the authored tree (it is gitignored in the package's own repository
  too). @status:spec/done
- @fact:DENYLIST-NOT-SELECTION The denylist formalises "what was never source", it does not introduce
  selection. @status:spec/done
- @fact:shippable-rejected **Considered and rejected:** **hashing and copying build output too** — rejected: non-deterministic and potentially gigabytes, the file-count/byte-count failure PROP-022 §1.1 exists to avoid (`##REJ-HASH-BUILD-OUTPUT`); **a per-file `[ship]` / `[files]` allow-list in the manifest** — rejected: it resurrects the per-file write list PROP-009 §2.6 retired, and a denylist keeps *"what ships" == "the source"*, preserving the verbatim guarantee (`##REJ-ALLOW-LIST`, `##VERBATIM-PRESERVED`). @status:spec/done
- @fact:shippable-revisit **Revisit when:** a package ships in a language whose build output the four-name denylist does not cover and `.vibeignore` alone cannot carry — Python (`__pycache__/`, `.venv/`), JVM (`build/`), Go (`vendor/`) are the near candidates. **The fired state is mechanically observable and already asserted:** a package whose `content_hash` differs between a clean and a built checkout, which is exactly what `##ACC-HASH-EXCLUDES` tests. Observation point: that acceptance check, run over the published package set. @status:spec/done

### 2.3 Code materialises, then builds consumer-side into a gitignored target {#build}

@fact:req-build `req r1` @status:spec/done

- @fact:MATERIALISE-AS-TODAY **Decision.** `vibe install` materialises the shippable tree — including code —
  into the `vibedeps/` slot, as today. @status:spec/done
- @fact:BUILD-CONSUMER-SIDE Turning that source into a runnable tool is consumer-side and **must never
  write into the committed slot or the hash**: @status:spec/done

- @fact:HOOK-BUILD A code-bearing tool package builds via a **`post-install` hook**
  ([PROP-020](../modules/vibe-workspace/PROP-020-install-hooks.md)) whose build
  output is directed to a **gitignored** location (e.g.
  `<project-root>/.vibe/<pkg>-target/`), never the slot. To let a hook address
  that location, the hook runner gains a `VIBE_PROJECT_ROOT` environment
  variable (the workspace absolute root) alongside the existing
  `VIBE_PACKAGE_DIR` (the slot) — a small [PROP-020 §2.2](../modules/vibe-workspace/PROP-020-install-hooks.md#script-selection)
  addition. @status:spec/done
- @fact:NATIVE-CONSUMER-SKIP A **language-native consumer** (vibevm itself is a Rust consumer) MAY instead
  reference the shipped crates directly through its own toolchain (§2.4) and
  skip the build hook entirely — the hook is the path for a consumer that wants
  a ready binary without driving the language's build system itself. @status:spec/done

@fact:HOOK-RESET-UNAFFECTED The hook's own reset semantics (PROP-020 §2.1 — slot re-materialised on update,
edits never compound) are unaffected: the build output lives outside the slot,
so there is nothing in the slot to reset. @status:spec/done

### 2.4 Consuming shipped code — external-path-dep, no nested workspace {#consume}

@fact:req-consume `req r1` @status:spec/done

- @fact:OWN-WORKSPACE **Decision.** A code-bearing package carries its **own** workspace manifest
  (for Rust, a root `Cargo.toml` with `[workspace]`) — it is a standalone,
  independently-buildable project. @status:spec/done
- @fact:PATH-DEP-LAW A language-native consumer that needs a shipped
  crate — a proc-macro that compiles *into* the consumer's own code (the
  `specmark` case), or a binary it invokes (the `conform`/`specmap` case) —
  references it **by path into the materialised slot**: @status:spec/done

```toml
# consumer's root Cargo.toml — one pinned alias, updated once per package bump
[workspace.dependencies]
specmark = { path = "vibedeps/stack-rust-ai-native-lang/0.2.0/crates/specmark" }

[workspace]
exclude = ["vibedeps", "packages"]   # disclaim the package's own workspaces
```

- @fact:WORKSPACE-EXCLUDE The consumer **excludes** `vibedeps/` (and, for a self-hosting repo, the
  in-repo `packages/` source) from its `[workspace]`, so the slot's crates
  belong to the *package's* workspace, not the consumer's — Cargo forbids a
  directory living in two workspaces, and this is the standard resolution for a
  repo that contains a sub-project with its own workspace. @status:spec/done
- @fact:PIN-ONCE The slot path is version-qualified; pinning it once in
  `[workspace.dependencies]` means a package version bump touches a single line. @status:spec/done

@fact:BINARY-RUN-FORM A binary tool (`conform`, `specmap`) is run from the package's workspace —
`cargo run --manifest-path vibedeps/<slot>/Cargo.toml --bin conform -- …` with
`CARGO_TARGET_DIR` pointed at a gitignored dir (§2.3) — so building it pollutes
neither the slot nor the consumer's own `target/`. @status:spec/done

- @fact:SPIKE-FIRST **Spike before the irreversible move.** Cross-workspace path-deps and the
  `exclude` topology are validated empirically on the target host (Windows, where
  `canonicalize()` adds a `\\?\` prefix and Cargo path handling has sharp edges)
  *before* any crate is physically relocated. @status:spec/done
- @fact:SPIKE-FALLBACK The fallback, if cross-workspace path-deps prove unworkable on a host, is §4's
  rejected-but-retained alternative (the consumer adds the slot crates as its
  own workspace members) — chosen only on evidence. @status:spec/done
- @fact:own-workspace-why **Why:** an external constraint, not a preference — *"Cargo forbids a directory living in two workspaces, and this is the standard resolution for a repo that contains a sub-project with its own workspace"* (`##WORKSPACE-EXCLUDE`). Giving the package its own workspace is what makes it *"a standalone, independently-buildable project"* (`##OWN-WORKSPACE`) and keeps a version bump to one pinned line (`##PIN-ONCE`). @status:spec/done
- @fact:own-workspace-rejected **Considered and rejected:** **the consumer adding the slot crates as its own workspace members** (no package workspace, no cross-workspace path-dep) — *considered*, and rejected as the **primary** model because it denies the package standalone-buildability and couples the consumer's workspace membership to generated `vibedeps/` state (`##REJ-CONSUMER-MEMBERS`). **Retained, not discarded**: it is the §2.4 fallback, *"chosen only on evidence"* (`##SPIKE-FALLBACK`). @status:spec/done
- @fact:own-workspace-revisit **Revisit when:** cross-workspace path-deps prove unworkable on a supported host — the condition `##SPIKE-FALLBACK` already names, here given its observation point: a clean-checkout `cargo build` failing to resolve `vibedeps/<slot>/crates/<crate>` on any of the three platforms of PROP-000 §11 `##PLATFORMS-TRIO`, Windows first (`##SPIKE-FIRST`). The fired state has a landing place already specified, so reopening is a switch, not a redesign. @status:spec/done

### 2.5 Self-hosting bootstrap — the toolchain is vendored {#bootstrap}

@fact:req-bootstrap `req r1` @status:spec/done

- @fact:SELF-HOST-VENDORED **Decision.** vibevm consumes its own discipline toolchain from the **committed**
  `vibedeps/` slot. @status:spec/done
- @fact:CLEAN-CLONE-BUILDS Because `vibedeps/` is committed (PROP-009 §2.1), a fresh clone
  builds from a clean checkout **with no prior `vibe install`** — the path-dep
  target (`vibedeps/stack-rust-ai-native-lang/0.2.0/crates/specmark`, …) already exists
  in the tree. @status:spec/done
- @fact:NO-CHICKEN-EGG There is no chicken-and-egg: the toolchain a build needs is vendored
  beside the code that needs it. @status:spec/done
- @fact:bootstrap-why **Why:** committing the slot is what makes a fresh clone build *"from a clean checkout with no prior `vibe install`"* — the path-dep target already exists in the tree (`##CLEAN-CLONE-BUILDS`), so there is no chicken-and-egg between the toolchain a build needs and the build that would fetch it (`##NO-CHICKEN-EGG`). Acceptance already asserts it: `##ACC-CLEAN-CLONE`. @status:spec/done
- @fact:bootstrap-rejected **Considered and rejected:** **`materialization = "in-place"` for the tool packages** — rejected: `in-place` slots are `.gitignore`d and unversioned (PROP-022 §2.4/§2.7), and the toolchain must be vendored and versioned so a clone is buildable offline (`##REJ-IN-PLACE`). **Publishing the tool crates to crates.io and depending on the published versions** — **deferred, not rejected**: *"the installed package is the distribution"*, and crates.io publication is *"an optional later convenience for non-vibe Rust consumers, not a requirement of this model"* (`##REJ-CRATES-IO`). @status:spec/done
- @fact:bootstrap-revisit **Revisit when:** the committed slot's cost outgrows its guarantee — `git count-objects -vH` and the slot's on-disk size showing a clean clone expensive enough to outweigh the offline-buildability it buys (numeric threshold unset — owner 2026-08-01; event-shaped until set) — **or** the deferred demand arrives: a Rust consumer outside the vibe ecosystem needs `conform` / `specmap` without installing a vibe package, recorded in the `F-NNN` findings ledger so the signal has a place to be observed (`##REJ-CRATES-IO`). @status:spec/done

@fact:DEV-LOOP-MUTABLE The development loop stays ergonomic: editing the in-repo package source under
`packages/org.vibevm.ai-native/rust-ai-native/…` re-materialises the slot automatically on
the next `vibe install` (PROP-011 §2.6 — in-workspace `file://` sources are
mutable), so the consumed `vibedeps/` copy tracks the edited source without a
manual `rm -rf`. @status:spec/done

### 2.6 Placement follows the layer model; the engine split is a follow-up {#placement}

@fact:req-placement `req r1` @status:spec/done

@fact:PLACEMENT-LAW **Decision.** The discipline's tools are code and obey the four-layer model:
L4 (implemented checkers) ships in the package whose language they check. @status:spec/done

- @fact:THIS-PASS-WHOLE-TOOLCHAIN For **this pass**, the **entire Rust discipline toolchain** — the conform
  engine (`conform-core`), its Rust frontend (`conform-frontend-rust`), the
  Rust traceability macros (`specmark`, `specmark-grammar`), the traceability
  engine (`specmap-core`), and the designated audit crate (`env-audit`) — ships
  in `stack:org.vibevm.ai-native/rust-ai-native-lang`. Its centre of gravity is Rust, and
  shipping the toolchain whole avoids carving language-neutral cores out under
  time pressure. @status:spec/done
- @fact:CORE-STAYS-PROMPT-ONLY **The condition fired.** `flow:org.vibevm.ai-native/core-ai-native` was to stay
  **prompt-only** (manifesto, card format, scaffold catalog, RAID, appendix) until a
  second language actually needed the shared engine. The TypeScript pilot shipped, so
  core-ai-native now **authors the neutral engines** (conform / specmap / specmark / mcp
  cores), which each `-lang` and `-mcp` package vendors byte-identically. @status:impl/done
- @fact:DEFERRED-ENGINE-SPLIT **Deferred (documented):** the language-neutral conform engine (`conform-core`)
  is a genuine L1 artifact — a future `conform-frontend-typescript` would reuse
  it unchanged. Extracting `conform-core` up into `discipline-core` is a clean
  follow-up, taken when the first non-Rust pilot needs it (YAGNI until then).
  Likewise the neutral half of `specmap-core` (markdown parse, index, ledger,
  test-gate) versus its Rust `rscan` frontend. The end state is symmetric; the
  ordering is driven by real second-language demand, not built speculatively.
  **Executed** — the TypeScript pilot was that demand, and the neutral halves now live
  in core-ai-native, vendored into each family by `cargo xtask sync-engines`. @status:impl/done
- @fact:placement-why **Why:** the toolchain's *"centre of gravity is Rust, and shipping the toolchain whole avoids carving language-neutral cores out under time pressure"* (`##THIS-PASS-WHOLE-TOOLCHAIN`); the layer model then decides placement rather than convenience — L4 ships with the language it checks. Recorded the same session the decision was taken (§7 `##HIST-DRAFT-1`, 2026-06-27). @status:spec/done
- @fact:placement-rejected **Considered and rejected:** **extracting the language-neutral cores (`conform-core`, the neutral half of `specmap-core`) up into core-ai-native in the same pass** — **deferred, not rejected**, with its condition stated: *"taken when the first non-Rust pilot needs it (YAGNI until then)"* (`##DEFERRED-ENGINE-SPLIT`). **The deferral has since been honoured**: the TypeScript pilot was that demand, the condition fired, and the neutral engines now live in core-ai-native, vendored by `cargo xtask sync-engines` (`##CORE-STAYS-PROMPT-ONLY`). @status:spec/done
- @fact:placement-revisit **Revisit when:** *(a successor trigger — the previous one fired with the TypeScript pilot and is spent)* a **third** language family arrives and the neutral engines do not cover it — observed as `cargo xtask sync-engines` being unable to vendor a core byte-identically into the new `-lang` package, or a third family needing a core the two existing ones do not share. Observation point: the `sync-engines` task and the set of `*-ai-native-lang` / `*-ai-native-mcp` packages. @status:spec/done

---

## 3. Manifest / schema surface {#surface}

- @fact:SURF-NO-NEW-FIELD **No new required manifest field.** `[boot_snippet].source` becomes
  `spec/`-relative (a value change, not a schema change). `[package].materialization`
  stays `snapshot` for a vendored code-bearing package. @status:spec/done
- @fact:SURF-VIBEIGNORE **Optional `.vibeignore`** at the package root — newline-delimited globs added
  to the §2.2 build-output denylist. @status:spec/done
- @fact:SURF-HOOKS **`[hooks].post-install`** (PROP-020) is the build lever; **`VIBE_PROJECT_ROOT`**
  is added to the hook environment (§2.3). @status:spec/done
- @fact:SURF-LOCK-UNAFFECTED **`vibe.lock`** is unaffected — identity is still `content_hash`, now over the
  shippable tree (§2.2); no schema bump. @status:spec/done

---

## 4. Rejected / deferred alternatives {#rejected}

- @fact:REJ-HASH-BUILD-OUTPUT **Hash and copy build output too** — rejected: non-deterministic and
  potentially gigabytes; it is the file-count/byte-count failure PROP-022 §1.1
  exists to avoid. Identity is source (§2.2). @status:spec/done
- @fact:REJ-CONSUMER-MEMBERS **Consumer adds the slot crates as its own workspace members** (no package
  workspace, no cross-workspace path-dep) — *considered*; rejected as the
  primary model because it denies the package standalone-buildability (a package
  would not be a project) and couples the consumer's workspace membership to
  generated `vibedeps/` state. **Retained as the §2.4 fallback** if
  cross-workspace path-deps prove unworkable on a host — a decision made on spike
  evidence, not by default. @status:spec/done
- @fact:REJ-IN-PLACE **`materialization = "in-place"` for tool packages** — rejected: `in-place`
  slots are `.gitignore`d and unversioned (PROP-022 §2.4/§2.7); the discipline
  toolchain must be **vendored and versioned** so a clone is buildable offline
  (§2.5). Snapshot-minus-build-output is the right mode. @status:spec/done
- @fact:REJ-CRATES-IO **Publish the tool crates to crates.io and depend on the published versions** —
  deferred: the installed package *is* the distribution, so the consumer depends
  on the slot, not a registry crate. crates.io publication is an optional later
  convenience for non-vibe Rust consumers, not a requirement of this model. @status:spec/done
- @fact:REJ-ALLOW-LIST **A per-file `[ship]`/`[files]` allow-list in the manifest** — rejected: it
  resurrects the per-file write list PROP-009 §2.6 retired. A denylist of build
  output (§2.2) keeps "what ships" == "the source", preserving the verbatim
  guarantee. @status:spec/done

---

## 5. Out of scope {#out-of-scope}

- @fact:OOS-AUTODETECT **Auto-detecting the build system / language** of a code-bearing package —
  the package declares its build via a `post-install` hook (PROP-020); vibevm
  does not infer `cargo` vs `npm`. @status:spec/done
- @fact:OOS-SANDBOX **Sandboxing the build hook** — inherits PROP-020 §4's posture (hooks run with
  the user's privileges; trust is the allow-list + consent). @status:spec/done
- @fact:OOS-TS-CHECKER **Out of scope when written; since delivered.** TypeScript shipped no
  implemented tool to relocate, so its cards kept `specified` checker statuses until a
  TS pilot existed (§2.6 deferral). The `typescript-ai-native` family shipped with its
  floor, conform and specmap engines — the deferral is closed. @status:impl/done

---

## 6. Acceptance {#acceptance}

- @fact:ACC-SHAPE A package may carry code at its root and prompt content under `spec/`;
  `vibe check` passes on it as a project (`spec/boot/` markdown-only). @status:spec/done
- @fact:ACC-HASH-EXCLUDES `content_hash`, the snapshot copy, and the materialised slot exclude `.git/`,
  `.vibe/`, `target/`, `node_modules/`, and `.vibeignore` globs; identical
  source produces an identical hash regardless of build state. @status:spec/done
- @fact:ACC-BOOT-RESOLVES `[boot_snippet].source` resolves `spec/`-relative; the generated `INDEX.md`
  names `vibedeps/<slot>/spec/boot/<file>`. @status:spec/done
- @fact:ACC-CLEAN-CLONE vibevm builds from a clean checkout, consuming its discipline toolchain (incl.
  the `specmark` proc-macro the product crates compile against) from the
  committed `vibedeps/` slot, with no prior `vibe install`. @status:spec/done
- @fact:ACC-EXTERNAL-CONSUMER An external Rust project can install `stack:org.vibevm.ai-native/rust-ai-native-lang` and run
  `conform` / `specmap` against its own code. @status:spec/done
- @fact:ACC-FLOOR-GREEN Full `self-check.sh` green; conform 0/0/0; specmap clean. @status:spec/done

---

## 7. Version history {#history}

- @fact:HIST-DRAFT-1 **2026-06-27 — draft 1.** Owner-directed: make the discipline self-sufficient
  by letting packages ship runnable code (`spec/` for prompts, the root for
  code), then relocate the Rust toolchain (conform + specmap/specmark) out of the
  vibevm workspace and into `stack:org.vibevm.ai-native/rust-ai-native-lang`. The frozen
  `VIBEVM-SPEC.md` sanction was granted the same session (§0). Decisions taken in
  the owner session: the prompt directory is `spec/` (singular, project-identical,
  not `specs/`); the full traceability stack moves alongside conform; conform is
  productised to run on an arbitrary external project (config-driven, not
  vibevm-hardcoded); `conform-core` ships in the Rust stack now with the L1
  engine-extraction deferred (§2.6). @status:spec/done
