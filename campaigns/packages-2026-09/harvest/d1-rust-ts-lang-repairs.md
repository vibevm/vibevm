# D1 — repairs in `rust-ai-native-lang` v0.7.0 and `typescript-ai-native-lang` v0.6.0

_Eight obligations, all `falsifier: self` — every falsifying reference sits
inside the subject package, so route (a) of
[PHASE-D-BATCH-PLAN §3.6](../PHASE-D-BATCH-PLAN.md#which-side) applies without
judgement: the package is wrong about itself and the package yields._

Marker vocabulary in play is binary in both packages — `@impl/done` and
`@spec/done` only (`grep -rhoE "@[a-z]+/[a-z-]+" spec/ | sort | uniq -c`:
rust 398/130, typescript 431/146), so a `missing-support` demotion per §3.3 is
`@impl/done` → `@spec/done` and nothing finer is available.

**No `##ANCHOR` id was added, removed or renamed by any entry below.** The
addressable set of every touched file is unchanged; no re-mirror is owed.

## F-192 — the `init` result carries no path, the no-zombie property no test, and stderr no reader

**Outcome:** EDITED
**Files touched:** `C:\Users\olegc\git\v\vibevm\packages\org.vibevm.ai-native\rust-ai-native-lang\v0.7.0\spec\rust\mechanisms\TCG-ORACLE-RUST-v0.1.md` (three anchors)

**Re-verification:** all three sub-reasons hold; each was checked separately.

*Anchor 1 — `#RESOLVED-PATH-AND-VERSION-LAND-IN-INIT`.* Perimeter: every file
under the v0.7.0 slot.

```
$ cd packages/org.vibevm.ai-native/rust-ai-native-lang/v0.7.0
$ grep -rn "ra_path\|ra_version" --include=*.rs --include=*.md --include=*.toml .
./crates/rust-ai-native-tcg/src/serve.rs:78:        "ra_version": oracle.capabilities().server_version,
./spec/rust/mechanisms/TCG-PROTOCOL-RUST-v0.1.md:57:- ##OP-INIT **`init`** `{root}` → `{ra_version, ra_path, toolchain, root_files,
```

`ra_path` exists nowhere in code. The whole `init` result is
`crates/rust-ai-native-tcg/src/serve.rs:76-85` — `ra_version`,
`position_encoding`, `pull_diagnostics`, `quiescent` and nothing else — and the
struct it is built from carries no path field either
(`crates/rust-ai-native-tcg-bridge/src/client.rs:36-40`: `position_encoding`,
`pull_diagnostics`, `server_version`). Reason holds.

*Anchor 2 — `#GRACEFUL-EXIT-AND-THE-NO-ZOMBIE-PROPERTY`.* Perimeter: every file
under the v0.7.0 slot, not just tests.

```
$ grep -rniE "surviving[- _]?pid|process[- _]?table|tasklist" . | wc -l
1
$ grep -rniE "no-zombie|surviving pid" spec/
spec/rust/mechanisms/TCG-ORACLE-RUST-v0.1.md:194,196,197   (the fact itself)
spec/rust/tools/vibe-agentic-tcg-rust.md:205               (the fact of F-281)
```

The single match is the spec sentence under judgement. The live test's last
line is `oracle.shutdown().expect("shutdown");`
(`crates/rust-ai-native-tcg-bridge/tests/live_oracle.rs:116`) — it asserts the
call returned, nothing about the process afterwards. Reason holds.

*Anchor 3 — `#STDOUT-CARRIES-LSP-FRAMES-ONLY`.* The reason holds **but names the
cause exactly right**, and the anchor's own conclusion survives it:

```
$ grep -rn "stderr" crates/rust-ai-native-tcg-bridge/ --include=*.rs
crates/rust-ai-native-tcg-bridge/src/client.rs:303:            .stderr(std::process::Stdio::null())
crates/rust-ai-native-tcg-bridge/src/lib.rs:60:         registry respawns once; run the op one-shot to see stderr"
crates/rust-ai-native-tcg-bridge/src/lib.rs:163:        .stderr(std::process::Stdio::null())
$ grep -rniE "debug_log|tracing::|log::|env_logger|RUST_LOG" crates/rust-ai-native-tcg-bridge/ crates/rust-ai-native-tcg/ --include=*.rs
(no output)
```

Both spawn sites null the child's stderr; the reader thread
(`client.rs:320-336`) reads stdout only, and there is no logging facility in
either crate for anything to be "surfaced" into.

**What changed and why:** three anchors, three different repairs, all inside
the existing ids. (1) The version half was kept and named, the path half stated
plainly as unbuilt — *«The resolved path does NOT — no field carries it; it is
specified here and unbuilt»* — and the marker demoted **`@impl/done` →
`@spec/done`** per §3.3. (2) The LSP shutdown/exit dance and kill-on-drop are
real, so they keep their claim; *«test-asserted»* became *«spike-proven … and
NOT test-asserted»*, naming what the live test does assert, and the marker
demoted **`@impl/done` → `@spec/done`**. The `spike-proven` attribution was
kept because the Phase-0 spike is a cited artefact elsewhere in this same file
(`spec/rust/mechanisms/TCG-ORACLE-RUST-v0.1.md:80,119,215`) and the verdict
falsified the *test* claim, not the spike. (3) The mechanism was corrected from
*«drained and discarded by the reader (surfaced only in bridge debug logging)»*
to *«discarded at the pipe (the child is spawned with stderr null …)»*; the
marker **stays `@impl/done`** because the anchor's own subject — stdout carries
frames only, streams stay clean — is implemented, and the only absence was in
the mechanism sentence, which is now gone. Nothing was built and no rule was
weakened: the `init` result *should* carry the path, and the document now says
so as a specification rather than as a fact.

**Twin in another stack:** all three have twins in `go-ai-native-lang`, **not
touched** —
`packages/org.vibevm.ai-native/go-ai-native-lang/v0.1.0/spec/go/mechanisms/TCG-ORACLE-GO-v0.1.md:46`
(`#INIT-RESULT-CARRIES-PATH-AND-VERSION`, the same sentence verbatim), `:203`
(no-zombie, shorter wording), `:210` (gopls stderr chatter, same shape). The
Phase C reasons say the Go twin fails identically in two of the three cases.
There is no typescript twin for anchors 1 and 3; the typescript no-zombie
sentence
(`packages/org.vibevm.ai-native/typescript-ai-native-lang/v0.6.0/spec/typescript/mechanisms/TCG-ORACLE-v0.1.md:138-140`)
is worded differently and carries **no obligation** in the registry — see the
next block.

**New obligations noticed:**

- `packages/org.vibevm.ai-native/typescript-ai-native-lang/v0.6.0/spec/typescript/mechanisms/TCG-ORACLE-v0.1.md:138-140`
  makes the same no-zombie claim for the TS stack (*«no surviving pid on this
  box»*, `@impl/done`) and carries no obligation row. If it fails the same way
  the rust and go ones do, Phase C missed it. Not checked here — outside my
  eight, and not fixed.
- `packages/org.vibevm.ai-native/rust-ai-native-lang/v0.7.0/spec/rust/mechanisms/TCG-PROTOCOL-RUST-v0.1.md:57`
  (`#OP-INIT`) still advertises `ra_path` in the wire schema. That is **F-211**
  (`release` route, spans rust + go) and is already owned; recorded here only
  so the boss can see the two must land consistently — F-192's demotion and
  F-211's repair describe the same missing field.

## F-275 — the card said its conform rule was unimplemented; one of its two rules ships

**Outcome:** EDITED
**Files touched:** `C:\Users\olegc\git\v\vibevm\packages\org.vibevm.ai-native\rust-ai-native-lang\v0.7.0\spec\cards\scaffold-d-differential-oracle.md` (one anchor, `#card-is-beta`)

**Re-verification:** the reason holds, on both halves.

*A conform rule for this card IS shipped.* `CellHasOracle` is pushed into the
Rust gate's rule set at
`crates/rust-ai-native-conform/src/lib.rs:77` (`out.push(Box::new(rules::CellHasOracle));`),
and its definition names this very card:

```
$ sed -n '148,175p' crates/vendor/core-ai-native-conform/src/rules/structure.rs
/// Class D — `cell-has-oracle`: every `#[cell]`-manifested type
/// is referenced from at least one integration-test file of its
/// crate — the differential / characterization oracle the
/// replacement protocol requires (card scaffold-d, R-040).
…
    fn id(&self) -> &'static str { "cell-has-oracle" }
```

The registry row agrees with the code and not with the fact:
`spec/cards/INDEX.md:13` — *«shipped: oracle-presence (`cell-has-oracle`,
rust-ai-native-conform); the oracle itself stays authored»*.

*The rule the CHECKER row names is genuinely absent.* Perimeter: every file
under the v0.7.0 slot.

```
$ grep -rn "replacement-has-oracle\|replacement_has_oracle\|ReplacementHasOracle" .
./spec/cards/scaffold-d-differential-oracle.md:78:##CHECKER **Checker:** conform T-sem rule `replacement-has-oracle` …
```

One hit, and it is the CHECKER row itself — so that row is right and
`#card-is-beta` was wrong to generalise it to *«the conform rule»*.

**What changed and why:** the parenthetical justification was replaced with the
two-part truth — oracle-presence ships as `cell-has-oracle`, the
`replacement-has-oracle` rule the CHECKER names does not — so the card, its own
CHECKER row and its own registry row now say the same thing. The BETA verdict
itself is left standing: it is what `spec/cards/INDEX.md:2` and the CHECKER row
both conclude, and nothing in the evidence falsifies it. **Marker unchanged at
`@impl/done`** — this is a `contradiction`, not a `missing-support`, so §3.3's
demotion does not apply and the evidence falsifies only the justification.

**Twin in another stack:** the sentence has near-twins in **both** siblings, and
**neither is a defect, so neither was touched**.
`packages/org.vibevm.ai-native/go-ai-native-lang/v0.1.0/spec/cards/scaffold-d-differential-oracle.md:12`
and
`packages/org.vibevm.ai-native/typescript-ai-native-lang/v0.6.0/spec/cards/scaffold-d-differential-oracle.md:11`
carry the same sentence, but their registry rows agree with it — go's
`spec/cards/INDEX.md:19` reads *«specified (pilot: `research/go-demo` fuzz
differential)»* and typescript's `spec/cards/INDEX.md:13` reads *«specified
(pilot)»*. Only the Rust row says `shipped`, so only the Rust card
self-contradicts. The fix is therefore correctly stack-local and does **not**
leave a sibling out of step.

**New obligations noticed:** none beyond the eight.

## F-277 — the README pointed the specmark proc-macro at a path this package has never had

**Outcome:** EDITED
**Files touched:** `C:\Users\olegc\git\v\vibevm\packages\org.vibevm.ai-native\rust-ai-native-lang\v0.7.0\README.md` (one anchor, `#SHIPS-SPECMARK-PROC-MACRO`)

**Re-verification:** the reason holds exactly as written — **and it settles the
type, which the registry has wrong.** Both halves checked.

*`crates/specmark` is absent from this package.* Perimeter: the v0.7.0 slot's
whole crate tree.

```
$ ls crates/
rust-ai-native-cli  rust-ai-native-conform  rust-ai-native-conform-frontend
rust-ai-native-env-audit  rust-ai-native-specmap  rust-ai-native-tcg
rust-ai-native-tcg-bridge  vendor
$ ls crates/vendor/
core-ai-native-conform  core-ai-native-specmap
core-ai-native-specmark  core-ai-native-specmark-grammar
```

Widened to the whole repository, every `crates/specmark` string is a
`.vibe/cache/` copy of a superseded `core-ai-native` v0.6.0 / `discipline-core`
slot — the *upstream* package's own old layout, never this package's.

*The proc-macro itself is shipped and wired.* `crates/vendor/core-ai-native-specmark/Cargo.toml:2,14`
declares `name = "core-ai-native-specmark"` with `proc-macro = true`; its
`src/lib.rs:78` and `:112` export `pub fn spec` and `pub fn scope`; the
workspace lists it as a member (`Cargo.toml:21`) and aliases it
(`Cargo.toml:46`: `specmark = { package = "core-ai-native-specmark", path = "crates/vendor/core-ai-native-specmark" }`).

**What changed and why:** the path was corrected to
`crates/vendor/core-ai-native-specmark` and the `specmark` alias named, so a
reader who types the path finds the crate. **Marker deliberately left at
`@impl/done`, and the boss should read this as a re-type of the row.** The
registry files F-277 as `missing-support` / `build-or-demote`, whose closure is
§3.3 demotion; but the evidence shows an artefact that exists at a different
address, not an artefact that is absent. The honest type is `reality-mismatch`
(§1.1: *«a path … described wrongly … the defect is a discrepancy»*), the
honest route is a prose path fix, and demoting a shipped proc-macro to
`@spec/done` would have made the README **less** true. Nothing was built.

**Twin in another stack:** «none found» for the sentence — the go and
typescript packages ship no Rust proc-macro and their READMEs carry no such
bullet (`grep -rn "specmark" packages/org.vibevm.ai-native/*/v*/README.md`
returns three lines: this one, this file's `:83` wiring pointer, and
`core-ai-native/v0.8.0/README.md:21`, which is a different sentence in a
package that is not mine and is already correct about its own layout).

**New obligations noticed:** none beyond the eight. (The re-type above is a
correction to F-277's own row, not a new finding.)

## F-280 — "replay goldens pin BOTH hops" when nothing pins the outer one and no golden exists

**Outcome:** EDITED
**Files touched:** `C:\Users\olegc\git\v\vibevm\packages\org.vibevm.ai-native\rust-ai-native-lang\v0.7.0\spec\rust\mechanisms\TCG-PROTOCOL-RUST-v0.1.md` (one anchor)

**Re-verification:** the reason holds, and re-verifying it surfaced a **second
false word in the same sentence** (see below).

*Nothing drives the serve loop.* Perimeter: every `.rs` under the v0.7.0 slot.

```
$ grep -rn "run_serve" . --include=*.rs
./crates/rust-ai-native-tcg/src/main.rs:102:        Cmd::Serve { root } => rust_ai_native_tcg::serve::run_serve(&resolve_root(&root))?,
./crates/rust-ai-native-tcg/src/serve.rs:215:pub fn run_serve(root: &Path) -> Result<i32> {
```

Two hits: the CLI dispatch and the definition. No test. The crate's only
integration test is `crates/rust-ai-native-tcg/tests/finding_parity.rs`, whose
own header says it gates *«the relay's per-file enrichment»* against the gate's
scan — it calls `enrich_validate`, not the loop — and its only unit module,
`crates/rust-ai-native-tcg/src/lib/tests.rs`, is headed *«Enrichment-layer
tests»* and asserts on the enriched struct, never on a frame.

*No goldens exist either.*

```
$ grep -rniE "golden|transcript" --include=*.rs crates/
crates/vendor/core-ai-native-conform/src/config.rs:110:    /// substrings is skipped (fixtures, goldens, vendored trees).
crates/vendor/core-ai-native-conform/src/store.rs:370:/// goldens/fixtures, and build output, mirroring go-extract's own skip
```

Both hits are the conform engine's *skip list* prose in the vendored crate —
neither is a golden, and neither is in the TCG crates.

*The second falsehood, found by re-verification and not in the verdict.* The
inner half said *«recorded LSP transcripts»*. They are not recordings: the
`Scripted` transport is a hand-built `VecDeque` of `serde_json::Value` frames
constructed in the test body (`crates/rust-ai-native-tcg-bridge/src/client/tests.rs:18-30`,
`Scripted::new(frames: Vec<serde_json::Value>)`), reading no fixture file. The
test headers call them *«scripted transports»* (`client/tests.rs:1`,
`oracle/tests.rs:1`) and so does the repaired sentence.

**What changed and why:** the sentence now states which hop is pinned and by
what — *«Scripted LSP frames pin the INNER hop … The outer hop is NOT pinned:
no replay golden exists and no test drives `run_serve` over recorded outer
frames»* — and the marker demoted **`@impl/done` → `@spec/done`**, matching how
this corpus already marks facts describing something not yet built (e.g.
`#MARKERS-RESERVATION-MAY-BE-FILLED-IN-A-FUTURE-MINOR` at `:167`, `@spec/done`).
The requirement is untouched: the outer hop *should* be replay-pinned, and the
document still says so — as a specification now, not as a fact. The anchor id
still says `PIN-BOTH-HOPS` and was deliberately **not** renamed; renaming it
would change the file's addressable set for no gain in truth.

**Twin in another stack:**
`packages/org.vibevm.ai-native/go-ai-native-lang/v0.1.0/spec/go/mechanisms/TCG-PROTOCOL-GO-v0.1.md:158`
(`#REPLAY-GOLDENS-PIN-BOTH-HOPS`) is the same sentence under a shorter id, and
the Phase C reason says *«The Go twin fails identically»*. **Not touched** —
another worker owns that package. No typescript twin
(`grep -rn "PIN-BOTH-HOPS" packages/org.vibevm.ai-native/ --include=*.md`
returns only the go and rust lines).

**New obligations noticed:** none new; but note that the *«recorded LSP
transcripts»* inaccuracy above was inside F-280's own sentence and is repaired
with it rather than filed separately.

## F-281 — the house lesson set was claimed four-for-four; the fourth is asserted nowhere

**Outcome:** EDITED
**Files touched:** `C:\Users\olegc\git\v\vibevm\packages\org.vibevm.ai-native\rust-ai-native-lang\v0.7.0\spec\rust\tools\vibe-agentic-tcg-rust.md` (one anchor, `#RISK-WINDOWS-CHILD-LIFECYCLE`)

**Re-verification:** the reason holds. Each of the four checked one at a time.

1. *verbatim-free paths into URIs* — **real and asserted.**
   `crates/rust-ai-native-tcg-bridge/src/lib.rs:129` defines
   `pub fn verbatim_free(path: &Path) -> PathBuf` stripping `\?\`, and
   `crates/rust-ai-native-tcg-bridge/src/oracle/tests.rs:97-100` asserts
   `!uri.contains("\\?\\")` with the message *«verbatim leaked into a URI»*.
2. *kill-on-drop* — **real.** `crates/rust-ai-native-tcg-bridge/src/client.rs:346-350`,
   `impl Drop for ChildTransport { fn drop(&mut self) { let _ = self.child.kill(); … } }`.
3. *shutdown/exit dance* — **real.** `crates/rust-ai-native-tcg-bridge/src/oracle.rs:356-361`
   issues the `shutdown` request then the `exit` notification, doc-commented
   *«The graceful LSP exit dance»*.
4. *no-zombie assertions* — **absent.** Perimeter: every file under the v0.7.0
   slot, all extensions.

```
$ grep -rniE "surviving[- _]?pid|process[- _]?table|tasklist" . | wc -l
1
```

The one hit is the sibling spec sentence in `spec/rust/mechanisms/TCG-ORACLE-RUST-v0.1.md`
(F-192's anchor 2), not a check. Three of four, and the fact said four.

**What changed and why:** the parenthetical now lists the three that are built
and names the fourth as not carried — *«The fourth, no-zombie assertions, is NOT
carried: no surviving-pid check exists in this package»*. `Phase-0-proven` was
kept on the three, because the spike is a cited artefact elsewhere in this same
document (`:65`, `:73`, `:85`) and the verdict falsified the *package's*
assertions, not the spike's findings. Marker demoted **`@impl/done` →
`@spec/done`**, on the principle applied uniformly across this batch: *if the
repaired sentence still names a mechanism the package does not carry, the unit
is specified rather than implemented.* The risk is not weakened — it still says
the Windows child lifecycle needs a no-zombie assertion, and now says the
package owes one.

**Twin in another stack:**
`packages/org.vibevm.ai-native/go-ai-native-lang/v0.1.0/spec/go/tools/vibe-agentic-tcg-go.md:192`
carries the same list (*«kill-on-drop + shutdown op + no-zombie assertions»*)
and **was not touched**. The typescript sibling
`packages/org.vibevm.ai-native/typescript-ai-native-lang/v0.6.0/spec/typescript/tools/vibe-agentic-tcg-ts.md:185`
also carries it; it is **not in my eight and carries no obligation row**, so it
was left alone — flagged below rather than fixed, because fixing it would mean
judging the TS stack's own no-zombie evidence, which no verdict has done.

**New obligations noticed:** the no-zombie claim appears in **four** more places
that carry no obligation row —
`typescript-ai-native-lang/v0.6.0/spec/typescript/mechanisms/TCG-ORACLE-v0.1.md:138-140`,
`typescript-ai-native-lang/v0.6.0/spec/typescript/tools/vibe-agentic-tcg-ts.md:185`,
`go-ai-native-lang/v0.1.0/spec/go/mechanisms/TCG-ORACLE-GO-v0.1.md:203`,
`go-ai-native-lang/v0.1.0/spec/go/tools/vibe-agentic-tcg-go.md:192`. Two of the
four (the go pair) sit in a package whose twin facts Phase C did judge, so the
gap is likelier in the typescript pair. Recorded, not fixed.

## F-161 — a mandated tsconfig row nothing checks, a rule id the gate never mounts, a wrong section pointer, and one figure that is not this package's to fix

**Outcome:** EDITED (4 of 5 anchors) · **anchor `#TOOLING-ASYMMETRY-STATED-HONESTLY` deliberately NOT edited — route (b), see below**
**Files touched:** `C:\Users\olegc\git\v\vibevm\packages\org.vibevm.ai-native\typescript-ai-native-lang\v0.6.0\spec\typescript\GUIDE-AI-NATIVE-TYPESCRIPT.md` (four anchors)

**Re-verification:** each anchor separately; one reason turned out to be partly
wrong about its own evidence, and one anchor turned out not to be this
package's defect.

*`#TSCONFIG-DEFECT-CATCHERS` — reason holds, and holds harder than written.*
The verdict argued from `research/ts-demo/tsconfig.json`, a **host** file. I
re-checked inside the package instead, which is the stronger case: the package
ships exactly one `tsconfig.json`, and it sets none of the five either.

```
$ cd packages/org.vibevm.ai-native/typescript-ai-native-lang/v0.6.0
$ find . -name "tsconfig*.json" -not -path "*/node_modules/*"
./tools/ts-oracle/test/fixtures/proj/tsconfig.json
$ cat tools/ts-oracle/test/fixtures/proj/tsconfig.json
{ "compilerOptions": { "strict": true, "noUncheckedIndexedAccess": true,
  "exactOptionalPropertyTypes": true, "erasableSyntaxOnly": true,
  "verbatimModuleSyntax": true, "allowImportingTsExtensions": true,
  "noEmit": true, "module": "nodenext", "moduleResolution": "nodenext",
  "target": "es2023", "types": [] }, "include": ["src"] }
```

And nothing reads a `tsconfig.json` to check it. Perimeter: every `.rs` in the
slot's `crates/`:

```
$ grep -rniE "noUnusedLocals|noImplicitReturns|allowUnreachableCode|noFallthroughCases" crates/ --include=*.rs
(no output)
```

The seven `tsconfig` hits in that tree are a transport test fixture string and
the conform engine's own Rust struct `TsConfig`
(`crates/vendor/core-ai-native-conform/src/config.rs:157`), which is
`conform.toml` config, not `tsconfig.json`.

*`#NO-IF-FLAG-IN-DOMAIN-CELLS` and `#RULE-FLAGS-READ-AT-THE-ROOT-AND-DISPATCHED`
— reason holds, verbatim.* The TypeScript gate mounts three rules:

```
$ sed -n '48,61p' crates/typescript-ai-native-conform/src/lib.rs
pub fn build_rules(config: &Config) -> Vec<Box<dyn Rule>> {
    let mut out: Vec<Box<dyn Rule>> = Vec::new();
    out.push(Box::new(rules::TsUnsafeInDomain));
    if let Some(cells_dir) = &config.typescript.cells_dir { … TsCellIsolation … }
    out.push(Box::new(rules::FileLength { max_lines: config.max_file_lines }));
    out
}
```

`FlagSites` — whose `id()` is `"R-001"`
(`crates/vendor/core-ai-native-conform/src/rules/structure.rs:33-35`) and which
is exported from the vendored engine (`rules/mod.rs:24`) — appears in that file
nowhere. It is built and unmounted on this language.

*`#AGENTIC-BATTERY-IS-THE-FIRST-MEASUREMENT` — reason holds.*

```
$ grep -n "^## " spec/typescript/tools/vibe-agentic-tcg-ts.md
…  140:## 4. Staged ambition   158:## 5. Licensing posture   169:## 6. The honest risk register …
```

§6 is the risk register. The battery is named at `:91` (§2, "the battery
measures the weak population (gpt-oss-20b)"), `:128` (§3, `bench` as its
harness), `:144` (§4) and `:173`/`:188` (§6) — and **defined in none of them**,
which is recorded below as a new obligation rather than repaired here.

*`#TOOLING-ASYMMETRY-STATED-HONESTLY` — the reason holds but the falsifier is
NOT in this package.* Its own Phase-C text says so: "the clean-room posture the
anchor is named for holds unchanged", "the ~94 % is the corpus's own number,
quoted correctly", and on the `~74.8 %`: "The package contradicts itself across
its own two appendices; this document is faithful to one of them" — where "the
package" is `core-ai-native`, not mine. Verified:
`core-ai-native/v0.8.0/spec/appendix/CONTRADICTION-MAP.md:28,31` publishes
74.8 %, while `core-ai-native/v0.8.0/spec/appendix/ATLAS.md:106` — marked
"GENERATED from findings.jsonl (A2: derived, do not hand-edit)" at `:5` —
publishes 75.3 % (synthesis) and 70.2 % (translation) for the same finding
DR2-012. And the figure is a **corpus-wide convention**, not a local slip —
`grep -rn "74\.8" --include=*.md packages/org.vibevm.ai-native/` returns 12
sites:

```
core-ai-native            v0.7.0 CONTRADICTION-MAP.md:21,:24 · v0.8.0 :28,:31
go-ai-native-lang         spec/go/tools/go-ai-native-tcg.md:31
rust-ai-native-lang       spec/rust/tools/rust-ai-native-tcg.md:11,:65,:78
typescript-ai-native-lang GUIDE-AI-NATIVE-TYPESCRIPT.md:39,:258,:270
                          spec/typescript/tools/typescript-ai-native-tcg.md:38,:104
```

Editing one of twelve is precisely what §4.5 calls "not a closure; a new
`duplication` obligation", and one of the twelve
(`rust-ai-native-lang/.../rust-ai-native-tcg.md#RISK-TRANSFER-UNPROVEN`) is
already **F-276**, owned elsewhere. So this anchor takes **§3.6 route (b)**: the
package does not move, and the obligation belongs to `core-ai-native`, whose two
appendices disagree with each other.

**What changed and why:**

- `#TSCONFIG-DEFECT-CATCHERS` — the five flags are **kept verbatim**; the row
  now says "Mandated here and enforced nowhere yet: no shipped rule reads
  `tsconfig.json`, and the stack's own oracle fixture sets none of the five",
  marker **`@impl/done` → `@spec/done`**. The mandate is not weakened, per the
  standing rule; only the claim of implementedness is withdrawn.
- `#NO-IF-FLAG-IN-DOMAIN-CELLS` — the rule kept, the citation qualified:
  "R-001 — defined in the shared conform engine, not yet mounted on the
  TypeScript gate", marker **`@impl/done` → `@spec/done`**.
- `#RULE-FLAGS-READ-AT-THE-ROOT-AND-DISPATCHED` — the rule kept, plus
  "Unchecked today — R-001 is unmounted here, so the `deviates` clause has
  nothing to fire against", marker **`@impl/done` → `@spec/done`**.
- `#AGENTIC-BATTERY-IS-THE-FIRST-MEASUREMENT` — the address repaired in prose,
  **not** as a relative link: the brief is now named (`vibe-agentic-tcg-ts`) and
  the sections that actually mention the battery given (§2 and §4) in place of
  the bare, wrong `§6`. Marker **unchanged at `@impl/done`** — the evidence
  falsified only the pointer.
- `#TOOLING-ASYMMETRY-STATED-HONESTLY` — **no edit, by decision.** Changing
  `~74.8 %` here would fork the figure away from eleven sibling sites and from
  `core-ai-native`'s own C-4 entry, which is the source of the corpus's use of
  it.

**Twin in another stack:** «none found» for the four edited sentences.
`grep -rn "TSCONFIG-DEFECT-CATCHERS\|noUnusedLocals" --include=*.md packages/org.vibevm.ai-native/`
returns only the edited line;
`grep -rn "NO-IF-FLAG-IN-DOMAIN-CELLS\|RULE-FLAGS-READ-AT-THE-ROOT" --include=*.md .`
likewise (the R-001 hits elsewhere are core-ai-native's C++/Go/Java **legacy
projections**, different sentences in a package that is not mine); and
`grep -rn "sibling brief" --include=*.md .` returns four lines, of which only
the edited one carried a section number — the others are file links inside their
own directory. The **fifth** anchor is the one with eleven twins, which is
exactly why it was left alone.

**New obligations noticed:**

1. **`core-ai-native` contradicts itself on the PLDI'25 figure.**
   `packages/org.vibevm.ai-native/core-ai-native/v0.8.0/spec/appendix/CONTRADICTION-MAP.md:28,31`
   says 74.8 %; `packages/org.vibevm.ai-native/core-ai-native/v0.8.0/spec/appendix/ATLAS.md:106`
   says 75.3 % / 70.2 % for the same finding DR2-012, and ATLAS is the derived,
   do-not-hand-edit appendix. Eleven downstream sites across four packages quote
   the 74.8 %. No obligation row covers the pair. This is a release-event-shaped
   family (§4.5) and it is **not fixed here**.
2. **`#TSCONFIG-BEYOND-STRICT` was confirmed on evidence that covers half of
   it.** `packages/org.vibevm.ai-native/typescript-ai-native-lang/v0.6.0/spec/typescript/GUIDE-AI-NATIVE-TYPESCRIPT.md:89`
   names four mandatory beyond-strict flags; the package's only `tsconfig.json`
   (`tools/ts-oracle/test/fixtures/proj/tsconfig.json`) and the host demo
   (`research/ts-demo/tsconfig.json`) each set two of the four —
   `noPropertyAccessFromIndexSignature` and `noImplicitOverride` appear in
   neither. Phase C recorded that anchor `confirmed` citing only the two that
   are set, and F-161's own reason repeats the error ("carries the four
   mandatory beyond-strict flags"). Recorded, not fixed — it is not in my eight.
3. **The two-arm battery is named four times in the sibling brief and defined
   nowhere** in the package — `grep -rn "two-arm\|two arms\|arm A\|arm B" spec/`
   returns three lines, all uses, no definition. A candidate `missing-support`
   on
   `packages/org.vibevm.ai-native/typescript-ai-native-lang/v0.6.0/spec/typescript/tools/vibe-agentic-tcg-ts.md`.
   Recorded, not fixed.

## F-282 — the sunset condition named a binary that was renamed two versions ago

**Outcome:** EDITED
**Files touched:** `C:\Users\olegc\git\v\vibevm\packages\org.vibevm.ai-native\typescript-ai-native-lang\v0.6.0\spec\cards\scaffold-d-differential-oracle.md` (one anchor, `#RISK-SUNSET`)

**Re-verification:** the reason holds. Perimeter: the whole `packages/` tree,
markdown and TOML.

```
$ grep -rn "vibe-tcg-ts" packages/ --include=*.md --include=*.toml
packages/org.vibevm.ai-native/typescript-ai-native-lang/v0.6.0/spec/cards/scaffold-d-differential-oracle.md:60:- ##RISK-SUNSET *Sunset condition:* if generation-time tools (`vibe-tcg-ts`) …
```

One hit, and it is the fact under judgement. Inside the v0.6.0 slot the same
search returns the same single line, so nothing in the package defines,
declares or builds that name. What the package does ship is declared in its own
manifest — `packages/org.vibevm.ai-native/typescript-ai-native-lang/v0.6.0/vibe.toml:45`,
`name = "typescript-ai-native-tcg"`, the 4th of four `[[binary]]` entries
(`:30` `typescript-ai-native`, `:35` `-conform`, `:40` `-specmap`, `:45`
`-tcg`). The surviving `vibe-tcg-ts` copies are in `.vibe/cache/` under the
superseded slots, exactly as the verdict said.

**What changed and why:** the stale binary name was replaced with the shipped
one and briefly identified — *«(`typescript-ai-native-tcg`, the stack's shipped
type oracle)»*. **Marker unchanged, and it was already `@spec/done`** before the
edit: the fact is a sunset *condition*, a hypothetical about the future, and it
was never claiming an implementation. **The boss should note that this row is
routed `build-or-demote` and had nothing to demote** — the closure is a
one-token rename, the type is `reality-mismatch` (a name described wrongly, §1.1)
rather than `missing-support`, and no code was written.

**Twin in another stack:** both siblings carry the same risk row and **neither
needed touching**.
`packages/org.vibevm.ai-native/go-ai-native-lang/v0.1.0/spec/cards/scaffold-d-differential-oracle.md:101`
names **no** binary at all (*«if generation-time tooling plus contracts ever
make …»*) and Phase C recorded it `confirmed`;
`packages/org.vibevm.ai-native/rust-ai-native-lang/v0.7.0/spec/cards/scaffold-d-differential-oracle.md:58`
names `vibe-tcg`, which is still a live name — it is the host-side product-seam
crate (`spec/typescript/tools/vibe-agentic-tcg-ts.md:130`, *«the `vibe-tcg`
crate — tool schemas, registry, slot dispatch»*) — and Phase C recorded that one
`confirmed` too. Only the TypeScript card carried a name that resolves nowhere.

**New obligations noticed:** none beyond the eight.

## F-284 — a posted `complete` latency target the bench harness never measures

**Outcome:** EDITED
**Files touched:** `C:\Users\olegc\git\v\vibevm\packages\org.vibevm.ai-native\typescript-ai-native-lang\v0.6.0\spec\typescript\mechanisms\TCG-ORACLE-v0.1.md` (one anchor)

**Re-verification:** the reason holds on its first half and is **wrong on its
second**, which changed the edit.

*First half — no complete-latency field, and the harness never calls
`complete`.* The report struct is exhaustive:

```
$ sed -n '42,50p' crates/typescript-ai-native-tcg/src/bench.rs
#[derive(Serialize)]
struct BenchReport {
    ts_version: String,
    cases: Vec<CaseResult>,
    agreement_pct: f64,
    cold_init_ms: f64,
    validate_p50_ms: f64,
    validate_p95_ms: f64,
}
```

and the loop that fills it only ever validates:

```
$ grep -n "complete\|validate" crates/typescript-ai-native-tcg/src/bench.rs
48: validate_p50_ms   49: validate_p95_ms
85: let _ = oracle.validate(warmup_file, None)?;      (measured as cold init)
100: let v = oracle.validate(&case.file, content.as_deref())?;
133,134: validate_p50_ms / validate_p95_ms = percentile(…)
148,152,153: the printed line
```

Zero calls to `complete`. The `validate` half of the target is measurable; the
`complete` half is not.

*Second half — «and no threshold constants» is not a defect.* The immediately
preceding fact in the same section,
`spec/typescript/mechanisms/TCG-ORACLE-v0.1.md:149-150`
(`#TARGETS-ARE-POSTED-AND-MEASURED-NEVER-GATED`, verdict **confirmed**), says
targets are *«POSTED and MEASURED, never CI-gated (timing gates on shared boxes
generate flakes, not signal)»*. A threshold constant is exactly what a design
that refuses to gate on timing must NOT have, so its absence confirms the
neighbour rather than falsifying this fact. The edit therefore repairs only the
`complete` gap and says nothing about constants.

**What changed and why:** the two target numbers are **kept unchanged** — this
is a posted target, and lowering or deleting it to match an unbuilt harness is
the forbidden move. What was added is the measurement state: *«(only the
`validate` half is measured today: the bench harness never calls `complete`, and
its report carries no complete-latency field)»*, marker **`@impl/done` →
`@spec/done`**, by the same principle used throughout this batch. Phase E
inherits a two-line build (call `complete` in the bench loop; add
`complete_p50_ms` / `complete_p95_ms` to `BenchReport`) if the boss wants the
target measured rather than merely posted.

**Twin in another stack:** the target exists in all three, split differently,
and **neither sibling was touched**.
`packages/org.vibevm.ai-native/rust-ai-native-lang/v0.7.0/spec/rust/mechanisms/TCG-ORACLE-RUST-v0.1.md:233`
carries the complete half under its own id `#TARGET-WARM-COMPLETE` (`complete`
p50 < 300 ms) — that anchor is **F-215**, an open obligation assigned outside my
eight, and although it sits in one of my two packages I left it alone rather
than collide with its owner. The Go sibling
(`packages/org.vibevm.ai-native/go-ai-native-lang/v0.1.0/spec/go/mechanisms/TCG-ORACLE-GO-v0.1.md:238,242`)
posts a `validate` target and a cold-init target and **no `complete` target at
all**, so it has nothing corresponding to repair.

**New obligations noticed:** none new — but the boss should notice that F-215
(rust `#TARGET-WARM-COMPLETE`) is the same defect in the sibling stack, and the
two should land with the same wording or the family goes out of step (§4.5).

## Batch verification (run after the last edit)

**Markup still parses over both slots, exit 0.** The exit gate's check 2
(`RULE-UPDATE-MARKERS` / `vibe progress check` green over all touched files):

```
$ ./target/release/vibe.exe progress check --path packages/org.vibevm.ai-native/rust-ai-native-lang/v0.7.0 --no-cache
progress check: clean (18 files, 0 warning(s))
EXIT=0
$ ./target/release/vibe.exe progress check --path packages/org.vibevm.ai-native/typescript-ai-native-lang/v0.6.0 --no-cache
progress check: clean (19 files, 0 warning(s))
EXIT=0
```

The 18 observed rust files are exactly the 18 markdown files in that slot that
carry anchors (`grep -rl "^##[A-Za-z]\|^- ##[A-Za-z]" --include=*.md .`), so all
five touched rust files were in the checked set; the same holds for the three
touched typescript files.

**The addressable set is unchanged.** Every anchor id touched was re-located
after the edit and found exactly once, under its original spelling: the 13
edited ids plus the one deliberately left alone, over 8 files, checked
programmatically. **No `##ANCHOR` id was added, removed or
renamed**, so no `vibe progress mirror` is owed before `merge-verdicts.py`
(§3.1's *«Revisit when»*).

**Files touched — the complete list (8):**

| file | anchors edited |
|---|---|
| `packages/org.vibevm.ai-native/rust-ai-native-lang/v0.7.0/README.md` | `SHIPS-SPECMARK-PROC-MACRO` |
| `packages/org.vibevm.ai-native/rust-ai-native-lang/v0.7.0/spec/cards/scaffold-d-differential-oracle.md` | `card-is-beta` |
| `packages/org.vibevm.ai-native/rust-ai-native-lang/v0.7.0/spec/rust/mechanisms/TCG-ORACLE-RUST-v0.1.md` | `RESOLVED-PATH-AND-VERSION-LAND-IN-INIT`, `GRACEFUL-EXIT-AND-THE-NO-ZOMBIE-PROPERTY`, `STDOUT-CARRIES-LSP-FRAMES-ONLY` |
| `packages/org.vibevm.ai-native/rust-ai-native-lang/v0.7.0/spec/rust/mechanisms/TCG-PROTOCOL-RUST-v0.1.md` | `REPLAY-GOLDENS-AND-RECORDED-TRANSCRIPTS-PIN-BOTH-HOPS` |
| `packages/org.vibevm.ai-native/rust-ai-native-lang/v0.7.0/spec/rust/tools/vibe-agentic-tcg-rust.md` | `RISK-WINDOWS-CHILD-LIFECYCLE` |
| `packages/org.vibevm.ai-native/typescript-ai-native-lang/v0.6.0/spec/cards/scaffold-d-differential-oracle.md` | `RISK-SUNSET` |
| `packages/org.vibevm.ai-native/typescript-ai-native-lang/v0.6.0/spec/typescript/GUIDE-AI-NATIVE-TYPESCRIPT.md` | `TSCONFIG-DEFECT-CATCHERS`, `NO-IF-FLAG-IN-DOMAIN-CELLS`, `RULE-FLAGS-READ-AT-THE-ROOT-AND-DISPATCHED`, `AGENTIC-BATTERY-IS-THE-FIRST-MEASUREMENT` |
| `packages/org.vibevm.ai-native/typescript-ai-native-lang/v0.6.0/spec/typescript/mechanisms/TCG-ORACLE-v0.1.md` | `TARGET-WARM-VALIDATE-AND-COMPLETE` |

Nothing outside these two package slots was modified. No `git` command was run.

**Marker changes, all in one direction (8 demotions, 0 promotions):**
`@impl/done` → `@spec/done` on `RESOLVED-PATH-AND-VERSION-LAND-IN-INIT`,
`GRACEFUL-EXIT-AND-THE-NO-ZOMBIE-PROPERTY`,
`REPLAY-GOLDENS-AND-RECORDED-TRANSCRIPTS-PIN-BOTH-HOPS`,
`RISK-WINDOWS-CHILD-LIFECYCLE`, `TSCONFIG-DEFECT-CATCHERS`,
`NO-IF-FLAG-IN-DOMAIN-CELLS`, `RULE-FLAGS-READ-AT-THE-ROOT-AND-DISPATCHED`,
`TARGET-WARM-VALIDATE-AND-COMPLETE` (8 anchors). Unchanged at `@impl/done`
where the repair removed the unbuilt claim rather than the built one:
`STDOUT-CARRIES-LSP-FRAMES-ONLY`, `card-is-beta`, `SHIPS-SPECMARK-PROC-MACRO`,
`AGENTIC-BATTERY-IS-THE-FIRST-MEASUREMENT`. Unchanged at `@spec/done`:
`RISK-SUNSET` (already demoted before this batch).

**The principle applied uniformly, stated so the boss can overrule it once
rather than eight times:** *after the repair, if the sentence still names a
mechanism the package does not carry, the unit is `@spec/done`; if the repair
removed the unbuilt claim and everything remaining is built, the marker stays
`@impl/done`.* Both packages' vocabulary is binary, so no finer marker exists.

**Two rows should be re-typed in the registry**, on evidence rather than
preference: **F-277** and **F-282** are both filed `missing-support` /
`build-or-demote`, and both turned out to be a *name or path described wrongly*
— the artefact ships, at another address. Their closure is a prose rename, not a
demotion; F-282's marker was already `@spec/done`, so `build-or-demote` had
nothing to demote at all.

**One anchor inside F-161 was deliberately not edited** —
`#TOOLING-ASYMMETRY-STATED-HONESTLY` — because its falsifier is
`core-ai-native`'s two appendices disagreeing with each other and the figure is
quoted at twelve sites across four packages. §3.6 route (b) and §4.5 both point
the same way: the package does not move, and the obligation is recorded against
the package that owns the contradiction.
