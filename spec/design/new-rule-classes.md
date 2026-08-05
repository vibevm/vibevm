# The three new rule classes — comment position, custom lints, pending cards {#root}

<status stage="spec" state="work" comment="boss design for волна Б батч 3 (B-036 + B-037 + B-038), captured 2026-08-04 on the E13 census trio; carries the map's fork №1 (computed cell names), taken by the owner the same day"/>

##authority-line **Non-normative** (`spec/design/` genre). The contract stays in
the PROPs and in the discipline packages' own specs; this document records why
batch 3 is shaped the way it is, and what the workers were told to elaborate.
The owner's rulings behind it were filed as backlog entries
[`{#b-036}`](../../BACKLOG.md#b-036), `B-037` and
[`{#b-038}`](../../BACKLOG.md#b-038), and they win on divergence — B-037's row
has since been drained (its TypeScript half built, its Rust half carried by
[`{#b-050}`](../../BACKLOG.md#b-050)), so its ruling lives in the commit that
closed it rather than at an address. @doc/done

##standing-on The measurement this design stands on — three read-only censuses
taken in parallel before a line was written:
[`harvest/e13-r1-comment-position-census.md`](../../campaigns/packages-2026-09/harvest/e13-r1-comment-position-census.md),
[`e13-r2-custom-lints-census.md`](../../campaigns/packages-2026-09/harvest/e13-r2-custom-lints-census.md),
[`e13-r3-pending-cards-census.md`](../../campaigns/packages-2026-09/harvest/e13-r3-pending-cards-census.md).
Every number below is theirs unless it says «boss-measured». @doc/done

##batch-shape **The batch's shape in one paragraph.** Three builds that look
unrelated turn out to share one spine: each is a *card the Discipline already
names but never built a checker for*. B-036 is `rule-position-is-a-resource`,
B-038 is `rule-closed-vocabulary-naming` plus a new card for R-060 — three of
the seven cards every stack index lists under «Pending cards (named, not yet
authored)» — and B-037 is the third diagnostics channel Scaffold F promises.
So batch 3 is one conveyor: **card + checker + exhibit, three times.** @doc/done

## 1. What the censuses changed about the plan {#census-changed}

##finding-no-severity **There is no severity class in the engine, and B-036
does not need one.** `Finding` carries no severity, `Rule` has no severity
method, and all three drivers fail iff `!new.is_empty()` against the ratchet
baseline. The backlog's «предупреждение — не блокирующий гейт на старте (урок
B-021)» therefore needs no new subsystem: **the baseline freeze IS that
semantic** — a new rule lands, its pre-existing findings freeze once, and only
a NEW violation reds the gate. Building a severity model here would be a
cross-cutting engine change bought for a requirement the ratchet already
satisfies. @spec/done

##finding-denominator-exists **The position denominator already ships.**
`Fact::FileMetrics { lines }` is emitted once per file by every frontend, so
«which third of the file is this line in» needs no new file-level input — only
the comment's line. @spec/done

##finding-comment-walks-exist **Two of three extractors already walk every
comment.** `go-extract`'s suppression pass and `ts-extract`'s scanner both read
each comment's text AND line today; only the Rust frontend loses plain `//`
comments, because `syn::parse_file` drops them — and it holds the full file
text, so a raw scan is a local addition, not an architecture change. @spec/done

##finding-vocabulary-is-absent **The marker vocabulary the guides evoke is
near-absent from our own tree** — `SAFETY:` 6, `INVARIANT:` 0, `PANICS` 0,
rustdoc `# Safety` 0, TS `@invariant` 0. This is a fact about our tidiness (600-line
files, invariants in module docs), not an argument against the rule: BUILD-FIRST
says a rule is never weakened for being unused, and the engine already handles
vacuity as a first-class state. It does change one thing — **the rule must be
exhibited on fixtures**, because the live tree will not exhibit it. @spec/done

##finding-toolchain-pins-stable **`rust-toolchain.toml` pins `stable`, and it
is the only toolchain file in the tree.** A `dylint` library links rustc
internals behind `#![feature(rustc_private)]` and cannot compile on it. This is
the single fact that reshapes B-037 (§3). Boss-measured on the box the same
day: no nightly toolchain installed, `cargo dylint` not present. @spec/done

##finding-ts-vehicle-is-present **The TS vehicle is already in the tree.**
`typescript-eslint ^8.46.0` is a devDependency of the consumer demo (resolving
`@typescript-eslint/utils`, which supplies `RuleCreator`), and the TS floor
runs a bare `eslint .` — so a plugin is picked up through the project's own
flat config with **no CLI change to the floor**. @spec/done

## 2. B-036 — the invariant-comment position rule {#b036}

##b036-rule **The rule.** `invariant-comment-position`: a comment carrying an
invariant marker, in a file long enough for «thirds» to mean anything, whose
line falls in the middle third, is a finding. The remedy in the message is the
guide's own: move it to the file's top or bottom, or split the file. @spec/done

##b036-inputs **Inputs, decided.** Denominator `Fact::FileMetrics { lines }`
(ships). Numerator: a NEW engine fact carrying the marker and its line. This is
a `Fact` **VARIANT**, the expensive kind — the WAL's
`##WAL-C-KIND-VS-VARIANT-RIPPLE` checklist binds the landing (every exhaustive
`Fact` match in the whole tree, the Rust frontend's total sort, the three health
censuses, the bridges' `RawFact` arms, `cargo clean -p`), and it bumps the
frontend versions, which re-extracts. Budgeted, not discovered. @spec/done

##b036-config **Where the knobs live.** Two ROOT keys, beside `max_file_lines`
and for the same reason — they are language-neutral budgets, not per-language
policy, and the v2 law keeps the root table for exactly that: the marker
vocabulary (a list of words, identical across languages) and the minimum file
length below which position is meaningless. The per-language sections stay
uniform and untouched. **Refinement point for the worker: name both keys in the
shape the existing root table uses, and state the defaults you chose in the
report.** The boss's defaults: markers = the measured vocabulary plus the ones
the guides name (`SAFETY:`, `INVARIANT:`, `PANICS`, `WARNING:`, `MUST`,
`NEVER`), minimum length = 120 lines. @spec/done

##b036-fingerprint **Fingerprint: never line-keyed.** The stop.rs lesson
(`budget.rs:71-75`) applies exactly — key by file + marker + per-file ordinal,
so an edit above the comment does not re-key a frozen baseline entry. @spec/done

##b036-scope Scope like `FileLength`: `in_src` only, test context out. @spec/done

##b036-exhibit **Exhibit, because the live tree will not.** The dirty fixture of
each stack gains an invariant comment in its middle third; the clean fixture
gains one at the top. The characterization coupling of
`##WAL-C-CHARACTERIZATION-COUPLING` applies — every by-rule count moves in the
same landing. @spec/done

## 3. B-037 — the custom REQ-citing lint layer {#b037}

##b037-promise **What is promised, per language, verbatim.** Rust: «custom
clippy lints name the rule and the remedy». TS: «Custom `@typescript-eslint`
rules whose messages cite the violated `spec://` REQ and the fix surface». Go:
**no vehicle at all** — only «custom checks emit the same grammar». All three
carry the R3-011 grammar, and the three stack cards name a checker
(`diagnostic-cites-req`) that does not exist. @spec/done

##b037-grammar-is-the-contract **The grammar is the contract, and it already
has one authoritative renderer** — `req_message` + the `matches_req_grammar`
acceptor in the engine, with 19 production call sites. Anything built here
reproduces that string exactly; a second spelling of the grammar would be the
bug this channel exists to prevent. @spec/done

##b037-ts-build **TypeScript — BUILD, fully.** A plugin package beside
`ts-extract` / `ts-oracle`, authoring the rule the TS card itself names,
`diagnostic-cites-req`, through `ESLintUtils.RuleCreator`; its messages
rendered by one helper that reproduces the engine grammar; tested with
`@typescript-eslint/rule-tester`; wired into the consumer demo's flat config.
Nothing in the floor changes. **Refinement points for the worker: (i) the exact
detection predicate for «a diagnostic» in TS — state what you chose and what it
cannot see, in the honest-limits style the `ts-seam-error-cites-req` note set;
(ii) the package name and its `package.json` shape, matching the two sibling
tool packages; (iii) whether the demo's config wires the plugin as `error` or
`warn`, and why.** @spec/done

##b037-rust-position **Rust — the vehicle is blocked by a toolchain policy, and
that is stated, not smuggled.** The only supported route to a custom clippy lint
is `dylint`, whose library must be built against a pinned nightly; this project
pins `stable` and ships exactly one toolchain file. Two things are true at once
and both are recorded: *(i)* Rust's custom-**check** layer exists and speaks the
promised grammar — it is the conform engine itself, 19 call sites of the one
renderer; *(ii)* what is genuinely missing is a **type-aware** vehicle, and
adopting one means adding a nightly-pinned lint library plus `cargo-dylint` as a
floor tool (the same shape Go already has with `staticcheck` and TS with
`eslint`: an installed binary with a documented install recipe and a hard
failure when absent). **That adoption is a toolchain-policy decision, so it is
the owner's** — routed as a named build, never as silence. The parity law's bar
is met the way the law itself words it: the gap carries a recorded reason and a
named route, and no rule is quietly relaxed. @spec/done

##b037-go-position **Go — no vehicle is promised, and the honest move is to
name one rather than invent one silently.** The floor already distributes and
invokes single-binary analyzers (`staticcheck`, `exhaustive` via `go install` +
`path_tool`), so the shape a Go custom lint would take is already exercised;
what is missing is the *decision* that Go's custom-lint carrier is an
`analysis.Analyzer`. Recorded here, routed with the Rust half. @spec/done

##b037-doc-correction **The guides' clause is corrected in the same landing.**
All three mark `##SCAFFOLD-F-STRUCTURED-DIAGNOSTICS` `@impl/done` while the
third channel is unbuilt — which is precisely the class of claim this campaign
exists to kill. TS's clause becomes a description of the built plugin; Rust's
and Go's state what enforces the grammar today and name the routed build. @spec/done

## 4. B-038 — the pending cards get cards and checkers {#b038}

##b038-fork **The owner's fork №1, taken 2026-08-04: computed cell names.** The
canonical cell name is `Pascal(variant)` followed by **the seam spelled as
written** — `SatDepSolver`, not `SatDepsolver`; the naive
«pascal both halves» composer mangles multi-word seams and is not the rule. @spec/done

##b038-fork-cost **The cost, boss-measured over the WHOLE tree** (the census's
own count was ~10 because its perimeter was the discipline packages and
`rust-demo`; the host was outside it — a lesson recorded for the next fork
packet): **40 manifest-bearing cells; 14 already compliant; 13 production
renames in the host `crates/`** (`vibe-resolver` ×5, `vibe-mcp` ×4,
`vibe-registry` ×2, `vibe-index` ×2), the rest test fixtures and regenerated
`.vibe/cache/**` copies that are not hand-edited. The 14 compliant ones are all
of `vibe-check` — `variant = "wal-freshness"` + `seam = "Check"` →
`WalFreshnessCheck` — so the convention is already the house style in the
largest cell family, which is the strongest argument the measurement produced.
No cell name is wire-visible (MCP tool names are separate string literals), so
every rename is compiler-checked and internal. @spec/done

##b038-one-rule-two-languages **One engine rule serves Rust and Go.** Go
practises `{Variant}{Seam}` today with **no machine check anywhere**, so the
build closes a Go gap in the same move rather than creating an asymmetry: the
rule reads the manifest (Rust `#[cell(seam, variant)]`, Go `//spec:cell seam=
variant=`) and compares the declared type name against the composed one. TS
carries a recorded reason — it has no cell manifest to compute from. @spec/done

##b038-lands-frozen **The rule lands with a frozen baseline; the renames ride a
separate, deliberate commit.** So the day the rule mounts, nothing reds, and the
13 renames are reviewed as what they are — a readability improvement the
compiler verifies (`Sat` → `SatDepSolver` and `EmbeddedProvider` →
`EmbeddedDepProvider` are the two that best explain why the rule is worth
having). @spec/done

##b038-r060 **R-060 — the card is new, and its checker lands vacuous.** «Test
matrices are declared as data, never a `2^n` sweep» is cited by two guides and
the host registry with no card and no checker behind it. The census found
**zero** combinatorial constructs in the tree, so the checker will fire on
nothing here — which is BUILD-FIRST's explicit case («a rule is never weakened
for being unused»), and the fixture is what exercises it. **Refinement point for
the worker: state the syntactic signature you keyed on and, for each language,
which declared-matrix idioms you deliberately treat as compliant.** @spec/done

##b038-id-space **The id space, measured: the R3-series registry IS `ATLAS.md`**
(R3-001…R3-015, R3-004 at `:55`); the **R-0NN series has no registry at all** —
zero `FINDING-R-0` records — and R-060 is the highest cited id. So the naming
card needs no id reservation, and R-060's card is authored against an id the
corpus already uses. The absent R-series registry is a finding in its own right
and is filed to the backlog rather than fixed in passing. @spec/done

## 5. What this batch deliberately does not do {#non-goals}

##non-goal-severity No severity/warning model in the engine (§1 — the ratchet
already carries the semantic). @spec/done

##non-goal-seven-cards Not all seven pending cards — three of them plus R-060's
new one. The remaining four ride the same conveyor later, which is exactly how
`01-PATTERN-CARD-FORMAT.md` describes its own scope. @spec/done

##non-goal-vocabulary No closed-token vocabulary and no synonym/shadow
detector — the fork's variant B needed all three and the owner did not take it;
the naming rule checks composition, which needs none of them. @spec/done

##non-goal-nightly No nightly toolchain and no `dylint` library until the owner
rules on the toolchain question (§3). @spec/done
