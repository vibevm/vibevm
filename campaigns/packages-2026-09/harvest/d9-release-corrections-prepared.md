# D9 — prepared release corrections, none applied

_Phase D, batch D9. **Nine items · 19 anchors across nine registry rows, 18 of
them drafted** (F-245's second anchor is the address family's and is named, not
drafted). Every correction below is
**PREPARED AND NOT APPLIED**: the `release` route puts publication behind the
owner ([§1.2](../PHASE-D-BATCH-PLAN.md#routes)), and §4's doctrine for the
owner-gated routes is «diffs prepared in advance, presented in batches,
**approved one at a time**». This file **is** that presentation — it is the
queue of diffs, not a record of edits._

**No file was edited but this one.** No package file, no spec file, no campaign
state, no verdict JSON, no `merge-verdicts.py`, no `vibe progress seal`, no git
write. `git` was run read-only (`rev-parse`, `log`, `status`).

**Measured at** `HEAD = 3c14d6af` (`docs(campaign): wave 8 in the LOG — the
release route re-verified, a third fell, and a strike was scoped by the wrong
reason`, 2026-07-31). Working tree at batch start carried one modification that
is **not** this batch's and was not touched: `M campaigns/packages-2026-09/OBLIGATIONS.md`
(`git status --porcelain`). Nothing this batch measured lives in that file.

**Every count below names the command that produced it**, and every «current
text» block is quoted from the file at HEAD, never from a brief or a prior
harvest — per
[`##THE-CAMPAIGN-IS-INSIDE-ITS-OWN-CORPUS`](../PHASE-D-BATCH-PLAN.md#delegation-lessons),
a figure carried rather than re-measured is a figure that decays.

**What was read to write this** — the reading list actually used, in order:

1. [`PHASE-D-BATCH-PLAN.md`](../PHASE-D-BATCH-PLAN.md) in full, and
   [§3.6 `#which-side`](../PHASE-D-BATCH-PLAN.md#which-side) +
   [§6.1 `#delegation-lessons`](../PHASE-D-BATCH-PLAN.md#delegation-lessons) as
   the binding rules for what a correction may claim.
2. [`PHASE-D-RELEASE-QUEUE.md`](../PHASE-D-RELEASE-QUEUE.md) in full — the
   wave-8 annotated state. **Every correction here matches its row's per-row
   outcome**, and where a row was restated in wave 8 the correction is written
   against the restated ground, never the registry's original `reason`.
3. [`harvest/d8b-stacks-audience-release-reverify.md`](d8b-stacks-audience-release-reverify.md)
   — the genre model for this file. Its «Proposed correction (NOT APPLIED)»
   blocks are the exact shape reproduced below.
4. [`harvest/d8a-stacks-package-own-release-reverify.md`](d8a-stacks-package-own-release-reverify.md)
   and [`harvest/d8c-world-compose-release-reverify.md`](d8c-world-compose-release-reverify.md)
   — the per-anchor evidence each correction must satisfy.
5. The subject files themselves, at HEAD, plus the binaries and manifests each
   correction is judged against.

**Not in this batch, and where its diff lives instead:**

- **F-189** (3 anchors, `##COMPONENT-THE-PRODUCT-SEAM` in go/rust/typescript) and
  **F-190** (3 anchors, the sweep skills' printed-policy step) — **already
  drafted** in
  [`d8b` §f-189](d8b-stacks-audience-release-reverify.md#f-189) and
  [`d8b` §f-190](d8b-stacks-audience-release-reverify.md#f-190). Not re-drafted
  here. **F-132 is covered by the same d8b F-189 draft.** Two riders travel with
  the F-189 diff and are the owner's to fold in or split: the **three
  `##three-processes-lead` ASCII diagrams** carrying the same retired topology
  with no anchor and no verdict (`vibe-agentic-tcg-rust.xml:104-107`,
  `vibe-agentic-tcg-go.xml:100-102`, `vibe-agentic-tcg-ts.xml:100-101`) — a diff
  that repairs the rows and leaves the diagrams ships two topologies per
  document; and the d8b requirement that the Go anchor's **recorded reason** be
  replaced before the diff is shown.
- **The address family** (`F-136`, `F-145`, and the 24 obligations over 22
  packages the governing-anchor join catches — [§A.1](../PHASE-D-RELEASE-QUEUE.md#addresses-scope))
  — its diff is not prose at all, it is
  [`tasks/address-repair.py`](../tasks/address-repair.py), a verified
  transformation. Out of scope here by construction.

---

## 1. F-153 — six bare paths in three stack boot snippets {#f-153}

**Queue row** ([§B](../PHASE-D-RELEASE-QUEUE.md#stacks)): «boot snippet cites
`rust/…`, `go/…`, `cards/INDEX.md`; all live under `spec/` — **wave 8: all six
STAND**», and [§A](../PHASE-D-RELEASE-QUEUE.md#addresses) explicitly separates
this from the address family: «It needs no tag and no decision, **only the
correct intra-package path**.» The correction below is exactly that and nothing
more. **6 anchors · 6 one-segment prefixes · no design choice owed.**

### Current text at HEAD

```console
$ sed -n '5,17p' packages/org.vibevm.ai-native/go-ai-native-lang/v0.1.0/spec/boot/20-stack-go-ai-native-lang.xml
```

```
 5  ##GO-CODE-FOLLOWS-THE-GO-GUIDE Go code in this project follows the AI-Native Go guide
 6  (`go/GUIDE-AI-NATIVE-GO.md` in this package). @impl/done
…
12  ##CARD-REGISTRY-FOR-GO Card registry for Go: `cards/INDEX.md` in this package (trigger → card;
13  the nine executable scaffolds A–I in their Go shape). @impl/done
```

```console
$ sed -n '5,13p' packages/org.vibevm.ai-native/rust-ai-native-lang/v0.7.0/spec/boot/20-stack-rust-ai-native-lang.xml
```

```
 5  ##RUST-CODE-FOLLOWS-THE-RUST-GUIDE Rust code in this project follows the AI-Native Rust guide
 6  (`rust/GUIDE-AI-NATIVE-RUST.md` in this package). @impl/done
…
12  ##CARD-REGISTRY-FOR-RUST Card registry for Rust: `cards/INDEX.md` in this package (trigger → card;
13  the nine executable scaffolds A–I in their Rust shape). @impl/done
```

```console
$ sed -n '5,14p' packages/org.vibevm.ai-native/typescript-ai-native-lang/v0.6.0/spec/boot/20-stack-typescript-ai-native-lang.xml
```

```
 5  ##TYPESCRIPT-CODE-FOLLOWS-THE-TYPESCRIPT-GUIDE TypeScript code in this
 6  project follows the AI-Native TypeScript guide
 7  (`typescript/GUIDE-AI-NATIVE-TYPESCRIPT.md` in this package). @impl/done
…
13  ##CARD-REGISTRY-FOR-TYPESCRIPT Card registry for TypeScript: `cards/INDEX.md` in this package (trigger →
14  card; the nine executable scaffolds A–I in their TypeScript shape). @impl/done
```

All three files carry `<status stage="impl" state="done"/>` at `:3`, and all six
anchors carry `@impl/done`. **Neither moves** — the content was never in
question, only the address, which is what `relocation` means.

### The measurement the correction must satisfy

**Both lanes, tested by existence.** The phrase «in this package» fixes the
origin at the package root, and the reader of a boot snippet stands in the
*consuming project* with the file open inside its install slot — so the same
relative form must resolve from **two** roots. Tested from both:

```console
$ for root in packages/org.vibevm.ai-native/{go-ai-native-lang/v0.1.0,rust-ai-native-lang/v0.7.0,typescript-ai-native-lang/v0.6.0} \
              vibedeps/stack-{rust-ai-native-lang/0.7.0,typescript-ai-native-lang/0.6.0}; do … [ -e "$root/$cand" ] …
```

| root (lane) | `<lang>/GUIDE-…` | `spec/<lang>/GUIDE-…` | `cards/INDEX.md` | `spec/cards/INDEX.md` |
|---|---|---|---|---|
| `packages/…/go-ai-native-lang/v0.1.0` | MISSING | **EXISTS** | MISSING | **EXISTS** |
| `packages/…/rust-ai-native-lang/v0.7.0` | MISSING | **EXISTS** | MISSING | **EXISTS** |
| `packages/…/typescript-ai-native-lang/v0.6.0` | MISSING | **EXISTS** | MISSING | **EXISTS** |
| `vibedeps/stack-rust-ai-native-lang/0.7.0` | MISSING | **EXISTS** | MISSING | **EXISTS** |
| `vibedeps/stack-typescript-ai-native-lang/0.6.0` | MISSING | **EXISTS** | MISSING | **EXISTS** |

**There is no Go install slot in this host to test against** — and that is the
intended state, not a gap:

```console
$ find . -maxdepth 6 -type d -name "stack-go-ai-native-lang" -not -path "./.git/*" -not -path "*/target/*"
(no output)
```

Per [§3.8 / §B.1](../PHASE-D-BATCH-PLAN.md#audience) the Go stack is «a prototype
specification, deliberately unused in this project, and it must stay unused», so
the Go correction is judged on the package lane alone, which is where it is
decided anyway.

**The compiled lane is not involved, and this is what separates F-153 from the
address family.** The snippet body is *not* inlined into `spec/boot/STATIC.xml` —
the host names the file by full slot path and the reader opens it in the slot:

```console
$ grep -rn "GUIDE-AI-NATIVE\|cards/INDEX.md" spec/boot/
(no output; exit 1)

$ grep -n "ai-native-lang" spec/boot/INDEX.md
22:  path = "vibedeps/stack-rust-ai-native-lang/0.7.0/spec/boot/20-stack-rust-ai-native-lang.md"
26:  path = "vibedeps/stack-typescript-ai-native-lang/0.6.0/spec/boot/20-stack-typescript-ai-native-lang.md"
```

So no `@spec://` tag is needed to survive compilation — nothing compiles. The
**release** route is still correct, because the reader reads the *slot* copy and
the slot copy only changes on a re-vendor.

### Proposed correction (NOT APPLIED) — six one-segment prefixes

Each block is the anchor's full current lines with the single path corrected;
anchor id unchanged, `@impl/done` unchanged, line wrap unchanged.

```
##GO-CODE-FOLLOWS-THE-GO-GUIDE Go code in this project follows the AI-Native Go guide
(`spec/go/GUIDE-AI-NATIVE-GO.md` in this package). @impl/done
```

```
##CARD-REGISTRY-FOR-GO Card registry for Go: `spec/cards/INDEX.md` in this package (trigger → card;
the nine executable scaffolds A–I in their Go shape). @impl/done
```

```
##RUST-CODE-FOLLOWS-THE-RUST-GUIDE Rust code in this project follows the AI-Native Rust guide
(`spec/rust/GUIDE-AI-NATIVE-RUST.md` in this package). @impl/done
```

```
##CARD-REGISTRY-FOR-RUST Card registry for Rust: `spec/cards/INDEX.md` in this package (trigger → card;
the nine executable scaffolds A–I in their Rust shape). @impl/done
```

```
##TYPESCRIPT-CODE-FOLLOWS-THE-TYPESCRIPT-GUIDE TypeScript code in this
project follows the AI-Native TypeScript guide
(`spec/typescript/GUIDE-AI-NATIVE-TYPESCRIPT.md` in this package). @impl/done
```

```
##CARD-REGISTRY-FOR-TYPESCRIPT Card registry for TypeScript: `spec/cards/INDEX.md` in this package (trigger →
card; the nine executable scaffolds A–I in their TypeScript shape). @impl/done
```

### The alternative form, and why it is not offered as a choice

A `spec://` qualified address would also resolve —
`spec://org.vibevm.ai-native/rust-ai-native-lang/GUIDE#anchor` is PROP-029's own
worked example for exactly this package
(`spec/common/PROP-029-fully-qualified-addresses.xml:22`, `##CARRIER-SPEC-URI`) —
and it resolves from *anywhere*, not just the two roots above. It is **not**
offered as an option because the phase has already ruled this shape twice and
the repaired text is live at HEAD:

```console
$ grep -n "MAP-RUST-GUIDE\|READ-STACK-GUIDE" -r packages/org.vibevm.ai-native/core-ai-native/v0.8.0/
README.md:31:4. ##READ-STACK-GUIDE The active language stack's GUIDE (e.g. `spec/rust/GUIDE-AI-NATIVE-RUST.md` in the Rust stack). @impl/done
spec/00-MANIFESTO.md:172:- ##MAP-RUST-GUIDE `spec/rust/GUIDE-AI-NATIVE-RUST.md` in `stack:org.vibevm.ai-native/rust-ai-native-lang` — … @impl/done
```

Both took a plain `spec/`-prefixed path, **no tag**, and both kept `@impl/done`.
Six more of the same is the consistent move; introducing `spec://` here would
make the third instance of one family differ from the first two.
**Design choice owed: NO.**

### The unjudged twins — listed so ONE approval can cover them deliberately

**Not drafted here** (the brief scopes this item to the six judged anchors), and
**not repaired by the six blocks above**. Each carries F-153's exact defect and
**no verdict at all** — verified against the registry, not assumed:

```console
$ python -c "…json.load('run/state/obligations.json') … anchors …"
rows: 158
STACK-SHIPS-ITS-OWN-CARDS-PROJECTION       obligations=[]
corpus-lives-here-lead                     obligations=[]
CORPUS-GUIDING-LAYER                       obligations=[]
CORPUS-OPERATING-PLAYBOOKS                 obligations=[]
CORPUS-MECHANISM-SPECS                     obligations=[]
CORPUS-APPENDIX                            obligations=[]
CARD-REGISTRY                              obligations=[]
CARDS-AND-CHECKERS-PER-STACK               obligations=[]
```

**(i) `##STACK-SHIPS-ITS-OWN-CARDS-PROJECTION`, all three snippets** — two lines
below an anchor this obligation *does* convict:

| file | line | current text (HEAD) |
|---|---:|---|
| `go-ai-native-lang/v0.1.0/spec/boot/20-stack-go-ai-native-lang.md` | 15-17 | «This stack ships its own `cards/` projection — …» |
| `rust-ai-native-lang/v0.7.0/spec/boot/20-stack-rust-ai-native-lang.md` | 15-17 | «This stack ships its own `cards/` projection — …» |
| `typescript-ai-native-lang/v0.6.0/spec/boot/20-stack-typescript-ai-native-lang.md` | 16-19 | «This stack ships its own `cards/` projection — …» |

**(ii) `core-ai-native/v0.8.0/spec/boot/10-flow-core-ai-native.md:9-18, 38`** —
the package all three stacks depend on, naming five bare paths and one bare
`cards/INDEX.md`:

```
 7  ##corpus-lives-here-lead The language-neutral corpus lives in this package: @impl/done
 9  - ##CORPUS-GUIDING-LAYER the guiding layer (`00-MANIFESTO.xml`,
10    `01-PATTERN-CARD-FORMAT.xml`, `02-EXECUTABLE-SCAFFOLDS.xml`), @impl/done
11  - ##CORPUS-OPERATING-PLAYBOOKS the operating playbooks (`03-RAID-PLAYBOOK.xml`
12    campaigns, `04-SWEEP-PLAYBOOK.xml` the standing sweep, `05-CAMPAIGN-FORM.xml`
13    the campaign paper trail, `06-WAL-CONVENTION.xml` session-durable state —
14    optional but preferred), @impl/done
15  - ##CORPUS-MECHANISM-SPECS the mechanism specs under `mechanisms/`
16    (ENGINE-CONFORM, PROP-014 specmap, BROWNFIELD-PROTOCOL, LEDGER-INTENT — the
17    units `spec://org.vibevm.ai-native/core-ai-native/…` tags cite), @impl/done
18  - ##CORPUS-APPENDIX and `appendix/`. @impl/done
…
38  ##CARD-REGISTRY Card registry: the active language stack's `cards/INDEX.md` (trigger →
```

Every one of those lives under `spec/`:

```console
$ ls packages/org.vibevm.ai-native/core-ai-native/v0.8.0/spec/
00-MANIFESTO.xml  01-PATTERN-CARD-FORMAT.xml  02-EXECUTABLE-SCAFFOLDS.xml
03-RAID-PLAYBOOK.xml  04-SWEEP-PLAYBOOK.xml  05-CAMPAIGN-FORM.xml  06-WAL-CONVENTION.xml
appendix  boot  legacy-projections  mechanisms
```

**Why this matters to the approval and not just to the ledger:** `core-ai-native`
ships into the same boot lane as the three stacks. If F-153's six are published
and these are not, the fix arrives in a lane that still carries the identical
broken form **one entry above it**, in the package the three stacks depend on —
which is §4.5's «a fix landed in one consumer and not the others is not a
closure; it is a new `duplication` obligation». **The arithmetic of the wider
approval, so it can be given deliberately:** 6 judged anchors + 3
`##STACK-SHIPS-ITS-OWN-CARDS-PROJECTION` twins + core-ai-native's 5
(`##CORPUS-GUIDING-LAYER`, `##CORPUS-OPERATING-PLAYBOOKS`,
`##CORPUS-MECHANISM-SPECS`, `##CORPUS-APPENDIX`, `##CARD-REGISTRY`) = **14
anchors in 4 files**. The owner's one approval can cover all fourteen if he says
so; it cannot cover them by accident.

---

## 2. F-211 — `##OP-INIT` in two TCG protocols, each judged against its own binary {#f-211}

**Queue row** ([§B](../PHASE-D-RELEASE-QUEUE.md#stacks)): «**stands (wave 8):**
go as recorded; rust restated with **Rust's own key names** — the missing keys
are `ra_path` / `toolchain` / `root_files`, **not Go's gopls trio**.» The whole
point of this item is that the two diffs are **not interchangeable**: the key
names differ in every position but `root_files`. **2 anchors.**

### Current text at HEAD

```console
$ sed -n '57,62p' packages/org.vibevm.ai-native/go-ai-native-lang/v0.1.0/spec/go/mechanisms/TCG-PROTOCOL-GO-v0.1.xml
```

```
- ##OP-INIT **`init`** `{root}` → `{gopls_version, gopls_path, go_version,
  root_files, ready}` — resolves and spawns gopls (ORACLE-GO §1),
  negotiates capabilities (§2), applies §3 config, waits for readiness
  bounded by a deadline. Re-`init` on a live session restarts the
  child; overlays are cleared. The relay self-inits at `serve` start,
  so a host's first frame may be any op. @impl/done
```

```console
$ sed -n '69,75p' packages/org.vibevm.ai-native/rust-ai-native-lang/v0.7.0/spec/rust/mechanisms/TCG-PROTOCOL-RUST-v0.1.xml
```

```
- ##OP-INIT **`init`** `{root}` → `{ra_version, ra_path, toolchain, root_files,
  quiescent}` — resolves and spawns the analyzer (ORACLE-RUST §1),
  negotiates capabilities (§2), applies §3 config, waits for
  quiescence bounded by a deadline. Re-`init` on a live session
  restarts the child; overlays are cleared. The relay self-inits at
  `serve` start, so a host's first frame may be any op (client init
  frames remain re-init). @impl/done
```

Both documents carry `<status stage="spec" state="done"/>` at `:3`; both anchors
carry `@impl/done`. **The marker is kept** — the correction does not change
whether the op is implemented, only which keys it is documented to return.

### The measurement the correction must satisfy — verified per binary, not carried

**Go, shipped return keys** (`crates/go-ai-native-tcg/src/serve.rs:74-84`):

```rust
fn init_result(oracle: &GoOracle<ChildTransport>) -> serde_json::Value {
    serde_json::json!({
        "gopls_version": oracle.capabilities().server_version,
        "position_encoding": match oracle.capabilities().position_encoding {
            go_ai_native_tcg_bridge::position::PositionEncoding::Utf8 => "utf-8",
            go_ai_native_tcg_bridge::position::PositionEncoding::Utf16 => "utf-16",
        },
        "pull_diagnostics": oracle.capabilities().pull_diagnostics,
        "ready": oracle.ready(),
    })
}
```

**Rust, shipped return keys** (`crates/rust-ai-native-tcg/src/serve.rs:76-86`) —
and note that only `position_encoding` and `pull_diagnostics` are shared:

```rust
fn init_result(oracle: &RustOracle<ChildTransport>) -> serde_json::Value {
    serde_json::json!({
        "ra_version": oracle.capabilities().server_version,
        "position_encoding": match oracle.capabilities().position_encoding {
            rust_ai_native_tcg_bridge::position::PositionEncoding::Utf8 => "utf-8",
            rust_ai_native_tcg_bridge::position::PositionEncoding::Utf16 => "utf-16",
        },
        "pull_diagnostics": oracle.capabilities().pull_diagnostics,
        "quiescent": oracle.quiescent(),
    })
}
```

**The five documented-but-absent keys, searched as exact JSON keys in each
package's own `crates/` (excluding `target/`):**

```console
$ for k in '"gopls_path"' '"go_version"' '"root_files"' '"ra_path"' '"toolchain"'; do
    grep -rn -F "$k" packages/…/{go,rust}-ai-native-lang/v*/crates/ | grep -v /target/ | wc -l ; done
"gopls_path"     go=0 rust=0
"go_version"     go=0 rust=0
"root_files"     go=0 rust=0
"ra_path"        go=0 rust=0
"toolchain"      go=0 rust=0
```

**`{root}` is never read.** In both relays the `init` op is answered *before*
`handle_op` — the only function that receives `&frame.params` — so the parameter
object is unreachable from the `init` branch:

```console
$ grep -n "params" …/go-ai-native-tcg/src/serve.rs   → :309 handle_op(&policy, &mut oracle, id, &op, &frame.params)
$ sed -n '294,301p' …/go-ai-native-tcg/src/serve.rs
if op == "init" {  // Re-init: a fresh gopls session (overlays cleared).
    match GoOracle::spawn(&root, READINESS_BUDGET) { …          ← run_serve's root

$ grep -n "params" …/rust-ai-native-tcg/src/serve.rs → :297 handle_op(…, &frame.params)
$ sed -n '215,218p' …/rust-ai-native-tcg/src/serve.rs
pub fn run_serve(root: &Path) -> Result<i32> {
    let root = rust_ai_native_tcg_bridge::verbatim_free(
        &root.canonicalize().unwrap_or_else(|_| root.to_path_buf()), );
$ sed -n '282,285p' …/rust-ai-native-tcg/src/serve.rs
if op == "init" { match RustOracle::spawn(&root, QUIESCENCE_BUDGET) { …    ← same root
```

**Per stack: 2 of 5 documented keys produced, 3 absent, 2 undocumented keys
returned, `{root}` ignored.** And the two halves are **opposite defects**, which
is why the correction touches both sides of the arrow:

- the two **undocumented** keys are *sanctioned* — `##PARITY-ADDITIVE-ONLY-EVOLUTION`
  (`TCG-PROTOCOL-GO-v0.1.xml:30-32`, same clause at the Rust document's `:30-31`)
  says «new response fields — non-breaking». Documentation lag on a permitted
  change;
- the three **documented-and-never-produced** keys are the contract breach: a
  client written against `##OP-INIT` reads `root_files` and gets `undefined`.

**No proto bump either way.** `ORACLE_PROTOCOL = 1` in both relays
(`go …/serve.rs:24`, `rust …/serve.rs:26`) and the wire never carried the three
keys — so correcting the *document* changes no wire behaviour, and adding them
would be additive. The constant stays at 1 under both routes below.

### Proposed correction (NOT APPLIED) — route (b), the document matches its own binary

**Go** (`spec/go/mechanisms/TCG-PROTOCOL-GO-v0.1.md:57-62`):

```
- ##OP-INIT **`init`** `{}` → `{gopls_version, position_encoding,
  pull_diagnostics, ready}` — resolves and spawns gopls (ORACLE-GO §1),
  negotiates capabilities (§2), applies §3 config, waits for readiness
  bounded by a deadline. Re-`init` on a live session restarts the
  child; overlays are cleared. The relay self-inits at `serve` start,
  so a host's first frame may be any op. The relay serves ONE project,
  so `init` takes no parameters — the root is `serve`'s own process
  root. @impl/done
```

**Rust** (`spec/rust/mechanisms/TCG-PROTOCOL-RUST-v0.1.md:69-75`) — the same
shape in Rust's own strings, and `quiescent`/`ra_version` are **not** Go's
`ready`/`gopls_version`:

```
- ##OP-INIT **`init`** `{}` → `{ra_version, position_encoding,
  pull_diagnostics, quiescent}` — resolves and spawns the analyzer
  (ORACLE-RUST §1), negotiates capabilities (§2), applies §3 config,
  waits for quiescence bounded by a deadline. Re-`init` on a live
  session restarts the child; overlays are cleared. The relay
  self-inits at `serve` start, so a host's first frame may be any op
  (client init frames remain re-init). The relay serves ONE project,
  so `init` takes no parameters — the root is `run_serve`'s own
  canonicalized process root. @impl/done
```

### The other route, named with its cost — and the queue has already characterised it

**Route (a), build the three fields** (recorded in
[`d8a` §F-211](d8a-stacks-package-own-release-reverify.md)): add `gopls_path` /
`go_version` / `root_files` and `ra_path` / `toolchain` / `root_files` to each
`init_result` and *document* `position_encoding` / `pull_diagnostics`. Cost: two
crate edits plus their tests, in Phase E's lane, not Phase D's
([§3.3](../PHASE-D-BATCH-PLAN.md#demote): «Phase D does not write the
mechanism»). Benefit: all three stacks then answer `init` the same way, which
`##ONE-PRODUCT-CLIENT-DRIVES-ALL-THREE-RELAYS` (`TCG-PROTOCOL-GO-v0.1.xml:34-39`)
already assumes — and the TypeScript twin **already ships the shape**, so this is
a build gap rather than an over-specified contract.

**The queue's own §Ask says «Group B — no product decisions remain; publish the
corrections»**, which reads route (b) as settled. That is why route (b) is
drafted in full above and route (a) is named rather than drafted. **Design
choice owed: NO by the queue's ask — but the owner can take route (a) instead,
and if he does, this diff is dropped rather than amended.**

---

## 3. F-188 — `##MOTIVATION` in three `scaffold-i-codemods` cards {#f-188}

**Queue row** ([§B](../PHASE-D-RELEASE-QUEUE.md#stacks)): «**stands, restated per
stack (wave 8):** the go card prints the **rust** CLI's five-parameter signature
(shipped go verb takes two, writes three files — and the recorded «no Example
stub» clause is **false**, the stub IS written); the rust and ts cards cite
`vibe codemod rename-seam` — `vibe` has no `codemod` verb, `rename-seam` has zero
implementations tree-wide, `ts-morph` is absent from the TS package.» **3 anchors,
three different edits.**

**The genre question, answered before drafting.** A `##MOTIVATION` is a
**capability sketch** — it exists to make the reader want the operation, and
[§6.1's capability/practice test](../PHASE-D-BATCH-PLAN.md#delegation-lessons)
says «an unexercised capability is not a false capability». What convicts these
three is not that they *want* a codemod; it is that each prints **an executable
command line with named flags in the present tense** («performs the change
atomically and verifiably»), and this card's declared reader is the weakest agent
tier, who types it. So the minimal correction **keeps the motivating scenario and
the constrained-decoding thesis verbatim** and touches only the clause that
asserts a signature or a tool.

All three cards carry `<status stage="spec" state="done"/>` at `:3` and the
anchor carries `@spec/done`; **both stay** — nothing here changes the card's
stage, only what it claims ships. **All three `##MOTIVATION` lines are single
unwrapped lines**, so the corrections are single lines too.

### 3a. Go — the printed signature belongs to the Rust binary

**Current text at HEAD**, `go-ai-native-lang/v0.1.0/spec/cards/scaffold-i-codemods.md:19`:

```
##MOTIVATION Motivation: A weak agent asked to "add a planner variant" must create the cell package, the conformance assertion, the directive tags, the registry arm, and the Example stub — five files in lockstep. `go-ai-native codemod add-cell <pkg> <cell> <seam> <variant> <spec-uri>` performs the change atomically and verifiably; the agent fills five parameters instead of coordinating five files. This mirrors how constrained decoding lifts weak models (DR1-015): collapse the hard task into a constrained, parameterized one. @spec/done
```

**The measurement — this stack's own shipped verb only.**

```console
$ sed -n '148,162p' packages/…/go-ai-native-lang/v0.1.0/crates/go-ai-native-cli/src/main.rs
enum CodemodCmd {
    /// Add a new cell package: doc.go with its `//spec:scope` marker,
    /// the cell source with a New constructor, and a smoke test with
    /// an executed Example — post-checked by running the new cell's
    /// tests and rolled back on failure.
    AddCell {
        #[arg(long)] cell: String,        // Cell package name, lowercase letters/digits.
        #[arg(long)] spec_uri: String,    // The spec:// unit the cell implements — required
    },
}
```

**Two named flags, not five positional parameters.** And three files, not five
(`crates/go-ai-native-cli/src/codemod.rs:109-114`):

```rust
write("doc.go", doc_source(cell, spec_uri))?;
write(&format!("{cell}.go"), cell_source(cell, &type_name))?;
write(&format!("{cell}_test.go"), smoke_test_source(cell, &type_name))?;
```

**The Example stub IS written** — the clause the queue calls false — at
`codemod.rs:56-59`, inside `smoke_test_source`:

```
func ExampleNew() {
	fmt.Println(New().Name())
	// Output: {cell}
}
```

**Of the five artefacts the sentence promises, three are written and two are
not.** `doc_source` (`codemod.rs:16-24`) emits the `//spec:scope {spec_uri} r=1`
directive; `cell_source` (`:26-39`) emits `type X struct{}`, `New()` and
`Name()` and **no** seam-conformance assertion; and:

```console
$ grep -n "registry\|var _ Seam\|Seam =" …/go-ai-native-cli/src/codemod.rs
(no output)
```

So: **package + directive + Example written; conformance assertion + registry arm
not.** The correction must state that split, not the verdict's.

**Proposed correction (NOT APPLIED):**

```
##MOTIVATION Motivation: A weak agent asked to "add a planner variant" must create the cell package, the conformance assertion, the directive tags, the registry arm, and the Example stub — five artefacts in lockstep. `go-ai-native codemod add-cell --cell <cell> --spec-uri <uri>` collapses the scaffolding half into two parameters: it writes `doc.go` with its `//spec:scope` directive, the cell source with its `New` constructor, and a smoke test carrying an executed `Example` — atomically, post-checked by the new package's own `go test`, rolled back on failure. The seam-conformance assertion and the registry arm stay the author's. This mirrors how constrained decoding lifts weak models (DR1-015): collapse the hard task into a constrained, parameterized one. @spec/done
```

**Do NOT import the Rust five-parameter form.** It is
`rust-ai-native-cli`'s `AddCell` verbatim (`crates/rust-ai-native-cli/src/main.rs:177-197`:
`crate_dir`, `cell`, `seam`, `variant`, `spec_uri`, doc comment naming «module +
`#[cell]` manifest + REQ edge + smoke test + **lib.rs registration**»), and
transposing it is the mechanism by which this defect arrived.

**Two neighbours are NOT convicted and need no edit** — checked because
[§6.1](../PHASE-D-BATCH-PLAN.md#delegation-lessons) says to read the neighbours:
`##STRUCTURE-AND-PARTICIPANTS` (`:21`, «or the shipped CLI verb») and the Band-3
step (`:42`, «or use the shipped `go-ai-native codemod add-cell`») are both
**true** — the verb does ship; only its printed signature was wrong. Editing them
would be [`##A-REAL-DEFECT-CONVICTING-THE-WRONG-SENTENCE`](../PHASE-D-BATCH-PLAN.md#delegation-lessons)
in reverse.

### 3b. Rust — the tool does not exist, and the stack ships a different verb

**Current text at HEAD**, `rust-ai-native-lang/v0.7.0/spec/cards/scaffold-i-codemods.md:19`:

```
##MOTIVATION Motivation: A weak agent asked to "rename this seam across its 7 call-sites + the registry + the error enum" desynchronizes them. `vibe codemod rename-seam --from X --to Y` performs the change atomically and verifiably; the agent fills two parameters instead of coordinating seven edits. This mirrors how constrained decoding lifts weak models (DR1-015): collapse the hard task into a constrained, parameterized one. @spec/done
```

**The measurement.** `vibe` ships **27 verbs and no `Codemod`**:

```console
$ sed -n '91,241p' crates/vibe-cli/src/cli.rs | grep -cE "^\s{4}[A-Z][A-Za-z]+"
27
$ grep -c "Codemod" crates/vibe-cli/src/cli.rs
0
```

(`Init List Install Outdated Search Mcp Aiui Term Frame Skill Agentic Drain
Uninstall Update Reinstall Check Show Prefs Tree Registry Workspace Vvm Bin Trace
Vars Progress Version` — `pub enum Command` at `cli.rs:91`.)

**`rename-seam` has zero implementations**, in any language, anywhere in the tree
outside prose:

```console
$ grep -rn "rename-seam\|rename_seam\|RenameSeam" --include=*.rs --include=*.ts --include=*.go --include=*.toml . \
    | grep -v "/target/" | grep -v "^./legacy-spec/" | grep -v "^./campaigns/" | grep -v "/.git/"
(no output)
```

(`campaigns/**` and `legacy-spec/**` excluded per
[`##THE-CAMPAIGN-IS-INSIDE-ITS-OWN-CORPUS`](../PHASE-D-BATCH-PLAN.md#delegation-lessons)
and the owner's `legacy-spec` ruling.)

**And this stack does ship a codemod verb — a different one**
(`crates/rust-ai-native-cli/src/main.rs:177-197`, binary `rust-ai-native` per
`Cargo.toml:19-20`): `codemod add-cell --crate-dir --cell --seam --variant
--spec-uri`.

**Proposed correction (NOT APPLIED)** — the command becomes the target rather
than the present tense, and the shipped verb is named so the sentence is not
merely negative:

```
##MOTIVATION Motivation: A weak agent asked to "rename this seam across its 7 call-sites + the registry + the error enum" desynchronizes them. A `codemod rename-seam --from X --to Y` **would** perform the change atomically and verifiably, the agent filling two parameters instead of coordinating seven edits — that operation is specified and not yet built. The shipped codemod surface today is one verb, `rust-ai-native codemod add-cell --crate-dir <dir> --cell <cell> --seam <seam> --variant <variant> --spec-uri <uri>`, which scaffolds a cell atomically and rolls back on failure. This mirrors how constrained decoding lifts weak models (DR1-015): collapse the hard task into a constrained, parameterized one. @spec/done
```

**One thing the d8a draft proposed that this draft deliberately drops, and the
owner should rule on it rather than inherit it silently.** `d8a` suggested
appending «see `spec://org.vibevm.core/vibevm/common/PROP-031#beachhead`». Two problems, both
measured:

1. **That anchor does not exist.** `grep -n "{#beachhead}" spec/common/PROP-031-algorithmic-refactoring.xml`
   → no match; the section is `## 1. Problem statement {#problem}` and the
   sentences are `##BEACHHEAD-SCAFFOLD-I` / `##BEACHHEAD-LIMITS` at `:21-22`.
   Shipping it would mint a *new* dangling pointer inside the diff that closes a
   pointer family.
2. **It points a shipped `ai-native` card at a host PROP.** Per
   [§3.8](../PHASE-D-BATCH-PLAN.md#audience) these packages ship to consumers who
   do not have vibevm's `spec/common/`. Every one of the 27 `spec://org.vibevm.core/vibevm/`
   strings in `packages/org.vibevm.ai-native/*/v*/spec/` today is an
   **illustrative example inside a code sample** (`GUIDE-*-v0.1.md`'s
   `@spec implements spec://org.vibevm.core/vibevm/…#req-…`), not a live cross-boundary pointer.
   This would be the first.

   **Design choice owed: YES, small** — cite PROP-031 (and if so, with a
   *resolvable* anchor: `spec://org.vibevm.core/vibevm/common/PROP-031#problem`, or the
   `##BEACHHEAD-LIMITS` id) and accept the first live host pointer in an
   `ai-native` card; or leave the sentence self-contained as drafted above. The
   host side of the relationship is unaffected either way —
   `spec/common/PROP-031-algorithmic-refactoring.xml:21-22` already cites this
   card and already writes the correction («today it is one operation
   (`add-cell` scaffolding only)»), which is the strongest evidence the sentence
   reads as a fact claim.

### 3c. TypeScript — decided inside the package, no host observable

**Current text at HEAD**, `typescript-ai-native-lang/v0.6.0/spec/cards/scaffold-i-codemods.md:19`:

```
##MOTIVATION Motivation: A weak agent asked to "rename this seam across its 7 call-sites + the barrel re-export + the discriminated error union" desynchronizes them. `vibe codemod rename-seam --from X --to Y`, built on `ts-morph`, performs the change atomically and verifiably; the agent fills two parameters instead of coordinating seven edits. This mirrors how constrained decoding lifts weak models (DR1-015): collapse the hard task into a constrained, parameterized one. @spec/done
```

**The measurement — three absences, all inside the package** (§3.8's legitimate
bench for TypeScript):

```console
$ grep -rn "rename-seam\|rename_seam" packages/org.vibevm.ai-native/typescript-ai-native-lang/v0.6.0/ | grep -v /target/
…/spec/cards/scaffold-i-codemods.md:19        ← the sentence itself, and nothing else

$ grep -rn "ts-morph" packages/…/typescript-ai-native-lang/v0.6.0/ --include=*.json --include=*.ts --include=*.rs --include=*.toml | grep -v /target/
(no output)

$ sed -n '199,213p' …/crates/typescript-ai-native-cli/src/main.rs
enum CodemodCmd {
    /// Add a new cell: the seam module (`index.ts` with a file-level
    /// `@scope` marker) + a node:test smoke test, post-checked …
    AddCell { #[arg(long)] cell: String, #[arg(long)] spec_uri: String }
}
$ sed -n '60,69p' …/crates/typescript-ai-native-cli/src/codemod.rs
write(&format!("{seam}.ts"),      seam_source(…))?;
write(&format!("{seam}.test.ts"), smoke_test_source(…))?;
```

The operation name occurs **once in the whole package** — in the sentence that
promises it; the library it says the operation is «built on» is **not** a
dependency, tool or fixture anywhere in the package; and the package's own CLI
(binary `typescript-ai-native`, `Cargo.toml:13-14`) ships **one** codemod verb
taking two flags and writing two files.

**Proposed correction (NOT APPLIED):**

```
##MOTIVATION Motivation: A weak agent asked to "rename this seam across its 7 call-sites + the barrel re-export + the discriminated error union" desynchronizes them. A `codemod rename-seam --from X --to Y` over TypeScript's mature AST tooling **would** perform the change atomically and verifiably, the agent filling two parameters instead of coordinating seven edits — that operation is specified and not yet built. The shipped codemod surface today is one verb, `typescript-ai-native codemod add-cell --cell <cell> --spec-uri <uri>`, which writes the seam module and its `node:test` smoke test atomically and rolls back on failure. This mirrors how constrained decoding lifts weak models (DR1-015): collapse the hard task into a constrained, parameterized one. @spec/done
```

**`##INTENT` at `:11` is NOT convicted and must not be swept up.** «TypeScript's
mature codemod ecosystem (`ts-morph`, `jscodeshift`, typed ESLint autofix) makes
this the most achievable scaffold here» is a true statement **about the
ecosystem**, and the same is true of the Band-3 step at `:42` («Implement a
ts-morph / jscodeshift codemod») and `##STRUCTURE-AND-PARTICIPANTS` at `:11`.
What is convicted is only the sentence that turns achievability into a shipped
command — which is why the draft above says «over TypeScript's mature AST
tooling» rather than deleting the ecosystem claim.

---

## 4. F-251 — `##package-contents-lead` in two world READMEs {#f-251}

**Queue row** ([§D](../PHASE-D-RELEASE-QUEUE.md#arithmetic)): «**Wave 8
re-verified both anchors: STAND** … the correction remains two words, «four» →
«three», gated only by publication.» **2 anchors · one word each.**

### Current text at HEAD

```console
$ sed -n '32p' packages/org.vibevm.world/spec-genres/v0.1.0/README.md
##package-contents-lead This package ships four pieces of content plus a boot snippet: @impl/done

$ sed -n '22p' packages/org.vibevm.world/tool-design-lessons/v0.1.0/README.md
##package-contents-lead This package ships four pieces of content plus a boot snippet: @impl/done
```

The two sentences are **byte-identical**, which is why the merge is sound and the
repair is the same word in each.

### The measurement — each package's own tree

```console
$ ls packages/org.vibevm.world/spec-genres/v0.1.0/spec/flows/spec-genres/*.md
SPEC-GENRES-PROTOCOL.xml   design-docs.xml   when-to-write-what.xml            → 3

$ ls packages/org.vibevm.world/tool-design-lessons/v0.1.0/spec/flows/tool-design-lessons/*.md
TOOL-DESIGN-LESSONS.xml    packaging-lessons.xml    self-updating-tools.xml    → 3

$ grep -c '^- ##CONTENT-' …/spec-genres/v0.1.0/README.md …/tool-design-lessons/v0.1.0/README.md
spec-genres:4    tool-design-lessons:4
```

**Both packages ship exactly three flow documents**, and in both READMEs the
**fourth** `##CONTENT-` bullet **is** the boot snippet
(`##CONTENT-THE-BOOT-SNIPPET`, `spec-genres:45`, `tool-design-lessons:38`). So
«four pieces of content **plus** a boot snippet» promises five things, counts the
snippet twice, and over-counts the content documents by one. The sentence is
decidable inside each package with no consumer and no host observable.

The house convention the 14 conforming siblings keep is live in the exemplar:

```console
$ grep -n "package-contents-lead" packages/org.vibevm.world/addressable-specs/v0.1.0/README.md
22:##package-contents-lead This package ships three pieces of content plus a boot snippet: @impl/done
```

— three flow documents, four bullets, the fourth being the snippet, listed for
completeness. **The number names the flow documents.**

### Proposed correction (NOT APPLIED)

`packages/org.vibevm.world/spec-genres/v0.1.0/README.md:32`:

```
##package-contents-lead This package ships three pieces of content plus a boot snippet: @impl/done
```

`packages/org.vibevm.world/tool-design-lessons/v0.1.0/README.md:22`:

```
##package-contents-lead This package ships three pieces of content plus a boot snippet: @impl/done
```

Anchor id unchanged, `@impl/done` unchanged, nothing else on either line moves.
**Design choice owed: NO.**

---

## 5. F-186 residue — one character in the rust `scaffold-i` evidence line {#f-186}

**Queue row** ([§B](../PHASE-D-RELEASE-QUEUE.md#stacks)): «go+rust `scaffold-g`
FELL; **the survivor is `scaffold-i`'s typo'd id `DL1-015` → `DR1-015`**,
single-package, left the route.» Confirmed against the registry — F-186 is now a
one-anchor `prose-edit` row:

```console
$ python -c "…run/state/obligations.json… F-186"
 anchors: ['packages/org.vibevm.ai-native/rust-ai-native-lang/v0.7.0/spec/cards/scaffold-i-codemods.xml#EVIDENCE-AND-TRANSFER-STRENGTH']
 route: prose-edit   status: open   wave: 1
```

**1 anchor · 1 character.** It is here rather than in a boss lane because the
reader reads the **slot** copy and the slot copy carries the typo (below), so it
still rides the publication.

### Current text at HEAD

```console
$ sed -n '33p' packages/org.vibevm.ai-native/rust-ai-native-lang/v0.7.0/spec/cards/scaffold-i-codemods.xml
##EVIDENCE-AND-TRANSFER-STRENGTH Evidence & Transfer-strength: first-principles from R3-013 (ownership graph bounds throughput) + R2C-006 (edit size drives Rust failure) + DL1-015 (constraints lift weak models). NOT in the follow-up. Class: theory. Tag: **[E-hyp]**. @spec/done
```

### The measurement

```console
$ grep -c "DL1-" packages/org.vibevm.ai-native/core-ai-native/v0.8.0/spec/appendix/ATLAS.xml
0
$ grep -n "DR1-015" …/ATLAS.xml
181:- ##FINDING-DR1-015 **DR1-015** — Constrained decoding helps weak models most; can hurt strong ones
```

**The ATLAS carries no `DL1-` prefix at all**, and the record whose title the
card glosses («constraints lift weak models») is `DR1-015`. The **Go copy of the
same card already writes `DR1-015`** and is judged `confirmed`, so the intended
referent is not in doubt:

```console
$ grep -n "EVIDENCE-AND-TRANSFER-STRENGTH" packages/…/{go,rust,typescript}-ai-native-lang/v*/spec/cards/scaffold-i-codemods.md
go   :33  … + R2C-006 (edit size drives failure) + DR1-015 (constraints lift weak models). …
rust :33  … + R2C-006 (edit size drives Rust failure) + DL1-015 (constraints lift weak models). …
ts   :33  … + R2C-006 (edit size drives failure) + DL1-015 (constraints lift weak models). …
```

And the same card's own `##MOTIVATION` (`:19`, item 3 above) writes `DR1-015`
correctly **fourteen lines up**, in all three stacks — the card contradicts
itself within one screen.

### Proposed correction (NOT APPLIED)

```
##EVIDENCE-AND-TRANSFER-STRENGTH Evidence & Transfer-strength: first-principles from R3-013 (ownership graph bounds throughput) + R2C-006 (edit size drives Rust failure) + DR1-015 (constraints lift weak models). NOT in the follow-up. Class: theory. Tag: **[E-hyp]**. @spec/done
```

One character. Anchor id unchanged, `@spec/done` unchanged, the single-line shape
unchanged. **Design choice owed: NO.**

### The TypeScript twin carries the identical typo, is judged `confirmed`, and carries no obligation

Read from `run/cache.json` **as an instrument**, per
[§6.1's first cheap check](../PHASE-D-BATCH-PLAN.md#delegation-lessons):

```console
$ python -c "…run/cache.json… scaffold-i-codemods … EVIDENCE-AND-TRANSFER-STRENGTH"
go-ai-native-lang           -> confirmed
rust-ai-native-lang         -> drift        ← F-186's only anchor
typescript-ai-native-lang   -> confirmed    ← same `DL1-015`
```

**A `confirmed` verdict resting on the string a `drift` verdict convicts**, and
the measurement above says the `confirmed` one is wrong. Both slot copies already
ship it:

```console
$ grep -n "DL1-015" vibedeps/stack-{rust-ai-native-lang/0.7.0,typescript-ai-native-lang/0.6.0}/spec/cards/scaffold-i-codemods.md
rust …/spec/cards/scaffold-i-codemods.md:18  … + DL1-015 (constraints lift weak models). …
ts   …/spec/cards/scaffold-i-codemods.md:18  … + DL1-015 (constraints lift weak models). …
```

**Consequence for the approval, stated and not decided:** publishing the Rust fix
alone ships one repaired card and one byte-identical broken one into the same
family — §4.5's «a fix landed in one consumer and not the others is not a
closure». The TypeScript copy takes the identical one-character fix, **but it
needs its `confirmed` verdict re-judged first** — it cannot be re-judged against
a change to a different package's card.

---

## 6. F-219 residue — `##COMPOSES-ATOMIC-COMMITS` in the addressable-specs README {#f-219}

**Queue row** ([§C](../PHASE-D-RELEASE-QUEUE.md#composes)): «the campaign-plans
half FELL … the addressable-specs half **stands restated**: the misattribution is
real (`git-atomic-commits`' own boot `:22` delegates format to
`git-conventional-commits`) — single-package now, left the route.» **1 anchor ·
one row rewritten · a design choice on its shape.**

*(Line-number correction, and it matters because the diff will be read against
it: at HEAD the delegating sentence is at `:26`, not `:22` — `:22` is a blank
line and `:21` is the `## Message format {#message-format}` heading. The `:22`
figure is the queue quoting the original verdict; the installed copy's offset is
the known Phase B markup difference,
[§3.5](../PHASE-D-BATCH-PLAN.md#vendored), and carries no drift signal.)*

### Current text at HEAD

```console
$ sed -n '64,65p' packages/org.vibevm.world/addressable-specs/v0.1.0/README.md
- ##COMPOSES-ATOMIC-COMMITS `flow:git-atomic-commits` — commit bodies cite `spec://` URIs; this
  package defines what those URIs resolve to. @impl/done
```

Its neighbours in `## Composition {#composition}` (`:59-72`), read before
drafting per
[`##READ-FURTHER-BEFORE-SEARCHING-WIDER`](../PHASE-D-BATCH-PLAN.md#delegation-lessons):
`##COMPOSES-TWO-PROCESS-MODEL`, this row, `##COMPOSES-CONFLICT-PROTOCOL`,
`##COMPOSES-WAL`, `##COMPOSES-DECISION-RECORDS`. **`git-conventional-commits` is
not in the list at all** — so the README does not merely name a second true
sibling; it routes this fact to the one sibling that disclaims it and omits the
one that owns it.

### The measurement

**The rule is authored in `git-conventional-commits`:**

```console
$ grep -rn "spec://" packages/org.vibevm.world/git-conventional-commits/
…/spec/boot/31-flow-conventional-commits.md:24:##CITE-SPEC-URIS-WHERE-RELEVANT Cite `spec://…` URIs where relevant. @impl/done
…/spec/flows/conventional-commits/conventional-commits.md:75:  measurement, or conversation that drove it. Use `spec://…` URIs
…/spec/flows/conventional-commits/conventional-commits.md:142:Cited by spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-001#freshness.
```

with the full rule at `conventional-commits.xml:74-77` (`##INCLUDE-WHY-THIS-CHANGE-WAS-MADE`).

**`git-atomic-commits` carries no citation rule, and disclaims the class in its
own snippet:**

```console
$ grep -rn "spec://" packages/org.vibevm.world/git-atomic-commits/
…/spec/boot/30-flow-atomic-commits.md:24:  `spec://org.vibevm.world/git-conventional-commits/…#root`. @impl/done
…/spec/flows/atomic-commits/ATOMIC-COMMITS-PROTOCOL.md:78: (`spec://org.vibevm.world/decision-records/…#root`)
…/spec/flows/atomic-commits/splitting-large-changes.md:96: `git-conventional-commits` flow: `spec://…#root`. @impl/done
```

**All three are pointers to siblings; none is a citation rule.** And the snippet
draws the line the composition row erases:

```console
$ sed -n '21,26p' packages/org.vibevm.world/git-atomic-commits/v0.1.0/spec/boot/30-flow-atomic-commits.xml
21  ## Message format {#message-format}
23  ##COMMIT-MESSAGES-FOLLOW-THE-CONVENTIONAL-COMMITS-FLOW Commit messages follow the **git-conventional-commits** flow — a sibling package:
24  `spec://org.vibevm.world/git-conventional-commits/flows/conventional-commits/conventional-commits#root`. @impl/done
26  ##CONVENTIONAL-COMMITS-IS-THE-FORMAT-THIS-FLOW-IS-THE-ATOMICITY Conventional Commits is the *format*; this flow is the *atomicity* (one commit, one idea). @impl/done
```

**Both flows are installed and pinned** (`vibe.lock:293,296`), so the
misattribution is checkable against the shipped payload.

**The behaviour itself is real, and the figure is re-measured at this HEAD rather
than carried** ([`##THE-CAMPAIGN-IS-INSIDE-ITS-OWN-CORPUS`](../PHASE-D-BATCH-PLAN.md#delegation-lessons):
this campaign is the dominant contributor to the practice it measures, so a
figure names its HEAD):

```console
$ git log --grep="spec://" --oneline | wc -l   →  582
$ git log --oneline | wc -l                     →  2219
   at HEAD = 3c14d6af
```

**582 of 2 219** — d8c measured 579 of 2 212 at `f2b11b0a`, seven commits back.
Neither figure is «716», which was a **line** count mis-read as a commit count.

**The model for the fix is already live**, because the identical misattribution in
a sibling world package was routed §3.6(a) and closed as F-253 in wave 3:

```console
$ sed -n '144,148p' packages/org.vibevm.world/sync-from-code/v0.1.0/spec/flows/sync-from-code/when-to-apply.xml
- ##BOUNDARY-FLOW-ATOMIC-COMMITS **`flow:git-atomic-commits`** handles commit discipline: one sync,
  one commit, one logical idea. The message *format* — Conventional
  Commits, with `docs(spec)` as the type a sync commit carries — is
  defined by the sibling `flow:git-conventional-commits`, not by the
  atomicity flow and not here. @impl/done
```

### Proposed correction (NOT APPLIED) — THREE shapes, and the choice is real

The anchor id `##COMPOSES-ATOMIC-COMMITS` is **immutable** (`RULE-ANCHORS-IMMUTABLE`),
and that is precisely what makes this a choice rather than a rewrite: option A
leaves an anchor named `ATOMIC-COMMITS` on a row that no longer mentions that
flow.

**Option A — re-point the row at the flow that owns the rule.** Minimal; the row
becomes true in one edit.

```
- ##COMPOSES-ATOMIC-COMMITS `flow:git-conventional-commits` — commit bodies cite `spec://` URIs
  (`##CITE-SPEC-URIS-WHERE-RELEVANT`); this package defines what those
  URIs resolve to. @impl/done
```

*Cost:* the anchor id contradicts its own row's subject, and
`git-atomic-commits` leaves the Composition list entirely — a flow this package
genuinely does compose with (one commit, one idea, so a spec change and its code
land as one citable unit) is simply dropped.

**Option B — keep the subject, draw the line.** Mirrors the F-253 repaired
wording exactly, keeps the anchor id honest, and states both flows in one row.

```
- ##COMPOSES-ATOMIC-COMMITS `flow:git-atomic-commits` — one commit, one logical idea, so a spec
  change and the code that satisfies it land as one citable unit. The
  rule that commit bodies cite `spec://` URIs is the sibling
  `flow:git-conventional-commits`', which owns the message *format*;
  this package defines what those URIs resolve to. @impl/done
```

*Cost:* one row now carries two flows, which reads slightly against the
section's one-row-per-flow shape.

**Option C — correct the row AND add the missing sibling.** The only shape that
leaves the Composition list complete.

```
- ##COMPOSES-ATOMIC-COMMITS `flow:git-atomic-commits` — one commit, one logical idea, so a spec
  change and the code that satisfies it land as one citable unit. @impl/done
- ##COMPOSES-CONVENTIONAL-COMMITS `flow:git-conventional-commits` — commit bodies cite `spec://` URIs;
  this package defines what those URIs resolve to. @impl/done
```

*Cost, and it is procedural rather than editorial:* a **new anchor changes the
document's anchor set**, which is exactly [§3.1's «Revisit
when»](../PHASE-D-BATCH-PLAN.md#closure) — `vibe progress mirror` must run
**before** `merge-verdicts.py`, or the merge refuses anchors the mirror has not
seen.

**Design choice owed: YES.** All three are factually correct after the edit; they
differ in what the Composition section ends up saying about this package's
dependencies. **Nothing is decided here.**

---

## 7. F-212 residue — `##RATCHET-CENSUS-REGRESSIONS` in the go sweep skill {#f-212}

**Queue row** ([§B](../PHASE-D-RELEASE-QUEUE.md#stacks)): «rust FELL … **the go
half survives restated — the collector emits no per-kind, per-package census at
all, and three of its five names mismatch shipped kinds** — single-package now,
left the route.» **1 anchor.** `falsifier: self` — the package's own skill against
the package's own binary, so [§3.8](../PHASE-D-BATCH-PLAN.md#audience) is not
engaged in either direction.

**Modelled on [`d8b`'s F-190 correction](d8b-stacks-audience-release-reverify.md#f-190)**:
where a step tells a reader to look for a token, the corrected step **quotes the
line the tool actually prints**.

### Current text at HEAD

```console
$ sed -n '79,83p' packages/org.vibevm.ai-native/go-ai-native-lang/v0.1.0/spec/skills/go-ai-native-sweep/SKILL.md
5. ##RATCHET-CENSUS-REGRESSIONS **census regressions** (`init_in_cell` / `ambient_call_in_cell` /
   `naked_go_in_cell` / `error_string_match` / `seam_error_missing_req`
   non-zero on a gated package) — drain immediately; restructure beats
   testify. On an ungated package they are the adoption backlog: **flip a
   package into `gated_packages` only after it drains to zero.** @impl/done
```

### The measurement — three defects, and only the first is a rename

**(i) Three of the five named kinds exist nowhere in the package.**

```console
$ for n in init_in_cell ambient_call_in_cell naked_go_in_cell init_decl blank_import \
           ambient_call naked_go error_string_match seam_error_missing_req; do
    grep -rn "$n" …/go-ai-native-lang/v0.1.0/{crates,tools} | grep -v /target/ | wc -l ; done
init_in_cell             hits=0
ambient_call_in_cell     hits=0
naked_go_in_cell         hits=0
init_decl                hits=8
blank_import             hits=7
ambient_call             hits=13
naked_go                 hits=7
error_string_match       hits=9
seam_error_missing_req   hits=7
```

The shipped vocabulary is `init_decl | blank_import | ambient_call | naked_go |
error_string_match | seam_error_missing_req`
(`crates/vendor/core-ai-native-conform/src/rules/go.rs:110-130`). **Two of the
five named strings are correct; three name nothing.**

**(ii) `gated_packages` names nothing at either level.**

```console
$ grep -rn "gated_packages" …/go-ai-native-lang/v0.1.0/ | grep -v /target/
spec/go/GUIDE-AI-NATIVE-GO.md:626
spec/go/tools/conform-frontend-go.md:110
spec/skills/go-ai-native-sweep/SKILL.md:83        ← this anchor
   — three documents, zero code.
```

The gating list is the shared top-level `gated_crates`
(`crates/vendor/core-ai-native-conform/src/config.rs:44`), and it is **not** a
`[go]`-table alias — `GoConfig` (`config.rs:106-140`) carries `roots`,
`exclude_substrings`, `cells_dir`, `seams_pkg`, `registry_pkg`, `floor_disable`
and **no gating key of its own**.

**(iii) The step asks for a reading the collector does not produce, and this is
what makes a rename insufficient.** «non-zero **on a gated package**» requires a
per-kind, per-package split. The Go health snapshot carries **two aggregate
integers over all `go_unsafe` facts**, split only by whether a reason is present:

```console
$ sed -n '141,144p' …/crates/go-ai-native-cli/src/health.rs
        "ban_census": {
            "reasoned": census_reasoned,
            "unreasoned": census_unreasoned,
        },
$ sed -n '92,98p' …/health.rs
go_ai_native_extract_bridge::RawFact::GoUnsafe { reason, .. } => {
    if reason.is_some() { census_reasoned += 1; } else { census_unreasoned += 1; }
}
```

**And here is the line the tool actually prints** — format string at
`crates/go-ai-native-cli/src/health.rs:161-174`, and the same line captured
verbatim from the package's own health run
(`campaigns/packages-2026-09/harvest/go-ai-native-lang-health.md:7`, cited as
**captured run output of this package's binary**, not as a campaign finding):

```
health: 5 file(s) in scope; 1 over budget, 0 in the danger band; ban census 1 reasoned / 18 unreasoned; 0/11 exports carry Examples; orphan backlog 0. Snapshot at discipline/health/latest-go.json.
```

**No kind name appears in it, and no package name.** So renaming `init_in_cell` →
`init_decl` would leave the sentence still unobservable — merely differently
wrong. The corrected step must name what the output can match.

### Proposed correction (NOT APPLIED)

```
5. ##RATCHET-CENSUS-REGRESSIONS **census regressions** — `go-ai-native health`'s printed
   `ban census {N} reasoned / {M} unreasoned` (and `ban_census` in the
   snapshot): every `go_unsafe` fact without a `//spec:deviates` reason
   counts unreasoned. The kinds behind the count are `init_decl`,
   `blank_import`, `ambient_call`, `naked_go`, `error_string_match` and
   `seam_error_missing_req`; the collector reports one project-wide total,
   **not** a per-kind or per-package split, so compare the figure against
   the previous run rather than expecting a breakdown. Drain immediately;
   restructure beats testify. Outside the gate they are the adoption
   backlog: **flip a package into `gated_crates` only after it drains to
   zero.** @impl/done
```

Anchor id unchanged, `@impl/done` unchanged, numbered-item shape and 3-space
continuation indent unchanged. **Design choice owed: NO** — the step names only
strings the shipped output and the shipped config carry.

*(One friction worth stating rather than hiding: the config key is
`gated_crates` even in the Go stack, because it is the shared top-level key, not
a Go-specific one. The corrected sentence therefore says «flip a **package** into
`gated_crates`», which reads slightly odd and is nonetheless exactly what the
tool requires.)*

### Two sibling sentences carry the same wrong key — one of them is `confirmed`

Neither is in this obligation, and neither is repaired by the block above:

```console
$ sed -n '626p' …/spec/go/GUIDE-AI-NATIVE-GO.md
- ##SWEEP-FLIP-ONLY-AFTER-DRAIN **Flip-only-after-drain:** a package enters `gated_packages` only at zero findings;

$ sed -n '110p' …/spec/go/tools/conform-frontend-go.md
`registry_pkg` (default `internal/registry`), `gated_packages` /
```

The GUIDE row is judged **`confirmed`** while the SKILL row it cites as support
is judged **`drift`**:

```console
$ python -c "…run/cache.json…"
spec/go/GUIDE-AI-NATIVE-GO.md      | SWEEP-FLIP-ONLY-AFTER-DRAIN -> confirmed
spec/skills/go-ai-native-sweep/SKILL.md | RATCHET-CENSUS-REGRESSIONS -> drift
```

Per [§6.1's first cheap check](../PHASE-D-BATCH-PLAN.md#delegation-lessons) one of
the two attributions is wrong, and the measurement says it is the `confirmed`
one. **If only the SKILL line is published, the GUIDE keeps telling Go adopters
to edit a key that does not exist, and the `confirmed` verdict cites a line that
no longer says what it cited.** The GUIDE and tool-doc fix is the same two-word
`gated_packages` → `gated_crates` swap, **but it needs the GUIDE's `confirmed`
verdict re-judged first.** Listed, not drafted — the owner decides whether the
approval covers all three sentences.

---

## 8. F-115 residue — `##AGG-FRONT-DOOR` in the typescript-ai-native umbrella README {#f-115}

**Queue row** ([§B](../PHASE-D-RELEASE-QUEUE.md#stacks)): «go and rust FELL …
**The TypeScript half is real** — `typescript-ai-native-lang` is the only one of
the 42 shipped versions with no `README.md`, never in git history — and its
closure is a **build** (write the README) **or a repoint**.» Confirmed against
the registry — F-115 is now a one-anchor `prose-edit` row:

```console
$ python -c "…run/state/obligations.json…"
AGG-FRONT-DOOR -> [('F-115', 'prose-edit', 'open',
  ['packages/org.vibevm.ai-native/typescript-ai-native/v0.6.0/README.md#AGG-FRONT-DOOR'])]
```

**1 anchor · TWO OPTIONS · nothing decided.**

### Current text at HEAD

```console
$ sed -n '22,24p' packages/org.vibevm.ai-native/typescript-ai-native/v0.6.0/README.md
##AGG-FRONT-DOOR The consumer front door — wiring, floor, sweep — is
documented in the `-lang` package's README and
`spec/typescript/GUIDE-AI-NATIVE-TYPESCRIPT.md`. @impl/done
```

The file carries `<status stage="doc" state="done" audience="user"/>` at `:3`;
the anchor carries `@impl/done`.

### The measurement

```console
$ for p in go-ai-native-lang/v0.1.0 rust-ai-native-lang/v0.7.0 typescript-ai-native-lang/v0.6.0; do
    f="packages/org.vibevm.ai-native/$p/README.md"; [ -e "$f" ] && echo "EXISTS $f" || echo "MISSING $f"; done
EXISTS  …/go-ai-native-lang/v0.1.0/README.md
EXISTS  …/rust-ai-native-lang/v0.7.0/README.md
MISSING …/typescript-ai-native-lang/v0.6.0/README.md

$ ls -a packages/org.vibevm.ai-native/typescript-ai-native-lang/v0.6.0/ | grep -v '^target$'
.  ..  Cargo.lock  Cargo.toml  LICENSE.xml  crates  spec  tools  vibe.toml
```

**The sentence's first target does not exist**, and the second does
(`spec/typescript/GUIDE-AI-NATIVE-TYPESCRIPT.md`, verified in
[item 1's table](#f-153)). Both halves of the correction therefore turn on one
missing file.

### Option (b) — repoint the sentence at the file the package does ship

*(Drafted first because it is the smaller change; the order is not a
recommendation.)*

```
##AGG-FRONT-DOOR The consumer front door — wiring, floor, sweep — is
documented in the `-lang` package's
`spec/typescript/GUIDE-AI-NATIVE-TYPESCRIPT.md` (§15 wiring, §16 sweep
idioms). @impl/done
```

The section numbers are verified rather than copied from the Rust sibling —
**they differ, and this is exactly where a family-wide edit would go wrong:**

```console
$ grep -nE "^## " …/typescript-ai-native-lang/v0.6.0/spec/typescript/GUIDE-AI-NATIVE-TYPESCRIPT.md
276:## 15. Wiring a consumer (the shipped toolchain) *(≈ Rust §13)* {#wiring}
292:## 16. Sweep idioms *(≈ Rust §14)* {#sweep}
```

**Cost of (b):** three-word edit, closes the anchor immediately, no new file, no
new anchors, no mirror run. **But** it ratifies a package with no front door and
leaves the family saying two different things — go and rust point at a `-lang`
README, typescript does not. It also loses the «Running the tools» / lifecycle
material that has no home in the GUIDE.

### Option (a) — write the missing `typescript-ai-native-lang/v0.6.0/README.md`

Then the umbrella sentence needs **no edit at all** — it becomes true as written.

**Every claim below was verified against the TS package's own tree**, and the
places where it diverges from the Rust model are named after the draft rather
than silently smoothed over.

*(Fenced with four backticks because the file itself contains `sh` fences.)*

````markdown
# AI-Native TypeScript (stack:org.vibevm.ai-native/typescript-ai-native-lang) {#root}

<status stage="doc" state="done" audience="user"/>

##TYPESCRIPT-PROJECTION-SHIPS-A-RUNNABLE-TOOLCHAIN The TypeScript projection of the AI-Native Code Discipline — and the
**runnable toolchain** that enforces it (PROP-024 code-bearing packages):
installing this stack yields working checkers and procedures, not
descriptions of them. @impl/done

##NEUTRAL-METHOD-COMES-FROM-THE-CORE-DEPENDENCY The language-neutral method (manifesto, playbooks, mechanism specs)
comes from its dependency `flow:org.vibevm.ai-native/core-ai-native`. @impl/done

## What ships {#what-ships}

- ##SHIPS-FOUR-BINARIES **Four binaries** (this package's own Cargo workspace, `crates/`,
  declared as `[[binary]]` in `vibe.toml` for PROP-025 lockfile
  dispatch): @impl/done
  - ##SHIPS-TYPESCRIPT-AI-NATIVE-UMBRELLA `typescript-ai-native` — the umbrella tool: `init` (bootstrap
    policies + registries), `floor` (the portable verification floor —
    prettier → tsc → tests → eslint → conform → specmap → test-gate, one
    exit code), `conform`, `specmap`, `trace`, `test-gate`, `tripwire`,
    `health`, `fast-loop`, `codemod`. @impl/done
  - ##SHIPS-TYPESCRIPT-AI-NATIVE-CONFORM `typescript-ai-native-conform` — the conformance gate alone (ENGINE-CONFORM). @impl/done
  - ##SHIPS-TYPESCRIPT-AI-NATIVE-SPECMAP `typescript-ai-native-specmap` — the traceability engine alone (PROP-014). @impl/done
  - ##SHIPS-TYPESCRIPT-AI-NATIVE-TCG `typescript-ai-native-tcg` — the agentic type oracle (TCG-ORACLE-v0.1 /
    TCG-PROTOCOL-v0.1): a persistent enriching `serve` relay for MCP
    hosts plus one-shot `validate` / `scope` / `complete` / `type` /
    `bench`, answered by the CONSUMER's own `typescript` install over
    in-memory overlays with the gate's own conform rules merged in.
    **Prerequisite:** node ≥ 22.6 and the project's own `typescript`
    devDependency — the same install the `tsc` floor step needs, so the
    oracle adds no new dependency. The floor stays the truth; the oracle
    exists so the floor stays green on the first try. @impl/done
- ##SHIPS-GUIDE-AND-CARDS **The TypeScript guide and cards**
  (`spec/typescript/GUIDE-AI-NATIVE-TYPESCRIPT.md`, `spec/cards/` — the
  nine scaffolds A–I in their TypeScript shape, Band-3 ops blocks for
  weak readers). @impl/done
- ##SHIPS-TWO-AGENT-SKILLS **Two agent skills** (`vibe skill install` projects them):
  `/typescript-ai-native-terraform` (brownfield adoption per BROWNFIELD-PROTOCOL)
  and `/typescript-ai-native-sweep` (the recurring sweep per the Sweep
  Playbook). @impl/done
- ##SHIPS-TWO-NODE-SIDE-TOOLS **Two node-side tools** (`tools/`, run directly under node ≥ 22.6
  type-stripping): `ts-extract`, the Compiler-API fact extractor the
  conform frontend and the specmap scanner drive; and `ts-oracle`, the
  long-lived language-service oracle behind `typescript-ai-native-tcg`.
  Both resolve the CONSUMER project's `typescript` at runtime. @impl/done
- ##SHIPS-MECHANISM-SPECS **The TypeScript mechanism specs and tool briefs** —
  `spec/typescript/mechanisms/TCG-ORACLE-v0.1.md` and
  `TCG-PROTOCOL-v0.1.xml`, plus `spec/typescript/tools/`. @impl/done
- ##NEUTRAL-ENGINES-RIDE-ALONG-VENDORED **The neutral engines ride along as vendored copies**
  (`crates/vendor/core-ai-native-{conform,specmap,specmark,specmark-grammar}`),
  so the slot is its own Cargo workspace and builds standalone. @impl/done

## Running the tools {#running-the-tools}

##running-forms-lead Three supported forms, from your project root (where `vibedeps/` is): @impl/done

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

##STACK-SLOT-IS-THE-MATERIALISED-DIRECTORY `<stack-slot>` is this package's materialised directory (e.g.
`stack-typescript-ai-native-lang/0.6.0` — check your `vibe.lock`). @impl/done

##SLOT-BUILD-DROPS-A-TARGET-DIRECTORY Building in the
slot drops a `target/` there; add `vibedeps/**/target/` to your
`.gitignore` (build output is already excluded from the package's content
hash, PROP-024 §2.2). A repo that also carries Rust keeps
`[workspace] exclude = ["vibedeps"]`. @impl/done

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

##wiring-and-sweep-pointers The wiring recipe — install, binaries, project toolchain, bootstrap,
the generation-time oracle — is GUIDE §15; the sweep idioms are GUIDE
§16. @impl/done

##POLICIES-STAY-WITH-THE-CONSUMER-PROJECT The policies (`conform.toml`, `specmap.toml`) stay with YOUR project:
this package ships engines, never policy (PROP-024 §2.2). @impl/done
````

**Where this deliberately diverges from the Rust model, and why — each checked,
not assumed:**

| rust README has | TS draft | why |
|---|---|---|
| `##SHIPS-SPECMAP-WIRE-SCHEMA` (`schemas/specmap.jtd.json`) | **dropped** | `ls …/typescript-ai-native-lang/v0.6.0/schemas` → no such directory |
| `##SHIPS-SPECMARK-PROC-MACRO` («the `#[spec]`/`scope!` tags your code carries») | **replaced** by `##NEUTRAL-ENGINES-RIDE-ALONG-VENDORED` | TS carries `spec://` URIs in **JSDoc tags** (`/** @implements spec://… */`), GUIDE §9 `##SPEC-URIS-CARRIED-BY-JSDOC-OR-DECORATORS` — there is no proc-macro to import, though `crates/vendor/core-ai-native-specmark{,-grammar}` are vendored for the Rust-side parsers |
| «rustup component add rust-analyzer» prerequisite | **node ≥ 22.6 + the project's own `typescript`** | `tools/ts-oracle/package.json` («Run directly with node >= 22.6 … consumer-resolved typescript»); boot snippet `##STRUCTURAL-GATE-PARSES-THROUGH-THE-PROJECT-TYPESCRIPT` |
| GUIDE **§13 / §14** for wiring / sweep | GUIDE **§15 / §16** | the TS guide's own headings (`:276`, `:292`), which carry «*(≈ Rust §13)*» / «*(≈ Rust §14)*» in their own titles |
| — | **`##SHIPS-TWO-NODE-SIDE-TOOLS`** added | `tools/ts-extract` + `tools/ts-oracle` have no Rust counterpart and are half the toolchain |

**Cost of (a):** a new ~90-line file with **~18 new anchors**. Two procedural
consequences, both from [§3.1's «Revisit when»](../PHASE-D-BATCH-PLAN.md#closure):
the document set changes, so `vibe progress mirror` must run **before**
`merge-verdicts.py`; and the new file arrives **unjudged**, so it enters the next
wave's surface rather than shrinking this one. §3.3 also says «Phase D does not
write the mechanism» — a README is prose rather than a mechanism, but writing a
missing artefact is nearer a build than an edit, which is why
[`d8a`](d8a-stacks-package-own-release-reverify.md) flagged it as a Phase E item
the `release` gate should carry.

**What the corpus argues, stated as evidence and not as a decision.** 42 shipped
package versions; 41 carry a `README.md`; the one that does not is the one this
sentence points at — and it is not a deletion:

```console
$ git log --oneline --all -- "packages/org.vibevm.ai-native/typescript-ai-native-lang/*/README.md"
(no output)
```

**Design choice owed: YES.** (a) closes the anchor without editing a true
sentence and leaves the three aggregators saying the same thing, at the cost of a
new unjudged file; (b) is three words and ratifies a package with no front door.
**Nothing is decided here.**

### By-catch, relevant because option (a) would model on it

`spec/typescript/GUIDE-AI-NATIVE-TYPESCRIPT.md`'s `##WIRE-5-GENERATION-TIME-ORACLE`
(`:284`) still describes the **retired** topology — «the `tcg_*` MCP tools
(`vibe mcp serve`; vibevm PROP-026) hold a persistent oracle per language» —
which is the same defect [`d8b`'s F-189](d8b-stacks-audience-release-reverify.md#f-189)
convicts three `##COMPONENT-THE-PRODUCT-SEAM` rows for. It is judged
**`confirmed`** and carries **no obligation**:

```console
$ python -c "…run/state/obligations.json…"  WIRE-5-GENERATION-TIME-ORACLE -> []
$ python -c "…run/cache.json…"              …GUIDE-AI-NATIVE-TYPESCRIPT.xml | WIRE-5-GENERATION-TIME-ORACLE -> confirmed
```

Recorded so a README written under option (a) does not copy it, and so the owner
can decide whether it rides the F-189 diff.

---

## 9. F-245 — the read-once claim in the qualified-naming boot snippet {#f-245}

**Registry row** (`run/state/obligations.json`, `prose-edit`, `open`, 2 anchors,
`falsifier: mixed`). **1 anchor is this item's; the other is the address
family's, and the difference is measured rather than assumed** — per
[`d8b`'s closing rule](d8b-stacks-audience-release-reverify.md#summary), «before
any strike-by-ruling, check each anchor's **own** recorded reason, never the
row's»:

| anchor | its own recorded reason | belongs to |
|---|---|---|
| `#IT-IS-A-DESIGN-DISCIPLINE-NOT-A-RUNTIME-RULE` | the row's `reason`: «self-defeating by construction … it sits inside `spec/boot/STATIC.xml` … so it IS read every session» | **this item** |
| `#fork-by-fork-rationale-pointer` | `reasons[0]`: «DRIFT, third of three. **The target exists** and carries four forks … the `../flows/…` link resolves nowhere in the host for the same reason as its two siblings» | **the address family** — [§A.1](../PHASE-D-RELEASE-QUEUE.md#addresses-scope) counts F-245 as «1 of 2» on a repaired link; its diff is `tasks/address-repair.py`, **out of scope here** |

So: **1 anchor drafted · 1 anchor already covered by the address transformation ·
a real design choice on the one drafted.**

### Current text at HEAD

```console
$ sed -n '9,10p' packages/org.vibevm.world/qualified-naming/v0.1.0/spec/boot/67-flow-qualified-naming.xml
##IT-IS-A-DESIGN-DISCIPLINE-NOT-A-RUNTIME-RULE It is a design discipline, not a runtime
rule: read it once while shaping identifiers, not on every session. @impl/done
```

File-level `<status stage="impl" state="done"/>` at `:3`; anchor `@impl/done`.

### The measurement

**(i) The sentence is in the compiled lane, and the lane is read first and in
full.**

```console
$ sed -n '1001,1008p' spec/boot/STATIC.xml
1001  <!-- vibe:static org.vibevm.world/qualified-naming — vibedeps/flow-qualified-naming/0.1.0/spec/boot/67-flow-qualified-naming.md -->
1003  # Flow: Qualified Naming {#root}
1005  This project ships the **qualified-naming** practice for *ecosystem
1006  designers* — anyone defining a namespace for packages, plugins,
1007  extensions, or artifacts. It is a design discipline, not a runtime
1008  rule: read it once while shaping identifiers, not on every session.
```

`CLAUDE.md`'s generated boot block instructs every session: «`spec/boot/STATIC.xml`
— … The static (priority) lane: **read it first and in full**.»

**(ii) The manifest declares no condition, and the field it would use is named
`when`.**

```console
$ cat packages/org.vibevm.world/qualified-naming/v0.1.0/vibe.toml
…
[boot_snippet]
source = "spec/boot/67-flow-qualified-naming.md"
category = "flow"
```

The grammar is `BootSnippet` in `crates/vibe-core/src/manifest/package.rs:500-520`
— `source`, `category`, `link`, and:

```rust
/// Activation condition (PROP-009 §2.4 / §2.6). When set, the snippet
/// is a conditional contribution: the computed-view engine renders it
/// as a `dynamic` `INDEX.md` entry — regardless of `link` — carrying
/// this condition, and the agent reads the file at boot only when it
/// holds. For v1 the only condition is an OS match (`when = "os:linux"`).
#[serde(default, skip_serializing_if = "Option::is_none")]
pub when: Option<WhenCondition>,
```

**(iii) And this is the fact that decides how option (a) can be written: the
condition vocabulary is OS-only, by parser.**

```console
$ sed -n '36,39p;62,78p' crates/vibe-core/src/manifest/package/when.rs
pub enum WhenCondition {
    /// Activates only when the session's operating system is the named one.
    Os(TargetOs),
}
…
let Some(os_name) = s.strip_prefix("os:") else {
    return Err(Error::BadWhenCondition { reason: "unrecognised condition — expected `os:<name>`" });
};
match os_name { "windows" | "macos" | "linux" => …, other => Err(…) }
```

**There is no condition that expresses «while shaping identifiers».** Anything
but `os:windows` / `os:macos` / `os:linux` is a hard parse error. And **no
package in the tree uses `when` at all**:

```console
$ grep -rn "^when *=" packages/*/*/v*/vibe.toml vibedeps/*/*/vibe.toml
(no output)
```

**(iv) A dynamic link alone would not make the sentence true.** `spec/boot/INDEX.md`'s
own generated header (`:1-8`): «Read every file the `[[entry]]` list names, in
order. … A `kind = "dynamic"` entry: an INCLUDE resolved at boot — when it **also**
carries `when = "os:<name>"`, read the file only if …». Only the condition gates
the read.

**(v) The compiled-lane placement is the HOST's choice, not the package's — and
this was not in the recorded reason.** `qualified-naming` declares no `link`, so
it defaults to `LinkType::Dynamic` (`package.rs:375-376`), and the package's
`link` is «only a hint — the consumer's own `link` declaration always wins»
(`package.rs:509-512`). The host pulls it **transitively through redbook**, which
it links static:

```console
$ grep -n "redbook" vibe.toml
28:"flow:org.vibevm.world/redbook" = { version = "^0.2.0", link = "static-transitive" }

$ grep -n "qualified-naming" packages/org.vibevm.world/redbook/v0.2.0/vibe.toml
46:"flow:org.vibevm.world/qualified-naming" = "=0.1.0"
```

### The README twin's ruling — found, and it runs the other way

The row's reason ends «Ruled the same way as its README twin
`##IT-IS-A-DESIGN-TIME-DISCIPLINE-READ-ONCE`». **That twin is `confirmed`, and
the ground is precisely that it is NOT compiled** — `run/cache.json`, read as an
instrument:

```console
$ python -c "…run/cache.json… qualified-naming"
README.md | IT-IS-A-DESIGN-TIME-DISCIPLINE-READ-ONCE      -> confirmed
   ev: spec/boot/STATIC.xml  `grep -n 'design-time' spec/boot/STATIC.xml` returns nothing
       - the sentence is not in the compiled lane at all
   ev: packages/…/vibe.toml  [boot_snippet] source = "spec/boot/67-flow-qualified-naming.md"
       - the README is not the source of the boot snippet
67-flow-qualified-naming.xml | IT-IS-A-DESIGN-DISCIPLINE-NOT-A-RUNTIME-RULE -> drift
67-flow-qualified-naming.xml | fork-by-fork-rationale-pointer                -> drift
```

Re-run at HEAD: `grep -n "design-time" spec/boot/STATIC.xml` → no output (exit 1).

**So «ruled the same way» means the same TEST was applied — «does this sentence
reach the lane the reader is told to read first and in full?» — and it produced
opposite verdicts.** The README (`:19-21`) may keep its read-once claim because
nobody is compelled to read the README; the snippet may not, because everybody is
compelled to read the snippet. **Both options below preserve that asymmetry**;
neither proposes touching the README.

### Option (a) — route the snippet conditionally, so the claim becomes true

The field is **`[boot_snippet].when`**, and per its own doc comment it renders the
snippet as a `dynamic` `INDEX.md` entry **regardless of the consumer's `link`** —
so it would override the host's `static-transitive` pull through redbook, which
is exactly what this option needs.

```toml
[boot_snippet]
source = "spec/boot/67-flow-qualified-naming.md"
category = "flow"
when = "<a condition that does not exist yet>"
```

**Cost, and it is the decisive fact:** measurement (iii) shows the vocabulary is
OS-only by parser. There is no `when` value that means «while shaping
identifiers», so **option (a) is not a manifest edit today — it is a host code
change** (extend `WhenCondition` beyond `Os`, and define what evaluates it),
which is Phase E's lane, not Phase D's
([§3.3](../PHASE-D-BATCH-PLAN.md#demote): «Phase D does not write the
mechanism»). Taking (a) means the anchor stays open across the D exit gate with a
Phase E DRIFT task recorded — and it would be the **first** `when` in the corpus.

### Option (b) — the sentence stops claiming read-once-ness

```
##IT-IS-A-DESIGN-DISCIPLINE-NOT-A-RUNTIME-RULE It is a design discipline, not a runtime
rule: it binds the moment an identifier is minted, not every edit that
uses one. @impl/done
```

Anchor id unchanged, `@impl/done` unchanged, ~70-char wrap unchanged. **The
design-vs-runtime distinction — the sentence's actual load — survives intact;
only the read-frequency claim, which the lane falsifies, is dropped.** Consistent
with the README twin: the README says «read once while shaping identifiers» and
stays `confirmed`, because the README is not in the lane.

**Cost:** the package loses a true and useful instruction to its reader, on
account of a lane placement the **consumer** chose (measurement (v)).

### The which-side question both options sit on top of, surfaced and not answered

[§6.1's capability/practice/rule test](../PHASE-D-BATCH-PLAN.md#delegation-lessons)
asks what genre this sentence is. «Read it once … not on every session» is
addressed to a **reader** — it is a **rule**, and «a rule the consumer breaks is
[§3.6(b)](../PHASE-D-BATCH-PLAN.md#which-side), not a wrong sentence». Under that
reading neither option applies: the package does not move, a host obligation is
recorded (the host compiles a read-once flow into its always-read lane through
`link = "static-transitive"`), and the row goes `status: deferred` naming it. The
recorded reason's «self-defeating **by construction**» is the competing reading —
that a sentence compiled into the always-read lane is not merely unheeded but
false about itself.

**Design choice owed: YES**, and it is two-layered — first which side moves
(§3.6(a) edit vs §3.6(b) route-out vs §3.6(c) recorded host exception), then, if
the package moves, (a) or (b). **Nothing is decided here.**

---

## Rulings received — 2026-07-31, owner, in session {#rulings}

The four design choices were put to the owner with recommendations and all four
were ruled, verbatim: «1. F-188-rust - вариант ii  2 - B  3 - a  4. §3.6(a) +
вариант (b) "It is a design discipline, not a runtime rule: it binds the moment
an identifier is minted, not every edit that uses one"».

- **F-188-rust → (ii), no PROP-031 citation.** The §3b draft is FINAL as
  written; the d8a citation suggestion is declined (nonexistent anchor; would be
  the first live host pointer in an `ai-native` card, against §3.8's audience).
  Release route — the text lands with the publication batch.
- **F-219 → option B.** The two-flow row drawing the format/atomicity line,
  modelled on F-253's landed wording. Options A and C declined. Prose-edit
  route after wave-8 re-clustering — **applied and closed this session.**
- **F-115 → option (a).** The missing `typescript-ai-native-lang/v0.6.0/README.md`
  is written from the §8 draft (every claim tree-verified); the umbrella
  sentence needs no edit and re-judges `confirmed`. Option (b) declined.
  Prose-edit route — **applied and closed this session.** The new file arrives
  unjudged and enters the corpus at the next mirror pass, by design.
- **F-245 → §3.6(a) + option (b)**, with the owner supplying the exact
  sentence: «It is a design discipline, not a runtime rule: it binds the moment
  an identifier is minted, not every edit that uses one.» The which-side
  determination is (a) — the read-once promise is unsatisfiable in the shipped
  loading model (`when` is OS-only by parser; a dynamic entry without `when` is
  still read unconditionally), so the package's sentence was wrong about the
  world as shipped, and no Phase E `when`-vocabulary task is filed: a
  «while shaping identifiers» condition is not decidable at boot time, which is
  worth this record so Phase E does not inherit an unimplementable ask.
  Prose-edit route — **applied and closed this session.**

**The riders' boundary, checked before touching anything.** The two
`gated_packages` siblings split by route: `conform-frontend-go.xml`'s two
anchors belong to **open F-185 on `sync-from-code`** — its diff is the owner's
queue's, NOT touched here. `GUIDE-AI-NATIVE-GO.xml:626 ##SWEEP-FLIP-ONLY-AFTER-DRAIN`
and the F-186 TypeScript twin (`scaffold-i-codemods.xml:33`, `DL1-015`) are
**false confirms with no obligation** — treated verdict-first: re-judged
`drift` with measured reasons, minted by the registry, then closed under their
own obligations in the same session, so the repair passes through the
measurement instead of around it.

## The batch in one screen {#summary}

| # | item | anchors | diff size | design choice owed |
|---|---|---|---|---|
| 1 | [**F-153**](#f-153) | 6 — `#*-CODE-FOLLOWS-THE-*-GUIDE` ×3 · `#CARD-REGISTRY-FOR-*` ×3, in the go/rust/ts `20-stack-*-ai-native-lang.md` | 6 lines, one `spec/` prefix each; 3 files | **no** — the queue and the D4 precedent both fix the shape |
| 2 | [**F-211**](#f-211) | 2 — `#OP-INIT` in `TCG-PROTOCOL-GO-v0.1.xml` + `TCG-PROTOCOL-RUST-v0.1.xml` | 2 bullets rewritten (~7 lines each); 2 files; **not interchangeable** | **no** per the queue's ask — route (a) *build* named as the owner's override |
| 3 | [**F-188**](#f-188) | 3 — `#MOTIVATION` in the go/rust/ts `scaffold-i-codemods.xml` | 3 single lines, 3 different edits; 3 files | **yes, small** — whether the rust line cites PROP-031 (its `#beachhead` anchor does not exist; it would be the first live host pointer in an `ai-native` card) |
| 4 | [**F-251**](#f-251) | 2 — `#package-contents-lead` in the spec-genres + tool-design-lessons READMEs | one word each; 2 files | **no** |
| 5 | [**F-186**](#f-186) | 1 — `#EVIDENCE-AND-TRANSFER-STRENGTH`, rust `scaffold-i-codemods.xml:33` | **one character**; 1 file | **no** — but the ts twin carries the same typo, is `confirmed`, and needs re-judging before it can take the same fix |
| 6 | [**F-219**](#f-219) | 1 — `#COMPOSES-ATOMIC-COMMITS`, addressable-specs `README.md:64-65` | one composition row; 3 drafted shapes (2-line / 5-line / 2 rows) | **yes** — re-point vs name-both vs add-a-sibling-row (the third needs `vibe progress mirror` first) |
| 7 | [**F-212**](#f-212) | 1 — `#RATCHET-CENSUS-REGRESSIONS`, `go-ai-native-sweep/SKILL.md:79-83` | one 5-line step → ~11 lines; 1 file | **no** — but 2 sibling sentences keep `gated_packages`, one of them `confirmed` |
| 8 | [**F-115**](#f-115) | 1 — `#AGG-FRONT-DOOR`, typescript-ai-native `README.md:22-24` | **(a)** new ~90-line README, ~18 new anchors · **(b)** 3 words | **yes** — write the missing file vs repoint at the GUIDE |
| 9 | [**F-245**](#f-245) | 2 in the row; **1 drafted** — `#IT-IS-A-DESIGN-DISCIPLINE-NOT-A-RUNTIME-RULE` (`#fork-by-fork-rationale-pointer` is the address family's) | **(a)** a `when` that has no vocabulary yet → host code change · **(b)** one sentence, 2 lines | **yes, two-layered** — §3.6(a)/(b)/(c) first, then (a) vs (b) |

**Totals: 18 anchors drafted in 15 files · 4 items owe a design choice · 5 do
not.** Four riders are listed and deliberately **not** drafted, so one approval
can cover them on purpose rather than by accident: F-153's 8 unjudged twins
(item 1), F-186's TypeScript twin (item 5), F-212's GUIDE + tool-doc siblings
(item 7), and the TS GUIDE's `##WIRE-5-GENERATION-TIME-ORACLE` retired-topology
sentence (item 8's by-catch).

**Nothing above was applied.** Working tree at write-out: the one pre-existing
modification to `campaigns/packages-2026-09/OBLIGATIONS.md` this batch did not
touch, plus this file.
