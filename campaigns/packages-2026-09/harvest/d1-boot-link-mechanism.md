# D1 — the boot-link mechanism: where the lane is built, and what a rewrite would cost

Evidence gathered 2026-07-29 against the working tree at `C:\Users\olegc\git\v\vibevm`.
Every claim below carries a `path:line` citation or a command with its real output.
No repository file was edited; no `git` command was run.

---

## 1 Where the lane is generated

**Crate:** `vibe-workspace`.
**File:** `crates/vibe-workspace/src/boot_artifacts.rs`.
**Function that renders the static lane:** `render_static` — declared at
`crates/vibe-workspace/src/boot_artifacts.rs:220`.
**Function that renders the manifest:** `render_index` —
`crates/vibe-workspace/src/boot_artifacts.rs:171`.
**Function that writes both to disk:** `write_boot_artifacts` —
`crates/vibe-workspace/src/boot_artifacts.rs:479`.

The exact lines where a snippet's content is copied into the output —
`crates/vibe-workspace/src/boot_artifacts.rs:253-260`:

```rust
        let body = if entry.format.is_normal() {
            compile_normal_entry(entry, workspace_root)?
        } else {
            let abs = workspace_root.join(&entry.path);
            fs::read_to_string(&abs).map_err(|e| io_err(&abs, e))?
        };
        out.push_str(body.trim_end());
        out.push_str("\n\n");
```

Line 257 reads the installed snippet; line 259 pushes it into the output buffer.
The two write sites are:

- `crates/vibe-workspace/src/boot_artifacts.rs:490` — `INDEX.md`
- `crates/vibe-workspace/src/boot_artifacts.rs:496` — `STATIC.md`

**`render_static` is the *only* renderer in the tree.** The per-unit compiler
(PROP-038) that emits a package's own `vibedeps/<slot>/spec/boot/STATIC.xml`
calls the same function:

```
$ grep -n -E 'fn |render_static|render_index|fs::write' crates/vibe-workspace/src/install/bootgen/hybrid_emit.rs
264:fn emit_effective(
283:    fs::write(
285:        boot_artifacts::render_index(effective, Some(fingerprint))?,
289:    match boot_artifacts::render_static(effective, workspace_root)? {
290:        Some(text) => fs::write(&static_path, text).map_err(|e| io_err(&static_path, e))?,
```

The driver that assembles the boot graph and calls into it is
`regenerate_boot_from` — `crates/vibe-workspace/src/install/bootgen.rs:31`, with
the per-node call at `crates/vibe-workspace/src/install/bootgen.rs:110`.

---

## 2 Transforms applied

**The body is copied verbatim.** Between reading the installed slot and writing
into `STATIC.md`, exactly one transform is applied to a `simple` package's body:

> `crates/vibe-workspace/src/boot_artifacts.rs:259` — `out.push_str(body.trim_end());`

`trim_end()` strips trailing whitespace. Nothing else touches the text. No link,
path, or reference is rewritten. The governing doc-comment says so in as many
words at `crates/vibe-workspace/src/boot_artifacts.rs:216-218`:

> `/// A `simple` contribution is carried **verbatim** (PROP-035 §3); a `normal``
> `/// one is **compiled** to its `#use`/`#source`-resolved, tree-shaken closure`

Two things the renderer *does* synthesise — both are new text prepended around
the body, never edits to it:

- `crates/vibe-workspace/src/boot_artifacts.rs:245-248` — an HTML-comment
  provenance marker `<!-- vibe:static <origin> — <path> -->`;
- `crates/vibe-workspace/src/boot_artifacts.rs:236-239` — for a soft-hoisted
  entry, a `#use spec://<origin>` marker written *instead of* the body.

The one conditional content pass is the `#embed` expander,
`crates/vibe-workspace/src/boot_artifacts.rs:266-271`, guarded by
`has_embed_directive` (`:278-280`). It is inert on this host — the lane carries
no directives at all:

```
$ grep -c -E '^\s*#(embed|use|source) ' spec/boot/STATIC.xml
0
```

And `expand_embeds` itself does not rewrite links either — every non-directive
line is pushed through unchanged (`crates/vibe-spec/src/embed.rs:64-69`).

**Materialisation is likewise byte-for-byte.** `crates/vibe-workspace/src/vibedeps.rs:352-364`:

```rust
fn place_file(src: &Path, dest: &Path, mode: CopyMode) -> Result<(), WorkspaceError> {
    match mode {
        CopyMode::Copy => {
            fs::copy(src, dest).map_err(|e| io_err(dest, e))?;
        }
        CopyMode::Hardlink => {
            if fs::hard_link(src, dest).is_err() {
                fs::copy(src, dest).map_err(|e| io_err(dest, e))?;
            }
        }
    }
    Ok(())
}
```

The module header states the contract: *"This module owns only the **layout**
and the **verbatim copy**"* — `crates/vibe-workspace/src/vibedeps.rs:16`.

---

## 3 Where the targets really live

The three probes, resolved.

**`flow-wal`** — snippet `vibedeps/flow-wal/0.2.0/spec/boot/10-flow-wal.md`
declares three links:

```
$ grep -n -o -E '\]\(\.\./flows/[^)]*\)' vibedeps/flow-wal/0.2.0/spec/boot/10-flow-wal.md
33:](../flows/wal/session-end-hook.md)
44:](../flows/wal/cold-resume.md)
58:](../flows/wal/WAL-PROTOCOL.md)
```

**`flow-campaign-plans`** — snippet
`vibedeps/flow-campaign-plans/0.1.0/spec/boot/40-flow-campaign-plans.md`:

```
$ grep -n -o -E '\]\(\.\./flows/[^)]*\)' vibedeps/flow-campaign-plans/0.1.0/spec/boot/40-flow-campaign-plans.md
15:](../flows/campaign-plans/CAMPAIGN-PLAN-FORMAT.md)
33:](../flows/campaign-plans/phase-gates.md)
35:](../flows/campaign-plans/execution-ledger.md)
```

**`flow-two-process-model`** — snippet
`vibedeps/flow-two-process-model/0.1.0/spec/boot/05-flow-two-process-model.md`:

```
$ grep -n -o -E '\]\(\.\./flows/[^)]*\)' vibedeps/flow-two-process-model/0.1.0/spec/boot/05-flow-two-process-model.md
33:](../flows/two-process-model/files-as-ipc.md)
53:](../flows/two-process-model/TWO-PROCESS-MODEL.md)
54:](../flows/two-process-model/cognitive-load-split.md)
55:](../flows/two-process-model/files-as-ipc.md)
```

The real on-disk locations, listed:

```
$ ls -l vibedeps/flow-wal/0.2.0/spec/flows/wal/WAL-PROTOCOL.md \
        vibedeps/flow-campaign-plans/0.1.0/spec/flows/campaign-plans/CAMPAIGN-PLAN-FORMAT.md \
        vibedeps/flow-two-process-model/0.1.0/spec/flows/two-process-model/TWO-PROCESS-MODEL.md
-rw-r--r-- 1 olegc 197121 10195 Jul 12 22:54 vibedeps/flow-campaign-plans/0.1.0/spec/flows/campaign-plans/CAMPAIGN-PLAN-FORMAT.md
-rw-r--r-- 1 olegc 197121  7077 Jul 12 22:54 vibedeps/flow-two-process-model/0.1.0/spec/flows/two-process-model/TWO-PROCESS-MODEL.md
-rw-r--r-- 1 olegc 197121  9898 Jul 12 22:54 vibedeps/flow-wal/0.2.0/spec/flows/wal/WAL-PROTOCOL.md
```

And the directory the compiled lane's links actually name does not exist:

```
$ ls -d spec/flows
ls: cannot access 'spec/flows': No such file or directory

$ ls spec/
WAL.md
boot
common
design
manual-tests
modules
terraforms
```

So the three targets are:

| link in the compiled lane | resolves from `spec/boot/` to | real path |
| --- | --- | --- |
| `../flows/wal/WAL-PROTOCOL.md` | `spec/flows/wal/WAL-PROTOCOL.md` (absent) | `vibedeps/flow-wal/0.2.0/spec/flows/wal/WAL-PROTOCOL.md` |
| `../flows/campaign-plans/CAMPAIGN-PLAN-FORMAT.md` | `spec/flows/campaign-plans/CAMPAIGN-PLAN-FORMAT.md` (absent) | `vibedeps/flow-campaign-plans/0.1.0/spec/flows/campaign-plans/CAMPAIGN-PLAN-FORMAT.md` |
| `../flows/two-process-model/TWO-PROCESS-MODEL.md` | `spec/flows/two-process-model/TWO-PROCESS-MODEL.md` (absent) | `vibedeps/flow-two-process-model/0.1.0/spec/flows/two-process-model/TWO-PROCESS-MODEL.md` |

**A finding that changes the shape of the problem: the link is correct where the
snippet lives, for most packages.** From
`vibedeps/flow-wal/0.2.0/spec/boot/`, `../flows/wal/WAL-PROTOCOL.md` resolves to
`vibedeps/flow-wal/0.2.0/spec/flows/wal/WAL-PROTOCOL.md` — which exists. The
`INDEX.md` dynamic lane therefore works: it names the snippet at its installed
path (`crates/vibe-workspace/src/install/bootgen.rs:327`), and a session opening
that file resolves the sibling link correctly. It is *concatenation into
`spec/boot/STATIC.xml`* that severs the link, not installation.

**Except for five packages, where the link is broken in the slot too.** Their
snippet is declared at a bare `boot/`, not `spec/boot/`, while the flow docs sit
under `spec/flows/`:

```
$ for f in vibedeps/*/*/vibe.toml; do s=$(grep -m1 '^source = ' "$f" | sed 's/source = //;s/"//g'); case "$s" in spec/*) ;; "") ;; *) echo "$f -> $s";; esac; done
vibedeps/flow-dev-runtime-docs/0.1.0/vibe.toml -> boot/58-flow-dev-runtime-docs.md
vibedeps/flow-git-atomic-commits/0.1.0/vibe.toml -> boot/30-flow-atomic-commits.md
vibedeps/flow-git-autonomy/0.1.0/vibe.toml -> boot/32-flow-autonomy.md
vibedeps/flow-git-conventional-commits/0.1.0/vibe.toml -> boot/31-flow-conventional-commits.md
vibedeps/flow-sync-from-code/0.1.0/vibe.toml -> boot/20-flow-sync-from-code.md
```

```
$ find vibedeps/flow-git-atomic-commits/0.1.0 -type f | sort
vibedeps/flow-git-atomic-commits/0.1.0/LICENSE
vibedeps/flow-git-atomic-commits/0.1.0/README.md
vibedeps/flow-git-atomic-commits/0.1.0/boot/30-flow-atomic-commits.md
vibedeps/flow-git-atomic-commits/0.1.0/spec/flows/atomic-commits/ATOMIC-COMMITS-PROTOCOL.md
vibedeps/flow-git-atomic-commits/0.1.0/spec/flows/atomic-commits/splitting-large-changes.md
vibedeps/flow-git-atomic-commits/0.1.0/vibe.toml
```

`../flows/atomic-commits/…` from `boot/` names `<slot>/flows/atomic-commits/…`,
one `spec/` short of the real file. This trait matches the campaign's earlier
in-package finding at
`spec/terraforms/PACKAGES-ACTUALIZATION-CAMPAIGN-v0.1.xml:1538-1546` — 8 broken
links across the five bare-`boot/` packages.

---

## 4 The exact counts

### 4.1 `](../flows/` in the compiled lane

```
$ grep -o -F '](../flows/' spec/boot/STATIC.xml | wc -l
69
```

(69 occurrences on 69 distinct lines — `grep -c` returns the same 69.)

### 4.2 Root-relative `spec/flows/` paths in the compiled lane

```
$ grep -o -F 'spec/flows/' spec/boot/STATIC.xml | wc -l
41
```

Of those 41, **38 are the *label* of one of the 69 links** — the markdown reads
``[`spec/flows/wal/WAL-PROTOCOL.md`](../flows/wal/WAL-PROTOCOL.md)``, i.e. the
human-visible text already asserts the host-root path that does not exist:

```
$ grep -c -E '\[`spec/flows/[^`]*`\]\(\.\./flows/' spec/boot/STATIC.xml
38
```

The remaining **3 are bare prose mentions with no link at all** — invisible to
any `\.\./flows/` scan:

```
$ grep -n -F 'spec/flows/' spec/boot/STATIC.xml | grep -v -F '](../flows/'
457:- This snippet and `spec/flows/attribution-policy/` are the **only**
651:- This snippet and `spec/flows/attribution-policy/` are the **only**
1422:- This flow owns only the protocol files under `spec/flows/wal/`, the
```

### 4.3 Distinct target documents

```
$ grep -o -E '\]\(\.\./flows/[^)#]*' spec/boot/STATIC.xml | sort -u | wc -l
60

$ grep -o -E '\]\(\.\./flows/[^/)]*' spec/boot/STATIC.xml | sort -u | wc -l
25
```

**60 distinct documents** across **25 distinct flow directories**. (61 distinct
link strings including anchors; the extra is
`../flows/discovery-prompt/usage.md#re-derive` alongside the bare
`../flows/discovery-prompt/usage.md`.)

### 4.4 How the 69 distribute across contributions

```
$ awk '/^<!-- vibe:static /{ if(src!="") printf "%3d  %s\n", n, src; src=$0; sub(/^<!-- vibe:static [^—]*— /,"",src); sub(/ -->$/,"",src); n=0; next } { n+=gsub(/\]\(\.\.\/flows\//,"&") } END{ if(src!="") printf "%3d  %s\n", n, src }' spec/boot/STATIC.xml
  3  vibedeps/flow-addressable-specs/0.1.0/spec/boot/15-flow-addressable-specs.md
  3  vibedeps/flow-campaign-plans/0.1.0/spec/boot/40-flow-campaign-plans.md
  3  vibedeps/flow-comparative-research/0.1.0/spec/boot/52-flow-comparative-research.md
  3  vibedeps/flow-conflict-protocol/0.1.0/spec/boot/35-flow-conflict-protocol.md
  3  vibedeps/flow-decision-records/0.1.0/spec/boot/25-flow-decision-records.md
  1  vibedeps/flow-dev-runtime-docs/0.1.0/boot/58-flow-dev-runtime-docs.md
  3  vibedeps/flow-discovery-prompt/0.1.0/spec/boot/50-flow-discovery-prompt.md
  2  vibedeps/flow-git-atomic-commits/0.1.0/boot/30-flow-atomic-commits.md
  3  vibedeps/flow-git-attribution-policy/0.1.0/spec/boot/55-flow-attribution-policy.md
  1  vibedeps/flow-git-autonomy/0.1.0/boot/32-flow-autonomy.md
  1  vibedeps/flow-git-conventional-commits/0.1.0/boot/31-flow-conventional-commits.md
  0  vibedeps/flow-git-practices/0.1.0/spec/boot/STATIC.md
  2  vibedeps/flow-git-atomic-commits/0.1.0/boot/30-flow-atomic-commits.md
  3  vibedeps/flow-git-attribution-policy/0.1.0/spec/boot/55-flow-attribution-policy.md
  1  vibedeps/flow-git-autonomy/0.1.0/boot/32-flow-autonomy.md
  1  vibedeps/flow-git-conventional-commits/0.1.0/boot/31-flow-conventional-commits.md
  3  vibedeps/flow-health-audit/0.1.0/spec/boot/42-flow-health-audit.md
  1  vibedeps/flow-licensing/0.1.0/spec/boot/60-flow-licensing.md
  3  vibedeps/flow-managed-blocks/0.1.0/spec/boot/65-flow-managed-blocks.md
  1  vibedeps/flow-manual-tests/0.1.0/spec/boot/44-flow-manual-tests.md
  2  vibedeps/flow-operating-modes/0.1.0/spec/boot/45-flow-operating-modes.md
  3  vibedeps/flow-qualified-naming/0.1.0/spec/boot/67-flow-qualified-naming.md
  3  vibedeps/flow-secrets-hygiene/0.1.0/spec/boot/57-flow-secrets-hygiene.md
  3  vibedeps/flow-source-mirrors/0.1.0/spec/boot/62-flow-source-mirrors.md
  3  vibedeps/flow-spec-genres/0.1.0/spec/boot/17-flow-spec-genres.md
  3  vibedeps/flow-sync-from-code/0.1.0/boot/20-flow-sync-from-code.md
  3  vibedeps/flow-tool-design-lessons/0.1.0/spec/boot/70-flow-tool-design-lessons.md
  4  vibedeps/flow-two-process-model/0.1.0/spec/boot/05-flow-two-process-model.md
  3  vibedeps/flow-wal/0.2.0/spec/boot/10-flow-wal.md
  1  vibedeps/flow-wal-specspaces/0.1.0/spec/boot/11-flow-wal-specspaces.md
  0  vibedeps/flow-redbook/0.2.0/spec/boot/03-flow-redbook.md
```

Sum = 69. The four rows between `flow-git-practices` and `flow-health-audit`
(2 + 3 + 1 + 1 = 7) are **nested** provenance markers carried *inside*
`vibedeps/flow-git-practices/0.1.0/spec/boot/STATIC.md`, which is itself a
compiled per-unit artifact concatenated into the host lane as one entry
(`crates/vibe-workspace/src/install/bootgen.rs:323-325`). So:

- **62 links** come from directly-linked snippets;
- **7 links** ride in inside the aggregator's own compiled `STATIC.md`;
- **7 links** are exact **duplicates** — the git-family blocks appear twice, once
  directly and once via `git-practices` (the double-write documented in the
  in-code REVIEW note at `crates/vibe-workspace/src/install/bootgen.rs:89-106`).

### 4.5 Would a rewrite actually land the links? Two resolutions, measured

Script: `resolve.py` / `resolve2.py` (scratchpad), reading `spec/boot/STATIC.xml`,
tracking the in-scope `vibe:static` provenance marker, and `os.path.isfile`-ing
each of the 69 targets.

**View A — resolve against the snippet directory named by the marker (including
the nested ones), i.e. the "perfect provenance" case:**

```
$ python resolve.py
total ../flows links in STATIC.md                    : 69
resolvable against the snippet dir  (naive rewrite)  : 57
DANGLING  against the snippet dir                    : 12
resolvable against <slot>/spec/flows/ (fallback)     : 69
DANGLING  against <slot>/spec/flows/                 : 0
```

The 12 failures are exactly the bare-`boot/` five (dev-runtime-docs 1,
git-atomic-commits 2 ×2 copies, git-autonomy 1 ×2, git-conventional-commits 1 ×2,
sync-from-code 3).

**View B — resolve against `BootEntry.path`, which is what `render_static`
actually holds in its loop (nested markers are text, not entries):**

```
$ python resolve2.py
total ../flows links                                    : 69
[outer entry.path only] naive resolve OK                : 54
[outer entry.path only] naive DANGLING                  : 15
[outer entry.path only] <slot>/spec/flows fallback OK   : 62
[outer entry.path only] <slot>/spec/flows DANGLING      : 7

fallback failures under the outer-entry view:
   entry: vibedeps/flow-git-practices/0.1.0/spec/boot/STATIC.md
    link: ../flows/atomic-commits/splitting-large-changes.md -> vibedeps/flow-git-practices/0.1.0/spec/flows/atomic-commits/splitting-large-changes.md
   entry: vibedeps/flow-git-practices/0.1.0/spec/boot/STATIC.md
    link: ../flows/atomic-commits/ATOMIC-COMMITS-PROTOCOL.md -> vibedeps/flow-git-practices/0.1.0/spec/flows/atomic-commits/ATOMIC-COMMITS-PROTOCOL.md
   entry: vibedeps/flow-git-practices/0.1.0/spec/boot/STATIC.md
    link: ../flows/attribution-policy/disclosure-alternative.md -> vibedeps/flow-git-practices/0.1.0/spec/flows/attribution-policy/disclosure-alternative.md
   entry: vibedeps/flow-git-practices/0.1.0/spec/boot/STATIC.md
    link: ../flows/attribution-policy/ATTRIBUTION-POLICY.md -> vibedeps/flow-git-practices/0.1.0/spec/flows/attribution-policy/ATTRIBUTION-POLICY.md
   entry: vibedeps/flow-git-practices/0.1.0/spec/boot/STATIC.md
    link: ../flows/attribution-policy/enforcement-checklist.md -> vibedeps/flow-git-practices/0.1.0/spec/flows/attribution-policy/enforcement-checklist.md
   entry: vibedeps/flow-git-practices/0.1.0/spec/boot/STATIC.md
    link: ../flows/autonomy/AUTONOMY-PROTOCOL.md -> vibedeps/flow-git-practices/0.1.0/spec/flows/autonomy/AUTONOMY-PROTOCOL.md
   entry: vibedeps/flow-git-practices/0.1.0/spec/boot/STATIC.md
    link: ../flows/conventional-commits/conventional-commits.md -> vibedeps/flow-git-practices/0.1.0/spec/flows/conventional-commits/conventional-commits.md
```

`flow-git-practices` is a pure aggregator with no flow documents of its own:

```
$ find vibedeps/flow-git-practices/0.1.0 -type f | sort
vibedeps/flow-git-practices/0.1.0/LICENSE
vibedeps/flow-git-practices/0.1.0/README.md
vibedeps/flow-git-practices/0.1.0/spec/boot/INDEX.md
vibedeps/flow-git-practices/0.1.0/spec/boot/STATIC.md
vibedeps/flow-git-practices/0.1.0/vibe.toml
```

So a one-pass rewrite keyed on `entry.path` mis-targets 7 of 69 no matter which
resolution rule it uses.

---

## 5 Rewrite precedent

**There is none.** Nothing in `vibe install` or the boot compiler rewrites any
path or link in package content. The checks that establish the absence:

1. **Materialisation.** `crates/vibe-workspace/src/vibedeps.rs:352-364` is
   `fs::copy` / `fs::hard_link` and nothing else; the module header at
   `crates/vibe-workspace/src/vibedeps.rs:16` calls it *"the **verbatim copy**"*.
   `copy_tree` (`:311-347`) walks and places; it never opens a file for reading.
2. **Compilation.** `crates/vibe-workspace/src/boot_artifacts.rs:253-260` — read,
   `trim_end`, push. The only content pass, `expand_embeds`
   (`:266-271`), is guarded and inert here (§2), and itself copies every
   non-directive line verbatim (`crates/vibe-spec/src/embed.rs:64-69`).
3. **Perimeter search over all Rust in the repo.** Every `.replace(` call in
   `crates/` and `xtask/` was enumerated and read:

```
$ rg -n --no-heading '\.replace\(' -g '*.rs' crates/ xtask/
```

   Every hit is one of: `replace('\\', "/")` path-separator normalisation
   (e.g. `crates/vibe-core/src/rel_path.rs:44`,
   `crates/vibe-workspace/src/install/bootgen.rs:327`), HTML escaping in a
   progress report (`crates/progress-core/src/report.rs:141-144`), an env-var
   name uppercase (`crates/vibe-publish/src/redirect_sync.rs:283`), or test
   fixture mutation (`xtask/src/batch_review/*`). None edits markdown link text.
4. **`vibe skill`.** The skill-install command carries one `.replace` and it is
   path-separator normalisation for JSON output:

```
$ rg -n 'fs::write|fs::copy|read_to_string|replace\(' -g '*.rs' crates/vibe-cli/src/commands/skill/
crates/vibe-cli/src/commands/skill/mod.rs:215:                    "source": s.source.display().to_string().replace('\\', "/"),
```

**Perimeter covered:** all `*.rs` under `crates/` and `xtask/`; the whole of
`crates/vibe-workspace/src/vibedeps.rs`, `boot_artifacts.rs`,
`boot_artifacts/normal.rs`, `install/bootgen.rs`,
`install/bootgen/hybrid_emit.rs`; `crates/vibe-spec/src/embed.rs`. No rewriting
of package content exists anywhere in that perimeter.

**The closest thing to a precedent** is synthesis, not rewriting: the renderer
*emits* text of its own around a body — the `<!-- vibe:static … -->` marker
(`crates/vibe-workspace/src/boot_artifacts.rs:245-248`) and, for a hoisted
entry, a `#use spec://<origin>` line written *in place of* the body
(`:236-239`). Both add lines; neither modifies a line that came from a package.

---

## 6 The two repairs, costed

### Repair A — fix the snippets

Command and real output (`--no-ignore` so ignored trees are not silently
skipped; `tr` normalises Windows separators before filtering nested copies):

```
$ rg -c --no-ignore -F '](../flows/' packages/ | tr '\\' '/' | grep -v '/vibedeps/' | grep -v '/\.vibe/' | sort
packages/org.vibevm.fractality/delegation-first/v0.1.0/spec/boot/76-flow-delegation-first.xml:1
packages/org.vibevm.fractality/delegation-rules/v0.1.0/spec/boot/77-flow-delegation-rules.xml:2
packages/org.vibevm.world/addressable-specs/v0.1.0/spec/boot/15-flow-addressable-specs.xml:3
packages/org.vibevm.world/campaign-plans/v0.1.0/spec/boot/40-flow-campaign-plans.xml:3
packages/org.vibevm.world/comparative-research/v0.1.0/spec/boot/52-flow-comparative-research.xml:3
packages/org.vibevm.world/conflict-protocol/v0.1.0/spec/boot/35-flow-conflict-protocol.xml:3
packages/org.vibevm.world/decision-records/v0.1.0/spec/boot/25-flow-decision-records.xml:3
packages/org.vibevm.world/dev-runtime-docs/v0.1.0/spec/boot/58-flow-dev-runtime-docs.xml:1
packages/org.vibevm.world/discovery-prompt/v0.1.0/spec/boot/50-flow-discovery-prompt.xml:3
packages/org.vibevm.world/git-atomic-commits/v0.1.0/spec/boot/30-flow-atomic-commits.xml:2
packages/org.vibevm.world/git-attribution-policy/v0.1.0/spec/boot/55-flow-attribution-policy.xml:3
packages/org.vibevm.world/git-autonomy/v0.1.0/spec/boot/32-flow-autonomy.xml:1
packages/org.vibevm.world/git-conventional-commits/v0.1.0/spec/boot/31-flow-conventional-commits.xml:1
packages/org.vibevm.world/health-audit/v0.1.0/spec/boot/42-flow-health-audit.xml:3
packages/org.vibevm.world/licensing/v0.1.0/spec/boot/60-flow-licensing.xml:1
packages/org.vibevm.world/managed-blocks/v0.1.0/spec/boot/65-flow-managed-blocks.xml:3
packages/org.vibevm.world/manual-tests/v0.1.0/spec/boot/44-flow-manual-tests.xml:1
packages/org.vibevm.world/operating-modes/v0.1.0/spec/boot/45-flow-operating-modes.xml:2
packages/org.vibevm.world/qualified-naming/v0.1.0/spec/boot/67-flow-qualified-naming.xml:3
packages/org.vibevm.world/secrets-hygiene/v0.1.0/spec/boot/57-flow-secrets-hygiene.xml:3
packages/org.vibevm.world/source-mirrors/v0.1.0/spec/boot/62-flow-source-mirrors.xml:3
packages/org.vibevm.world/spec-genres/v0.1.0/spec/boot/17-flow-spec-genres.xml:3
packages/org.vibevm.world/sync-from-code/v0.1.0/spec/boot/20-flow-sync-from-code.xml:3
packages/org.vibevm.world/tool-design-lessons/v0.1.0/spec/boot/70-flow-tool-design-lessons.xml:3
packages/org.vibevm.world/two-process-model/v0.1.0/spec/boot/05-flow-two-process-model.xml:4
packages/org.vibevm.world/wal-specspaces/v0.1.0/spec/boot/11-flow-wal-specspaces.xml:1
packages/org.vibevm.world/wal/v0.2.0/spec/boot/10-flow-wal.xml:3

$ … | wc -l
27

$ … | awk -F: '{s+=$NF} END {print s}'
65
```

**27 canonical snippet files, 65 link occurrences** under
`packages/org.vibevm.*/` (25 in `org.vibevm.world`, 2 in `org.vibevm.fractality`).

If the vendored dependency copies inside package trees are counted too, the
figure rises to **70 files / 184 occurrences**:

```
$ rg -c --no-ignore -F '](../flows/' packages/ | wc -l
70
$ rg -c --no-ignore -F '](../flows/' packages/ | awk -F: '{s+=$NF} END {print s}'
184
```

Those extra 43 files are `packages/org.vibevm.fractality/{delegation-rules,fractality}/v0.1.0/vibedeps/**`
— regenerated dependency copies that follow their upstream, not authored surface.

Three cost riders that the raw count hides:

- **The 3 bare prose mentions** (`spec/boot/STATIC.xml:457, 651, 1422`) carry no
  `](` and are outside the 65. So are the **38 backticked labels** — each of the
  65 links generally carries a `` `spec/flows/…` `` label that would need the
  same edit, doubling the textual churn per link.
- **The 5 bare-`boot/` packages** need their link *depth* fixed (`../flows/` →
  `../spec/flows/` or equivalent), not just its root — a different edit from the
  other 22.
- **A snippet edit does not reach the host** until each package is republished
  and reinstalled: `vibedeps/` is committed content
  (`spec/modules/vibe-workspace/PROP-009-loading-model.xml:38`), and
  `spec/boot/STATIC.xml` is regenerated from it.

### Repair B — fix the compiler

**The single function that would have to change: `render_static`,
`crates/vibe-workspace/src/boot_artifacts.rs:220`.** It is the sole renderer of
the static lane; both the host node
(`crates/vibe-workspace/src/boot_artifacts.rs:494`) and every per-unit slot
(`crates/vibe-workspace/src/install/bootgen/hybrid_emit.rs:289`) go through it,
so one edit at line 259 covers both. The rewrite would sit between the read at
line 257 and the push at line 259: scan the body for markdown links matching
`](../flows/…)`, resolve each against the directory of the contributing snippet,
and re-express the result relative to the node's `spec/boot/`.

**What the correct target path is from `spec/boot/`.** `spec/boot/` is two
levels below the workspace root, so a workspace-relative target `T` becomes
`../../T`. For `flow-wal` that is
`../../vibedeps/flow-wal/0.2.0/spec/flows/wal/WAL-PROTOCOL.md`. In general:
`../../<slot>/spec/flows/<name>/<doc>.md`, where `<slot>` is
`vibedeps/<kind>-<name>/<version>`.

**Is the information available at that point in the code? Yes — for the slot,
and it is already used there.** `BootEntry` carries `path`, documented as
*"Workspace-root-relative, forward-slashed path of the boot file"*
(`crates/vibe-workspace/src/boot.rs:131-132`), plus `origin`, the
`<group>/<name>` pkgref (`crates/vibe-workspace/src/boot.rs:141-143`).
`render_static` already reads both — it formats them into the provenance marker
at `crates/vibe-workspace/src/boot_artifacts.rs:245-248` and joins `entry.path`
against `workspace_root` at line 256. Deriving the slot is a prefix of
`entry.path`; deriving `../../` is a constant, since the artifact is always
written to `<node>/spec/boot/`
(`crates/vibe-workspace/src/boot_artifacts.rs:484`). No new plumbing, no new
parameter, no new lookup is needed for the ordinary case.

**Two cases where the available information is *not* sufficient**, measured in
§4.5:

- **The bare-`boot/` five (12 of 69 links).** Resolving `../flows/…` against
  `entry.path`'s directory yields `<slot>/flows/…`, which does not exist. To
  land these the rewrite must *not* be a faithful relative resolution but a
  heuristic — "resolve, and if that misses, retry under `<slot>/spec/`". The
  information for the heuristic is present (the slot root is a prefix of
  `entry.path`); the information for a *correct* resolution is not, because the
  source link is itself wrong.
- **The aggregator's compiled `STATIC.md` (7 of 69 links).** For
  `vibedeps/flow-git-practices/0.1.0/spec/boot/STATIC.md`, `entry.path` names
  the aggregator, but the seven links belong to four *other* packages.
  `flow-git-practices` has no `spec/flows/` at all, so **both** the faithful and
  the heuristic resolution produce a dangling path. The true provenance exists
  only as HTML-comment markers *inside the body text* — the compiler would have
  to parse its own output, or the rewrite would have to be applied recursively
  at per-unit emission time (with the outer pass then required to leave
  already-rewritten links alone).

---

## 7 What would make a repair wrong

### 7.1 `spec/flows/` is *not* a directory the design intends to exist

The governing decision retires it explicitly.
`spec/modules/vibe-workspace/PROP-009-loading-model.xml:40-41`:

> `- ##MIRROR-RETIRED **Consequence — the mirror layout is retired.**`
> `VIBEVM-SPEC.md` §13.1's mirror layout (a package's `[writes]` entry is both
> source and target path) worked only because a dependency landed at one fixed
> path in every project.
>
> `- @fact:WRITES-RETIRED-WHY A materialised package is now its own verbatim subtree
> under `vibedeps/<slot>/`; **a package's internal cross-references must become
> package-relative or `spec://` URIs.**`

That second clause is a standing, un-discharged obligation on the *packages*.
The lockfile confirms nothing will ever create `spec/flows/`:

```
$ grep -o 'files_written = .*' vibe.lock | sort | uniq -c
     36 files_written = []
```

And `spec/modules/vibe-workspace/PROP-009-loading-model.xml:34` forbids the
alternative outright: *"`vibe install` **never writes into any node's authored
`spec/`**"*.

### 7.2 …but a stale statement in the spec tree still says it does

`spec/common/PROP-000.xml:173`, §13 "Package layout convention":

> @fact:mirror-example Concretely, the canonical `flow:wal@0.1.0` payload … contains
> `spec/flows/wal/WAL-PROTOCOL.md` at exactly that relative path; after `vibe
> install flow:wal`, the file lives at `spec/flows/wal/WAL-PROTOCOL.md` inside
> the user's project. **No mapping, no rewriting.** @status:spec/done

It is marked `@spec/done` and carries no supersession note pointing at
PROP-009 §2.1. Read on its own it says both that `spec/flows/` *is* the
consumer-side path (contradicted by §7.1) and that vibevm does not rewrite paths
(true today, and an argument against Repair B).

A matching stale claim sits in the code:
`crates/vibe-workspace/src/vibedeps.rs:17-20`

> `//! additive: it never retires the legacy `[writes]` mirror layout`
> `//! (`VIBEVM-SPEC.md` §13.1). That retirement is the `vibe install``
> `//! switch-over — a later PROP-009 phase …`

PROP-009 §2.1 declares that retirement `@impl/done`. The module comment and the
PROP disagree about whether it has happened.

### 7.3 "Verbatim" is a word in the decision Repair B would touch

`spec/modules/vibe-workspace/PROP-009-loading-model.xml:61`:

> `- ##ARTIFACT-STATIC-MD **`STATIC.md`** — the **verbatim** concatenation, in
> priority order, of every `static`-typed (§2.4) contribution …` @status:impl/done

Repeated at `:99` for the link type itself:

> `- ##LINK-STATIC `link = "static"` — the contribution's boot text is compiled
> **verbatim** into `STATIC.md` …`

A compile-time link rewrite makes `STATIC.md` no longer a verbatim concatenation.
That is a spec amendment, not just a code change — and it would also break the
byte-identity property the per-unit compiler deliberately preserves
(`crates/vibe-workspace/src/install/bootgen.rs:38-40`: *"For a tree with no
intermediate static edge this is a no-op, keeping the node artifacts
byte-identical (PROP-038 §5)"*).

### 7.4 The dynamic lane is not broken, so a repair could over-reach

`spec/boot/INDEX.md` names each snippet at its installed path
(`crates/vibe-workspace/src/install/bootgen.rs:327`), e.g.
`vibedeps/flow-delegation-rules/0.1.0/spec/boot/77-flow-delegation-rules.md`
(`spec/boot/INDEX.md:30`). Opened there, `../flows/…` resolves correctly for the
22 `spec/boot/`-rooted packages. Any repair that changes the *snippet* text
(Repair A) changes what that working reader sees; any repair that changes only
the *compiled lane* (Repair B) leaves two divergent renderings of the same
sentence on disk.

### 7.5 The double-write means a rewrite would land twice

Seven of the 69 links are duplicates arriving by two paths (§4.4). The
duplication itself is an open design question the code refuses to paper over —
`crates/vibe-workspace/src/install/bootgen.rs:89-106` (a `REVIEW: DRIFT-030 §4`
comment ending *"Which mechanism owns the dedup is a design question, so
DRIFT-030 stopped on its §8 rather than paper over it at this call."*). A
compiler rewrite would have to be correct under duplication; a snippet fix is
indifferent to it.

### 7.6 A third target exists that neither repair addresses

`CLAUDE.md:141` names a root-relative path in prose:

> the full protocol is `spec/flows/wal-specspaces/SPECSPACES-PROTOCOL.md` inside
> that package

```
$ grep -n -F 'spec/flows/' CLAUDE.md
141:This repository can host **specspaces**: … the full protocol is `spec/flows/wal-specspaces/SPECSPACES-PROTOCOL.md` inside that package. …
```

Same dangling path, in a hand-authored file that no compiler touches and no
package owns. The three bare mentions in the compiled lane
(`spec/boot/STATIC.xml:457, 651, 1422`) are the package-side analogue: prose, not
links, invisible to a `\.\./flows/` scan and therefore to a rewrite that only
matches markdown link syntax.
