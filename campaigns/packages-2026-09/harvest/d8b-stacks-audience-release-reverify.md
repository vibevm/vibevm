# D8b — re-verifying the `release`-route stack verdicts under the audience ruling

_Phase D, wave 8, batch D8b. Four obligations over 11 drift verdicts in
`go-ai-native-lang/v0.1.0`, `rust-ai-native-lang/v0.7.0` and
`typescript-ai-native-lang/v0.6.0`. All four close through
[`release`](../PHASE-D-BATCH-PLAN.md#routes) — **the owner gates publication** —
and all four lean, somewhere, on HOST-consumer evidence, which
[the owner's ruling of 2026-07-31](../PHASE-D-BATCH-PLAN.md#audience) voids for
Go and TypeScript. **A re-verdict that edits nothing produces no spec diff, and
that is the only basis on which this batch may run without the owner.** So:
**no package file was edited, no spec file was edited, no campaign state or
verdict JSON was written, and no `merge-verdicts.py` / `vibe progress seal` /
git write was run.** This file is evidence and a recommendation; every verdict
and every edit is the boss's._

**Measured at** `HEAD = f2b11b0a` (`fix(campaign): the registry snapshot on disk
was two waves stale, and it read as open work`, 2026-07-31), working tree clean
at batch start. Every count below names the command that produced it, per wave
6's lesson that a recorded figure decays.

**HEAD advanced mid-batch and the advance is accounted for**, per
[`##THE-CAMPAIGN-IS-INSIDE-ITS-OWN-CORPUS`](../PHASE-D-BATCH-PLAN.md#delegation-lessons)
(«this campaign is the dominant contributor to the practice it measures»). At
write-out `HEAD = 12640d7c`, three commits on — `5b8c9cb6`, `a49a74c1`,
`12640d7c`, all this campaign's or the host's own bookkeeping.
`git diff --name-only f2b11b0a..12640d7c` touches six files:
`campaigns/packages-2026-09/run/cache.json`, three `crates/vibe-{check,workspace}`
sources and their tests, and one `org.vibevm.world` flow document. **Not one of
them is a file this batch measured** — no `ai-native` package file, no
`spec/modules/vibe-mcp/`, no `crates/vibe-mcp/` or `crates/vibe-cli/`, no
`discipline/`. The cache diff (219 changed lines) mentions none of this batch's
four anchor names. **Every figure below therefore holds at both HEADs**, and the
working tree carried no modification at any point — only three untracked sibling
harvest files (`d8a`, `d8b`, `d8c`).

**Route check, run first per
[`##ROUTE-BEFORE-FALSIFIER`](../PHASE-D-BATCH-PLAN.md#delegation-lessons).** All
four report `closure_route: release` in `run/state/obligations.json`: **F-187**
(3 anchors, `falsifier: mixed`), **F-189** (3, `mixed`), **F-190** (3, `self`),
**F-213** (2, `mixed`) — 4 obligations · 11 anchors.

**The standing perimeter.** Unless an entry narrows it, every search was run from
the repository root over: `packages/**` **including
`packages/org.vibevm.fractality/**`** (the second complete project that adopted
this discipline — [§3.7's wave-6 extension](../PHASE-D-BATCH-PLAN.md#compliance-blindness)),
`vibedeps/**`, `crates/**`, `xtask/**`, `tools/**`, `spec/**`, `discipline/**`,
`terraform/**`, `research/**` (including `research/rust-demo`, `research/ts-demo`,
`research/go-demo`), `campaigns/**` **minus `campaigns/*/run/**`**, `fixtures/**`,
`schemas/**`, `docs/**`, `manual-tests/**` and the repository root's own `*.md` /
`*.toml` / `*.json` / `*.sh` / `*.ps1` — minus `legacy-spec/**` (owner ruling
2026-07-31: not evidence of practice in either direction), `**/target/**`,
`.git/**`, `**/node_modules/**` and `campaigns/*/run/**`. `refs/**` is
third-party and is reported separately where it was searched at all.
`campaigns/**` hits that are this campaign's own records are named as such and
are not used as evidence ([`##THE-CAMPAIGN-IS-INSIDE-ITS-OWN-CORPUS`](../PHASE-D-BATCH-PLAN.md#delegation-lessons)).
`campaigns/packages-2026-09/run/cache.json` was read as an **instrument** — to
recover each anchor's own verdict text and its neighbours' — never as evidence.

---

## §3.8 — the audience ruling this batch applies, stated in full {#audience}

_[Batch plan §3.8](../PHASE-D-BATCH-PLAN.md#audience) · [release queue §B.1](../PHASE-D-RELEASE-QUEUE.md#stacks-audience), owner ruling 2026-07-31._

1. **The `-lang` packages are built first and foremost for EXTERNAL consumers** —
   language support that VibeVM's *clients* use, in other projects, in code trees
   we cannot see. How they use it is unknown to us, and their absence here is not
   evidence of anything.
2. **`go-ai-native-lang` is a prototype specification, deliberately unused in this
   project, and it must stay unused.** So must `typescript-ai-native-lang` as an
   adopted stack. **These two can be checked only by their own artefacts and
   their TESTS** — their `crates/` (including `crates/vendor/`), their `spec/`,
   cards, skills, `vibe.toml`, the `tools/go-extract` and `tools/ts-extract`
   fixtures, and in-crate tests.
3. **Host evidence is VOID against a Go or TypeScript sentence**: `.claude/skills/`,
   `.agents/skills/`, `.opencode/skills/`, `vibe.lock`, `vibedeps/` slots, the
   host's PROP-026 dispatch roster, host floors, `research/ts-demo` as an adopted
   stack. A verdict that convicts a Go or TypeScript sentence because *this repo*
   does not do, dispatch, install or instantiate the thing **is measuring the
   wrong consumer and is false on that ground alone** — no widening of the
   perimeter fixes it, because the right consumer is not in the tree.
4. **`rust-ai-native-lang` is the exception.** Part of VibeVM itself is written in
   AI-Native Rust, so for the Rust stack the host genuinely is a consumer and host
   evidence counts normally. **Rust reasoning is never carried to Go or TypeScript
   and vice versa** — that is the parallel-corpus trap running in both directions.
5. **The caution that runs the other way, and it decides F-189.** If a sentence
   **explicitly asserts something about THIS host or its PROPs** — names
   `(vibevm, PROP-026)` as its subject, names the `vibe-tcg` crate, names
   `vibe mcp serve` — then what the host's written contract says is **not**
   wrong-consumer evidence: **it is the subject**. §3.8 voids host evidence used
   as a *proxy for a consumer*; it does not exempt a package sentence that makes
   a claim about vibevm's own machinery.

**So the question asked of every anchor below, before any measurement:** *whose
behaviour does this sentence describe* — the package's own shipment, an adopting
consumer in general, or this host by name?

---

## F-187 — the sentence says the package *ships* two skills; the verdict measured whether *this host installed* them {#f-187}

**Outcome:** the finding is STRUCK by §B.1, and re-verification on the legitimate
bench agrees for **all three anchors** — the claim each snippet makes is true of
the package that makes it, and true of the host too where the host is a consumer.
**3 of 3 → FALLS.**

**Anchors:** 3 of 3.

**The verdict's own reason, and where it points.** «the two Go skills are **not
installed**: `.claude/skills/` carries `rust-ai-native-sweep`,
`rust-ai-native-terraform`, `typescript-ai-native-sweep`,
`typescript-ai-native-terraform` and `vibevm` — four of the six, and no Go pair.
`vibe.lock` carries no `go-ai-native` entry and `vibedeps/` no
`stack-go-ai-native-lang` slot». Every one of those four observables is
**host DEPLOYMENT state**. Read against
[§3.7's four layers](../PHASE-D-BATCH-PLAN.md#compliance-blindness), the sentence
lives at the SPEC layer and its mechanism at the DRIVER layer; the verdict
measured DEPLOYMENT, and for Go and TypeScript it measured it **in a project that
is not a consumer** (§3.8 ¶3).

**All three obligation-level reasons are byte-identical.** Instrument check on
`run/state/obligations.json`: `reasons` holds two entries, both the same
host-install text — so unlike F-189 below, there is no per-stack reasoning to
separate. The single reason is entirely host-deployment.

### Anchor 1 — go · `spec/boot/20-stack-go-ai-native-lang.md:80-82` → **FALLS**

**Current text at HEAD:**

```
80  ##PROCEDURES-AS-AGENT-SKILLS Procedures as agent skills: `/go-ai-native-sweep`
81  (recurring), `/go-ai-native-terraform` (brownfield adoption) —
82  `vibe skill install` projects them. @impl/done
```

**Whose behaviour does this sentence describe.** **The package's own shipment,
plus a capability of the vibe CLI.** Parsed literally it makes exactly two
claims: *(a)* this stack's two procedures exist as agent skills under those two
names, and *(b)* `vibe skill install` is the mechanism that projects them into an
agent. It says nothing about any particular project having run that command, and
it names no consumer at all. «Projects them» is a **capability**, not a practice
([`##A-REAL-DEFECT-CONVICTING-THE-WRONG-SENTENCE`](../PHASE-D-BATCH-PLAN.md#delegation-lessons)
*(ii)*: «an unexercised capability is not a false capability»).

**Re-verified on the legitimate bench (package-own artefacts + the shipped CLI).**

*(a) The package declares both skills in its own manifest.*

```
$ grep -n -A4 "\[\[skill\]\]" packages/org.vibevm.ai-native/go-ai-native-lang/v0.1.0/vibe.toml
49:[[skill]]
50-name = "go-ai-native-sweep"
51-path = "spec/skills/go-ai-native-sweep"
52-description = "The recurring AI-Native discipline sweep for Go: the seven-step floor, then the health collector's ratchet items"
54:[[skill]]
55-name = "go-ai-native-terraform"
56-path = "spec/skills/go-ai-native-terraform"
57-description = "Adopt the AI-Native discipline on an existing Go codebase: inventory-not-gate, the registries, characterization, then card raids package by package"
```

*(b) Both skills ship, non-empty, with valid front-matter whose `name` matches
the declaration.*

```
$ find packages/org.vibevm.ai-native/go-ai-native-lang/v0.1.0/spec/skills -type f | while read f; do echo "$(wc -l < $f) $f"; done
151 .../spec/skills/go-ai-native-sweep/SKILL.md
128 .../spec/skills/go-ai-native-terraform/SKILL.md

$ head -3 .../spec/skills/go-ai-native-sweep/SKILL.md
---
name: go-ai-native-sweep
description: Run the recurring AI-Native discipline sweep on this Go project — the seven-step floor first, then …
```

*(c) The projection mechanism is real and package-agnostic.* `vibe skill install`
exists (`crates/vibe-cli/src/cli/skill.rs:31 Install(SkillInstallArgs)`) and its
collector reads **every installed package's `vibedeps/` slot manifest** and
projects each declared `[[skill]]`:

```
$ grep -n "installed packages" crates/vibe-cli/src/commands/skill/mod.rs
73:    // (b) installed packages — read each lockfile entry's slot manifest.
90:        for decl in &manifest.skills {
```

There is no per-language branch and no allow-list: any project that installs
`stack:org.vibevm.ai-native/go-ai-native-lang` gets both skills projected by that
same code path. **Both claims hold on the package's own tree.**

**RECOMMENDATION: FALLS.** The verdict's whole basis is host DEPLOYMENT state
against a package this host is ruled out of consuming (§3.8 ¶2–3); and on the
legitimate bench the sentence is true clause by clause. Re-judge `confirmed`.

### Anchor 2 — rust · `spec/boot/20-stack-rust-ai-native-lang.md:64-66` → **FALLS**

**Current text at HEAD:**

```
64  ##PROCEDURES-AS-AGENT-SKILLS Procedures as agent skills:
65  `/rust-ai-native-sweep` (recurring), `/rust-ai-native-terraform` (brownfield
66  adoption) — `vibe skill install` projects them. @impl/done
```

**Whose behaviour does this sentence describe.** The same two claims as the Go
copy — package shipment + CLI capability. **And here host evidence counts**
(§3.8 ¶4), so the sentence can be checked on *both* benches.

**Package bench.** `vibe.toml:49-57` declares `rust-ai-native-sweep` and
`rust-ai-native-terraform`; `spec/skills/rust-ai-native-sweep/SKILL.md` (150
lines) and `spec/skills/rust-ai-native-terraform/SKILL.md` (136 lines) ship with
matching front-matter.

**Host bench — and this is what makes the Rust anchor fall on the verdict's own
terms.** The verdict's own reason *lists* the two Rust skills as present. Re-run:

```
$ ls .claude/skills/
rust-ai-native-sweep   rust-ai-native-terraform
typescript-ai-native-sweep   typescript-ai-native-terraform   vibevm

$ ls .agents/skills/ ; ls .opencode/skills/
rust-ai-native-sweep   rust-ai-native-terraform   typescript-ai-native-sweep   typescript-ai-native-terraform
(same four in both)

$ grep -n "rust-ai-native-lang" vibe.lock
88:    "stack:org.vibevm.ai-native/rust-ai-native-lang@=0.7.0",
118:name = "rust-ai-native-lang"
121:source_url = "file:///C:/Users/olegc/git/v/vibevm/packages/org.vibevm.ai-native/rust-ai-native-lang/v0.7.0"

$ ls vibedeps/ | grep rust-ai-native
stack-rust-ai-native   stack-rust-ai-native-lang
```

Installed in all three agent directories, locked, and slotted. **The Rust anchor
was convicted by a family-wide restatement of a Go-specific (and now void)
observation** — exactly [§3.7's corollary](../PHASE-D-BATCH-PLAN.md#compliance-blindness),
«when a verdict says it was restated for consistency, re-verify the whole set, not
the row», with the twist that here consistency propagated the error onto a stack
where the evidence *in the verdict's own sentence* refutes it.

**RECOMMENDATION: FALLS.** True on the package bench and true on the host bench.
Re-judge `confirmed`.

### Anchor 3 — typescript · `spec/boot/20-stack-typescript-ai-native-lang.md:91-93` → **FALLS**

**Current text at HEAD:**

```
91  ##PROCEDURES-AS-AGENT-SKILLS Procedures as agent skills: `/typescript-ai-native-sweep` (recurring),
92  `/typescript-ai-native-terraform` (brownfield adoption) — `vibe skill install`
93  projects them. @impl/done
```

**Whose behaviour does this sentence describe.** Package shipment + CLI
capability, as above. Host evidence is void here (§3.8 ¶2–3).

**Package bench.** `vibe.toml:49-57` declares both;
`spec/skills/typescript-ai-native-sweep/SKILL.md` (123 lines) and
`spec/skills/typescript-ai-native-terraform/SKILL.md` (116 lines) ship with
matching front-matter. Both claims hold.

*(Recorded and **not** used: the two TypeScript skills also happen to be present
in this host's three agent directories. That is void as evidence about the
package per §3.8 ¶3 — noted only so a later pass does not read its absence from
this entry as an absence in the tree.)*

**RECOMMENDATION: FALLS.** Re-judge `confirmed`.

### What is left over, and it is not this obligation's

The verdict's Go observation is *factually accurate about the host* — there is no
`stack-go-ai-native-lang` slot, no `go-ai-native` lockfile entry, and no Go pair
in `.claude/skills/`. Under §B.1 that is **the intended state**, not a gap: «Go is
a deliberately unused prototype specification and it must stay unused». So there
is no host obligation to record either — this is not a §3.6(b) route-out, it is a
non-event. Nothing is owed on either side.

---

## F-189 — the registry reason is one of THREE, and the other two were never wrong-consumer arguments {#f-189}

**Outcome:** the finding as *stated in the registry* is struck by §B.1 — but the
registry's `reason` is the **Go** verdict only. **The Rust and TypeScript verdicts
carry different reasons that rest on the host's own PROP-026, which every one of
these sentences names as its subject** (§3.8 ¶5). All three sentences describe
the same retired topology, and all three are false. **0 of 3 FALLS · 1
STANDS-RESTATED (go) · 2 STANDS (rust, typescript).**

**Anchors:** 3 of 3.

### The instrument finding that reframes this obligation

`run/state/obligations.json` shows F-189 merged `by shared anchor
#COMPONENT-THE-PRODUCT-SEAM` — **not** by reason text. Its `reason` field carries
one of the three; the per-anchor verdicts in `run/cache.json` carry three
distinct ones (read as an instrument, not as evidence):

| stack | the verdict's own reason (from `run/cache.json`) | rests on |
|---|---|---|
| **go** | «**the host does not dispatch `go`** … PROP-026 accepts `"typescript"` and `"rust"` and names `"go"` as the example of an unsupported value» | **the host's dispatch roster → VOID per §B.1** |
| **rust** | «**the dispatch named was deleted.** the vibe-tcg crate and the vibe-mcp `tcg_*` adapters are DELETED (PROP-026:42), the tools ship in the per-family server (:33), and `language` survives only as a validated compat parameter (:39)» | **PROP-026's own supersession — never a consumer observable** |
| **typescript** | «the `vibe-tcg` crate does not exist … PROP-026:11 says it «was deleted whole», the grammar now served by the per-family MCP servers» | **PROP-026's own supersession** |

So the ruling voids **one third** of this obligation's reasoning, and the
registry happened to promote that third into the `reason` field. §B.1's own words
are precise about what was struck: «`F-189` rested on `PROP-026` designating
`"go"` unsupported. That is a statement about the host's own TCG dispatch …
Void.» That kills the *contradiction* reading. It does not reach the
*supersession* reading, which is a different fact and was already the recorded
basis for two of the three anchors.

**Whose behaviour do these sentences describe — the batch's sharpest attribution
question.** Every one of the three opens `**The product seam** (vibevm,
PROP-026)`. It is not a claim about an adopting project; it is a claim about
**vibevm's own product surface**, made by a package that ships into it. §3.8 ¶3
voids host evidence used as a *proxy for an absent consumer*; §3.8 ¶5 is the
other side of the same rule — when the sentence's declared subject *is* vibevm,
vibevm's written contract is the subject, not a proxy. Convicting the Go sentence
because «the host does not dispatch Go» measures the wrong consumer; observing
that «the `language`-dispatch seam PROP-026 defined has been retired for every
language» measures exactly the thing the sentence names.

**Perimeter searched.** The standing perimeter for the *thing* rather than the
string: a `vibe-tcg` crate anywhere; `tcg_*` adapters in `crates/vibe-mcp/`; the
four tool names in every `-mcp` package; the `language` parameter's actual
runtime behaviour in all three `-mcp` servers; PROP-026 and PROP-027 read in
full at the head; and each brief read **from `## 3. Component shape` through
`## 4. Staged ambition`** before any search
([`##READ-FURTHER-BEFORE-SEARCHING-WIDER`](../PHASE-D-BATCH-PLAN.md#delegation-lessons)).

**What the measurement shows — one topology fact, four ways.**

*(a) PROP-026 retired the topology in its own head matter, at HEAD:*

```
$ grep -n "SUPERSEDED-TOPOLOGY\|TOOLS-NEW-HOME\|ENUM-BET-REREAD\|LANGUAGE-COMPAT-PARAM\|TCG-CRATE-DELETED\|RETIRED-SECTIONS-KEPT" \
    spec/modules/vibe-mcp/PROP-026-tcg-tool-family.xml
27:- ##SUPERSEDED-TOPOLOGY **SUPERSEDED IN TOPOLOGY, 2026-07-07 …
33:- ##TOOLS-NEW-HOME The tools now ship in the per-language
37:- ##ENUM-BET-REREAD The §2 enum-value bet re-reads as «a new language is a new mcp package
39:- ##LANGUAGE-COMPAT-PARAM `language` survives as a validated
42:- ##TCG-CRATE-DELETED `vibe-tcg` and the vibe-mcp `tcg_*` adapters are
44:- ##RETIRED-SECTIONS-KEPT §3–§5 below describe the retired topology and stay as the
```

`##SUPERSEDED-TOPOLOGY` (`:27-32`) names the retired half exactly: «the TOPOLOGY
half (one multiplexed product server, **`language` as the dispatch parameter**,
the `vibe-tcg` registry crate) is retired». The GRAMMAR half — four ops, params,
answer shapes — «is unchanged and remains normative».

*(b) The crate is gone, and so are the adapters.*

```
$ find . -type d -name "vibe-tcg" -not -path "./.git/*" -not -path "*/target/*" -not -path "./refs/*"
(no output)

$ ls crates/
progress-core  vibe-actions  vibe-check  vibe-cli  vibe-core  vibe-graph  vibe-index
vibe-install  vibe-llm  vibe-mcp  vibe-publish  vibe-registry  vibe-resolver
vibe-settings  vibe-spec  vibe-test-support  vibe-wire  vibe-workspace

$ grep -rn "tcg_" crates/vibe-mcp/src/
crates/vibe-mcp/src/skill_template.md:106  `tcg_validate` / `tcg_scope` / `tcg_complete` / `tcg_type` /
crates/vibe-mcp/src/skill_template.md:107  `tcg_bench` over a persistent language-service session. …
```

Three hits, all inside a **prose template**, and the template's own heading says
the opposite of the anchors: «### The discipline servers (per-language MCP,
PROP-027) — The discipline toolchain and the agentic type oracle **no longer ride
THIS server**: each language ships its own standalone MCP server as an `mcp`-kind
package» (`crates/vibe-mcp/src/skill_template.md:98-101`). There is no `tcg_*`
adapter in `vibe-mcp`.

*(c) `language` does not dispatch — it refuses.* All three per-family servers
implement it identically as a compat guard:

```
$ grep -n -A10 "fn language_mismatch" packages/org.vibevm.ai-native/go-ai-native-mcp/v0.1.0/crates/go-ai-native-mcp/src/tools_discipline.rs
19:pub(crate) fn language_mismatch(args: &Value) -> Option<ToolOutput> {
20:    let asked = args.get("language").and_then(Value::as_str)?;
21:    if asked == "go" { return None; }
24:    Some(ToolOutput::failed(format!(
25:        "this server serves language `go`; asked for `{asked}` — mount that \
26:         language's own discipline server (mcp:org.vibevm.ai-native/{asked}-ai-native-mcp) \
28:         and call it there (PROP-027; PROP-026 §2)"
```

(`rust-ai-native-mcp/src/tools_discipline.rs:20-30` with `"rust"`,
`typescript-ai-native-mcp/src/tools_discipline.rs:24-34` with `"typescript"`;
each asserted by its own `language_mismatch_refuses_with_the_recipe` test —
`:334`, `:359`, `:355` respectively.) **A value other than the server's own
language is refused, not routed.** Nothing dispatches by `language` anywhere.

*(d) The four tools do exist per language — in the `-mcp` packages.*

```
$ grep -n "tcg_validate\|tcg_scope\|tcg_complete\|tcg_type" packages/org.vibevm.ai-native/*/v*/crates/*-mcp/src/lib.rs
go-ai-native-mcp/…/src/lib.rs:60-63    "tcg_complete", "tcg_scope", "tcg_type", "tcg_validate",
rust-ai-native-mcp/…/src/lib.rs:60-63  (same four)
typescript-ai-native-mcp/…/src/lib.rs:61-64 (same four)
```

So the **grammar** half of every sentence («the SAME four `tcg_*` tools») is
true, including for Go. Only the **topology** half is false — and it is false for
all three languages equally.

### Anchor 1 — go · `spec/go/tools/vibe-agentic-tcg-go.md:127-130` → **STANDS-RESTATED**

**Current text at HEAD:**

```
127  - ##COMPONENT-THE-PRODUCT-SEAM **The product seam** (vibevm, PROP-026): the SAME four `tcg_*` tools;
128    `language: "go"` dispatches through the lockfile to this package's
129    slot artifact. No new tools, no new PROP — the enum-value promise,
130    cashed a second time. @impl/done
```

**Whose behaviour does this sentence describe.** **This host, by name.** It opens
`(vibevm, PROP-026)` and its verb — «dispatches through the lockfile to this
package's slot artifact» — is an assertion about vibevm's MCP surface, not about
any adopting Go project. Under §3.8 ¶5 that makes vibevm's own written contract
the subject rather than a wrong-consumer proxy.

**What survives §B.1 and what does not.** The verdict's reason («the host does
not dispatch `go`»; PROP-026 «names `"go"` as the example of an unsupported
value», `:71-73`, `:169-171`) is **void** — and doubly so, because both cited
lines sit in the parts of PROP-026 the head matter re-read or retired
(`##PARAM-LANGUAGE` in §2, explicitly re-read by `##ENUM-BET-REREAD` at `:37`;
`##ACC-RECIPES` in §7's acceptance for the multiplexed server). Quoting them
against a Go package convicts it of a consumer's silence.

**But the sentence is still false, on package-visible grounds, three ways:**

1. **The named mechanism is retired for every language** — (a)–(c) above. There
   is no lockfile `language`-dispatch to a slot artifact for `"go"`, `"rust"` or
   `"typescript"`.
2. **«No new PROP» is falsified by a PROP that exists and that this very package
   cites.** `spec/modules/vibe-mcp/PROP-027-mcp-packages.xml` is exactly the new
   PROP the retirement created, and the Go stack's **own boot snippet** names it:
   `spec/boot/20-stack-go-ai-native-lang.md:60` «`mcp:org.vibevm.ai-native/go-ai-native-mcp`
   — **PROP-027**». Package-own evidence, no host observable involved.
3. **The document contradicts itself twelve lines down, and the other row is
   already right** ([`##READ-FURTHER-BEFORE-SEARCHING-WIDER`](../PHASE-D-BATCH-PLAN.md#delegation-lessons)).
   `##STAGE-A-CONSULTATION-ORACLE` (`:137-150`, the clause at `:141-144`) reads
   «All four ops ship as one-shots beside the relay … and **the MCP delivery half
   ships in the `go-ai-native-mcp` package (`tools_tcg.rs`)**». Same document,
   same section pair, new topology stated correctly — seven lines below the
   anchor that still names the old one.

**Proposed replacement reason (NOT APPLIED)** — this is the whole point of the
recommendation, since the recorded one may not be shown to the owner:

> **The topology this row names was retired, for every language.** PROP-026's own
> head matter says so — `##SUPERSEDED-TOPOLOGY` (`:27`) retires «one multiplexed
> product server, `language` as the dispatch parameter, the `vibe-tcg` registry
> crate»; `##TCG-CRATE-DELETED` (`:42`) says `vibe-tcg` and the vibe-mcp `tcg_*`
> adapters are DELETED; `##TOOLS-NEW-HOME` (`:33`) puts the tools in the
> per-language `mcp` packages of PROP-027. In the shipped surface `language` is a
> compat guard that refuses a mismatch rather than a dispatch key
> (`go-ai-native-mcp/src/tools_discipline.rs:19-29`). «No new tools» is true —
> `go-ai-native-mcp/src/lib.rs:60-63` mounts the same four; «no new PROP» is not —
> PROP-027 is that PROP, and this package's own boot snippet cites it
> (`spec/boot/20-stack-go-ai-native-lang.md:60`). This stack's own
> `##STAGE-A-CONSULTATION-ORACLE` (`:141-144`) already states the new topology
> correctly. **Nothing here rests on whether this repository consumes Go.**

**Proposed correction (NOT APPLIED, for the owner's diff):**

```
- ##COMPONENT-THE-PRODUCT-SEAM **The product seam** (vibevm, PROP-027; PROP-026 §2 keeps the
  grammar): the SAME four `tcg_*` tools, served by this family's own standalone
  MCP package `mcp:org.vibevm.ai-native/go-ai-native-mcp` — no new tools, and
  `language` survives only as a validated compat parameter that refuses a
  mismatch with the recipe naming the right server. The multiplexed
  `vibe mcp serve` + `vibe-tcg` slot-dispatch topology this row described was
  retired whole (PROP-026 ##SUPERSEDED-TOPOLOGY, ##TCG-CRATE-DELETED); the
  enum-value promise re-reads as «a new language is a new mcp package shipping
  the SAME tool grammar» (##ENUM-BET-REREAD). @impl/done
```

**RECOMMENDATION: STANDS-RESTATED.** Drift, and the recorded reason must be
replaced before any diff is shown to the owner — otherwise the owner is asked to
approve a correction justified by exactly the argument §B.1 struck.

### Anchor 2 — rust · `spec/rust/tools/vibe-agentic-tcg-rust.md:132-135` → **STANDS**

**Current text at HEAD:**

```
132  - ##COMPONENT-THE-PRODUCT-SEAM **The product seam** (vibevm, PROP-026): the SAME four `tcg_*` tools;
133    `language: "rust"` dispatches through the lockfile to this package's
134    slot artifact. No new tools, no new PROP — the enum-value promise,
135    cashed. @impl/done
```

**Whose behaviour does this sentence describe.** **This host, by name** — and here
the host is *also* a genuine consumer (§3.8 ¶4), so both readings point the same
way. Word-for-word the Go sentence with `"rust"` for `"go"` and «cashed» for
«cashed a second time».

**Re-verification.** The verdict's own reason is accurate and needs no repair: the
`vibe-tcg` crate and the vibe-mcp `tcg_*` adapters are deleted (`find` and
`grep crates/vibe-mcp/src/` above), the tools ship in `rust-ai-native-mcp`
(`src/lib.rs:60-63`), and `language` is a compat guard
(`rust-ai-native-mcp/src/tools_discipline.rs:20-30`, asserted at `:359`). The
Rust stack's own boot snippet already names the new home:
`spec/boot/20-stack-rust-ai-native-lang.md:46` «also served over MCP by
`mcp:org.vibevm.ai-native/rust-ai-native-mcp` — **PROP-027**».

**One clause of the recorded reason is wrong and should be dropped when it is
quoted.** It ends «The TypeScript twin was recorded drift for the same reason in
this same batch, **and so was the Go one**». The Go verdict was recorded on a
*different* reason (the dispatch roster), which is precisely the reason §B.1
struck. Carrying that clause forward would re-import the void argument into a
sound verdict.

**Note for the owner's diff, not for the verdict.** The stale topology appears
**twice** in this brief and the second instance belongs to a different anchor:
the ASCII diagram under `##three-processes-lead` (`:100`, drawn at `:104-107`)
still shows `agent ──MCP (…, language:"rust")──▶ vibe mcp serve (vibe-tcg
registry: lazy spawn, slot dispatch, consent)` — `:105-107`. Attributing that to
`##COMPONENT-THE-PRODUCT-SEAM` would be
[`##A-REAL-DEFECT-CONVICTING-THE-WRONG-SENTENCE`](../PHASE-D-BATCH-PLAN.md#delegation-lessons)
in miniature — but a diff that repairs the row and leaves the diagram ships two
topologies in one document. The same doubling holds in the Go brief
(`##three-processes-lead` `:94`, diagram `:100-102`) and the TypeScript brief
(`##three-processes-lead` `:96`, diagram `:100-101`). **Flagged: three diagrams
carrying the retired topology, and no obligation covers any of them.**

**RECOMMENDATION: STANDS.** Drift, reason accurate on its substance; drop the
trailing «and so was the Go one» clause.

### Anchor 3 — typescript · `spec/typescript/tools/vibe-agentic-tcg-ts.md:130-135` → **STANDS**

**Current text at HEAD:**

```
130  - ##COMPONENT-THE-PRODUCT-SEAM **The product seam** (vibevm, PROP-026): the `vibe-tcg` crate — tool
131    schemas, registry, slot dispatch via the PROP-025 model — mounted by
132    vibe-mcp as four `tcg_*` tools with a `language` parameter
133    (`"typescript"` today; the Rust twin adds a value, not new tools).
134    Deliberately liftable into a standalone MCP server (zero vibe-mcp
135    imports) — the owner's portability amendment. @impl/done
```

**Whose behaviour does this sentence describe.** **This host, by name, and more
explicitly than either sibling** — it names a host crate (`vibe-tcg`), a host
server (`vibe-mcp`), and a host mounting relation. There is no consumer in it at
all. §3.8 ¶3 does not reach it; §3.8 ¶5 governs, and the host's contract is the
subject.

**Re-verification.** The verdict's reason is accurate: `vibe-tcg` does not exist
(`find -type d -name vibe-tcg` → no output; `crates/` listing above), and
PROP-026 `:11` says it «was deleted whole». «Mounted by vibe-mcp as four `tcg_*`
tools» is false — `crates/vibe-mcp/src/` carries no adapter, and its own skill
template says the oracle «no longer rides THIS server»
(`skill_template.md:98-101`).

**The clause that came TRUE, and the correction must keep it.** The row's last
sentence — «Deliberately liftable into a standalone MCP server (**zero vibe-mcp
imports**) — the owner's portability amendment» — is not merely still true, **it
is what happened**. The lift was performed:

```
$ grep -n -A12 "^\[dependencies\]" packages/org.vibevm.ai-native/typescript-ai-native-mcp/v0.6.0/crates/typescript-ai-native-mcp/Cargo.toml
13:[dependencies]
14:mcp-core.workspace = true
15:specmark.workspace = true
…  typescript-ai-native-{conform,specmap,cli,tcg,tcg-bridge}.workspace = true

$ grep -rn "vibe-mcp\|vibe_mcp" packages/org.vibevm.ai-native/*-ai-native-mcp/v*/crates/*/Cargo.toml
(no output)
```

Zero `vibe-mcp` imports in any of the three per-family servers. So the honest
shape of the correction is *not* «this was wrong» but «the amendment was
exercised, and the pre-lift topology is what is stale».

**Proposed correction (NOT APPLIED, for the owner's diff):**

```
- ##COMPONENT-THE-PRODUCT-SEAM **The product seam** (vibevm, PROP-027; PROP-026 §2 keeps the
  grammar normative): the same four `tcg_*` tools, served by this family's own
  standalone MCP package `mcp:org.vibevm.ai-native/typescript-ai-native-mcp` with
  `language` as a validated compat parameter (a mismatch refuses with the recipe
  naming the right server). The `vibe-tcg` registry crate and the vibe-mcp
  `tcg_*` adapters this row described were deleted whole
  (PROP-026 ##TCG-CRATE-DELETED) — **the owner's portability amendment was
  cashed**, not deferred: `typescript-ai-native-mcp` depends on `mcp-core` and
  has zero vibe-mcp imports. @impl/done
```

**RECOMMENDATION: STANDS.** Drift, reason accurate. Nothing about this verdict
ever depended on the host being a TypeScript consumer.

### The family consequence the boss must decide before any diff

All three rows are the same claim about the same retired mechanism, and the
correction is one shape in three copies — **this is the rare case where a
family-wide edit is right**, and it is right because the measurement above
establishes the topology is retired for every language, not because the sentences
look alike. Two constraints on that edit: the Go copy's *recorded reason* must be
replaced first (above), and the TypeScript copy's portability clause must be kept
and re-marked as cashed rather than dropped in a mechanical sweep.

---

## F-190 — `DISABLED by policy` is shipped verbatim in the two stacks that quote it, and one of the three sentences never quotes it at all {#f-190}

**Outcome:** the queue's prior is confirmed and then some. **`falsifier: self`, so
§3.8 does not reach this obligation at all** — the sweep skill is the package's
own artefact checked against the package's own binaries. The **`Defaulted` half is
right in all three**; the **`DISABLED by policy` half is wrong in the two stacks
that quote it and absent from the third**. And the three sentences differ because
**the three products differ** — the Rust floor has no disable mechanism to print.
**3 of 3 → STANDS-RESTATED.**

**Anchors:** 3 of 3.

**Perimeter searched.** Each stack's **own** tree only (`falsifier: self`; §3.8's
legitimate bench regardless): `packages/org.vibevm.ai-native/<stack>/v*/crates/**`
minus `target/`, case-sensitively for `Defaulted` / `DEFAULTED` and
case-insensitively for `disabled by policy`; then the printing sites read in
full; then the three floor-run captures. **The three floor captures under
`campaigns/packages-2026-09/harvest/*-floor.md` are cited below as CAPTURED RUN
OUTPUT of each package's own binary** — that is package-own evidence about what
the tool prints, and it is named as a campaign artefact so it is not mistaken for
a campaign *finding*
([`##THE-CAMPAIGN-IS-INSIDE-ITS-OWN-CORPUS`](../PHASE-D-BATCH-PLAN.md#delegation-lessons)).

**What the measurement shows — the `Defaulted` half (verdict RIGHT, all three).**
`Defaulted` is a Rust **enum variant name**, never a printed token. The printing
sites are:

```
$ grep -rn "ConfigOrigin::Defaulted => eprintln" packages/org.vibevm.ai-native/*-ai-native-lang/v*/crates/*-conform/src/lib.rs
go   crates/go-ai-native-conform/src/lib.rs:31
rust crates/rust-ai-native-conform/src/lib.rs:33
ts   crates/typescript-ai-native-conform/src/lib.rs:28
```

and each prints a line with no `Defaulted` in it:

| stack | printed on a defaulted policy | source | captured run |
|---|---|---|---|
| go | `go-ai-native-conform: NO conform.toml — topology default in force (roots = ["."], no cells gate); run \`go-ai-native init\` to write a starting policy.` | `go-ai-native-conform/src/lib.rs:31-35` | `harvest/go-ai-native-lang-floor.md:29` |
| rust | `conform: NO conform.toml — topology default in force, nothing is gated; run \`rust-ai-native init\` to write a starting policy.` | `rust-ai-native-conform/src/lib.rs:33-36` | `harvest/rust-ai-native-lang-floor.md:184` |
| ts | `typescript-ai-native-conform: NO conform.toml — topology default in force (roots = ["src"], no cells gate); run \`typescript-ai-native init\` to write a starting policy.` | `typescript-ai-native-conform/src/lib.rs:28-32` | `harvest/typescript-ai-native-lang-floor.md:31` |

The captured lines are byte-for-byte the format strings, so the capture confirms
the read rather than adding to it. **`Defaulted` is printed nowhere.** The single
place any spelling of it reaches stderr is uppercase and in a different tool:

```
$ grep -rn "DEFAULTED" packages/org.vibevm.ai-native/{go,rust,typescript}-ai-native-lang
go-ai-native-lang/v0.1.0/crates/go-ai-native-tcg/src/lib.rs:55
    conform_core::ConfigOrigin::Defaulted => "DEFAULTED — run `go-ai-native init`",
```

— the tcg oracle's own policy banner (`go-ai-native-tcg: policy conform.toml
(DEFAULTED — run \`go-ai-native init\`).`, `lib.rs:51-57`), not a floor line, and
not what any of the three anchors is about.

**What the measurement shows — the `DISABLED by policy` half (verdict WRONG where
it applies).** The string ships **verbatim, in the exact casing the skills
quote**, in both CLIs that have the mechanism:

```
$ grep -rni "disabled by policy" packages/org.vibevm.ai-native/{go,rust,typescript}-ai-native-lang/v*/crates/
go   crates/go-ai-native-cli/src/floor.rs:66   "floor: step `{}` DISABLED by policy — {} (conform.toml [go])"
go   crates/go-ai-native-cli/src/floor.rs:220  "\nfloor: all green ({} step(s) run, {} disabled by policy)."
ts   crates/typescript-ai-native-cli/src/floor.rs:62   "floor: step `{}` DISABLED by policy — {} (conform.toml [typescript])"
ts   crates/typescript-ai-native-cli/src/floor.rs:221  "\nfloor: all green ({} step(s) run, {} disabled by policy)."
rust (no output)
```

`floor.rs:65-68` (go) and `:61-64` (ts) sit inside `run_floor`'s opening loop over
`config.<lang>.floor_disable` — one line printed per disabled step, before any
step runs. **This is a `DISABLED by policy` *line*, which is exactly the noun both
skills use** («every `DISABLED by policy` line is a standing decision to
re-question weekly»).

**Why the verdict missed it, recorded because the shape recurs.** The Go verdict's
own evidence list cites `go-ai-native-cli/src/floor.rs:220` — the *summary*
counter, lowercase — and concludes the string «appears in no shipped string». The
verbatim uppercase form is **154 lines above the line the verdict quoted, in the
file it quoted, in the function that calls the code it quoted.** The TypeScript
verdict is sharper still: it cites
`crates/vendor/core-ai-native-conform/src/config.rs:173 pub floor_disable:
Vec<FloorDisable>` — the *config field that drives the print* — as evidence the
print does not exist. This is
[`##READ-FURTHER-BEFORE-SEARCHING-WIDER`](../PHASE-D-BATCH-PLAN.md#delegation-lessons)
in its purest form: the disproof was in the cited file.

**Why the Rust sentence is different, and why that is correct.** The Rust CLI has
no floor-disable mechanism at all — nothing to print:

```
$ grep -n "floor_disable\|DISABLED\|disabled" packages/org.vibevm.ai-native/rust-ai-native-lang/v0.7.0/crates/rust-ai-native-cli/src/floor.rs
(no output)

$ grep -n "pub floor_disable" packages/org.vibevm.ai-native/rust-ai-native-lang/v0.7.0/crates/vendor/core-ai-native-conform/src/config.rs
129:    pub floor_disable: Vec<FloorDisable>,   (inside `pub struct GoConfig`, :106)
173:    pub floor_disable: Vec<FloorDisable>,   (inside `pub struct TsConfig`, :157)
```

There is **no `[rust]` `floor_disable` section in the shared config**, and the
Rust floor's green summary is the bare `eprintln!("\nfloor: all green")`
(`rust-ai-native-cli/src/floor.rs:135`) with no disabled count. **So the three
sentences are not word-identical because the three products are not identical,
and the Rust omission is the package being right about itself.** A family-wide
edit that pushed a `DISABLED by policy` clause onto the Rust skill would break a
correct sentence — the [release queue §B](../PHASE-D-RELEASE-QUEUE.md#stacks)
shape, with Rust as the odd member.

### Anchor 1 — go · `spec/skills/go-ai-native-sweep/SKILL.md:47-50` → **STANDS-RESTATED**

**Current text at HEAD:**

```
47  ##CHECK-THE-PRINTED-POLICY-LINES Check the printed policy lines: a `Defaulted` conform policy
48  means the project is not bootstrapped (`go-ai-native init`), and every
49  `DISABLED by policy` line is a standing decision to re-question weekly — a
50  floor that shrank quietly is the failure mode this line exists to catch. @impl/done
```

**Whose behaviour does this sentence describe.** **The package's own binaries** —
what `go-ai-native floor` / `go-ai-native conform` print. `falsifier: self`; no
consumer appears in it, so §3.8 is not engaged in either direction.

**Re-verification.** `DISABLED by policy` → **shipped verbatim**
(`crates/go-ai-native-cli/src/floor.rs:66`). `Defaulted` → **not printed**; the
line the tool actually emits is the `NO conform.toml — topology default in force`
line above.

**Proposed replacement reason (NOT APPLIED):**

> **Half the step quotes a string the tool prints and half quotes one it does
> not.** `DISABLED by policy` is shipped verbatim, in this casing, one line per
> disabled step (`crates/go-ai-native-cli/src/floor.rs:65-68`) — that clause is
> correct and must not be touched. `Defaulted` is a `ConfigOrigin` enum variant
> (`crates/vendor/core-ai-native-conform/src/config.rs:231`) and never reaches
> stderr: on a defaulted policy the tool prints `go-ai-native-conform: NO
> conform.toml — topology default in force (roots = ["."], no cells gate); run
> \`go-ai-native init\` to write a starting policy.`
> (`crates/go-ai-native-conform/src/lib.rs:31-35`, captured verbatim in the
> package's own floor run). A reader grepping the output for `Defaulted` finds
> nothing.

**Proposed correction (NOT APPLIED)** — one clause, keeping the guidance intact:

```
##CHECK-THE-PRINTED-POLICY-LINES Check the printed policy lines: a
`NO conform.toml — topology default in force` line means the project is not
bootstrapped (`go-ai-native init`) and any green under it is vacuous, and every
`DISABLED by policy` line is a standing decision to re-question weekly — a
floor that shrank quietly is the failure mode this line exists to catch. @impl/done
```

**RECOMMENDATION: STANDS-RESTATED.** Drift is real but only in the first clause;
the recorded reason convicts a second clause that is verbatim correct and would,
if applied as written, delete a true sentence.

### Anchor 2 — rust · `spec/skills/rust-ai-native-sweep/SKILL.md:46-49` → **STANDS-RESTATED**

**Current text at HEAD:**

```
46  ##CHECK-THE-PRINTED-POLICY-ORIGIN-LINES Check
47  the printed policy-origin lines: a `Defaulted` policy means the project is
48  not bootstrapped (`rust-ai-native init`), and a green on a defaulted
49  policy is vacuous. @impl/done
```

**Whose behaviour does this sentence describe.** The package's own binaries.
`falsifier: self`.

**Re-verification.** The sentence **never mentions `DISABLED by policy`** — so the
recorded reason («the **two** strings the sweep tells a reader to look for») is
false of this anchor on its face, and correctly so: the Rust floor has no disable
mechanism (measurement above). What remains is the `Defaulted` clause, and that
token is not printed: the line is `conform: NO conform.toml — topology default in
force, nothing is gated; run \`rust-ai-native init\` to write a starting policy.`
(`crates/rust-ai-native-conform/src/lib.rs:33-36`; captured verbatim at
`harvest/rust-ai-native-lang-floor.md:184`).

**The reading that would make this anchor FALL, stated so the boss can choose.**
`Defaulted` in backticks may be naming the **state** (`ConfigOrigin::Defaulted`,
`config.rs:231`) rather than quoting a printed token — and the sentence's
imperative is «Check the printed policy-**origin** lines», which is exactly what
that line is. Under that reading the guidance is correct and the anchor should be
re-judged `confirmed`. Two things weigh against it and for STANDS-RESTATED:
*(i)* the Go and TypeScript copies put `Defaulted` in grammatical parallel with
`DISABLED by policy`, which **is** a verbatim printed token, so the family's own
usage reads backticks here as output; *(ii)* a sweep skill is an executable
procedure, and a step that names an unmatchable token is harder to run than one
that quotes the line. **The boss decides; the measurement is the same either
way.** The second clause — «a green on a defaulted policy is vacuous» — is
correct and unaffected, and the printed line's own «nothing is gated» says it.

**Proposed replacement reason (NOT APPLIED):**

> This anchor names **one** string, not two — there is no `DISABLED by policy`
> clause here, and there could not be: the Rust CLI has no floor-disable
> mechanism (`crates/rust-ai-native-cli/src/floor.rs` has no `floor_disable`, and
> the shared config carries `floor_disable` only under `[go]` and `[typescript]`,
> `config.rs:129`, `:173`). The `Defaulted` token is a `ConfigOrigin` enum variant
> and is never printed; the policy-origin line reads `conform: NO conform.toml —
> topology default in force, nothing is gated; run \`rust-ai-native init\` to write
> a starting policy.` The step's *guidance* is right; the *token* it names is not
> matchable in the output.

**Proposed correction (NOT APPLIED):**

```
##CHECK-THE-PRINTED-POLICY-ORIGIN-LINES Check the printed policy-origin lines:
`conform: NO conform.toml — topology default in force, nothing is gated` means
the project is not bootstrapped (`rust-ai-native init`), and a green on a
defaulted policy is vacuous. @impl/done
```

**RECOMMENDATION: STANDS-RESTATED.** The recorded reason must be replaced
regardless of the outcome, because it convicts this anchor of quoting a string it
does not quote.

### Anchor 3 — typescript · `spec/skills/typescript-ai-native-sweep/SKILL.md:48-51` → **STANDS-RESTATED**

**Current text at HEAD:**

```
48  ##CHECK-THE-PRINTED-POLICY-LINES Check the printed policy lines: a `Defaulted` conform policy means
49  the project is not bootstrapped (`typescript-ai-native init`), and every
50  `DISABLED by policy` line is a standing decision to re-question weekly —
51  a floor that shrank quietly is the failure mode this line exists to catch. @impl/done
```

**Whose behaviour does this sentence describe.** The package's own binaries.
`falsifier: self` — **§3.8 does not void this anchor**, and it is the one
TypeScript anchor in the batch that needed no audience ruling at all.

**Re-verification.** Identical to the Go anchor with `[typescript]` for `[go]`:
`DISABLED by policy` **shipped verbatim** at
`crates/typescript-ai-native-cli/src/floor.rs:62` (inside the `floor_disable`
loop, `:53-65`); `Defaulted` **not printed** — the line is
`typescript-ai-native-conform: NO conform.toml — topology default in force
(roots = ["src"], no cells gate); run \`typescript-ai-native init\` to write a
starting policy.` (`crates/typescript-ai-native-conform/src/lib.rs:28-32`,
captured at `harvest/typescript-ai-native-lang-floor.md:31`).

**Proposed replacement reason and correction (NOT APPLIED):** the Go anchor's,
with `typescript-ai-native` for `go-ai-native`, `roots = ["src"]` for
`roots = ["."]`, and `crates/typescript-ai-native-cli/src/floor.rs:61-64` for the
shipped-clause citation.

**RECOMMENDATION: STANDS-RESTATED.** Same split as Go: first clause drift, second
clause correct and load-bearing.

### The family consequence

The correction is **two copies, not three**: Go and TypeScript take the same
two-clause repair; Rust takes a one-clause repair and **must not** acquire a
`DISABLED by policy` clause. The recorded reason is unusable for all three — it
is false for two and describes a string the third does not contain.

---

## F-213 — `discipline/golden/` is the ADOPTING project's directory, and the verdict looked for it inside the package {#f-213}

**Outcome:** **2 of 2 → FALLS.** This is
[§3.7's compliance blindness](../PHASE-D-BATCH-PLAN.md#compliance-blindness) in
its textbook form — «a search confined to `packages/` cannot see compliance at
all, and reads every successful adoption back as a missing mechanism» — with a
second error stacked on top: **the Go anchor does not contain the word
`capture.sh`**, so the verdict's reason does not describe it at all.

**Anchors:** 2 of 2.

**The verdict's reason, and the two things wrong with it.** «`capture.sh` exists
only at the HOST's `discipline/golden/capture.sh`; **no ai-native package carries
a `discipline/` directory at all**, so the re-capture step has no script where the
skill puts it.» *(i)* The skill does not put it in the package — it puts it in
**the project the skill is run on**. *(ii)* The verdict's own evidence list cites
`discipline/golden/capture.sh:2` — the script's existence read as its absence.

**Whose behaviour do these sentences describe — settled by the skills' own text,
not by inference.** Both anchors sit in a *sweep skill*, and a sweep skill is a
procedure executed **in a consuming project's tree**:

- the frontmatter says so: «Run the recurring AI-Native discipline sweep on **this
  Go project**» (`go-ai-native-sweep/SKILL.md:3`), «…**on this Rust project**»
  (`rust-ai-native-sweep/SKILL.md:3`);
- `##IF-NOT-ON-PATH-INSTALL-OR-RUN-IN-PLACE` (`go`, `:29-33`) tells the reader to
  run `cargo install --path vibedeps/<stack-slot>/crates/go-ai-native-cli` —
  a `vibedeps/` slot path, which exists only in a consumer;
- and **every other `discipline/` path in the same skill is the consumer's**:
  `discipline/registry/debt.json` (`go`, `:89`),
  `discipline/health/latest-go.json` (`go`, `:58`),
  `discipline/DEBT.md` / `INTENT.md` (`rust`, `:87`).

So `discipline/golden/` is the consumer's directory by the same reading that makes
`discipline/registry/debt.json` the consumer's — and no one has proposed
convicting *that* bullet because the package ships no `debt.json`.

**The four layers, written down before searching**
([§3.7](../PHASE-D-BATCH-PLAN.md#compliance-blindness)):

| layer | where this mechanism lives | found at |
|---|---|---|
| **SPEC** | the brownfield protocol's «characterization of record» | `vibedeps/flow-core-ai-native/0.8.0/spec/mechanisms/BROWNFIELD-PROTOCOL-v0.1.md:75-77` — §6: «At inventory time, capture golden transcripts for currently-passing observable flows … stability oracles, not correctness claims» |
| **DRIVER / procedure** | each `-lang` package's **terraform** skill creates the directory; its **sweep** skill checks it weekly | `go-ai-native-terraform/SKILL.md:56-58`, `rust-ai-native-terraform/SKILL.md:51-55` |
| **RULE** | «must fail loudly, never auto-update» stated in each package's own card + guide | `go`: `spec/cards/scaffold-d-differential-oracle.md:85-87`, `spec/go/GUIDE-AI-NATIVE-GO.md:483` · `rust`: `spec/cards/scaffold-d-differential-oracle.md:47`, `spec/rust/GUIDE-AI-NATIVE-RUST.md:123` |
| **DEPLOYMENT** | the adopting project's `discipline/golden/` + its capture script | `discipline/golden/` at the host — `capture.sh` + five `*.transcript.md` |

**The perimeter, and the one place the artefact exists.**

```
$ find . -type d -name "golden" -not -path "./.git/*" -not -path "*/target/*" -not -path "./refs/*" -not -path "*/node_modules/*"
./discipline/golden

$ find . -name "capture.sh" -not -path "./.git/*" -not -path "*/target/*" -not -path "./refs/*"
./discipline/golden/capture.sh

$ ls discipline/golden/
capture.sh  check-installed.transcript.md  init.transcript.md
install-qualified.transcript.md  install-short-name.transcript.md  uninstall.transcript.md
```

One adopting project in this repository has run the terraform procedure, and it
produced exactly what the procedure says it produces. `research/rust-demo`,
`research/ts-demo`, `research/go-demo` and `packages/org.vibevm.fractality/**`
carry no `discipline/golden/` — they are partial adoptions, and their silence is
[`##ABSENCE-NAMES-ITS-PERIMETER`](../PHASE-D-BATCH-PLAN.md#delegation-lessons)
material, not evidence against a rule.

**The wave-6 precedent this repeats, and it is the same shape inverted.** D6c's
`##INVARIANT-THE-ANCESTRY-GATE` (`harvest/d6c-mirrors-licensing-absences.md:63-68`,
`:138-149`) was recommended **route (b) — package does not move** because *the
flow's own reference implementation performed the mechanism* and only the host's
port omitted it. Here the same question («does the package's own companion
artefact implement the thing before you demote on its absence?») has an even
cleaner answer: **the companion artefact is the sibling skill in the same
package**, and the consumer that ran it has the directory. There is no route-out
here because nothing is missing on either side.

### Anchor 1 — go · `spec/skills/go-ai-native-sweep/SKILL.md:96-98` → **FALLS**

**Current text at HEAD:**

```
96  - ##DRIFT-GOLDEN-TRANSCRIPTS Golden transcripts (`discipline/golden/`, `testdata/` goldens): must
97    fail loudly, re-captured deliberately, never auto-updated — the
98    `-update` flag never runs in CI. @impl/done
```

**Whose behaviour does this sentence describe.** **An adopting Go project's**, and
it is a **RULE**, not a claim of fact: «must fail loudly … never auto-updated».
Under §3.8 this host is not that project and must not become it, so the
legitimate bench is the package's own artefacts — «a rule the consumer breaks is
§3.6(b), not a wrong sentence»
([§6.1 *(ii)*](../PHASE-D-BATCH-PLAN.md#delegation-lessons)), and here there is
not even a consumer to break it.

**The verdict does not describe this anchor.** The word `capture.sh` **does not
appear in the Go sweep skill at all**:

```
$ grep -n "capture" packages/org.vibevm.ai-native/go-ai-native-lang/v0.1.0/spec/skills/go-ai-native-sweep/SKILL.md
(no output)
```

The Go bullet names no script. Its second half is a **Go-native** convention —
`testdata/` goldens and the `go test -update` flag — with no `discipline/`
dependency whatsoever.

**Re-verified on the legitimate bench — the package specifies every clause of it,
in three of its own documents.**

*(a) The directory is created by this package's own sibling skill:*

```
$ sed -n '56,58p' packages/org.vibevm.ai-native/go-ai-native-lang/v0.1.0/spec/skills/go-ai-native-terraform/SKILL.md
6. ##INVENTORY-CHARACTERIZE **Characterize** currently-passing observable behavior (golden
   transcripts under `discipline/golden/`, normalized for volatile
   fields). A pinned bug is visible debt; an unpinned bug is a landmine. @impl/done
```

**The terraform skill writes it; the sweep skill checks it.** They are the
adoption pair this package ships (`vibe.toml:49-57`), and the sweep skill's own
description names the terraform skill as its counterpart.

*(b) The rule is stated verbatim, including the `-update` parenthetical, in this
package's own card:*

```
$ sed -n '85,87p' packages/org.vibevm.ai-native/go-ai-native-lang/v0.1.0/spec/cards/scaffold-d-differential-oracle.xml
- ##CONSEQUENCE-GOLDENS-ENSHRINE-CURRENT-BEHAVIOR (−) Characterization goldens enshrine current behavior including bugs — pair with a
  spec edge marking intentional vs incidental; goldens must fail loudly, never
  auto-update (the `-update` flag never runs in CI). @spec/done
```

*(c) The `testdata/` half is this package's own guide:*
`spec/go/GUIDE-AI-NATIVE-GO.md:483` «Characterization goldens live in `testdata/`
and follow the promotion protocol — CI …», `:276` «the conventional `-update`
flag …», and `spec/skills/go-ai-native-terraform/SKILL.md:89-90` «goldens follow
the promotion protocol».

*(d) The SPEC layer is the installed core flow:*
`vibedeps/flow-core-ai-native/0.8.0/spec/mechanisms/BROWNFIELD-PROTOCOL-v0.1.md:77`
(§6, `#characterization`). It establishes the *mechanism* — capture goldens at
inventory time — and names neither a path nor a script; the path is the
terraform skill's, which is why the package is the right place to look and the
package has it.

**RECOMMENDATION: FALLS.** The reason cites a string the anchor does not contain,
and its load-bearing clause («no ai-native package carries a `discipline/`») looks
for a consumer artefact inside a specification. Re-judge `confirmed`.

### Anchor 2 — rust · `spec/skills/rust-ai-native-sweep/SKILL.md:96-97` → **FALLS**

**Current text at HEAD:**

```
96  - ##DRIFT-GOLDEN-TRANSCRIPTS Golden transcripts (`discipline/golden/`): must fail loudly, re-captured
97    deliberately (`capture.sh`), never auto-updated. @impl/done
```

**Whose behaviour does this sentence describe.** **An adopting Rust project's** —
and per §3.8 ¶4 **this host is one**, so this anchor can be checked at the
DEPLOYMENT layer against a real consumer, which is what the verdict should have
done with the file it cited.

**Re-verification at the layer the sentence points to.** Every element of the
sentence resolves at the host, at the exact path the sentence composes:

- `discipline/golden/` — exists, with **five** committed `*.transcript.md`
  characterizations (`ls` above);
- `capture.sh` — exists at `discipline/golden/capture.sh`, i.e. literally
  «`discipline/golden/` … (`capture.sh`)»;
- **«re-captured deliberately, never auto-updated»** — the script's own header
  states exactly that property, and its determinism check is the sweep's:

```
$ head -7 discipline/golden/capture.sh
#!/usr/bin/env bash
# discipline/golden/capture.sh — Phase −1 characterization capture
# (PLAYBOOK-TERRAFORM-VIBEVM v0.2 Phase −1; BROWNFIELD-PROTOCOL §6).
#
# Regenerates every golden transcript deterministically from the current
# tree. Run it twice; `git diff discipline/golden` must be empty — that is
# the inventory's determinism check.
```

It is a hand-run script citing **BROWNFIELD-PROTOCOL §6** — the same SPEC layer
the package chain names — and nothing auto-invokes it: this repository has **no
CI at all** (`ls -a .github` → `no .github`), so «never auto-updated» is
unbreached by construction here.

- and the package's own layers agree: `rust-ai-native-terraform/SKILL.md:51-55`
  creates `discipline/golden/`; `:85-86` states «golden transcripts must fail
  loudly when stale, never auto-update»; `spec/rust/GUIDE-AI-NATIVE-RUST.md:123`
  and `spec/cards/scaffold-d-differential-oracle.md:47` state the same rule.

**The one real weakness, recorded and NOT converted into a verdict.** The
parenthetical `(capture.sh)` names a **filename the package never prescribes** —
neither `rust-ai-native-terraform/SKILL.md` nor BROWNFIELD-PROTOCOL §6 says the
capture script must be called `capture.sh`. In the one Rust consumer we can see
it is called exactly that, so the pointer resolves; in an external consumer it
might not. That is a *portability* nit about a parenthetical, it is a different
observation from the one recorded, and it is nowhere near «the re-capture step has
no script where the skill puts it». Flagged for the boss as an optional polish
(«re-captured deliberately (the project's capture script, e.g.
`discipline/golden/capture.sh`)»), **not** as a ground for keeping the drift.

**RECOMMENDATION: FALLS.** The verdict read the existence of
`discipline/golden/capture.sh` — which it cited as evidence — as proof of its
absence, because it required the file to be inside the package. Re-judge
`confirmed`.

### By-catch, host-side, out of scope

`terraform/BASELINE.md:91-92` links the capture script as a markdown link whose
target is the relative path `golden/capture.sh` — which resolves to
`terraform/golden/capture.sh`, and `ls terraform/` shows no `golden/` directory;
the file is at `discipline/golden/capture.sh`. That is a stale link in a **host**
document, no obligation covers it, and it is recorded here only so the next pass
does not read it as a second capture script.

---

## `refs/**`, reported separately {#refs}

`refs/**` is a third-party study corpus, not our shipped surface, and it was
searched only to be excluded from the counts above. It carries **no** occurrence
of `DISABLED by policy`, `Defaulted`, `discipline/golden` or `capture.sh`. It does
carry `refs/ts/vibe-tcg-ts.md` — «Tool Spec (high-level): `vibe-tcg-ts` … Status:
vision / component brief» — an **earlier draft ancestor** of the TypeScript tcg
brief, four `vibe-tcg` mentions, none of them evidence about the shipped surface.
Named here so a later pass does not read it as a surviving `vibe-tcg`.

---

## Summary {#summary}

| obligation | anchor (short) | stack | recommendation |
|---|---|---|---|
| **F-187** | `##PROCEDURES-AS-AGENT-SKILLS` — `spec/boot/20-stack-…:80` | go | **FALLS** |
| **F-187** | `##PROCEDURES-AS-AGENT-SKILLS` — `spec/boot/20-stack-…:64` | rust | **FALLS** |
| **F-187** | `##PROCEDURES-AS-AGENT-SKILLS` — `spec/boot/20-stack-…:91` | typescript | **FALLS** |
| **F-189** | `##COMPONENT-THE-PRODUCT-SEAM` — `vibe-agentic-tcg-go.xml:127` | go | **STANDS-RESTATED** |
| **F-189** | `##COMPONENT-THE-PRODUCT-SEAM` — `vibe-agentic-tcg-rust.xml:132` | rust | **STANDS** |
| **F-189** | `##COMPONENT-THE-PRODUCT-SEAM` — `vibe-agentic-tcg-ts.xml:130` | typescript | **STANDS** |
| **F-190** | `##CHECK-THE-PRINTED-POLICY-LINES` — `go-ai-native-sweep/SKILL.md:47` | go | **STANDS-RESTATED** |
| **F-190** | `##CHECK-THE-PRINTED-POLICY-ORIGIN-LINES` — `rust-ai-native-sweep/SKILL.md:46` | rust | **STANDS-RESTATED** |
| **F-190** | `##CHECK-THE-PRINTED-POLICY-LINES` — `typescript-ai-native-sweep/SKILL.md:48` | typescript | **STANDS-RESTATED** |
| **F-213** | `##DRIFT-GOLDEN-TRANSCRIPTS` — `go-ai-native-sweep/SKILL.md:96` | go | **FALLS** |
| **F-213** | `##DRIFT-GOLDEN-TRANSCRIPTS` — `rust-ai-native-sweep/SKILL.md:96` | rust | **FALLS** |

**Totals over 11 anchors: 5 FALLS · 4 STANDS-RESTATED · 2 STANDS · 0
ROUTE-OUT-CANDIDATE.** Two whole obligations fall (F-187, F-213); **no obligation
survives with its recorded reason intact for every anchor** — F-189 keeps two of
three, F-190 keeps none.

**What the boss must not do with this file.** Four of the eleven anchors are
recommended to keep their drift **only under a replaced reason**. Sending the
recorded reasons to the owner would ask him to approve, on the same page: a
correction justified by the argument he struck on 2026-07-31 (F-189 go), and a
correction that deletes two sentences quoting a string the shipped binaries print
verbatim (F-190 go, typescript).

**The one finding that reaches past this batch.** F-189's registry `reason` is one
of three merged verdicts, and the merge was `by shared anchor`, not by reason
text — so the registry row *silently promoted one anchor's reasoning over three
anchors*. §B.1 then struck the finding on the strength of that one promoted
reason, and two anchors whose reasoning it never touched were struck with it. Any
obligation merged `by shared anchor` may carry the same shape; the check is one
`run/cache.json` lookup per anchor. **Recommendation: before any further
strike-by-ruling on this campaign, verify the ruling against each anchor's own
recorded reason rather than the obligation's.**
