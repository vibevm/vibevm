# Phase D — repairs prepared in `go-ai-native-lang/v0.1.0`

_Ten obligations: F-153 · F-160 · F-167 · F-190 · F-211 · F-212 · F-270 · F-271
· F-272 · F-273. All ten carry `falsifier: self` — every falsifying reference
sits inside the package or its own install slot, so §3.6 route **(a)** applies
and the package yields._

**Perimeter of every edit:**
`packages/org.vibevm.ai-native/go-ai-native-lang/v0.1.0/`. Nothing outside it
was touched. The rust and typescript twins named below are **left alone by
design** — another worker owns them — and are recorded because a family fix
landing in one member and not its siblings is a new `duplication` obligation,
not a closure ([PHASE-D-BATCH-PLAN.md §4.5](../PHASE-D-BATCH-PLAN.md#release)).

**No `git` was run.** No `init` command was run anywhere.

---

## F-153 — the boot snippet cites `go/…` and `cards/…`; both live under `spec/`

**Outcome:** EDITED
**Files touched:**
`packages/org.vibevm.ai-native/go-ai-native-lang/v0.1.0/spec/boot/20-stack-go-ai-native-lang.xml`

**Re-verification:**

The two cited paths do not resolve from the package root:

```console
$ cd packages/org.vibevm.ai-native/go-ai-native-lang/v0.1.0
$ ls go/GUIDE-AI-NATIVE-GO.md cards/INDEX.md
ls: cannot access 'go/GUIDE-AI-NATIVE-GO.md': No such file or directory
ls: cannot access 'cards/INDEX.md': No such file or directory
$ ls spec/go/GUIDE-AI-NATIVE-GO.md spec/cards/INDEX.md
spec/cards/INDEX.md
spec/go/GUIDE-AI-NATIVE-GO.md
```

…and they do not resolve from the **install slot** either, which is where the
reader actually stands. The boot lane reads the snippet in place inside the
slot:

```console
$ grep -n "stack-rust-ai-native-lang\|20-stack" spec/boot/INDEX.md
22:path = "vibedeps/stack-rust-ai-native-lang/0.7.0/spec/boot/20-stack-rust-ai-native-lang.md"
26:path = "vibedeps/stack-typescript-ai-native-lang/0.6.0/spec/boot/20-stack-typescript-ai-native-lang.md"
```

and the slot has the same `spec/`-rooted shape as the package (rust is the only
lang stack installed in this tree, so it is the one that can be measured):

```console
$ ls vibedeps/stack-rust-ai-native-lang/0.7.0/
Cargo.lock  Cargo.toml  LICENSE.xml  README.md  crates  spec  specmap.toml  vibe.toml
$ ls -d vibedeps/stack-rust-ai-native-lang/0.7.0/rust vibedeps/stack-rust-ai-native-lang/0.7.0/cards
ls: cannot access 'vibedeps/stack-rust-ai-native-lang/0.7.0/rust': No such file or directory
ls: cannot access 'vibedeps/stack-rust-ai-native-lang/0.7.0/cards': No such file or directory
```

So the bare `go/…` / `cards/…` address is wrong from the package root, wrong
from the slot root, and wrong relative to the snippet's own directory
(`spec/boot/go/…` does not exist). The reason holds, and holds for the cause it
states.

**What changed and why:** two address repairs, `spec/` prefixed onto both cited
paths — `go/GUIDE-AI-NATIVE-GO.md` → `spec/go/GUIDE-AI-NATIVE-GO.md` (line 6)
and `cards/INDEX.md` → `spec/cards/INDEX.md` (line 12). The qualifier «in this
package» is kept, so the address is read against the package/slot root, which is
the one root that exists in both the dev tree and the consumer's `vibedeps/`.
This is a `relocation`: the content was never in question, only where it lives,
so no marker moved — both facts stay `@impl/done`. No markdown link was
introduced; the paths stay backticked prose, per the rule against
package-relative links that resolve only in this dev tree.

**Twin in another stack:** yes, and **not edited** —
`packages/org.vibevm.ai-native/rust-ai-native-lang/v0.7.0/spec/boot/20-stack-rust-ai-native-lang.xml`
lines 6 and 12 (`rust/GUIDE-AI-NATIVE-RUST.md`, `cards/INDEX.md`) and
`packages/org.vibevm.ai-native/typescript-ai-native-lang/v0.6.0/spec/boot/20-stack-typescript-ai-native-lang.xml`
lines 7 and 13 (`typescript/GUIDE-AI-NATIVE-TYPESCRIPT.md`, `cards/INDEX.md`).
F-153's own anchor list names all six. This obligation is a **release event**
across three packages: the go-side edit alone is not the closure.

**New obligations noticed:** in the same file,
`##STACK-SHIPS-ITS-OWN-CARDS-PROJECTION` (line 15-17) says the stack ships «its
own `cards/` projection» — the same bare directory address, one anchor away from
F-153's pair and not in its anchor list. It reads more as a name than as an
address, which is presumably why the verdict did not take it, but if F-153's
repair is the rule then this line is inconsistent with it. Recorded, not fixed.

---

## F-160 — five facts promise a Go corpus, a ledger entry, a path key and a warning that do not exist

**Outcome:** EDITED (demotion per [§3.3](../PHASE-D-BATCH-PLAN.md#demote) — no code written)
**Files touched:**
`packages/org.vibevm.ai-native/go-ai-native-lang/v0.1.0/spec/go/mechanisms/TCG-ORACLE-GO-v0.1.xml`

**Re-verification:** all five reasons hold, each for the cause stated.

`#INIT-RESULT-CARRIES-PATH-AND-VERSION` — the shipped result has four keys and
none of them is a path:

```console
$ sed -n '74,84p' crates/go-ai-native-tcg/src/serve.rs
fn init_result(oracle: &GoOracle<ChildTransport>) -> serde_json::Value {
    serde_json::json!({
        "gopls_version": oracle.capabilities().server_version,
        "position_encoding": match oracle.capabilities().position_encoding {
            ... => "utf-8", ... => "utf-16",
        },
        "pull_diagnostics": oracle.capabilities().pull_diagnostics,
        "ready": oracle.ready(),
    })
}
```

`#DIAGNOSTICS-CHANNEL-HISTORY` and `#DIFFERENTIAL-CORPUS-PINS-DIAGNOSTIC-CLASSES`
— there is no Go corpus. Perimeter: the whole repository.

```console
$ ls research/tcg-bench/
RUNBOOK.md  corpus  corpus-rust  reports  research  run-battery.sh  tasks  work
$ find . -iname "*corpus*" -print | grep -v "\.git/"
./campaigns/packages-2026-09/run/state/corpus.json
./campaigns/progress-2026-08/run/state/corpus.json
./research/tcg-bench/corpus
./research/tcg-bench/corpus-rust
```

`corpus` is the TypeScript one and `corpus-rust` the Rust one; there is no
`corpus-go` and no Go case directory anywhere under that perimeter.

`#BENCH-HARNESS-RECORDS-DISTRIBUTIONS` — the harness half holds, the ledger half
does not:

```console
$ ls research/tcg-bench/reports/
REPORT-2026-07-07-control.md  REPORT-2026-07-07-rust-baseline.md
REPORT-2026-07-07-with-tools.md  bench-2026-07-07-baseline.json
bench-rust-2026-07-07-baseline.json  control-2026-07-07-0634.jsonl
with-tools-2026-07-07-0701.jsonl
```

Two baselines, TypeScript and Rust. No Go run.

`#LARGE-WORKSPACE-COLD-INIT-WARNING` — no 60 s ceiling is emitted or documented.
Perimeter: the go stack's `crates/`, then `spec/modules/vibe-mcp/`, then
`spec/` + `packages/` repo-wide.

```console
$ grep -rn "60" <go-stack>/crates/ --include=*.rs | grep -i "sec|ceiling|warn|first"
crates/go-ai-native-cli/src/health.rs:7://! Sections: file-length early warning (the `[540, 600)` danger band),
crates/go-ai-native-cli/src/main.rs:95:  /// Per-cell first-signal budget, seconds (card default: 60).
crates/go-ai-native-tcg-bridge/tests/live_oracle.rs:31:  GoOracle::spawn(root, Duration::from_secs(60))...
$ grep -rn "60 s|60s|60 seconds" spec/modules/vibe-mcp/ ; echo "grep exit=$?"
grep exit=1
$ grep -rn "first-request ceiling" spec/ packages/
packages/.../go-ai-native-lang/v0.1.0/spec/go/mechanisms/TCG-ORACLE-GO-v0.1.md:235
packages/.../rust-ai-native-lang/v0.7.0/spec/rust/mechanisms/TCG-ORACLE-RUST-v0.1.md:243
  (+5 vendored copies of the rust file under packages/org.vibevm.fractality/**)
```

None of the three `60`s is a first-request ceiling — a health file-length band,
a fast-loop per-cell budget, and the live test's own spawn timeout. **The one
bound that ships is 45 s**, and it is the readiness budget, not a product
ceiling: `crates/go-ai-native-tcg/src/lib.rs:32` —
`pub const READINESS_BUDGET: std::time::Duration = std::time::Duration::from_secs(45);`.
The eager-init half of the fact holds
(`crates/go-ai-native-tcg/src/serve.rs:232-235`).

**What changed and why:** five demotions, no code. Nothing prescriptive was
deleted — the corpus rule, the mapping table, the ledger discipline and the
warning obligation all stay on the page; each sentence now says plainly what is
built and what is not, and four markers move `@impl/done` → `@spec/done`
(`#INIT-RESULT-CARRIES-PATH-AND-VERSION`,
`#DIFFERENTIAL-CORPUS-PINS-DIAGNOSTIC-CLASSES`,
`#BENCH-HARNESS-RECORDS-DISTRIBUTIONS`, `#LARGE-WORKSPACE-COLD-INIT-WARNING`).
`#DIAGNOSTICS-CHANNEL-HISTORY` was **already** `@spec/done`, so only its prose
changed: it now says the channel is negotiated per session and carried as
`pull_diagnostics`, and that the corpus recording is specified and uncommitted.
`#LARGE-WORKSPACE-COLD-INIT-WARNING` additionally drops the invented 60 s and
names the 45 s readiness budget that actually ships.

**Note for the boss on three of the five:** `#INIT-RESULT-CARRIES-PATH-AND-VERSION`,
`#BENCH-HARNESS-RECORDS-DISTRIBUTIONS` and `#LARGE-WORKSPACE-COLD-INIT-WARNING`
are **compound** facts — one half shipped, one half not. §3.3's demotion applies
to the fact as a whole, so `@spec/done` under-states the built half. Splitting
each into two anchors would state it exactly and would cost three new anchor ids
plus a re-mirror, which the brief forbids by default. Flagged rather than taken.

**Twin in another stack:** `##BENCH-HARNESS-RECORDS-DISTRIBUTIONS` exists
verbatim as an anchor in both siblings —
`packages/org.vibevm.ai-native/rust-ai-native-lang/v0.7.0/spec/rust/mechanisms/TCG-ORACLE-RUST-v0.1.xml`
and
`packages/org.vibevm.ai-native/typescript-ai-native-lang/v0.6.0/spec/typescript/mechanisms/TCG-ORACLE-v0.1.xml`.
The 60 s sentence has a near-verbatim twin at
`…/rust-ai-native-lang/v0.7.0/spec/rust/mechanisms/TCG-ORACLE-RUST-v0.1.md:243`,
and the corpus sentence a paraphrase twin at the same file line 137
(`##DIFFERENTIAL-CORPUS-CURATES-NATIVE-COMPETENCE`). **None edited.** The other
three anchors are Go-only — `grep -rl "##<anchor>"` over both sibling packages
returns nothing for `INIT-RESULT-CARRIES-PATH-AND-VERSION`,
`DIAGNOSTICS-CHANNEL-HISTORY` and `LARGE-WORKSPACE-COLD-INIT-WARNING`.

**New obligations noticed:** the rust twin at
`…/TCG-ORACLE-RUST-v0.1.xml:243` carries the same unbacked 60 s ceiling and is
**not** in F-160's anchor list — so the Rust copy of this defect appears
unclaimed by any obligation I was given. Recorded, not fixed.

---

## F-167 — four facts assert measurement, a resolution order and a test assertion the code does not carry

**Outcome:** EDITED
**Files touched:**
`packages/org.vibevm.ai-native/go-ai-native-lang/v0.1.0/spec/go/mechanisms/TCG-ORACLE-GO-v0.1.xml`

**Re-verification:** all four hold; one holds for a *sharper* cause than stated.

`#RESOLUTION-GOPLS-ON-PATH` — the reason says the env override is tried before
PATH. It is, **and the failure is hard**: a non-file override returns before
PATH is ever probed.

```console
$ sed -n '144,158p' crates/go-ai-native-tcg-bridge/src/lib.rs
pub fn resolve_gopls(root: &Path) -> Result<PathBuf, TcgBridgeError> {
    if let Ok(overridden) = std::env::var(GOPLS_ENV_OVERRIDE) {
        let p = PathBuf::from(&overridden);
        if p.is_file() { return Ok(verbatim_free(&p)); }
        return Err(TcgBridgeError::GoplsMissing {
            detail: format!("{GOPLS_ENV_OVERRIDE}={overridden} is not a file"),
        });
    }
    // PATH: probe the bare name.
$ grep -n "GOPLS_ENV_OVERRIDE:" crates/go-ai-native-tcg-bridge/src/lib.rs
27:pub const GOPLS_ENV_OVERRIDE: &str = "GO_AI_NATIVE_GOPLS";
```

The code's own doc comment already names the true order («the env override, then
PATH, then `$GOBIN`, then `$(go env GOPATH)/bin`», lib.rs:140-143) — so the
implementation documents a step the mechanism spec omits.

`#GRACEFUL-EXIT-IS-THE-LSP-DANCE` — «zombie» occurs three times in the whole
package and never in an assertion. Perimeter: the entire package directory.

```console
$ grep -rn "zombie" packages/org.vibevm.ai-native/go-ai-native-lang/v0.1.0/
.../crates/go-ai-native-tcg-bridge/tests/live_oracle.rs:3://! surface), hover, completion, shutdown with no zombie. Requires
.../spec/go/mechanisms/TCG-ORACLE-GO-v0.1.md:203:the backstop; the no-zombie property is test-asserted. @impl/done
.../spec/go/tools/vibe-agentic-tcg-go.md:192:  no-zombie assertions). @impl/done
```

The first is a module doc comment; the live test's last statement is
`oracle.shutdown().expect("the LSP exit dance");` and nothing inspects the
process table.

`#TARGET-COMPLETE` — the harness has no instrument for it. `CaseOutcome` carries
one timing field, and it times warm `validate` ×3:

```console
$ sed -n '49,58p' crates/go-ai-native-tcg/src/bench.rs
struct CaseOutcome { name, pass, known_gap, oracle_error, build_red,
                     conform_rules, warm_ms: Vec<u128>, detail }
$ sed -n '133,142p' crates/go-ai-native-tcg/src/bench.rs
        // Oracle: warm ×3, keep the last enriched answer.
        let mut warm_ms = Vec::new();
        for _ in 0..3 {
            let started = Instant::now();
            let raw = oracle.validate(&case.file, Some(text.clone()))...;
            warm_ms.push(started.elapsed().as_millis());
```

No `complete` call is timed, and no p50 is computed anywhere in the file.

`#QUANTITIES-ARE-CAMPAIGN-MEASURED` — same evidence as F-160 above: the reports
directory carries a TypeScript and a Rust baseline and no Go run.

**What changed and why:** `#RESOLUTION-GOPLS-ON-PATH` now names the
`GO_AI_NATIVE_GOPLS` override ahead of PATH and states that a non-file value is
a hard, recipe-carrying failure behind which PATH is not probed — folded into
the existing step 1 rather than minting a new anchor for a new step 0.
`#GRACEFUL-EXIT-IS-THE-LSP-DANCE` keeps `@impl/done` (the dance and kill-on-drop
*are* shipped) and now says the no-zombie property itself is not yet asserted.
`#TARGET-COMPLETE` and `#QUANTITIES-ARE-CAMPAIGN-MEASURED` drop `@impl/done` →
`@spec/done`: the target is **kept, not weakened** — it stays posted, and the
sentence says it has no instrument yet.

**Twin in another stack:** the target twin is
`packages/org.vibevm.ai-native/rust-ai-native-lang/v0.7.0/spec/rust/mechanisms/TCG-ORACLE-RUST-v0.1.xml:233`
(`##TARGET-WARM-COMPLETE`, `complete` p50 < 300 ms) and
`packages/org.vibevm.ai-native/typescript-ai-native-lang/v0.6.0/spec/typescript/mechanisms/TCG-ORACLE-v0.1.xml:153-154`
(`##TARGET-WARM-VALIDATE-AND-COMPLETE`). **Not edited.** The other three anchors
are Go-only.

**New obligations noticed:** `#TARGET-COLD-INIT` («cold init-to-ready < 15 s»,
same list, still `@impl/done`) has the *identical* defect as `#TARGET-COMPLETE`
— the bench harness records `warm_ms` alone, so no cold-init figure is
instrumented either. It is not in my ten and I left it untouched. Closing
`#TARGET-COMPLETE` without it leaves a visibly inconsistent pair on one list.

---

## F-270 — the spec says overlay versions never reset; the code and its own test say a close resets them

**Outcome:** EDITED
**Files touched:**
`packages/org.vibevm.ai-native/go-ai-native-lang/v0.1.0/spec/go/mechanisms/TCG-ORACLE-GO-v0.1.xml`

**Re-verification:** the reason holds exactly. `update {content: null}` removes
the document from the overlay map, and `open_or_update` then takes the `None`
arm and inserts a fresh `DocState { version: 1 }`:

```console
$ sed -n '182,194p' crates/go-ai-native-tcg-bridge/src/oracle.rs
    pub fn update(&mut self, rel: &str, content: Option<String>) -> Result<u64, ...> {
        match content {
            Some(text) => self.open_or_update(rel, text),
            None => {
                if self.docs.remove(rel).is_some() { ... didClose ... }
                Ok(0)
```

and the bridge's own unit test asserts the reset as the intended behaviour:

```console
$ sed -n '29,49p' crates/go-ai-native-tcg-bridge/src/oracle/tests.rs
fn overlay_versions_are_monotonic_and_close_resets() {
    ... update("a.go", Some(...)) == 1
    ... update("a.go", Some(...)) == 2
    ... update("a.go", None)      == 0   // close
    // Re-open starts a fresh document at v1 (didOpen again).
    ... update("a.go", Some(...)) == 1
}
```

Implementation and test agree with each other and disagree with the spec line —
which is what makes this a `contradiction` and the package the side that yields:
the shipped behaviour is the LSP-correct one (a `didClose` ends a document
lifetime; a later `didOpen` legitimately starts at version 1), so the sentence
was over-stated rather than the code wrong.

**What changed and why:** the fact now scopes monotonicity to **one document's
open lifetime** and states that `didClose` ends it, so a re-opened file starts
again at version 1 — naming the bridge's own overlay-version test as what pins
it. The marker stays `@impl/done`: the corrected sentence is exactly what ships.

**Twin in another stack:**
`packages/org.vibevm.ai-native/rust-ai-native-lang/v0.7.0/spec/rust/mechanisms/TCG-ORACLE-RUST-v0.1.xml:114`
carries the same sentence word-for-word under a *different* anchor id
(`##OVERLAY-RULE-VERSIONS-NEVER-REPEAT`), so the registry's shared-anchor merge
could not see it and F-270 is filed as a Go-only singleton. **Not edited.**

**New obligations noticed:** the anchor id
`OVERLAY-VERSIONS-NEVER-REPEAT-OR-RESET` now over-states its own corrected
prose. Renaming it would change the file's addressable set and cost a
re-mirror, so the id stands and this is recorded instead — a boss call, not
mine. Separately, the rust twin above is the same defect under a different id
and appears to be claimed by no obligation in my ten.

---

## F-190 — the sweep names a `Defaulted` policy line the tool never prints (but its second string is real)

**Outcome:** EDITED — **with the verdict's stated cause corrected: half of it is
falsified.**
**Files touched:**
`packages/org.vibevm.ai-native/go-ai-native-lang/v0.1.0/spec/skills/go-ai-native-sweep/SKILL.md`

**Re-verification:** the reason says *«the two strings the sweep tells a reader
to look for — `Defaulted` and `DISABLED by policy` — appear in no shipped string
and in no captured run»*. **`DISABLED by policy` is a shipped string, verbatim:**

```console
$ grep -rn "DISABLED by policy" crates/
crates/go-ai-native-cli/src/floor.rs:66:
    "floor: step `{}` DISABLED by policy — {} (conform.toml [go])",
$ sed -n '58,68p' crates/go-ai-native-cli/src/floor.rs
    let disabled = &config.go.floor_disable;
    for d in disabled {
        ...
        eprintln!(
            "floor: step `{}` DISABLED by policy — {} (conform.toml [go])",
            d.step, d.reason
        );
```

It is absent from the captured run for a reason that is not drift: that run had
**no `conform.toml` at all**, so `floor_disable` was empty and the loop printed
nothing. Absence from one run of a line that only fires on a configured
condition is not evidence the line does not exist.

**The other half holds, for a cause the verdict did not name.** `Defaulted` is
an internal `ConfigOrigin` enum variant, never user-visible; the line actually
printed on a defaulted policy is different text entirely:

```console
$ sed -n '25,37p' crates/go-ai-native-conform/src/lib.rs
fn load_config(root: &Path) -> Result<Config> {
    let (cfg, origin) = Config::load_or_default(root)?;
    match origin {
        conform_core::ConfigOrigin::Loaded =>
            eprintln!("go-ai-native-conform: policy conform.toml (loaded)."),
        conform_core::ConfigOrigin::Defaulted => eprintln!(
            "go-ai-native-conform: NO conform.toml — topology default in force \
             (roots = [\".\"], no cells gate); run `go-ai-native init` \
             to write a starting policy."
        ),
    }
```

and the captured run prints exactly that:

```console
$ grep -n "topology default" campaigns/packages-2026-09/harvest/go-ai-native-lang-floor.md
29:go-ai-native-conform: NO conform.toml — topology default in force (roots = ["."], no cells gate); run `go-ai-native init` to write a starting policy.
```

So the real defect is **one** string, not two: a reader scanning output for the
word `Defaulted` finds nothing, because the tool says `NO conform.toml —
topology default in force`.

**What changed and why:** the first cited string is replaced by the line the
tool actually prints — `` `Defaulted` conform policy`` → `` `NO conform.toml —
topology default in force` ``. **The `DISABLED by policy` half was left exactly
as written**, because it is correct and editing it would have been the phase
rewriting a true sentence on a false report. Marker stays `@impl/done`: both
lines are shipped and printed. No prescription weakened — the weekly
re-question rule and the shrinking-floor rationale are untouched.

**Twin in another stack:** yes, and **not edited** —
`packages/org.vibevm.ai-native/rust-ai-native-lang/v0.7.0/spec/skills/rust-ai-native-sweep/SKILL.md:47`
(`##CHECK-THE-PRINTED-POLICY-ORIGIN-LINES`) and
`packages/org.vibevm.ai-native/typescript-ai-native-lang/v0.6.0/spec/skills/typescript-ai-native-sweep/SKILL.md:48`
(`##CHECK-THE-PRINTED-POLICY-LINES`). This is a **release event** across three
packages. The twins' replacement text is **not** the same string as Go's — the
captured runs show each tool printing its own wording
(`campaigns/packages-2026-09/harvest/rust-ai-native-lang-floor.md:184` reads
`conform: NO conform.toml — topology default in force, nothing is gated; run
\`rust-ai-native init\`…`, and
`…/typescript-ai-native-lang-floor.md:31` reads `typescript-ai-native-conform:
NO conform.toml — topology default in force (roots = ["src"], …)`). Whoever
repairs the twins must copy each stack's own line, not Go's.

**New obligations noticed:** the verdict behind F-190 is itself defective and
its two sibling verdicts carry the same wording verbatim, so **the same wrong
claim about `DISABLED by policy` is recorded against the rust and typescript
anchors too**. If those are closed by deleting the string, three packages lose a
correct instruction on a false report. Worth flagging to whoever owns the twins
before they edit.

---

## F-212 — four names in the ratchet item, none of them the shipped one

**Outcome:** EDITED
**Files touched:**
`packages/org.vibevm.ai-native/go-ai-native-lang/v0.1.0/spec/skills/go-ai-native-sweep/SKILL.md`

**Re-verification:** the reason holds on all four names, and the two names it did
*not* challenge check out as correct.

The config key is `gated_crates`:

```console
$ grep -n "gated_crates\|gated_packages" crates/vendor/core-ai-native-conform/src/config.rs
25:///      gated_crates = [\"app\"]\n\
31:/// assert_eq!(cfg.gated_crates, vec!["app".to_string()]);
44:    pub gated_crates: Vec<String>,
272:    anyhow::bail!("conform.toml: `gated_crates` carries a duplicate crate name");
...
```

`gated_packages` returns nothing under that perimeter (the vendored conform
crate, which is the only definer of the `conform.toml` schema).

The census kinds are `init_decl` / `ambient_call` / `naked_go`:

```console
$ grep -rn "init_decl\|ambient_call\|naked_go\|init_in_cell" crates/vendor/core-ai-native-conform/src/rules/go.rs
42:///         kind: "init_decl".into(),
104:  "init_decl" | "blank_import" | "ambient_call" | "naked_go"
110:  "init_decl" if !in_test => (
120:  "ambient_call" if !in_test => (
125:  "naked_go" if !in_test => (
294,318,342: kind: "ambient_call".into(),
```

`init_in_cell`, `ambient_call_in_cell` and `naked_go_in_cell` occur zero times
in the package. The remaining two names in the same list **are** shipped and
were left alone — `error_string_match` (go.rs:130, 348) and
`seam_error_missing_req` (go.rs:145, and go-ai-native-tcg/src/lib.rs:121).

**What changed and why:** four literal renames to the shipped spellings —
`init_in_cell`→`init_decl`, `ambient_call_in_cell`→`ambient_call`,
`naked_go_in_cell`→`naked_go`, `gated_packages`→`gated_crates` — plus a short
clause noting that the `conform.toml` key keeps the `crates` spelling, because a
Go reader who trusts the stack's own vocabulary would otherwise write
`gated_packages` into their config and get silence. Marker stays `@impl/done`:
the corrected sentence names things that exist. This item is instructions a
reader types into a config file, so a wrong name here fails closed and silently
— which is why the rename is the whole repair and nothing else moved.

**Twin in another stack:**
`packages/org.vibevm.ai-native/rust-ai-native-lang/v0.7.0/spec/skills/rust-ai-native-sweep/SKILL.md:75`
carries `##RATCHET-CENSUS-REGRESSIONS` with Rust's own kind list
(`unwrap_domain` / `env_nonroot` / …). **Not edited** — F-212 is a **release
event** over those two packages. Note the rust twin's *kinds* are a different
set, so only the `gated_crates` half is shared; whoever repairs it must
re-measure Rust's kind strings rather than copy this fix.

**New obligations noticed:** two, neither in my ten.
(1) A **terminology defect**: the Go stack's `conform.toml` key is spelled
`gated_crates` (`crates/vendor/core-ai-native-conform/src/config.rs:44`) and its
error text says «carries a duplicate **crate** name» (config.rs:272) — a
Rust-ism in a language that has no crates. My edit makes the doc match the code;
the better long-run fix is a code-side rename, which is Phase E's and needs the
owner because it is a breaking config change across three stacks.
(2) The shipped ban set includes a fifth kind the sweep item does not list —
`blank_import` (go.rs:104) — so a reader draining this census by the skill's
list will miss it.

---

## F-211 — `OP-INIT` prints a parameter and three result keys the relay has never produced

**Outcome:** EDITED (demotion per §3.3)
**Files touched:**
`packages/org.vibevm.ai-native/go-ai-native-lang/v0.1.0/spec/go/mechanisms/TCG-PROTOCOL-GO-v0.1.xml`

**Re-verification:** the reason holds on every clause. The three documented keys
occur **zero times in the entire go stack** — perimeter: the whole `crates/`
tree, the only place a wire result can be produced:

```console
$ grep -rn "gopls_path\|go_version\|root_files" crates/ ; echo "grep exit=$?"
grep exit=1
```

The shipped result carries four keys, two of which the protocol does not
document (`position_encoding`, `pull_diagnostics`) —
`crates/go-ai-native-tcg/src/serve.rs:74-84`, quoted under F-160 above.

The `{root}` parameter is equally absent: the op ignores `frame.params` entirely
and re-spawns on the relay's own root, which was fixed at `serve` start.

```console
$ sed -n '294,300p' crates/go-ai-native-tcg/src/serve.rs
        if op == "init" {
            // Re-init: a fresh gopls session (overlays cleared).
            match GoOracle::spawn(&root, READINESS_BUDGET) {
```

`root` here is the local binding from `run_serve`, not a field of the frame.

**What changed and why:** the wire shape is rewritten to the shipped one —
`{root}` → `{}`, and `{gopls_version, gopls_path, go_version, root_files,
ready}` → `{gopls_version, position_encoding, pull_diagnostics, ready}` — with
one sentence saying the op takes no parameters and why (the root is the relay's,
fixed at `serve` start), and the three absent keys retained as **specified and
not carried yet** rather than deleted, so Phase E keeps the signal. Marker
`@impl/done` → `@spec/done`. This is the one fact in the batch a consumer can
break on directly: a host written against the printed shape would read
`gopls_path` and get `undefined`, so the shipped shape had to become the primary
statement rather than a footnote.

**Same compound-fact tension as F-160:** the `init` op itself is fully
implemented; only the parameter and three keys are not. `@spec/done` therefore
under-states it, and splitting would cost a new anchor. Flagged, not taken.

**Twin in another stack:** yes, and **not edited** —
`packages/org.vibevm.ai-native/rust-ai-native-lang/v0.7.0/spec/rust/mechanisms/TCG-PROTOCOL-RUST-v0.1.xml#OP-INIT`,
which F-211's anchor list names as the second member. Its verdict records the
identical shape one language over: printed `{ra_version, ra_path, toolchain,
root_files, quiescent}` against a shipped `ra_version, position_encoding,
pull_diagnostics, quiescent`. **Release event over two packages** — the Go edit
alone is not the closure.

**New obligations noticed:** the anchor exists in a **third** package that
F-211's list does not name —
`packages/org.vibevm.ai-native/typescript-ai-native-lang/v0.6.0/spec/typescript/mechanisms/TCG-PROTOCOL-v0.1.xml#OP-INIT`:

```console
$ grep -rl "##OP-INIT" packages/org.vibevm.ai-native/rust-ai-native-lang packages/org.vibevm.ai-native/typescript-ai-native-lang
.../rust-ai-native-lang/v0.7.0/spec/rust/mechanisms/TCG-PROTOCOL-RUST-v0.1.md
.../typescript-ai-native-lang/v0.6.0/spec/typescript/mechanisms/TCG-PROTOCOL-v0.1.md
```

Whether the TypeScript `OP-INIT` carries the same defect I did not check (out of
perimeter), but a two-of-three release on a three-member family is worth a look
before publication.

---

## F-271 — Stage A claims to be proven by a corpus and a baseline that were never written

**Outcome:** EDITED (demotion per §3.3 — no code written)
**Files touched:**
`packages/org.vibevm.ai-native/go-ai-native-lang/v0.1.0/spec/go/tools/vibe-agentic-tcg-go.xml`

**Re-verification:** the reason holds, and both of its halves check out. The ops
ship — `Serve`, `Validate`, `Scope`, `Complete` (and `Type`, `Bench`) are real
subcommands at `crates/go-ai-native-tcg/src/main.rs:24-60`. The proof does not:
the corpus and reports evidence under F-160 above is the same evidence here —
`research/tcg-bench/` carries `corpus` (TypeScript) and `corpus-rust`, and
`research/tcg-bench/reports/` carries `bench-2026-07-07-baseline.json` and
`bench-rust-2026-07-07-baseline.json`. No Go corpus, no Go baseline, under a
repo-wide perimeter.

**What changed and why:** the clause «mechanics proven by the differential
corpus and the bench baseline on `research/go-demo`» is replaced by a statement
that separates what ships from what does not: the ops ship; the corpus and
baseline that would prove the mechanics are specified and not written. Marker
`@impl/done` → `@spec/done`. Nothing prescriptive was removed — Stage A's
*definition* (validate / scope / complete / type over LSP overlays,
discipline-enriched, MCP + one-shot delivery) is untouched, and the intent to
prove it by corpus and baseline stays on the page as the unbuilt half. The
dev-tree path `research/go-demo` is dropped from this fact as a side effect of
dropping the claim it qualified, which also removes one dev-tree-only address
from the shipped surface.

**Twin in another stack: YES — and this is the case where the twins must NOT be
touched, because in their packages the sentence is TRUE.** The anchor exists in
both siblings:

```console
$ grep -rl "##STAGE-A-CONSULTATION-ORACLE" packages/org.vibevm.ai-native/rust-ai-native-lang packages/org.vibevm.ai-native/typescript-ai-native-lang
.../rust-ai-native-lang/v0.7.0/spec/rust/tools/vibe-agentic-tcg-rust.md
.../typescript-ai-native-lang/v0.6.0/spec/typescript/tools/vibe-agentic-tcg-ts.md
```

Rust's copy (line 142-146) is near-verbatim — «mechanics proven by the
differential corpus and the bench baseline» — and Rust **has** both
(`research/tcg-bench/corpus-rust`, `reports/bench-rust-2026-07-07-baseline.json`).
TypeScript's (142-144) says «measured by the two-arm battery» and TS has that too
(`corpus`, `bench-2026-07-07-baseline.json`, and the control / with-tools
`.jsonl` arms). So the family shipped one sentence three times and **only the Go
instance was never earned** — which is why F-271 is correctly a Go-only
singleton, and why this edit does *not* create the one-member-fixed defect §4.5
warns about. It removes a false copy of a true sentence. **Neither twin edited.**

**New obligations noticed:** `##STAGE-C-DELIVERY-EXPERIMENTS` in the same list
(`spec/go/tools/vibe-agentic-tcg-go.md:150`, `@spec/done`) also names
`research/go-demo` as the battery root — a dev-tree address inside a shipped
package, the same genre as F-272's. It is `@spec/done` already so it claims
nothing built, but the address still does not resolve for a consumer. Two more
of the same in `spec/cards/INDEX.md:19` and `:57`. Not in my ten; recorded, not
fixed.

---

## F-272 — the pilot exists; «with the whole chain green» is a measurement nobody took

**Outcome:** EDITED — **and the recorded reason does not support its own
verdict.** See the ruling below.
**Files touched:**
`packages/org.vibevm.ai-native/go-ai-native-lang/v0.1.0/README.md`

**Re-verification, clause by clause.** The fact reads: «The worked pilot lives in
the vibevm dev tree at `research/go-demo` — a miniature reconciler **with the
whole chain green**.»

**Clause 1 — the pilot exists. CONFIRMED**, and the Phase C worker's correction
of themselves was right:

```console
$ ls research/go-demo/
README.md  cmd  conform.toml  discipline  go-ai-native-conform-baseline.json
go.mod  internal  spec  specmap.json  specmap.toml
$ find research/go-demo -name "*.go" | wc -l
15
$ ls research/go-demo/internal/cells/
batchplanner  naiveplanner
```

15 `.go` files, two cells in the prescribed layout, both policy files, a conform
baseline and a populated `discipline/`. A complete Go consumer.

**Clause 2 — «the whole chain green». NOT SETTLED BY ANYTHING ON THE RECORD.**

*Which evidence would settle it:* a floor run captured **against
`research/go-demo` as root** — `go-ai-native floor` with cwd `research/go-demo`
— in the same form as the other subjects' captures, terminating in the one
string that certifies green:

```console
$ sed -n '217,222p' <go-stack>/crates/go-ai-native-cli/src/floor.rs
    let red: Vec<&str> = outcomes.iter().filter(|o| !o.ok)...;
    if red.is_empty() {
        eprintln!("\nfloor: all green ({} step(s) run, {} disabled by policy).",
```

*Whether it exists:* **no.** Perimeter — every one of the 51 files in
`campaigns/packages-2026-09/harvest/`:

```console
$ grep -rln "go-demo" campaigns/packages-2026-09/harvest/
campaigns/packages-2026-09/harvest/d1-rust-ts-lang-repairs.md
```

The single hit is another worker's repair record, not a run capture. And the
capture that might be mistaken for the evidence is **not** against the pilot and
is **red**:

```console
$ sed -n '1,20p' campaigns/packages-2026-09/harvest/go-ai-native-lang-floor.md
# go-ai-native-lang — floor
_Captured 2026-07-28 against `packages/org.vibevm.ai-native/go-ai-native-lang/v0.1.0/`._
...
floor: `gofmt` FAILED
floor: `vet` FAILED
floor: `tests` FAILED
floor: `staticcheck` FAILED
```

Exactly as the brief anticipated: that run tests the package, not the pilot.

*What the pilot's own committed records do say* — these are the nearest thing to
evidence and they are **not** a floor verdict:

```console
$ cat research/go-demo/discipline/health/latest-go.json
{ "schema": 1, "collector": "go-ai-native health", "files_in_scope": 15,
  "file_length": { "over_budget": [], "danger_band": [] },
  "ban_census": { "reasoned": 0, "unreasoned": 2 },
  "export_examples": { "exports": 39, "with_examples": 3 },
  "orphan_backlog": [] }
$ head -c 200 research/go-demo/go-ai-native-conform-baseline.json
{ "schema": 1, "findings": [
  "go-cell-isolation|internal/cells/batchplanner/planner_test.go|reconcile-demo/internal/cells/naiveplanner#9" ] }
```

Two **unreasoned** suppressions, 36 of 39 exports without an `Example`, and one
`go-cell-isolation` violation that is green only because it is **baselined**.
Under the skill's own `##TRUTH-GATE-IS-TRUTH` («the gate is truth, the collector
is a guide») none of this makes the floor red — the collector is not the gate
and a baselined finding passes by construction. So this does **not** prove the
chain is red. It proves the claim is **unmeasured**, and it shows the pilot
carrying ratchet debt and a baselined violation that «the whole chain green»
reads as denying.

**Ruling on the verdict itself.** The recorded reason confirms clause 1 and says
nothing whatever about clause 2 — it is a `drift` verdict whose stated grounds
support `confirmed`. It is not that the reason is *wrong*; it is **incomplete**:
it re-checked the half it had previously got wrong and never reached the half
that carries the defect. The fact is nonetheless not true as written, for a
cause the reason never names.

**What changed and why:** clause 1 is kept and made more specific (the layout
that was actually listed); clause 2 — the unmeasured «with the whole chain
green» — is removed rather than restated, because a shipped README asserting a
green verdict that no run on the record supports is precisely what the campaign
mandate calls профанация. Marker stays `@impl/done`: everything the corrected
sentence claims was listed above. **I did not run any `init` command, or any
floor run, anywhere.**

**Twin in another stack:** «worked pilot» sentences exist in the sibling READMEs
but name each stack's own pilot, so there is no shared sentence to break;
`grep -rl "##WORKED-PILOT-IS-RESEARCH-GO-DEMO"` over
`packages/org.vibevm.ai-native/rust-ai-native-lang` and
`…/typescript-ai-native-lang` returns nothing. **None found.**

**New obligations noticed:** two.
(1) **The pilot's green claim is worth actually measuring.** If a floor run
against `research/go-demo` is captured and comes back green, the original
sentence was true and my edit removed a true claim for want of evidence — the
boss may want that run taken before accepting this diff. That is a one-command
check and I did not take it, because the brief forbade running `init` and I
could not establish that a bare `floor` on that tree would not attempt to
bootstrap policy.
(2) `research/go-demo` carries **two unreasoned suppressions** and **one
baselined `go-cell-isolation` finding in `internal/cells/batchplanner/`** — the
discipline's own worked example carrying debt the sweep tells consumers to
drain. That is a credibility-loop item for Phase F, not a document defect.

---

## F-273 — the floor is glossed as four steps; seven ship, and `build` is not one of them

**Outcome:** EDITED
**Files touched:**
`packages/org.vibevm.ai-native/go-ai-native-lang/v0.1.0/spec/go/tools/vibe-agentic-tcg-go.xml`

**Re-verification:** the reason holds exactly, including its sharpest claim —
there is no `build` step at all.

```console
$ sed -n '26,36p' crates/go-ai-native-cli/src/floor.rs
const STEPS: &[&str] = &[
    "gofmt", "vet", "tests", "staticcheck", "conform", "specmap", "test-gate",
];
$ grep -n '"build"' crates/go-ai-native-cli/src/floor.rs ; echo "grep exit=$?"
grep exit=1
$ sed -n '124,131p' crates/go-ai-native-cli/src/floor.rs
    // 3. Tests — per-module `go test` (build + run in one verb; the
    // compile IS the first half of the signal).
    if !is_disabled("tests") {
        header(opts, "go test ./...");
        cmd.args(["test", "./..."]);
```

The code's own comment states the substitution the doc missed. The package
already carries the correct wording elsewhere —
`spec/skills/go-ai-native-sweep/SKILL.md:41` (`##FLOOR-HAS-SEVEN-STEPS`) —
so this is one file lagging its own sibling, not an unknown.

**What changed and why:** the four-step gloss is replaced by the seven shipped
steps in run order, with an explicit note that there is no separate `build` and
that the compile is the first half of `go test ./...`. Wording is aligned to
`##FLOOR-HAS-SEVEN-STEPS` so the package now says one thing in two places.
Marker stays `@impl/done`. Nothing was weakened — «remains the truth, verbatim»
is untouched; the reader is simply told which seven verdicts that truth is made
of, including the three discipline steps (conform, specmap, test-gate) a
four-step gloss hides.

**Twin in another stack:** yes, one — **not edited**, and it looks like the same
genre of defect a stack over:

```console
$ grep -n -A1 "##FLOOR-REMAINS-THE-TRUTH" .../rust-ai-native-lang/v0.7.0/spec/rust/tools/vibe-agentic-tcg-rust.md
48:##FLOOR-REMAINS-THE-TRUTH The floor
49-(`rust-ai-native floor` → cargo check) remains the truth. @impl/done
```

A **one-step** gloss of the Rust floor. F-273's anchor list names only the Go
instance, so that line is claimed by no obligation I hold; whether `rust-ai-native
floor` really is one step is a measurement I did not take (out of perimeter). The
TypeScript package has no such anchor.

**New obligations noticed:** **the identical false gloss survives twice more in
this package**, at anchors not in my ten, and I left both untouched per «change
only what the evidence falsifies»:

1. `spec/go/mechanisms/TCG-ORACLE-GO-v0.1.md:153-155`,
   `##CLEAN-VALIDATE-DOES-NOT-CERTIFY-A-CLEAN-FLOOR` — «The floor
   (`go-ai-native floor` → gofmt/vet/build/test) remains the truth», word for
   word, `@impl/done`. This is in a file I edited for F-160/F-167/F-270, so the
   diff will show the same wrong string surviving three lines from a corrected
   one.
2. `spec/boot/20-stack-go-ai-native-lang.md:51-53`,
   `##TOOLCHAIN-GO-AI-NATIVE-UMBRELLA` — glosses the floor as
   «gofmt→vet→test→staticcheck+exhaustive→conform→specmap→test-gate», which is
   the **correct seven** and confirms the shipped list independently.

Closing F-273 without (1) leaves the package contradicting itself on the point
F-273 exists to fix. Flagged for the boss; not fixed, because it is not mine.

---

# Closing verification

## Files touched — six, all inside the permitted perimeter

| file (under `packages/org.vibevm.ai-native/go-ai-native-lang/v0.1.0/`) | obligations |
|---|---|
| `spec/boot/20-stack-go-ai-native-lang.md` | F-153 |
| `spec/go/mechanisms/TCG-ORACLE-GO-v0.1.md` | F-160, F-167, F-270 |
| `spec/go/mechanisms/TCG-PROTOCOL-GO-v0.1.md` | F-211 |
| `spec/go/tools/vibe-agentic-tcg-go.md` | F-271, F-273 |
| `spec/skills/go-ai-native-sweep/SKILL.md` | F-190, F-212 |
| `README.md` | F-272 |

Nothing else in the tree was edited. **No `git` command was run**, and **no
`init` command was run anywhere**. No code was written — every `missing-support`
closed by demotion per §3.3.

## The addressable set is provably unchanged

`RULE-ANCHORS-IMMUTABLE` is the one constraint a prose repair can break
silently, so it was measured rather than asserted: the current `##ID` set of each
touched file was compared against that file's Phase-B/C mirror record in
`campaigns/packages-2026-09/run/mirror/`.

```console
$ python -c "<compare each touched file's ##IDs to its run/mirror/*.json fact ids>"
OK  README.md: mirror=30 now=30 added=[] removed=[]
OK  spec/boot/20-stack-go-ai-native-lang.md: mirror=18 now=18 added=[] removed=[]
OK  spec/go/mechanisms/TCG-ORACLE-GO-v0.1.md: mirror=71 now=71 added=[] removed=[]
OK  spec/go/mechanisms/TCG-PROTOCOL-GO-v0.1.md: mirror=41 now=41 added=[] removed=[]
OK  spec/go/tools/vibe-agentic-tcg-go.md: mirror=37 now=37 added=[] removed=[]
OK  spec/skills/go-ai-native-sweep/SKILL.md: mirror=34 now=34 added=[] removed=[]

ANCHOR SET UNCHANGED ACROSS ALL SIX TOUCHED FILES
```

**No anchor was added and none was removed.** `merge-verdicts.py` can therefore
re-judge these anchors without `vibe progress mirror` running first (§3.1's
«Revisit when» condition does not fire).

## Marker moves — nine demotions, nine holds

| anchor | before | after |
|---|---|---|
| `GO-CODE-FOLLOWS-THE-GO-GUIDE` | `@impl/done` | `@impl/done` |
| `CARD-REGISTRY-FOR-GO` | `@impl/done` | `@impl/done` |
| `QUANTITIES-ARE-CAMPAIGN-MEASURED` | `@impl/done` | **`@spec/done`** |
| `RESOLUTION-GOPLS-ON-PATH` | `@impl/done` | `@impl/done` |
| `INIT-RESULT-CARRIES-PATH-AND-VERSION` | `@impl/done` | **`@spec/done`** |
| `DIAGNOSTICS-CHANNEL-HISTORY` | `@spec/done` | `@spec/done` (prose only) |
| `OVERLAY-VERSIONS-NEVER-REPEAT-OR-RESET` | `@impl/done` | `@impl/done` |
| `DIFFERENTIAL-CORPUS-PINS-DIAGNOSTIC-CLASSES` | `@impl/done` | **`@spec/done`** |
| `GRACEFUL-EXIT-IS-THE-LSP-DANCE` | `@impl/done` | `@impl/done` |
| `TARGET-COMPLETE` | `@impl/done` | **`@spec/done`** |
| `BENCH-HARNESS-RECORDS-DISTRIBUTIONS` | `@impl/done` | **`@spec/done`** |
| `LARGE-WORKSPACE-COLD-INIT-WARNING` | `@impl/done` | **`@spec/done`** |
| `CHECK-THE-PRINTED-POLICY-LINES` | `@impl/done` | `@impl/done` |
| `RATCHET-CENSUS-REGRESSIONS` | `@impl/done` | `@impl/done` |
| `OP-INIT` | `@impl/done` | **`@spec/done`** |
| `STAGE-A-CONSULTATION-ORACLE` | `@impl/done` | **`@spec/done`** |
| `WORKED-PILOT-IS-RESEARCH-GO-DEMO` | `@impl/done` | `@impl/done` |
| `FLOOR-REMAINS-THE-TRUTH` | `@impl/done` | `@impl/done` |

Eight demotions to `@spec/done`, all of them `missing-support` or a fact whose
support turned out absent; ten markers held because the corrected sentence
describes something that genuinely ships.

## The three things the boss should decide, not me

1. **Compound facts demoted whole (F-160 ×3, F-211, F-271).** Each has a shipped
   half and an unbuilt half under one anchor. §3.3 demotes the fact, so
   `@spec/done` under-states what ships. Splitting would state it exactly and
   costs new anchor ids plus a re-mirror. I took the conservative route and
   flagged every instance.
2. **F-190's verdict is partly false.** `DISABLED by policy` is a shipped literal
   (`crates/go-ai-native-cli/src/floor.rs:66`). I repaired only the string that
   is genuinely wrong. The two sibling verdicts carry the same false claim, so
   whoever closes the rust and typescript twins should be warned before they
   delete a correct instruction.
3. **F-272's verdict does not support its own outcome.** Its reason establishes
   the pilot exists and never addresses «with the whole chain green», which is
   the half that carries the defect. I removed the unmeasured claim; if the boss
   would rather measure it, one captured `go-ai-native floor` run rooted at
   `research/go-demo` settles it and would justify restoring the original
   sentence instead.

## Defects found that are NOT in my ten

Recorded, not fixed, per the brief.

| where | what |
|---|---|
| `spec/go/mechanisms/TCG-ORACLE-GO-v0.1.md:164` (`CLEAN-VALIDATE-DOES-NOT-CERTIFY-A-CLEAN-FLOOR`) | the **same** `gofmt/vet/build/test` gloss F-273 repairs, still `@impl/done`, in a file this batch edited |
| `spec/go/mechanisms/TCG-ORACLE-GO-v0.1.md` (`TARGET-COLD-INIT`) | same missing instrument as `TARGET-COMPLETE`; bench records `warm_ms` only |
| `spec/boot/20-stack-go-ai-native-lang.md:15` (`STACK-SHIPS-ITS-OWN-CARDS-PROJECTION`) | bare `cards/` address, same family as F-153 |
| `spec/go/tools/vibe-agentic-tcg-go.md:150`, `spec/cards/INDEX.md:19` and `:57` | three more dev-tree `research/go-demo` addresses on the shipped surface |
| `spec/skills/go-ai-native-sweep/SKILL.md:79` | the shipped ban set has a fifth kind the item omits — `blank_import` |
| `crates/vendor/core-ai-native-conform/src/config.rs:44,272` | terminology: a Go stack whose config key is `gated_crates` and whose error says «duplicate **crate** name» — a code-side rename is Phase E's and is a breaking config change across three stacks |
| `…/rust-ai-native-lang/v0.7.0/spec/rust/mechanisms/TCG-ORACLE-RUST-v0.1.md:243` | the rust twin of the unbacked 60 s ceiling, in no obligation I hold |
| `…/rust-ai-native-lang/v0.7.0/spec/rust/tools/vibe-agentic-tcg-rust.md:49` | `FLOOR-REMAINS-THE-TRUTH` glossed as **one** step (`cargo check`) — likely F-273's genre one stack over, unclaimed |
| `…/typescript-ai-native-lang/v0.6.0/spec/typescript/mechanisms/TCG-PROTOCOL-v0.1.md` | carries `##OP-INIT`; F-211 names only go + rust, so a three-member family is being released two-at-a-time |
| `research/go-demo/` | the discipline's own worked example carries 2 unreasoned suppressions, 36/39 exports without an `Example`, and a baselined `go-cell-isolation` finding — a Phase F credibility item, not a document defect |
