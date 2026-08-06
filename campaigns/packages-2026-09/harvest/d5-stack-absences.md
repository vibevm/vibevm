# D5 — re-verified absences across the three language stacks

_Phase D, batch D2 (`build-or-demote`). Ten obligations over
`rust-ai-native-lang/v0.7.0`, `go-ai-native-lang/v0.1.0` and
`typescript-ai-native-lang/v0.6.0`. Every one of the ten claims that some
mechanism, checker, artefact or record **does not exist**; §3.3 closes such a
fact by demotion, and [§6.1 `##ABSENCE-NAMES-ITS-PERIMETER`](../PHASE-D-BATCH-PLAN.md#delegation-lessons)
requires the perimeter to be named in the record **before** the marker moves.
This file is that record._

**Route check, run first per `##ROUTE-BEFORE-FALSIFIER`.** All ten report
`route: build-or-demote`, `release_event: false`. None is out of route.

```
python campaigns/packages-2026-09/tasks/drift-registry.py --task F-191   # … ×10
```

**The standing perimeter.** Unless an entry narrows or widens it, every search
below was run from the repository root over: `packages/**`, `vibedeps/**`,
`crates/**`, `xtask/**`, `tools/**`, `spec/**`, `discipline/**`,
`terraform/**`, `research/**` (including the three bootstrapped consumers
`research/rust-demo`, `research/ts-demo`, `research/go-demo` and their own
`vibedeps/`, `conform.toml`, `specmap.json`, `discipline/`),
`campaigns/**`, `legacy-spec/**` and the root. `target/` is excluded as build
output. `refs/**` is searched but reported separately — it is a third-party
study corpus, not our shipped surface, and a hit there is not an implementation
of ours.

---

## F-191 — three Rust guide rules marked `@impl/done` whose enforcement is unauthored

**Outcome:** DEMOTED
**Anchors:** 3 of 3 — `##CONTRACT-FIRST-ORDERING-WITHIN-AN-ITEM`,
`##ONE-ERROR-ENUM-PER-LAYER`, `##BAN-HIDDEN-CONTROL-FLOW`
**Files touched:**
`C:\Users\olegc\git\v\vibevm\packages\org.vibevm.ai-native\rust-ai-native-lang\v0.7.0\spec\rust\GUIDE-AI-NATIVE-RUST.md`
**Perimeter searched:** the standing perimeter above, three independent sweeps —
one per claimed absence. Globs: `-g '!target/**'` throughout; `-g '!refs/**'`
where noted; `find . -name "rule-*.md"` for the card; every `spec/cards/`
directory in `packages/org.vibevm.ai-native/*/` plus their vendored copies under
`vibedeps/`, `research/*/vibedeps/` and
`packages/org.vibevm.fractality/*/vibedeps/`.

**What the search found:**

*(a) the card `rule-contract-first-ordering`.* Cited in five `cards/INDEX.md`
files (rust, go, typescript and vendored copies) and nowhere else; no file of
that name exists.

```
$ find . -path ./target -prune -o -name "rule-*.md" -print
./refs/src/bazel/site/en/release/rule-compatibility.md
```

The only match in the tree is an unrelated Bazel document under `refs/`. The
Rust stack's own index already says so in plain words —
`spec/cards/INDEX.md:36` heads a section **"Pending cards (named, not yet
authored — pilot will prioritize)"** and line 39 lists
`rule-contract-first-ordering` inside it. No shipped checker inspects ordering
either; the rust stack's conform rule roster is:

```
$ rg -o 'id: *"[a-z][a-z0-9_-]+"|"[a-z][a-z0-9-]{6,}"' \
    packages/org.vibevm.ai-native/rust-ai-native-lang/v0.7.0/crates/vendor/core-ai-native-conform/src/rules/ | sort -u
ambient-env  audited  cell-has-oracle  env-audit  error-enum-cites-req
error-message-cites-req  file-length  go-cell-isolation  go-unsafe-in-domain
no-unwrap-in-domain  pub-doctest  seam-has-doctest  ts-cell-isolation
ts-unsafe-in-domain  unsafe-gate
```

*(b) `#[track_caller]`.* Zero occurrences in any shipped surface of ours.

```
$ rg -c "track_caller" -g '!target/**' .
```

returns hits in exactly three classes: `refs/src/cargo/**` and
`refs/src/warp/**` (third-party study sources — not ours), the guide's own line
plus its vendored copies, and campaign bookkeeping
(`campaigns/**`, `spec/terraforms/PACKAGES-ACTUALIZATION-CAMPAIGN-v0.1.md:2383`,
which independently records `#[track_caller]` zero repo-wide). No `.rs` file
under `packages/**`, `crates/**`, `xtask/**`, `tools/**` or the three
`research/*-demo` consumers carries the attribute.

*(c) `R-021`.* Cited freely, authored nowhere.

```
$ rg -n "##FINDING-R-0|##R-021|##RULE-R-021" -g '!target/**' -g '!refs/**' .
(no output)
```

The core ATLAS is the authored roster, and its id space does not contain R-021:

```
$ rg -o "##FINDING-[A-Z0-9]+-" packages/org.vibevm.ai-native/core-ai-native/v0.8.0/spec/appendix/ATLAS.md | sort -u
BLD-  DR1-  DR2-  R2C-  R3-
```

R-021 survives only as a citation — `ENGINE-CONFORM-v0.1.md:36`
(`##EXAMPLE-R-021-FORBIDDEN-IDIOM`, itself an *example* of a rule tier, not a
rule) and a dozen `legacy-projections/GUIDE-*.md` mentions. The campaign's own
governing spec already reached the same conclusion twice
(`spec/terraforms/PACKAGES-ACTUALIZATION-CAMPAIGN-v0.1.md:1896` — *"(R-021,
R-020) do not exist; R-002 does"* — and `:2385-2386`).

**Which layer has it, if any:** *nowhere*, for all three — with two honest
partials. `ONE-ERROR-ENUM-PER-LAYER`'s REQ-edge clause **is** built at the
engine layer (`error-enum-cites-req`, vendored `core-ai-native-conform`) and its
panic ban is carried by `no-unwrap-in-domain`; only the `#[track_caller]` clause
is unbuilt. `CONTRACT-FIRST-ORDERING` and `BAN-HIDDEN-CONTROL-FLOW` have no
implementation at spec, engine, driver, deployment or demo layer.

**Twin in another stack:** **found, and deliberately not touched.**
`##CONTRACT-FIRST-ORDERING` exists in
`go-ai-native-lang/v0.1.0/spec/go/GUIDE-AI-NATIVE-GO.md:222` and
`typescript-ai-native-lang/v0.6.0/spec/typescript/GUIDE-AI-NATIVE-TYPESCRIPT.md:127`,
both `@impl/done`, both resting on the same unauthored card (each stack's own
`cards/INDEX.md` lists it as pending). `R-021` is cited in the Go guide at
`:148` and at `:405` (`fan-in/fan-out topologies as public surface are hidden
control flow (R-021). @status:impl/done`). Both stacks are mine, so I could have
repaired them — I did not, because **no drift verdict covers those anchors**,
and §3.1 closes an obligation by editing *and re-judging every anchor in its
list*. Moving a marker no verdict can be re-judged against would put the
document and the cache out of step in the direction the registry cannot see.
They are recorded below instead. `ONE-ERROR-ENUM-PER-LAYER` /
`#[track_caller]` is Rust-only — `rg "ONE-ERROR-ENUM|track_caller"` over the Go
and TypeScript packages returns nothing.

**What changed and why:** each of the three prescriptions is kept **word for
word** and each gained a `*Specified, not built: …*` clause naming precisely
what is missing, then dropped `@impl/done` → `@spec/done`. For
`CONTRACT-FIRST-ORDERING-WITHIN-AN-ITEM` the clause names the pending card and
the absent conform rule. For `ONE-ERROR-ENUM-PER-LAYER` the clause is written as
a *partial* — it credits `error-enum-cites-req` and `no-unwrap-in-domain` by
name so a reader does not conclude the whole rule is vapour, and isolates
`#[track_caller]` as the wish. For `BAN-HIDDEN-CONTROL-FLOW` the clause records
that R-021 has no entry in the ATLAS roster and no forbidden-idiom scan ships.
No code was written and no prescription was deleted.

**New obligations noticed:**
1. `GUIDE-AI-NATIVE-GO.md#CONTRACT-FIRST-ORDERING` (`:222`) and
   `GUIDE-AI-NATIVE-TYPESCRIPT.md#CONTRACT-FIRST-ORDERING` (`:127`) are
   `@impl/done` on the same unauthored card as the Rust anchor just demoted.
   Parallel-corpus siblings of F-191, currently unjudged.
2. `GUIDE-AI-NATIVE-GO.md:405` (`##…-BINDING-…`, the fan-in/fan-out ban) is
   `@impl/done` on R-021, the rule this entry just showed does not exist.
   Third document to rest on R-021, as the governing spec predicted at
   `:2386`.
3. `ENGINE-CONFORM-v0.1.md:36-38` in `core-ai-native/v0.8.0` marks
   `##EXAMPLE-R-021-FORBIDDEN-IDIOM` and `##EXAMPLE-R-020-NAMING-VS-MANIFEST`
   `@impl/done`, citing two rule ids that the ATLAS roster does not carry.
   Not mine to edit (`core-ai-native` is another wave's package).

---

## F-192 — three TCG-ORACLE-RUST facts whose named mechanism is absent, half-built, or a spike

**Outcome:** DEMOTED (all three; two are explicitly recorded as *partials*)
**Anchors:** 3 of 3 — `##RESOLVED-PATH-AND-VERSION-LAND-IN-INIT`,
`##GRACEFUL-EXIT-AND-THE-NO-ZOMBIE-PROPERTY`, `##STDOUT-CARRIES-LSP-FRAMES-ONLY`
**Files touched:**
`C:\Users\olegc\git\v\vibevm\packages\org.vibevm.ai-native\rust-ai-native-lang\v0.7.0\spec\rust\mechanisms\TCG-ORACLE-RUST-v0.1.md`
**Perimeter searched:** the standing perimeter, plus a targeted read of every
`.rs` under `rust-ai-native-lang/v0.7.0/crates/` and its twin copy under
`rust-ai-native-mcp/v0.7.0/crates/`, all thirteen test files in the stack
(`find … -name "*.rs" -path "*tests*"`), and the vendored bridge copies under
`research/rust-demo/vibedeps/`, `research/ts-demo/vibedeps/` and
`vibedeps/`. Globs `-g '*.rs'`, `-g '!target/**'`, `-g '!refs/**'`.

**What the search found:**

*(a) the resolved path in the `init` result — half-built.*

```
$ rg -A14 "fn init_result" packages/.../crates/rust-ai-native-tcg/src/serve.rs
76:fn init_result(oracle: &RustOracle<ChildTransport>) -> serde_json::Value {
77-    serde_json::json!({
78-        "ra_version": oracle.capabilities().server_version,
79-        "position_encoding": …
83-        "pull_diagnostics": oracle.capabilities().pull_diagnostics,
84-        "quiescent": oracle.quiescent(),
85-    })
```

Four fields, no path. Widening the search past the crate to the whole
perimeter changes the picture in one useful way the opening verdict did not
record: **the path is resolved, it is just never emitted.**
`resolve_rust_analyzer` at `crates/rust-ai-native-tcg-bridge/src/lib.rs:146`
returns a `PathBuf` (via `rustup which rust-analyzer`, then a PATH probe).
`ra_path` as a *field name* occurs exactly once in the tree —
`TCG-PROTOCOL-RUST-v0.1.md:57`, the `##OP-INIT` shape, which is spec, not code.

*(b) the no-zombie property — mechanism built, proof absent.* The dance and the
backstop both ship:

```
$ sed -n '345,351p' .../rust-ai-native-tcg-bridge/src/client.rs
impl Drop for ChildTransport {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
```

The assertion does not. The whole of the live test's exit check is
`oracle.shutdown().expect("shutdown");` (`tests/live_oracle.rs:116`). A search
for any exit-code or pid probe over the stack, the MCP package, the other two
stacks, `research/**`, `discipline/**`, `crates/**`, `xtask/**` and `tools/**`
returns only implementation-side reaping — never a test:

```
$ rg -i "surviving.pid|no.zombie|process_table|sysinfo|pgrep|try_wait|\.wait\(\)" -g '!target/**' -g '!refs/**' … | grep -v "\.md:"
… client.rs:349:        let _ = self.child.wait();          # the Drop impl itself
… transport.rs:206/213/226 (ts-demo vendored copy)          # ditto
```

The two `status.success()` hits in the bridge (`lib.rs:152,166`) are the
rust-analyzer *discovery* probes, not the session child.

*(c) stderr draining — the conclusion holds, the mechanism does not.*

```
$ rg -n "stderr" -g '*.rs' packages/.../rust-ai-native-lang/v0.7.0/crates/
…/rust-ai-native-tcg-bridge/src/client.rs:303:            .stderr(std::process::Stdio::null())
…/rust-ai-native-tcg-bridge/src/lib.rs:163:        .stderr(std::process::Stdio::null())
```

`Stdio::null()` is the OS discarding at the pipe — there is no reader. And the
parenthetical's other half is empty too:

```
$ rg -n "log::|tracing::|eprintln!|debug!|RUST_LOG|VIBE_DEBUG|TCG_DEBUG" -g '*.rs' .../rust-ai-native-tcg-bridge/
(no output)
```

The bridge crate has no logging facility, so nothing could be "surfaced only in
bridge debug logging".

**Which layer has it, if any:** *engine crate*, partially, for all three. (a)
the path exists in the bridge engine and is dropped before the driver's `init`
response; (b) the lifecycle mechanism is fully built in the engine, only the
test is missing; (c) the stream-cleanliness outcome is real at the engine layer,
delivered by a different mechanism than the one written down. Nothing here is a
spec-only fiction; all three are engine-layer facts described inaccurately.

**Twin in another stack:**
- `##RESOLVED-PATH-AND-VERSION-LAND-IN-INIT` ↔ Go's
  `TCG-ORACLE-GO-v0.1.md#INIT-RESULT-CARRIES-PATH-AND-VERSION` (`:46`) — same
  sentence, same defect. It is mine and it carries its own verdict, as **F-160**
  in this same batch, so both are repaired here. No TypeScript twin
  (`typescript-…/spec/typescript/mechanisms/TCG-ORACLE-v0.1.md` has no such
  anchor).
- `##GRACEFUL-EXIT-AND-THE-NO-ZOMBIE-PROPERTY` ↔ Go's
  `TCG-ORACLE-GO-v0.1.md#GRACEFUL-EXIT-IS-THE-LSP-DANCE` (`:201`) — **not
  touched**: the registry assigns it to **F-167, route `sync-from-code`**, which
  §1.2 routes through the owner. Another wave owns it.
- `##STDOUT-CARRIES-LSP-FRAMES-ONLY` ↔ Go's identically-named anchor
  (`TCG-ORACLE-GO-v0.1.md:209`) and TypeScript's
  `##STDOUT-CARRIES-PROTOCOL-FRAMES-ONLY` (`TCG-ORACLE-v0.1.md:132`). Both are
  mine; **neither carries a verdict in any obligation**, so neither was touched,
  for the reason given under F-191. Recorded below.
- The field name `ra_path` also lives in
  `TCG-PROTOCOL-RUST-v0.1.md#OP-INIT`, which belongs to **F-211, route
  `release`** — a publication, not this batch. Left alone.

**What changed and why:** all three prescriptions kept verbatim; each gained a
`*Specified, not built …*` clause and dropped to `@spec/done`. Two of the three
are written as explicit *partials* because a flat "not built" would be its own
lie: (a) credits `ra_version` and names `resolve_rust_analyzer` as the place the
path already exists, so Phase E knows the build is a plumbing change, not a
feature; (b) credits the shutdown dance and the `Drop` backstop by file and line
and isolates the missing *assertion*, so nobody re-implements a working
lifecycle. (c) states that the outcome is genuinely true and only the
described route to it is fictional. No code written, nothing deleted.

**New obligations noticed:**
1. `TCG-ORACLE-GO-v0.1.md#STDOUT-CARRIES-LSP-FRAMES-ONLY` (`:209`) and
   `TCG-ORACLE-v0.1.md#STDOUT-CARRIES-PROTOCOL-FRAMES-ONLY` (`:132`, TypeScript)
   make the same drained-by-the-reader claim over transports that also use
   `Stdio::null()` (see `ts-demo` vendored `transport.rs`). Unjudged.
2. No test anywhere in the three stacks asserts a lifecycle property of the
   oracle child (exit code, surviving pid). If Phase E builds the assertion for
   Rust, the Go and TypeScript bridges need it too — one build, three consumers.

---

## F-214 — TCG-PROTOCOL-RUST names a product client that was deleted

**Outcome:** DEMOTED — but with a **perimeter catch**: one of the two proofs the
first anchor cites turned out to exist, and the prose now points at it instead
of denying it.
**Anchors:** 2 of 2 — `##ONE-PRODUCT-CLIENT-DRIVES-BOTH-RELAYS`,
`##PRODUCT-LINK-LAYER-SPECIAL-CASES-ONLY-ORACLE-CRASHED`
**Files touched:**
`C:\Users\olegc\git\v\vibevm\packages\org.vibevm.ai-native\rust-ai-native-lang\v0.7.0\spec\rust\mechanisms\TCG-PROTOCOL-RUST-v0.1.md`
**Perimeter searched:** the standing perimeter, plus: every `*.toml` in the tree
for a `vibe-tcg` crate or binary declaration; a directory search
`find . -type d -name "vibe-tcg*"`; `legacy-spec/terraforms/**`; the two sibling
`mcp`-kind packages `rust-ai-native-mcp/v0.7.0` and
`typescript-ai-native-mcp/v0.6.0` (which the four-layer rule says would hold the
DRIVER); `spec/modules/vibe-mcp/PROP-026-tcg-tool-family.md` in full; and a
fixture sweep `find packages/org.vibevm.ai-native -name "*.snap" -o -name "*golden*"`.

**What the search found:**

*(a) the client is genuinely gone.*

```
$ rg -n "OracleRegistry|oracle_registry" -g '!target/**' -g '!refs/**' .
legacy-spec/terraforms/TCG-STAGE-B-DELIVERY-PLAN-v0.1.md:112 …      # historical plan
legacy-spec/terraforms/MCP-SOVEREIGNTY-PLAN-v0.1.md:70 …            # historical plan
legacy-spec/terraforms/AGENTIC-TCG-TS-PLAN-v0.1.md:383 …            # historical plan
spec/modules/vibe-mcp/PROP-026-tcg-tool-family.md:110 ##ORACLE-REGISTRY …
$ find . -path ./target -prune -o -type d -name "vibe-tcg*" -print
(no output)
```

Zero `.rs` files. The only live document that mentions it is PROP-026, whose §4
is marked `@spec/done` under an explicit banner — `@fact:RETIRED-SECTIONS-KEPT
§3–§5 below describe the retired topology and stay as the design record` — and
whose §0 carries `##TOPOLOGY-RETIRED` ("the TOPOLOGY half … the `vibe-tcg`
registry crate — is retired") and `##TCG-CRATE-DELETED` at `:42`. **The host has
already demoted its own copy of this fact honestly**; the Rust package had not.

*(b) the posture survives, one layer over and per-language.* `rust-ai-native-mcp`
holds it server-local:

```
$ sed -n '1,40p' packages/.../rust-ai-native-mcp/v0.7.0/crates/rust-ai-native-mcp/src/tools_tcg.rs
//! … over ONE persistent rust-analyzer session shared by all five tools
//! … the posture the serve relay and the old host registry carried, now server-local
pub struct TcgSession { root: PathBuf, oracle: Option<Oracle> }
```

*(c) the perimeter catch — the two live-chain tests exist.* The opening verdict
did not mention them and a package-local grep would not have found them:

```
$ find packages/org.vibevm.ai-native -name "live_chain*.rs" -not -path "*/target/*"
packages/org.vibevm.ai-native/rust-ai-native-mcp/v0.7.0/crates/rust-ai-native-mcp/tests/live_chain.rs
packages/org.vibevm.ai-native/typescript-ai-native-mcp/v0.6.0/crates/typescript-ai-native-mcp/tests/live_chain.rs
```

Two of them, exactly as the fact says — but in the two `mcp` packages, per
language, **not** "at the product level".

*(d) the goldens do not exist.*

```
$ find packages/org.vibevm.ai-native -not -path "*/target/*" \( -name "*.snap" -o -name "*golden*" \)
(no output)
```

No outer-frame replay golden anywhere in the family.

*(e) the special-casing itself is real.*

```
$ sed -n '206,210p' packages/.../rust-ai-native-tcg/src/serve.rs
            let crashed = matches!(e, TcgBridgeError::OracleCrashed { .. });
```

**Which layer has it, if any:** *stack CLI* and *sibling MCP package* — never the
"product" layer, which no longer exists. The relay's `oracle-crashed`
special-case is in the driver (`rust-ai-native-tcg`); the shared-session posture
and the live-chain proof are in the `mcp`-kind package one layer over; the
language-generic client that the sentences are written around was deleted at
the product layer and not replaced.

**Twin in another stack:** `TCG-PROTOCOL-GO-v0.1.md#ONE-PRODUCT-CLIENT-DRIVES-ALL-THREE-RELAYS`
(`:34`) — same sentence, same defect, and it is mine. **Not touched:** the
registry assigns it to **F-210, route `sync-from-code`**, `reality-mismatch` —
an owner-approved diff, not this batch. TypeScript's
`spec/typescript/mechanisms/TCG-PROTOCOL-v0.1.md` has no such anchor.
`##PRODUCT-LINK-LAYER-…` has no twin in either stack.

**What changed and why:** both prescriptions kept verbatim, both dropped to
`@spec/done`, both gained a `*Specified, not built …*` clause. The first clause
is deliberately three-part rather than a flat denial, because the perimeter
search split the fact three ways: the client is **deleted** (named, with the
PROP-026 anchors that record it), the posture **survives per-language** as
`TcgSession`, and of the two pinning mechanisms the **live-chain tests exist**
while the **goldens do not**. Writing "not built" over the live-chain tests
would have repeated the exact failure §6.1 `##ABSENCE-NAMES-ITS-PERIMETER` was
written about. The second clause credits the surviving special-case by file and
line and isolates what is fictional: the layer, not the behaviour. No code, no
deletions, and no relative link to another package — PROP-026 is cited by name
in prose.

**New obligations noticed:**
1. There are no outer-frame replay goldens in any ai-native package, yet
   `TCG-PROTOCOL-*` in more than one stack cites them as the per-package pin.
   Worth one obligation covering the family rather than one per document.
2. PROP-026 `##ORACLE-REGISTRY` (`:110`) and its `##REG-*` children are
   `@spec/done` under a retired-topology banner, while `##REG-KILL-ON-DROP`
   still asserts «the no-zombie property is test-asserted» — the same claim
   F-192 just showed has no test behind it in any stack. Host-side, not mine.

---

## F-277 — README's specmark path is stale; the proc-macro itself ships

**Outcome:** CORRECTED (it exists elsewhere) — **not demoted**, and the marker
stays `@impl/done`
**Anchors:** 1 of 1 — `##SHIPS-SPECMARK-PROC-MACRO`
**Files touched:**
`C:\Users\olegc\git\v\vibevm\packages\org.vibevm.ai-native\rust-ai-native-lang\v0.7.0\README.md`
**Perimeter searched:** the standing perimeter, plus a whole-tree directory
search for the crate under any name — `find . -type d -name "specmark*"` — and
the package's own `Cargo.toml` workspace-member and dependency tables.

**What the search found:** the verdict's literal claim is true and its
conclusion is not. `crates/specmark` really does not exist in this package —

```
$ find . -path ./target -prune -o -type d -name "specmark*" -print | grep -v "/target/"
./.vibe/cache/org.vibevm/core-ai-native/v0.6.0/crates/specmark
./.vibe/cache/org.vibevm/discipline-core/v0.4.0/crates/specmark
./.vibe/cache/org.vibevm/rust-ai-native/v0.2.0/crates/specmark
… (and ~16 more, every one of them a `.vibe/cache/` copy of a SUPERSEDED slot)
```

— every `crates/specmark` in the tree is a cached copy of an older, superseded
package version, which is where the README's path came from. But the proc-macro
is not missing; it ships in this very package under the core stem:

```
$ ls -d .../v0.7.0/crates/vendor/core-ai-native-specmark
$ grep -n "^name" .../crates/vendor/core-ai-native-specmark/Cargo.toml
2:name = "core-ai-native-specmark"
$ grep -n "specmark" .../v0.7.0/Cargo.toml
21:    "crates/vendor/core-ai-native-specmark",
46:specmark = { package = "core-ai-native-specmark", path = "crates/vendor/core-ai-native-specmark" }
```

It is a workspace member, it is aliased back to the bare name `specmark`, and
five source files in the package call it that way (`specmark::scope!`,
`specmark::` in `rust-ai-native-cli/src/init.rs`, `…/tests/fresh_project.rs`,
the crate's own `tests/usage.rs`, and the grammar crate).

**Which layer has it, if any:** *engine crate*, vendored into this stack. The
capability is fully built and shipped; only the address in the prose was stale —
a rename artefact of the `core-ai-native-*` family-stem policy the guide's own
`##FAMILY-PREFIX-RULE` describes.

**Twin in another stack:** none found.
`rg "SPECMARK|specmark" packages/…/typescript-ai-native-lang/v0.6.0/README.md
packages/…/go-ai-native-lang/v0.1.0/README.md` returns nothing — neither README
makes this claim.

**What changed and why:** this is §3.3's opposite case and the brief's rule 4 —
"shipped under a different name" — so the closure is an address repair, not a
demotion. The prescription is untouched; the parenthetical now reads
`crates/vendor/core-ai-native-specmark`, and adds that the dependency is taken
under the alias `specmark`, because that alias is what a reader will actually
type in source and the bare path alone would leave them hunting. **The
`@impl/done` marker is deliberately kept**: the fact is now true, and demoting a
built thing would be as wrong as claiming an unbuilt one. No code written.

**New obligations noticed:** the very next README bullet,
`##SHIPS-SPECMAP-WIRE-SCHEMA` (`:41-43`), points at
`specmap-core/src/generated/` — a path with the same pre-rename shape as the one
just corrected, and the package vendors `core-ai-native-specmap`, not
`specmap-core`. Not in my anchor set; unverified; worth a look.

---

## F-278 — the tcg brief reserves a bare name for a crate that was deleted

**Outcome:** DEMOTED — and the search turned up a **host-side contradiction**
the verdict did not name (recorded, not fixed)
**Anchors:** 1 of 1 — `##RENAMED-FROM-VIBE-TCG`
**Files touched:**
`C:\Users\olegc\git\v\vibevm\packages\org.vibevm.ai-native\rust-ai-native-lang\v0.7.0\spec\rust\tools\rust-ai-native-tcg.md`
**Perimeter searched:** the standing perimeter, plus: the host crate roster
`ls crates/`; every `Cargo.toml` in `crates/` and `packages/` for a
`name = "vibe-tcg"` or a `[[bin]]` of that name; `spec/common/PROP-028-package-families.md`
§2.4 in full (`:136-150`); `spec/modules/vibe-mcp/PROP-026-tcg-tool-family.md`;
and the directory search from F-214 (`find . -type d -name "vibe-tcg*"` →
nothing).

**What the search found:** the sentence has three parts and they do not share a
verdict.

*The rename is done* — this file is `rust-ai-native-tcg.md`.

*The policy chain resolves.* All three anchors it leans on are authored and
`@impl/done` in the host:

```
$ sed -n '136,150p' spec/common/PROP-028-package-families.md
- ##D13-SUPERSEDED  **Supersession of the `-rust` suffix policy (D13).** …
- ##D13-LANGUAGE-LEADS `conform-rust` becomes `rust-ai-native-conform` …
- ##D13-NEUTRAL-OUTSIDE Language-NEUTRAL artifacts stay outside any family stem: vibevm's own generic
  `vibe-*` crates, the `vibe-tcg` product cell. @impl/done
```

*The referent does not exist.* The host's crate roster is eighteen crates and
none of them is it:

```
$ ls crates/
progress-core  vibe-actions  vibe-check  vibe-cli  vibe-core  vibe-graph
vibe-index  vibe-install  vibe-llm  vibe-mcp  vibe-publish  vibe-registry
vibe-resolver  vibe-settings  vibe-spec  vibe-test-support  vibe-wire  vibe-workspace
```

Every tcg crate in the tree is per-family — `rust-ai-native-tcg`,
`typescript-ai-native-tcg`, `go-ai-native-tcg`, each with its own `[[bin]]` — and
PROP-026 `##TCG-CRATE-DELETED` records why.

**The contradiction the verdict missed, found by widening to `spec/common/`:**
the last line quoted above, `##D13-NEUTRAL-OUTSIDE`, **still names "the
`vibe-tcg` product cell"** as a live example of a language-neutral artifact, and
is itself marked `@impl/done`. So two host documents disagree: PROP-026 deletes
the crate, PROP-028 still lists it. The package was not inventing this
reservation — it was faithfully repeating a host policy that is itself stale.
That does not rescue the package's sentence, which is present-tense about a
crate that is not there, but it moves the root cause host-side.

**Which layer has it, if any:** *spec* only, and inconsistently — PROP-028 §2.4
still asserts the reservation, PROP-026 §0 retracts the thing reserved. No
engine, driver, deployment or demo layer has a `vibe-tcg` anything.

**Twin in another stack:** none found.
`rg "belongs solely|RENAMED-FROM" -g '*.md' packages/org.vibevm.ai-native/`
returns this one line only — the TypeScript and Go tcg briefs carry no
equivalent rename record.

**What changed and why:** the prescription is kept word for word and the marker
drops `@impl/done` → `@spec/done`. The added clause is written as a three-way
split rather than a flat denial, because two thirds of the sentence are sound:
it credits the rename as done and names the supersession chain as authored, then
isolates the false part — the crate — with the PROP-026 anchor that killed it and
the per-family crates that replaced it. It also distinguishes the surviving
*policy* (`vibe-*` stays reserved for language-neutral vibevm crates) from the
dead *specific reservation*, so nobody reads this demotion as licence to take
the `vibe-*` stem for a language family. No code written, nothing deleted, and
PROP-026/PROP-028 are cited by name in prose rather than by relative link.

**New obligations noticed:**
1. `spec/common/PROP-028-package-families.md#D13-NEUTRAL-OUTSIDE` (`:148`) is
   `@impl/done` and names "the `vibe-tcg` product cell" as a current example of
   a language-neutral artifact. `spec/modules/vibe-mcp/PROP-026-tcg-tool-family.md#TCG-CRATE-DELETED`
   (`:42`) says that cell is DELETED. A host-side `contradiction` between two
   `spec/common` / `spec/modules` documents — outside my three packages and not
   touched.

---

## F-160 — five TCG-ORACLE-GO facts resting on a Go corpus, a Go baseline and a warning that do not exist

**Outcome:** DEMOTED (4 markers moved; 1 anchor was already `@spec/done` and got
prose only) — with **two perimeter catches** that changed what the clauses say
**Anchors:** 5 of 5 — `##INIT-RESULT-CARRIES-PATH-AND-VERSION`,
`##DIAGNOSTICS-CHANNEL-HISTORY`, `##DIFFERENTIAL-CORPUS-PINS-DIAGNOSTIC-CLASSES`,
`##BENCH-HARNESS-RECORDS-DISTRIBUTIONS`, `##LARGE-WORKSPACE-COLD-INIT-WARNING`
**Files touched:**
`C:\Users\olegc\git\v\vibevm\packages\org.vibevm.ai-native\go-ai-native-lang\v0.1.0\spec\go\mechanisms\TCG-ORACLE-GO-v0.1.md`
**Perimeter searched:** the standing perimeter, plus: a full file listing of
`research/tcg-bench/` (`find research/tcg-bench -type f`); a tree-wide sweep for
any corpus or baseline artefact
(`find . \( -iname "*corpus*" -o -iname "*baseline*" \)`, minus `target/`,
`node_modules`, `.vibe/cache`, `refs/`); every `.rs` in
`go-ai-native-lang/v0.1.0/crates/` and the sibling `go-ai-native-mcp/v0.1.0`;
`research/go-demo/` including its `discipline/`; and — the catch below — the
Go stack's own `spec/go/**` and `README.md`, not only its code.

**What the search found:**

*(a) the resolved path — half-built, exactly as in Rust.*

```
$ sed -n '74,84p' .../go-ai-native-tcg/src/serve.rs
fn init_result(oracle: &GoOracle<ChildTransport>) -> serde_json::Value {
    serde_json::json!({
        "gopls_version": …, "position_encoding": …,
        "pull_diagnostics": …, "ready": oracle.ready(),
```

Four fields, no path — while `resolve_gopls`
(`go-ai-native-tcg-bridge/src/lib.rs:145`) resolves one and returns it.

*(b, c) there is no Go corpus.* The whole of `research/tcg-bench` is two
corpora and two baselines, neither of them Go:

```
$ find research/tcg-bench -type f
research/tcg-bench/corpus/cases/01-clean-disk.json … 07-union.json        # TypeScript, 7 cases
research/tcg-bench/corpus-rust/cases/01-clean-disk.json … 09-missing-fields.json  # Rust, 9 cases
research/tcg-bench/reports/bench-2026-07-07-baseline.json                 # TypeScript
research/tcg-bench/reports/bench-rust-2026-07-07-baseline.json            # Rust
research/tcg-bench/reports/REPORT-2026-07-07-{control,rust-baseline,with-tools}.md
research/tcg-bench/run-battery.sh
```

No `corpus-go`, no Go report, no mapping table. Widening to the whole tree for
anything corpus- or baseline-shaped returns only conform ratchet baselines
(`discipline/registry/tests-baseline.json`, `…/conform/src/baseline.rs`) — a
different artefact entirely.

*(d) the bench harness exists; the ledger entry does not.*

```
$ grep -n "fn run_bench" -A6 .../go-ai-native-tcg/src/bench.rs
106:pub fn run_bench(root: &Path, corpus_rel: &str, report_rel: &str) -> Result<()> {
$ rg -n "warm_ms" -g '*.rs' .../go-ai-native-tcg/src/bench.rs
56:    warm_ms: Vec<u128>,        134/141/215/224 — three warm passes per case
```

The harness records distributions. Nothing has ever been run through it on
`research/go-demo`.

*(e) **first perimeter catch — a large-workspace warning DOES exist.*** A grep
over code alone says "no warning string", which is how the opening verdict read
it. Searching the stack's `spec/` finds one:

```
$ rg -i "cold init|first.request|ceiling" .../go-ai-native-lang/v0.1.0/spec/ .../README.md
spec/go/tools/vibe-agentic-tcg-go.md:186:- ##RISK-COLD-INIT-ON-LARGE-WORKSPACES **Cold init on large workspaces.** …
  posted targets (< 15 s demo-class) move only with a recorded REPORT reason;
  the eager-init-at-serve-start posture and the `degraded` flag are the mitigations. @spec/done
spec/go/mechanisms/TCG-ORACLE-GO-v0.1.md:227:- ##TARGET-COLD-INIT cold init-to-ready < 15 s. @impl/done
```

So consumers *are* warned — at the spec layer, in the sibling brief — just not
by anything the product emits at run time.

*(f) **second perimeter catch — the 60 s figure is supported by nothing, and
the stack's own numbers disagree with it.***

```
$ rg -n "const READINESS_BUDGET" -B2 -A1 -g '*.rs' .../go-ai-native-lang/v0.1.0/crates/
crates/go-ai-native-tcg/src/lib.rs:32:pub const READINESS_BUDGET: … Duration::from_secs(45);
```

45 s shipped, 15 s posted as the target two anchors above, 60 s asserted here.
**I did not change the number.** Rewriting 60 → 45 would be a spec-to-code sync,
which §1.2 routes through the owner as `sync-from-code`; this obligation is
`build-or-demote`. The clause records all three figures and leaves the
prescription verbatim for that owner decision.

*(g) the eager init is real* — `GoOracle::spawn(&root, READINESS_BUDGET)` at
`serve.rs:235`, before the first host frame.

**Which layer has it, if any:** mixed, and the split is the point. (a) *engine
crate* has the path, the *stack CLI* drops it. (b, c) *nowhere* — no corpus at
any layer, though the capability itself is negotiated live in the engine
(`client.rs:33`). (d) *stack CLI* has the harness, *nowhere* has the measurement.
(e) *spec* has the warning (sibling brief), *nowhere* has a runtime warning, and
the 60 s number exists at no layer at all.

**Twin in another stack:** `##INIT-RESULT-CARRIES-PATH-AND-VERSION` ↔ Rust's
`TCG-ORACLE-RUST-v0.1.md#RESOLVED-PATH-AND-VERSION-LAND-IN-INIT`. **Both are
mine and I repaired both** — Rust under F-192 earlier in this file, Go here —
with parallel clauses that each name their own `resolve_*` function, so the
family does not end up half-fixed. The other four anchors are Go-specific: the
TypeScript and Rust stacks have real corpora and real baselines, so the absence
does not transfer, and neither has a `LARGE-WORKSPACE-COLD-INIT-WARNING`
equivalent.

**What changed and why:** five prescriptions kept word for word; four markers
dropped `@impl/done` → `@spec/done`; `##DIAGNOSTICS-CHANNEL-HISTORY` was
**already** `@spec/done`, so it received the honesty clause with **no marker
change** — there was nothing to demote, only prose that over-claimed. Every
clause is a partial rather than a denial, because in four of the five cases the
mechanism half is genuinely built and only the *record* is missing: the path is
resolved but dropped, the channel is negotiated but unrecorded, the harness runs
but has never been run, the eager init works but the ceiling is fiction. The
fifth (`DIFFERENTIAL-CORPUS-PINS-…`) is a flat absence and says so. No code
written, nothing deleted, no number silently synced.

**New obligations noticed:**
1. **A number conflict inside one document.** `##LARGE-WORKSPACE-COLD-INIT-WARNING`
   says 60 s, `##TARGET-COLD-INIT` (`:227`) says < 15 s, and the shipped
   `READINESS_BUDGET` is 45 s. That is a `reality-mismatch` on an owner
   `sync-from-code` route, not mine to resolve — recorded here so the owner sees
   all three figures together.
2. `spec/go/tools/vibe-agentic-tcg-go.md#RISK-COLD-INIT-ON-LARGE-WORKSPACES`
   is the only place a Go consumer is actually warned, and the mechanism doc
   does not point at it. A cross-reference, not a defect — but the two would
   stop drifting if one named the other.
3. Neither the Rust nor the Go bench records a `cold_init_ms`, while the
   TypeScript one does (`typescript-ai-native-tcg/src/bench.rs:47,132`). If a Go
   baseline is ever taken, the cold-init target has no field to land in.

---

## F-184 — three Go guide rules; only one of the three claimed absences survived

**Outcome:** **MIXED — 1 DEMOTED, 1 RE-JUDGE: confirmed, 1 CORRECTED.** Two of
the three verdicts in this obligation were perimeter misses.
**Anchors:** 3 of 3 — `##BASELINE-BOUNDARY-VALIDATION` (demoted),
`##EXHAUSTIVENESS-IS-CARRIED-BY-A-LINTER` (confirmed, untouched),
`##SWEEP-CENSUS-REGRESSIONS` (corrected, marker kept)
**Files touched:**
`C:\Users\olegc\git\v\vibevm\packages\org.vibevm.ai-native\go-ai-native-lang\v0.1.0\spec\go\GUIDE-AI-NATIVE-GO.md`
**Perimeter searched:** the standing perimeter, plus: `C:\opt\gotools\` (the
tool install location); a live `go-ai-native floor` and `go-ai-native health` run
against `research/go-demo` with that directory on PATH; the Go conform engine
(`crates/vendor/core-ai-native-conform/src/rules/go.rs` and `.../facts.rs`), the
Go extractor (`tools/go-extract/extract.go`), the CLI's `floor.rs` and
`health.rs`; the captured runs under
`tools/go-extract/test/fixtures/{clean,dirty}/target/conform/`
(`test-baseline.json`, `report-go.sarif`); and both Go skills
(`spec/skills/go-ai-native-sweep/SKILL.md`,
`spec/skills/go-ai-native-terraform/SKILL.md`).

### Anchor 2 — `##EXHAUSTIVENESS-IS-CARRIED-BY-A-LINTER` → RE-JUDGE: confirmed

The verdict read: *«the linter the fact says carries this entirely is **not
installed**, and its floor step fails rather than skipping»*, quoting
`harvest/go-ai-native-lang-floor.md:25`. **That is a fact about the PATH of the
machine that captured the harvest, not about the mechanism.** The linters are
installed:

```
$ ls C:/opt/gotools/
exhaustive.exe   gopls.exe   staticcheck.exe
```

The step that runs them ships and is a hard gate, with a recipe on failure —
`floor.rs:138-161`, `crate::tools::path_tool(root, "exhaustive")` with
`.arg("./...")`. With that directory on PATH the whole floor runs green on the
bootstrapped Go consumer:

```
$ export PATH="/c/opt/gotools:$PATH"; cd research/go-demo
$ .../go-ai-native-lang/v0.1.0/target/debug/go-ai-native.exe floor
=== gofmt -l . ===
=== go vet ./... ===
=== go test ./... ===
ok      reconcile-demo/internal/cells/batchplanner  (cached)
ok      reconcile-demo/internal/cells/naiveplanner  (cached)
ok      reconcile-demo/internal/registry            (cached)
ok      reconcile-demo/internal/sim                 (cached)
=== staticcheck ./... && exhaustive ./... ===
=== go-ai-native-conform check ===
=== go-ai-native-specmap --check ===
=== test-gate (xfail-strict) ===
floor: all green (7 step(s) run, 0 disabled by policy).
```

**No edit. `@impl/done` stands** — the rule is carried by a linter, exactly as
written, and the linter runs clean.

### Anchor 3 — `##SWEEP-CENSUS-REGRESSIONS` → CORRECTED

The verdict read: *«the five census names have no producer: no census exists in
the stack, the host or the captured runs»*. **Wrong on all three counts.** The
producer is a three-stage chain and it runs. The extractor emits the kinds:

```
$ rg -n '"(init_decl|blank_import|ambient_call|naked_go|error_string_match|seam_error_missing_req)"' \
     .../go-ai-native-lang/v0.1.0/tools/go-extract/extract.go
251:  ex.unsafeAt("naked_go", ex.line(node.Pos()))
438:  ex.unsafeAt("init_decl", ex.line(d.Pos()))
534:  ex.unsafeAt("seam_error_missing_req", ex.line(s.Pos()))
581:  ex.unsafeAt("ambient_call", ex.line(sel.Pos()))
591:  ex.unsafeAt("error_string_match", ex.line(b.Pos()))
613:  ex.unsafeAt("error_string_match", ex.line(call.Pos()))
```

The conform engine consumes them — `rules/go.rs:104` scopes
`init_decl | blank_import | ambient_call | naked_go` to `cells_dir`, and
`facts.rs:121-123` documents the vocabulary. The collector summarises them:

```
$ cd research/go-demo && ... go-ai-native.exe health
health: 15 file(s) in scope; 0 over budget, 0 in the danger band;
ban census 0 reasoned / 2 unreasoned; 3/39 exports carry Examples;
orphan backlog 0. Snapshot at discipline/health/latest-go.json.

$ python -c "...json.load(open('discipline/health/latest-go.json'))..."
{ "collector": "go-ai-native health", "files_in_scope": 15,
  "ban_census": { "reasoned": 0, "unreasoned": 2 } }
```

And a **captured** run carries the kinds individually — the dirty fixture's
frozen baseline, committed in the package:

```
$ cat .../tools/go-extract/test/fixtures/dirty/target/conform/test-baseline.json
"go-unsafe-in-domain|internal/cells/plan/plan.go|ambient_call#32",
"go-unsafe-in-domain|internal/cells/plan/plan.go|error_string_match#36",
"go-unsafe-in-domain|internal/cells/plan/plan.go|init_decl#24",
"go-unsafe-in-domain|internal/cells/plan/plan.go|naked_go#34",
"go-unsafe-in-domain|internal/cells/plan/plan.go|seam_error_missing_req#17",  ...
```

Two of the guide's five names — `error_string_match`, `seam_error_missing_req` —
are the shipped kinds **verbatim**. The other three carry an `_in_cell` suffix
the engine does not use, because it expresses «in a cell» as a scope predicate
over `cells_dir` rather than in the name. That is a vocabulary gap, not an
absence. **The marker `@impl/done` is kept** and the roster gained a clause
mapping each name onto its producer.

**I deliberately did not rename the three names.** The identical five-name roster
is also the sweep skill's ratchet item
(`spec/skills/go-ai-native-sweep/SKILL.md:79`, `##RATCHET-CENSUS-REGRESSIONS`) —
same package, mine, no verdict on it. Renaming in the guide alone would have
split the pair; annotating leaves both correct and puts the mapping in one place.

### Anchor 1 — `##BASELINE-BOUNDARY-VALIDATION` → DEMOTED

This is the one that survived.

```
$ rg -n "DisallowUnknownFields" -g '!target/**' -g '!refs/**' -g '!campaigns/**' .
```

returns **no Go source at all** — only prose: this guide (`:152`), the core Go
projection `GUIDE-GO-v0.1.md:19` with its vendored copies, and
`spec/skills/go-ai-native-terraform/SKILL.md:53`, which lists «loose boundary
decoding (no `DisallowUnknownFields`)» as a *file-debt inventory item* the
brownfield adoption records. No conform rule and no floor step inspects boundary
decode. The `deny_unknown_fields` hits nearby are Rust `serde` attributes on the
engine's own config structs — the tooling keeping the rule on itself, not
checking a Go consumer.

**A correction to the verdict's framing, from the perimeter.** It said the one Go
consumer «does not use it». More precisely, `research/go-demo` decodes no JSON
anywhere:

```
$ rg -n "json\.(NewDecoder|Unmarshal|Decoder)" research/go-demo/ -g '!target/**' -g '!vibedeps/**'
(no output)
```

So the demo is not violating the rule; it never reaches a JSON boundary. What is
absent is the *checker*, and any demonstration — which is what the clause now
says, and why `@impl/done` was wrong.

**Which layer has it, if any:** anchor 2 — *stack CLI* (floor step) plus an
external evidence provider, fully built. Anchor 3 — *stack CLI* (`health`),
*engine crate* (`rules/go.rs`) and the Go *extractor*, fully built, three names
aliased. Anchor 1 — *spec* only, in three documents and one skill inventory; no
engine, driver, deployment or demo layer has it.

**Twin in another stack:** the boundary-validation rule's ancestor is
`core-ai-native/v0.8.0/spec/legacy-projections/GUIDE-GO-v0.1.md:19` — **another
package, not mine, not touched** — and it carries no `##ANCHOR` and no marker, so
it is prose rather than a claim. The census roster's twin is inside my own
package (`go-ai-native-sweep/SKILL.md:79`) and is discussed above. The
exhaustiveness rule is Go-specific by construction; neither the Rust nor the
TypeScript guide has an equivalent.

**What changed and why:** one demotion, one annotation, one no-op. The split
matters more than the count: had all three been demoted on the opening verdicts,
this pass would have marked a green-running floor gate and a live census as
unbuilt — the precise failure `##ABSENCE-NAMES-ITS-PERIMETER` exists to stop, and
in the linter's case the brief had already named the trap. No prescription was
deleted, no name was renamed, and no code was written.

**New obligations noticed:**
1. `campaigns/packages-2026-09/harvest/go-ai-native-lang-floor.md:25` records the
   floor failing on «the step's tool did not spawn (program not found)». That
   harvest artefact is now known to be a PATH artefact of the capturing machine;
   any other verdict resting on it should be re-checked with `C:\opt\gotools` on
   PATH before it is believed.
2. The three `_in_cell` census names, in the guide and in
   `go-ai-native-sweep/SKILL.md:79`, do not match the shipped kind vocabulary
   (`init_decl` / `ambient_call` / `naked_go`). That is a `terminology`
   obligation — the type §1.1 records as empty today. Recorded, not fixed.
3. `research/go-demo` exercises no JSON boundary at all, so several §1 boundary
   rules in the Go guide have no demonstration in the only Go consumer. Worth
   knowing before those rules are judged against it again.

---

## F-271 — the Go agentic brief calls Stage A «proven» by a corpus and a baseline that do not exist

**Outcome:** DEMOTED
**Anchors:** 1 of 1 — `##STAGE-A-CONSULTATION-ORACLE`
**Files touched:**
`C:\Users\olegc\git\v\vibevm\packages\org.vibevm.ai-native\go-ai-native-lang\v0.1.0\spec\go\tools\vibe-agentic-tcg-go.md`
**Perimeter searched:** the standing perimeter, and specifically the same
artefact sweep this file records under F-160 — a full file listing of
`research/tcg-bench/`, plus a tree-wide
`find . \( -iname "*corpus*" -o -iname "*baseline*" \)` minus `target/`,
`node_modules`, `.vibe/cache` and `refs/` — re-run rather than assumed, because
this is a second document resting on the same two artefacts. Also: the shipped
op surface (`crates/go-ai-native-tcg/src/main.rs`), the Go MCP package
`go-ai-native-mcp/v0.1.0/crates/go-ai-native-mcp/src/` for the delivery half,
and both sibling agentic briefs in the Rust and TypeScript stacks.

**What the search found:** the *oracle* ships and the *proof* does not.

Shipped — six subcommands, the four consultation ops plus the relay and the
bench harness:

```
$ rg -n "^    [A-Z][A-Za-z]* \{|^    [A-Z][A-Za-z]*,$" .../go-ai-native-tcg/src/main.rs
30:    Serve,       34:    Validate {   42:    Scope {
51:    Complete {   66:    Type {       77:    Bench {
```

and the MCP delivery half exists as its own package
(`go-ai-native-mcp/.../src/tools_tcg.rs`, beside `tools_discipline.rs`).

Absent — both named proofs:

```
$ find research/tcg-bench -type f
research/tcg-bench/corpus/cases/...            # TypeScript, 7 cases
research/tcg-bench/corpus-rust/cases/...       # Rust, 9 cases
research/tcg-bench/reports/bench-2026-07-07-baseline.json         # TypeScript
research/tcg-bench/reports/bench-rust-2026-07-07-baseline.json    # Rust
research/tcg-bench/reports/REPORT-2026-07-07-{control,rust-baseline,with-tools}.md
```

No `corpus-go`, no Go baseline, no Go report. Nothing has ever been run against
`research/go-demo` — which, as F-160 records, is the same pair of missing
artefacts that falsifies four anchors in `TCG-ORACLE-GO-v0.1.md`. One absence,
five anchors, two documents.

**Which layer has it, if any:** *stack CLI* and the sibling *mcp* package hold
the mechanism in full; *nowhere* holds the evidence. The word doing the damage in
this sentence is «proven», not the capability list before it.

**Twin in another stack:** `##STAGE-A-CONSULTATION-ORACLE` exists in both
siblings — `rust-ai-native-lang/v0.7.0/spec/rust/tools/vibe-agentic-tcg-rust.md:142`
and `typescript-ai-native-lang/v0.6.0/spec/typescript/tools/vibe-agentic-tcg-ts.md:142`,
both mine, both `@impl/done`. **Neither was touched, and neither should be:**
their claims are TRUE. Rust's cites «the differential corpus and the bench
baseline» and both exist (`corpus-rust/`, `bench-rust-2026-07-07-baseline.json`);
TypeScript's cites «the two-arm battery» and both arms are reported
(`REPORT-2026-07-07-control.md`, `REPORT-2026-07-07-with-tools.md`). This is a
parallel corpus where **only one member is wrong**, and the demotion clause says
so explicitly so a later reader does not «restore consistency» by demoting three.

Two neighbouring anchors in this same document belong to other routes and were
left alone: `##COMPONENT-THE-PRODUCT-SEAM` is **F-189, route `release`** (and is
the three-way cross-package family), and `##FLOOR-REMAINS-THE-TRUTH` is
**F-273, route `sync-from-code`**.

**What changed and why:** the prescription is kept word for word and the marker
drops `@impl/done` → `@spec/done`. The clause is written as a partial: it names
the six shipped subcommands and the MCP package first, so the demotion cannot be
misread as «Stage A does not exist», then isolates the two missing artefacts and
states the consequence in the document's own vocabulary — the mechanics are
unproven rather than proven. It closes by recording that the Rust and TypeScript
twins stand, which is the fact most likely to be lost between sessions. No code
was written and nothing was deleted.

**New obligations noticed:**
1. One missing pair of artefacts (a Go corpus and a Go bench baseline) now
   accounts for **five** demoted anchors across two documents — F-160's four and
   this one. If Phase E builds anything here, building the Go corpus and taking
   one baseline on `research/go-demo` re-judges all five at once. Worth carrying
   as a single Phase E item rather than five.
2. The file is named `vibe-agentic-tcg-go.md`, a `vibe-` bare name with a
   language `-go` suffix — both halves of the naming policy that
   `##FAMILY-PREFIX-RULE` and PROP-028 §2.4 superseded, and the same policy whose
   stale referent F-278 records. Its Rust and TypeScript siblings carry the same
   old shape (`vibe-agentic-tcg-rust.md`, `vibe-agentic-tcg-ts.md`). A
   three-package `relocation`/naming question, not touched, and a rename would be
   a release event under §4.5.

---

## F-168 — the TypeScript guide's flag tiers and flag registry, and one dangling evidence id

**Outcome:** DEMOTED (3 markers moved; the fourth anchor was already `@spec/done`
and got prose only)
**Anchors:** 4 of 4 — `##ADVANTAGE-1-TCD-EXISTS-FOR-TYPESCRIPT` (prose only),
`##TIER-BUILD-TIME`, `##TIER-RUNTIME`,
`##FLAG-REGISTRY-IS-TYPED-DATA-WITH-PROVENANCE`
**Files touched:**
`C:\Users\olegc\git\v\vibevm\packages\org.vibevm.ai-native\typescript-ai-native-lang\v0.6.0\spec\typescript\GUIDE-AI-NATIVE-TYPESCRIPT.md`
**Perimeter searched:** the standing perimeter, plus: the whole of
`research/ts-demo` — its `package.json`, `tsconfig.json`, `eslint.config.js`,
`conform.toml`, `specmap.toml`/`.json`, `discipline/`, `vibedeps/` and every
`.ts` under `src/`; the core ATLAS id roster
(`core-ai-native/v0.8.0/spec/appendix/ATLAS.md`); and the vendored conform engine
in all three stacks and their `mcp` siblings
(`crates/vendor/core-ai-native-conform/src/rules/{structure.rs,mod.rs,tests.rs}`),
because the four-layer rule puts a flag rule in the ENGINE, not the guide.

**What the search found:**

*(a) `DR1-014` is a dead id; `R2C-005` is not.* The distinction matters and the
opening verdict named only the failure:

```
$ rg -n "FINDING-DR1-013|FINDING-DR1-014|FINDING-DR1-015" .../core-ai-native/v0.8.0/spec/appendix/ATLAS.md
45:- ##FINDING-DR1-013 **DR1-013** — Token Sugar: reversible token-efficient shorthand for code
181:- ##FINDING-DR1-015 **DR1-015** — Constrained decoding helps weak models most; can hurt strong ones
$ rg -n "FINDING-R2C-005" .../ATLAS.md
107:- ##FINDING-R2C-005 **R2C-005** — Type-constrained decoding is PER-LANGUAGE manual work; no Rust impl exists
```

The roster steps `DR1-013` → `DR1-015`. Repo-wide, `DR1-014` survives only in
this guide, in the Rust tcg brief, and in campaign bookkeeping — never as an
anchor. The host's own campaign spec had already measured this
(`spec/terraforms/PACKAGES-ACTUALIZATION-CAMPAIGN-v0.1.md:2583`: *«`DR1-013` and
`DR1-015` exist; `DR1-014` has no anchor»*).

*(b) no bundler exists in the consumer.* The build-time tier needs one and there
is none:

```
$ cat research/ts-demo/package.json
"scripts":         { "test": "node --test", "floor": "typescript-ai-native floor" },
"devDependencies": { "@types/node", "eslint", "prettier", "typescript", "typescript-eslint" }
```

No vite, esbuild, webpack, rollup, parcel or tsup; no build script; no `define`
table. `rg "vite|esbuild|webpack|rollup|parcel|tsup|\"define\""` over
`package.json` and `tsconfig.json` returns nothing.

*(c) no registry object exists.* The whole TypeScript consumer is five files:

```
$ find research/ts-demo/src -type f -name "*.ts"
research/ts-demo/src/cells/farewell/index.ts   (+ index.test.ts)
research/ts-demo/src/cells/greeting/index.ts   (+ index.test.ts)
research/ts-demo/src/core/text.ts
$ rg -i "registry|flag|as const" research/ts-demo/src/
(no output)
```

*(d) the flag registry — and the one adjacent thing that DOES exist.* This is
where widening to the engine changed the answer. The vendored conform crate
exports a `FlagSites` rule:

```
$ rg -n "FlagSites" -g '*.rs' packages/org.vibevm.ai-native/typescript-ai-native-lang/
.../rules/structure.rs:26:  pub struct FlagSites { pub registry_file: String, pub gated_crate: String }
.../rules/structure.rs:33:  impl Rule for FlagSites   →  fn id() -> "R-001"
.../rules/mod.rs:24:        pub use structure::{CellHasOracle, CellIsolation, FlagSites};
.../rules/tests.rs:51, 238: let rule = rules::FlagSites { … }
```

It is real, and it is **not** this fact. It polices *where* flags become cells
(one legal construction site), not whether the table carries provenance, birth
and sunset; its fields are Rust-shaped (`gated_crate`, a `registry.rs` path); and
it is constructed **only in the engine's own tests** — a repo-wide search for
`FlagSites {` outside `rules/tests.rs` returns nothing in any stack or `mcp`
package. So the capability sits in the engine, unwired, and adjacent to the
claim rather than under it. The clause says exactly that, so nobody reads the
demotion as "there is no flag machinery at all" and nobody reads `FlagSites` as
satisfying this anchor.

**Which layer has it, if any:** (a) *spec* — one of two cited ids is authored in
`core-ai-native`, the other nowhere. (b, c) *nowhere* — not in spec beyond this
description, not in any engine crate, not in the stack CLI, not in the demo.
(d) *engine crate*, but for a neighbouring rule and only under test; nothing at
driver, deployment or demo layer.

**Twin in another stack:** found in both siblings, **none touched**.
`rust-ai-native-lang/v0.7.0/spec/rust/GUIDE-AI-NATIVE-RUST.md` carries
`##TWO-TIERS-OF-FLAGS` (`:89`) and `##FLAG-REGISTRY-IS-DATA-WITH-PROVENANCE`
(`:91`) — the direct twin of this anchor, in my own package, but **no obligation
covers either**, so no verdict could be re-judged against a moved marker.
`go-ai-native-lang/v0.1.0/spec/go/GUIDE-AI-NATIVE-GO.md#TWO-TIERS-NEVER-CONFUSED`
(`:381`) is **F-166, route `sync-from-code`** — an owner route. And the
neighbouring TypeScript anchor `##RULE-FLAGS-READ-AT-THE-ROOT-AND-DISPATCHED`
(`:177`), two lines below one I edited, is **F-161, also `sync-from-code`** —
deliberately left alone even though it sits in the same paragraph.

**What changed and why:** four prescriptions kept word for word; three markers
dropped `@impl/done` → `@spec/done`; `##ADVANTAGE-1-TCD-EXISTS-FOR-TYPESCRIPT`
was **already** `@spec/done`, so it took the honesty clause with **no marker
change** — the research claim is sound and only its second citation is dead, so
demoting it further would have been wrong. The `TIER-*` clauses each name the
consumer's actual contents (devDependency list, file list) rather than asserting
a bare absence, so a later reader can re-check them in one command. The
flag-registry clause is the most carefully hedged of the four for the reason in
(d). No code written, nothing deleted.

**New obligations noticed:**
1. `GUIDE-AI-NATIVE-RUST.md#FLAG-REGISTRY-IS-DATA-WITH-PROVENANCE` (`:91`) and
   `#TWO-TIERS-OF-FLAGS` (`:89`) are `@impl/done` and are the exact Rust twins of
   two anchors demoted here, resting on the same unbuilt machinery. **Unjudged by
   any obligation** — the parallel corpus is now knowingly half-demoted, and this
   is the single most important line in this entry for whoever plans wave 2.
2. `DR1-014` is cited in a second document,
   `rust-ai-native-lang/v0.7.0/spec/rust/tools/rust-ai-native-tcg.md#DERIVED-FROM-THE-EVIDENCE`
   (`:11`, `@spec/done`), with the same dead id. One absent ATLAS anchor, two
   packages.
3. `FlagSites` (`R-001`) is exported by the conform engine vendored into all six
   ai-native packages and is constructed only in tests. Either it should be wired
   into a check path or its export is dead surface — and its `R-001` id is
   another rule number with no ATLAS entry, the same class of gap F-191 recorded
   for R-021.

---

## F-282 — the scaffold-D sunset condition names a binary that was renamed, not deleted

**Outcome:** CORRECTED (it exists elsewhere) — **not demoted**; the marker was
already `@spec/done` and stays
**Anchors:** 1 of 1 — `##RISK-SUNSET`
**Files touched:**
`C:\Users\olegc\git\v\vibevm\packages\org.vibevm.ai-native\typescript-ai-native-lang\v0.6.0\spec\cards\scaffold-d-differential-oracle.md`
**Perimeter searched:** the standing perimeter, plus: a tree-wide
`rg -l "vibe-tcg-ts"`; the package's `vibe.toml` `[[binary]]` table; its
`crates/` directory and the candidate crate's `Cargo.toml`; the stack's whole
tool-brief directory `spec/typescript/tools/`; and the sibling
`scaffold-d-differential-oracle.md` cards in the Rust and Go stacks.

**What the search found:** the tool was **renamed, not removed** — the opposite
shape to F-214/F-278's deleted `vibe-tcg`, and worth separating carefully.

The old name is gone from every live surface:

```
$ rg -l "vibe-tcg-ts" -g '!target/**' .
./specmap.json                                    # host index
./spec/boot/90-user.md                            # host boot snippet
./spec/terraforms/PACKAGES-ACTUALIZATION-CAMPAIGN-v0.1.md
./legacy-spec/terraforms/{AGENTIC-TCG-TS-PLAN,DEFERRALS-CLOSEOUT-PLAN}-v0.1.md   # historical plans
./packages/…/typescript-ai-native-lang/v0.6.0/spec/cards/scaffold-d-differential-oracle.md   # this card
./research/ts-demo/vibedeps/…, ./vibedeps/…       # vendored copies of this card
./campaigns/packages-2026-09/tasks/evidence/…     # campaign bookkeeping
```

No crate, no binary, no `[[binary]]` entry. The new name carries both halves of
what the sentence needs. The binary ships:

```
$ grep -n -B3 -A3 "typescript-ai-native-tcg" .../typescript-ai-native-lang/v0.6.0/vibe.toml
44:[[binary]]
45:name = "typescript-ai-native-tcg"
46:crate = "crates/typescript-ai-native-tcg"
47:description = "the agentic type oracle: a persistent enriching relay (serve) over the
     language-service oracle, one-shot validate/scope/complete/type forms, and the bench harness"
$ grep -n "^name" .../crates/typescript-ai-native-tcg/Cargo.toml
2:name = "typescript-ai-native-tcg"      14:name = "typescript-ai-native-tcg"   # the [[bin]]
```

And the *generation-time* tier the sunset condition actually turns on is
specified under the same new name:

```
$ head -3 .../v0.6.0/spec/typescript/tools/typescript-ai-native-tcg.md
# Tool Spec (high-level): `typescript-ai-native-tcg` — Token-Level Type-Constrained Generation for TypeScript {#root}
<status stage="spec" state="done"/>
```

**The distinction that shaped the edit.** The shipped binary is the
*consultation* oracle; the *token-level generation* capability the sunset
condition depends on is `stage="spec"`, not built. So a bare rename would have
been a subtler error than the stale name it replaced — a reader would find a real
binary of that name and could conclude the sunset condition was live. The
correction therefore does two things: fixes the name, and states that the
condition is not met because the generation tier is still spec-stage.

**Which layer has it, if any:** *stack CLI* for the consultation binary
(`crates/typescript-ai-native-tcg`, wired through `vibe.toml`), and *spec* for
the token-level generation tier (the tool brief, `stage="spec" state="done"`).
Nothing at the deployment or demo layer, which is correct for a
not-yet-triggered sunset condition.

**Twin in another stack:** **found in my own Rust package, and NOT touched —
this one matters.**
`rust-ai-native-lang/v0.7.0/spec/cards/scaffold-d-differential-oracle.md:58`
carries the same sentence naming **`vibe-tcg`**, and its defect is *different*:
by F-278's evidence that crate was DELETED, not renamed, so the Rust card's
correct target would be `rust-ai-native-tcg` and its clause would have to say
something else. No obligation covers that anchor, so no verdict could be
re-judged against a change to it. Recorded below.
`go-ai-native-lang/v0.1.0/spec/cards/scaffold-d-differential-oracle.md:101` says
"generation-time tooling" with **no tool name at all** and is therefore not
broken — a useful accident: the Go card is the only one of the three that cannot
go stale this way.

**What changed and why:** the prescription is kept and the stale binary name is
repaired in place, exactly as under F-277 — this is the brief's rule 4
("shipped under a different name": correct the pointer, do not demote), not §3.3.
The rename is attributed to the family-prefix policy so the change reads as
policy compliance rather than an arbitrary substitution, and a second sentence
records that the condition remains unmet. **The `@spec/done` marker is
unchanged**, which was already the honest state: a sunset condition is
hypothetical by construction and was never claiming implementation. No code
written, nothing deleted, no relative link added.

**New obligations noticed:**
1. `rust-ai-native-lang/v0.7.0/spec/cards/scaffold-d-differential-oracle.md#RISK-SUNSET`
   (`:58`) names `vibe-tcg`, a crate PROP-026 records as DELETED. Same sentence,
   worse defect, **no obligation covers it**. Mine, unjudged, untouched.
2. `spec/boot/90-user.md` and the host's `specmap.json` still carry the
   `vibe-tcg-ts` name. `90-user.md` is a boot snippet every session reads and is
   owner-owned — host-side, explicitly not touched.
3. Three sibling cards state one norm in three wordings, one of which (Go's) is
   name-free and therefore rename-proof. If the family ever converges these
   cards, the Go phrasing is the one that survives contact with a rename.

---

# Tally and verification

## The ten, by outcome

| id | package | anchors | outcome |
|---|---|---:|---|
| `F-191` | rust-ai-native-lang | 3 | DEMOTED |
| `F-192` | rust-ai-native-lang | 3 | DEMOTED (2 written as partials) |
| `F-214` | rust-ai-native-lang | 2 | DEMOTED — perimeter catch: the two live-chain tests DO exist |
| `F-277` | rust-ai-native-lang | 1 | **CORRECTED** — proc-macro ships as `crates/vendor/core-ai-native-specmark` |
| `F-278` | rust-ai-native-lang | 1 | DEMOTED — plus a host PROP-026/PROP-028 contradiction, recorded |
| `F-160` | go-ai-native-lang | 5 | DEMOTED ×4 + 1 prose-only — 2 perimeter catches |
| `F-184` | go-ai-native-lang | 3 | **MIXED**: 1 DEMOTED, 1 **RE-JUDGE: confirmed**, 1 **CORRECTED** |
| `F-271` | go-ai-native-lang | 1 | DEMOTED |
| `F-168` | typescript-ai-native-lang | 4 | DEMOTED ×3 + 1 prose-only |
| `F-282` | typescript-ai-native-lang | 1 | **CORRECTED** — binary renamed, not deleted |

**24 anchors, all 24 closed. Route check: all ten `build-or-demote`,
`release_event: false` — none out of route, none touched that belonged to
`release` or `sync-from-code`.**

**Four of the twenty-four claimed absences did not survive re-verification** —
`F-184`'s exhaustive linter (installed, floor green), `F-184`'s census (a live
three-stage producer), `F-277`'s proc-macro (renamed) and `F-282`'s binary
(renamed) — plus two partial catches inside demotions: `F-214`'s live-chain
tests and `F-160`'s cold-init warning both exist. **Six of twenty-four, a
quarter, would have been demoted wrongly on the opening verdicts alone.**

## Anchors deliberately NOT touched, and why

| anchor | owner | why left |
|---|---|---|
| `TCG-PROTOCOL-{RUST,GO}#OP-INIT` | F-211 `release` | publication, §4.5 |
| `…tools/vibe-agentic-tcg-*.md#COMPONENT-THE-PRODUCT-SEAM` | F-189 `release` | publication, three-package family |
| `TCG-ORACLE-GO#GRACEFUL-EXIT-IS-THE-LSP-DANCE` | F-167 `sync-from-code` | owner route |
| `TCG-PROTOCOL-GO#ONE-PRODUCT-CLIENT-DRIVES-ALL-THREE-RELAYS` | F-210 `sync-from-code` | owner route |
| `GUIDE-AI-NATIVE-TYPESCRIPT#RULE-FLAGS-READ-AT-THE-ROOT-AND-DISPATCHED` | F-161 `sync-from-code` | owner route — sits two lines below an anchor I did edit |
| `GUIDE-AI-NATIVE-GO#TWO-TIERS-NEVER-CONFUSED` | F-166 `sync-from-code` | owner route |
| `GUIDE-AI-NATIVE-GO#FLOOR-REMAINS-THE-TRUTH`… `vibe-agentic-tcg-go.md#FLOOR-REMAINS-THE-TRUTH` | F-273 `sync-from-code` | owner route |
| `GUIDE-AI-NATIVE-RUST#FLAG-REGISTRY-IS-DATA-WITH-PROVENANCE`, `#TWO-TIERS-OF-FLAGS` | **no obligation** | twins of F-168, mine, unjudged |
| `GUIDE-AI-NATIVE-{GO,TYPESCRIPT}#CONTRACT-FIRST-ORDERING` | **no obligation** | twins of F-191, mine, unjudged |
| `scaffold-d-differential-oracle.md#RISK-SUNSET` (Rust) | **no obligation** | twin of F-282, mine, worse defect (`vibe-tcg` deleted) |
| `TCG-ORACLE-GO#STDOUT-CARRIES-LSP-FRAMES-ONLY`, `TCG-ORACLE-v0.1.md#STDOUT-CARRIES-PROTOCOL-FRAMES-ONLY` | **no obligation** | twins of F-192, mine, unjudged |
| everything in `core-ai-native`, `spec/common/**`, `spec/modules/**` | another package / the host | outside my three directories |

**The standing rule I applied to every unjudged twin:** §3.1 closes an obligation
by editing *and re-judging every anchor in its list*. An anchor no verdict covers
cannot be re-judged, so moving its marker would put the document and the verdict
cache out of step in the direction the registry cannot see. Every such twin is
named in its entry's «New obligations noticed» instead. **This leaves the
parallel corpus knowingly half-demoted in three places** (F-191's, F-168's and
F-282's twins) — that is a deliberate, recorded choice for the boss to rule on,
not an oversight.

## Verification, with real exit codes

Ten files changed, all inside the three package directories in scope:

```
$ git diff --stat -- packages/org.vibevm.ai-native/{rust-ai-native-lang,go-ai-native-lang,typescript-ai-native-lang}
 go-ai-native-lang/v0.1.0/spec/go/GUIDE-AI-NATIVE-GO.md                    | 18 +++-
 go-ai-native-lang/v0.1.0/spec/go/mechanisms/TCG-ORACLE-GO-v0.1.md         | 46 ++++++++++--
 go-ai-native-lang/v0.1.0/spec/go/tools/vibe-agentic-tcg-go.md             | 12 ++-
 rust-ai-native-lang/v0.7.0/README.md                                      |  4 +-
 rust-ai-native-lang/v0.7.0/spec/rust/GUIDE-AI-NATIVE-RUST.md              |  6 +--
 rust-ai-native-lang/v0.7.0/spec/rust/mechanisms/TCG-ORACLE-RUST-v0.1.md   | 26 +++++--
 rust-ai-native-lang/v0.7.0/spec/rust/mechanisms/TCG-PROTOCOL-RUST-v0.1.md | 24 ++++++-
 rust-ai-native-lang/v0.7.0/spec/rust/tools/rust-ai-native-tcg.md          |  2 +-
 typescript-ai-native-lang/v0.6.0/spec/cards/scaffold-d-differential-oracle.md | 2 +-
 typescript-ai-native-lang/v0.6.0/spec/typescript/GUIDE-AI-NATIVE-TYPESCRIPT.md | 8 ++--
 10 files changed, 125 insertions(+), 23 deletions(-)
```

*(The same working tree also carries edits under `core-ai-native/v0.8.0/` from a
concurrent worker — they cite F-088 and are not mine.)*

**`RULE-ANCHORS-IMMUTABLE` — no fact id was added, removed or renamed.** Checked
by comparing the anchor ids that open a line or a list item on each side of the
diff:

```
line-start anchor DECLARATIONS on added lines: 10 — every one also present on a removed line
SPURIOUS new declarations: NONE
LOST declarations:         NONE
```

The eight ids that appear only on added lines — `##D13-SUPERSEDED`,
`##D13-LANGUAGE-LEADS`, `##D13-NEUTRAL-OUTSIDE`, `##FINDING-R2C-005`,
`##TCG-CRATE-DELETED`, `##TOPOLOGY-RETIRED`, `##TARGET-COLD-INIT`,
`##RISK-COLD-INIT-ON-LARGE-WORKSPACES` — are **citations inside prose**, never at
a line's or list item's first token, which is the only position the scanner mints
a unit from (`core-ai-native-specmap/src/mdspec.rs:151-152, 187-188`).

**Marker arithmetic, and it reconciles exactly:**

```
removed lines carried  @impl/done x19   @spec/done x3
added   lines carry    @impl/done x1    @spec/done x21
NET  @impl/done: -18     NET @spec/done: +18
```

18 demotions (F-191 ×3, F-192 ×3, F-214 ×2, F-278 ×1, F-160 ×4, F-184 ×1,
F-271 ×1, F-168 ×3). The 19th removed `@impl/done` and the 1 added are the same
line — F-184's census roster, **corrected while keeping its marker**. The 3
removed / 3 of the 21 added `@spec/done` are the anchors that were already honest
and took prose only (F-160's `DIAGNOSTICS-CHANNEL-HISTORY`, F-168's
`ADVANTAGE-1-TCD-EXISTS-FOR-TYPESCRIPT`, F-282's `RISK-SUNSET`). F-277's marker
line is unchanged and so does not appear in the diff at all.

**The campaign's own gate, green:**

```
$ ./target/debug/vibe.exe progress check --campaign campaigns/packages-2026-09 --no-cache
progress check: clean (259 files, 0 warning(s))
EXIT=0
```

**No `git` command that writes was run; no `init` was run anywhere.** The two
tools executed against a consumer tree — `go-ai-native floor` and
`go-ai-native health` on `research/go-demo` — left it clean
(`git status --porcelain -- research/` → empty).

## What Phase E inherits from this batch

Not a list of edits — a list of builds these demotions now name honestly:

1. **One Go corpus + one Go bench baseline on `research/go-demo`** re-judges
   **five** anchors across two documents (F-160 ×4, F-271 ×1). Cheapest item
   here by a wide margin.
2. **Land the resolved path in the `init` result** — two lines, two stacks
   (F-192, F-160). The path is already resolved in both bridges; only the
   emission is missing. §3.3's «Revisit when: the mechanism is a two-line fix»
   applies to this one if anyone wants it closed sooner.
3. **A lifecycle assertion for the oracle child** (exit code / surviving pid) —
   absent in all three stacks (F-192), and the host's own PROP-026
   `##REG-KILL-ON-DROP` claims it too.
4. **R-021 and R-001**: two rule ids cited across the corpus with no ATLAS
   entry (F-191, F-168). Either author them or stop citing them.
5. **`DR1-014`**: one dead evidence id in two packages (F-168).
6. **A forbidden-idiom scan, an intra-item ordering check, and a boundary-decode
   check** — three rules now marked `@spec/done` because nothing enforces them
   (F-191 ×2, F-184 ×1).
