# W7 — managed-blocks, qualified-naming, tool-design-lessons: the three sources

_Captured 2026-07-28 at the W7 opening. Every number below is the output of the
command printed above it._

W7 is the batch whose three flows are addressed to **tool authors**, and this
repository is the tool. Each flow's subject is something `vibe` itself does:

- **`managed-blocks`** — how a tool writes into a file it does not own. `vibe`
  writes into `CLAUDE.md` / `AGENTS.md` / `GEMINI.md` inside `<vibevm>` markers,
  in this repository, right now.
- **`qualified-naming`** — how an ecosystem mints identifiers. `vibe.lock` stores
  the `(group, name, version, content-hash)` tuple, and PROP-008 is the host's own
  contract for it.
- **`tool-design-lessons`** — activation models, install pipelines, identity, and
  durable-environment edits for a self-updating CLI. `vibe self install` and the
  VVM instance manager (PROP-019) are exactly that tool.

**So source 2 here is unusually strong AND unusually easy to over-claim.** These
flows were extracted from this project's own experience, so a match is often
provenance rather than independent confirmation. Say which you found: a rule the
host restates, or a rule the host's code implements. They are different evidence.

## Source 1 — the package agreeing with itself {#source-1}

```console
$ python campaigns/packages-2026-09/tasks/source1-join.py \
    packages/org.vibevm.world/managed-blocks \
    packages/org.vibevm.world/qualified-naming \
    packages/org.vibevm.world/tool-design-lessons
source-1 join over 18 file(s) under packages/org.vibevm.world/managed-blocks, packages/org.vibevm.world/qualified-naming, packages/org.vibevm.world/tool-design-lessons
  relative .md citations resolved: 35
  broken: 0
```

**Thirty-five relative citations, none broken.** Clean.

## Source 3 — the installed reality {#source-3}

```console
$ python campaigns/packages-2026-09/tasks/source23-boot-join.py | grep -cE 'managed-blocks|qualified-naming|tool-design-lessons'
0
```

**Zero of W7's three slots appear on the join's problem list**, so all three are
INSTALLED, SOURCED and word-identical to what the host boots. This is the only
world batch where source 3 is clean across the whole batch — W4 had three
WORDS-DIFFER, W5 one, W6 one NO-SOURCE.

**The host has no `spec/flows/` directory**, so the `../flows/…` pointers in these
boot snippets resolve nowhere in the consuming project. It is a fact about the
pointer, not about the rule the pointer sits under.

## Source 2 — the host's observed conformance {#source-2}

### managed-blocks — the law is executed on three files in this repository {#s2-blocks}

> **CORRECTION (2026-07-29), and it is the same failure this campaign has now
> made four times: a truncated `grep` list read as if it were the output.** Two
> of this section's code greps below list fewer files than the command returns.
> **Run them; do not read the lists.**
>
> - `grep -rln 'managed.block\|MANAGED_BLOCK\|BLOCK_START\|marker' crates/vibe-workspace/src/ --include='*.rs'` returns **13**, not the 6 listed. The seven
>   dropped are `boot_artifacts.rs`, `install/bootgen.rs`,
>   `install/bootgen/hybrid_emit.rs`, `install/tests_hybrid.rs`, `lib.rs`,
>   `publish.rs`, `publish/staging.rs`.
> - `grep -rln 'vibevm>' crates/ --include='*.rs'` returns **10**, not 6. The four
>   dropped are `boot_artifacts.rs`, `install/bootgen.rs`, `install.rs`, `lib.rs`.
>
> The list named `boot_artifacts/tests.rs` and NOT `boot_artifacts.rs` — that is,
> it dropped **the entire implementation**: the marker constants, `BlockLocation`,
> `locate_block`, `write_managed_block`, the legacy-migration branch and the
> no-op guard. A worker trusting the list would have reported the flow's whole
> state machine unimplemented. Identical in shape to the W5 harvest's dropped
> `crates/vibe-publish/src/token.rs`.
>
> **ADDITION — there is a SECOND managed-block implementation in this repository,
> and neither grep above can see it** (wrong crate, different marker vocabulary).
> `crates/vibe-cli/src/commands/vvm/env.rs` writes into a shell rc file with
> `const BLOCK_BEGIN: &str = "# >>> vibevm (VVM) — managed, do not edit by hand >>>"`
> and a matching `BLOCK_END`. It turns four facts from illustration into
> observation, and the two writers share no constant, helper or import — which is
> the flow's cohabitation claim demonstrated inside one repository. It also
> diverges on three measured points: it scans with `text.find()` rather than
> line-anchored; **it has no malformed state at all** — `split_block` falls
> through to «no block» on a duplicated or reversed pair, where the primary
> implementation returns `Malformed` and refuses to write; and its do-not-edit
> notice sits in the marker rather than on the first line inside.
>
> **The other two sections were re-derived rather than assumed to share the
> defect, and they hold exactly.** §s2-naming: `grep -coE` over `vibe.lock` for
> qualified strings returns 38 and `grep -cE '^\s*name = "'` returns 36, both as
> printed. §s2-lessons: `ls crates/` returns 18 and every one is listed, and
> `ls crates/vibe-install/src/` matches its listing file for file. So the
> truncation is confined to §s2-blocks; the rest of this harvest can be read.

```console
$ grep -n '<vibevm>\|</vibevm>' CLAUDE.md AGENTS.md GEMINI.md
CLAUDE.md:211:<vibevm>
CLAUDE.md:212:<!-- Generated by vibe — do not edit inside this block; it is rewritten on `vibe install`. Text outside the <vibevm> markers is yours. -->
CLAUDE.md:228:</vibevm>
AGENTS.md:211:<vibevm>
AGENTS.md:212:<!-- Generated by vibe — do not edit inside this block; it is rewritten on `vibe install`. Text outside the <vibevm> markers is yours. -->
AGENTS.md:228:</vibevm>
GEMINI.md:211:<vibevm>
GEMINI.md:212:<!-- Generated by vibe — do not edit inside this block; it is rewritten on `vibe install`. Text outside the <vibevm> markers is yours. -->
GEMINI.md:228:</vibevm>
```

**One delimited block per file, at the same line span in all three, carrying its
own do-not-edit notice and the explicit statement that text outside the markers
belongs to the other tenant.** That is the flow's one-line law
(«Own exactly one delimited block; never touch a byte outside it») in force, and
the 210 lines above each block are the human's — including the four rules this
campaign runs under.

The implementing code, for the facts about the state machine, the three verbs and
the byte-scan:

```console
$ grep -rln 'managed.block\|MANAGED_BLOCK\|BLOCK_START\|marker' crates/vibe-workspace/src/ --include='*.rs'
crates/vibe-workspace/src/boot/hybrid/fingerprint.rs
crates/vibe-workspace/src/boot/hybrid/hoist.rs
crates/vibe-workspace/src/boot/hybrid/testkit.rs
crates/vibe-workspace/src/boot/hybrid.rs
crates/vibe-workspace/src/boot.rs
crates/vibe-workspace/src/boot_artifacts/tests.rs
$ grep -rln 'vibevm>' crates/ --include='*.rs'
crates/vibe-check/src/checks/redirect_block.rs   crates/vibe-check/src/lib.rs
crates/vibe-cli/src/exit_code.rs                 crates/vibe-cli/tests/cli_init.rs
crates/vibe-mcp/src/pkg_servers.rs               crates/vibe-workspace/src/boot_artifacts/tests.rs
```

**Read that code before writing a row about the state machine.** The flow's
absent / present / malformed states, its «never auto-repair a malformed block —
hard stop, precise report», and its «never rewrite a file when the result is
byte-identical» each have a distinct code surface and a distinct test. A fact
about behaviour is settled in `crates/`, not in the markdown.

### qualified-naming — the identity tuple is stored, and PROP-008 is the host contract {#s2-naming}

```console
$ sed -n '174,180p' vibe.lock
[[package]]
kind = "flow"
name = "addressable-specs"
group = "org.vibevm.world"
version = "0.1.0"
source_url = "file:///C:/Users/olegc/git/v/vibevm/packages/org.vibevm.world/addressable-specs/v0.1.0"
content_hash = "sha256:7663afa33398592e419a6bf5f19e07c181f6eadcf5bb5780a1a5e62b0ef1496c"
$ grep -coE '"?(flow|feat|stack|tool|mcp):[a-z0-9.]+/[a-z0-9-]+' vibe.lock
38
$ grep -cE '^\s*name = "' vibe.lock
36
```

**The lockfile stores `(kind, name, group, version, content_hash)` as fields and
qualified `kind:group/name@version` strings in every `dependencies` list** — the
flow's identity tuple, decomposed in one place and spelled out in the other.

**Do not read the bare `name = "…"` field as a short name.** It sits beside its own
`group`, so the pair is the qualified form; the flow's prohibition is on storing a
name that *cannot* be resolved to a group. Say which reading you applied.

The host's own contract is `spec/modules/vibe-registry/PROP-008-qualified-naming.md`,
and `spec/boot/90-user.md`'s `##REGISTRY-VIBESPECS` records the repo-naming
convention in force (`NamingConvention::Fqdn`, `org.vibevm_wal`, default since
M1.19) with the legacy `flow-*` repos archived read-only. Both are durable
citation targets. The collision-vs-conflict distinction, the rename-is-a-new-identity
law, and the «short names resolve only at the CLI boundary» rule each have a code
surface in `crates/vibe-registry/` and `crates/vibe-resolver/` — find it.

### tool-design-lessons — vibe is the self-updating tool the lessons are about {#s2-lessons}

```console
$ grep -rn 'self install' crates/vibe-cli/src/cli.rs
200:    /// Manager (VVM, PROP-019). `vibe self install <selector>` builds and
$ ls crates/vibe-install/src/
apply.rs   error.rs   events.rs   fetched.rs   lib.rs   plan.rs   record.rs
$ ls crates/
progress-core  vibe-actions  vibe-check   vibe-cli     vibe-core    vibe-graph
vibe-index     vibe-install  vibe-llm     vibe-mcp     vibe-publish vibe-registry
vibe-resolver  vibe-settings vibe-spec    vibe-test-support  vibe-wire  vibe-workspace
```

The lessons' subjects are all live here: the activation model and instance
directories (PROP-019, VVM), the install pipeline (`crates/vibe-install/`), package
identity (`content_hash` in `vibe.lock`), durable-environment edits (the
`<vibevm>` block above; PATH and pointer-file handling in VVM), and removal
(`vibe uninstall`).

**Each `never` in this flow is a separate measurement**, and several are testable
rather than arguable:

- «never make an environment variable the source of truth for the active version —
  read a live pointer file each launch» — find what VVM reads at launch;
- «never overwrite a file that may be in use — write a new instance directory and
  flip a pointer» — find the instance-directory code;
- «never content-hash gigabytes to establish identity — count instances» — the
  lockfile *does* content-hash packages, which is a different subject; do not
  conflate the two;
- «never ship prose describing tooling the consumer does not receive» — this one
  is checkable against `vibedeps/` for every package in the corpus;
- «never let a package's identity include build artifacts» — check what
  `content_hash` is computed over.

**The extraction caveat applies hardest here.** `tool-design-lessons` says its
lessons were «paid for by shipping such a tool» — this tool. A host practice
matching a lesson is usually the lesson's *source*, not independent confirmation
of it. Record the direction in `searched`.

## The fifteen files and their anchor counts {#files}

Measured from `campaigns/packages-2026-09/run/mirror/`; the total agrees with
`tasks/PHASE-C-BATCHES.json` (`W7 … 15 files, 703 markers, 603 anchors`).

```
managed-blocks (198)
  19  packages/org.vibevm.world/managed-blocks/v0.1.0/README.md
   9  …/spec/boot/65-flow-managed-blocks.md
  74  …/spec/flows/managed-blocks/MANAGED-BLOCKS-PROTOCOL.md
  44  …/spec/flows/managed-blocks/adoption-guide.md
  52  …/spec/flows/managed-blocks/rejected-designs.md
qualified-naming (190)
  29  packages/org.vibevm.world/qualified-naming/v0.1.0/README.md
  14  …/spec/boot/67-flow-qualified-naming.md
  48  …/spec/flows/qualified-naming/QUALIFIED-NAMING-PROTOCOL.md
  52  …/spec/flows/qualified-naming/naming-forks.md
  47  …/spec/flows/qualified-naming/ref-grammar.md
tool-design-lessons (215)
  19  packages/org.vibevm.world/tool-design-lessons/v0.1.0/README.md
  12  …/spec/boot/70-flow-tool-design-lessons.md
  43  …/spec/flows/tool-design-lessons/TOOL-DESIGN-LESSONS.md
  63  …/spec/flows/tool-design-lessons/packaging-lessons.md
  78  …/spec/flows/tool-design-lessons/self-updating-tools.md
```

`self-updating-tools.md` at 78 anchors is the single densest file left in the
phase, and `MANAGED-BLOCKS-PROTOCOL.md` at 74 is second. Both are one slice each.

**Scope:** §3.1 sources 1, 2 and 3 for the three flows of batch W7.
