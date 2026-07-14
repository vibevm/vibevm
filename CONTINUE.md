# CONTINUE.md — cold-resume checkpoint (2026-07-16)

> `spec/WAL.md` is the canonical living state; if this snapshot and the WAL diverge, the WAL wins.

## TL;DR

The **spec-compiler mission (PROP-035)** is complete and, on top of it, the **link-type rename** shipped —
vibevm's boot terminology now matches the CS static/dynamic-linking standard:

- **`static`** (was `inline`) — compiled verbatim into the **`STATIC.md`** lane ("the static compiler").
- **`dynamic`** (was `static`) — the **default**; resolved to a path in `INDEX.md`, read by reference, with
  an optional `when` gating the read.
- the old `dynamic` is **gone** — a conditional load is just a `dynamic` entry carrying a `when`.
- `inline-transitive`→`static-transitive`; `INLINE.md`→`STATIC.md`; `render_inline`→`render_static`;
  `compile_inline`→`compile_static`.

Also shipped this window: **`simple` is now the default package format** (fail-safe over fail-silent,
PROP-035 §3), and **`redbook@0.2.0` gained `wal-specspaces`** (every `org.vibevm.world` content package is
now in the edition).

**Everything is on `main` (== origin == github), full-workspace `bash tools/self-check.sh` is GREEN.**
No blocker.

## Where work stands

- Branch **`main`**, working tree clean, pushed both remotes. The rename spans `vibe-core` / `vibe-workspace`
  / `vibe-spec` / `vibe-index`, the package manifests, the specs, and the regenerated boot artifacts
  (`spec/boot/INLINE.md` → `spec/boot/STATIC.md`, git-detected rename; the `CLAUDE.md`/`AGENTS.md`/`GEMINI.md`
  redirect points at `STATIC.md`).
- **Gate state: `bash tools/self-check.sh` GREEN** — fmt, `cargo test --workspace`, clippy `-D warnings`,
  `vibe check` (0/0/0), `conform`, and the specmap ratchet all pass.

## Active blocker + the exact unblock

**None.**

## Next steps (post-mission, optional)

1. **Migration (§15)** — adopt the `normal` format on real packages, in order: demo corpus (done) → all of
   `org.vibevm.world` → vibevm's own core specs last. A package adopts by splitting into `contract/` +
   `source/`, using directives, and loading `spec/design/structural-loader.md` first.
2. **Equivalence testing (§16)** — differential-test the static compiler (reference semantics) against the
   structural loader on a corpus. Empirical, deferred by design.
3. **`link` × `format`** open question (PROP-035 §17): does a `normal` + `static` edge read eagerly or lazily?
4. Nested-section source-merge (the flat-contract case is done).
5. Fold `vibe-spec` into `conform.toml` `gated_crates` once it is spec-tagged + REQ-edged (it is `exempt`).

## Non-obvious findings (do not re-learn)

- **The link model is now two types**: `static` (verbatim `STATIC.md`) and `dynamic` (by-reference
  `INDEX.md`, `when` optional). `INDEX.md` `kind` is **derived from `when`** — conditional → `"dynamic"`,
  unconditional → `"static"` — which preserves the manifest output. `STATIC.md` replaces `INLINE.md`.
- **The rename had a swap trap**: old `Static` and old `Inline` change places, so `LinkType::Static` in code
  compiles but flips meaning. It was done with a temp-marker sed (`Static→TMP→Dynamic`, `Inline→Static`),
  never a blind find-replace; escaped strings (`\"inline\"`) were sed-missed and fixed by hand.
- **`simple` is the default format** (PROP-035 §3): a forgotten `format` over-loads (visibly working) rather
  than silently loading nothing.
- **The payoff is guarded** — `render_static` only compiles when the lane carries a `#embed`; a directive-free
  lane is byte-identical, so vibevm's own boot stays as-is until it adopts the format.
- **Reinstall re-materializes with LF line endings** — after `vibe install` the `vibedeps/` tree shows a huge
  CRLF→LF diff that is **noise** (content-identical); revert it (`git checkout -- vibedeps/`) and keep only
  the boot artifacts + trio + lock. The old `spec/boot/INLINE.md` is **orphaned** by the renamed bootgen
  (it manages `STATIC.md` now) — `git rm` it.
- **Commits:** heredoc only, never `-m` with backticks; **no AI-authorship trailers** (Rule 1).
- **Editing:** Edit/Write for `.md` with non-ASCII (§); sed is safe on `.rs` (ASCII) and byte-preserves `§`.
  WAL is too big for the Read tool — read its head via `Get-Content -TotalCount`.

## Repository map

- `crates/vibe-spec` — the spec compiler (address / doctree / resolver / directives / merge / embed /
  use_graph / pipeline (`compile_static`) / link_table / markers). Integration tests + `tests/fixtures/ws`.
- `crates/vibe-workspace` — install + **bootgen** (`render_static` / `INDEX.md`, the payoff, transitive-static
  in `install/bootgen.rs`); `boot.rs` (`static_entries` / `dynamic_entries`). `crates/vibe-core` — manifests +
  `LinkType` (`Static` / `Dynamic` / `StaticTransitive`). Other `crates/vibe-*` as before.
- `packages/org.vibevm.*/**` — practice flows + stacks + fractality. `spec/` — `boot/` (00-core, 90-user,
  generated INDEX.md + **STATIC.md**), `common/`, `modules/` (PROP-009 loading-model, PROP-034, **PROP-035**),
  `design/` (incl. `structural-loader.md`), `WAL.md`.
- Root: `CLAUDE.md`/`AGENTS.md`/`GEMINI.md` (byte-identical trio, redirect→STATIC.md), `conform.toml`,
  `vibe.toml`, `vibe.lock`.

## Recent commits (last 20)

```
refactor(rename): clean the last INLINE.md references (PROP-035)
refactor(boot): STATIC.md artifacts + the missed vibe-index wire (PROP-035)
refactor(spec): rename inline->static, static->dynamic (PROP-035)
refactor(packages): rename link wire values for static/dynamic (PROP-035)
refactor(vibe-spec): rename the inline compiler to the static compiler (PROP-035)
refactor(link): rename inline->static, static->dynamic (PROP-035)
spec(vibe-workspace): default package format is simple (PROP-035 §3)
feat(redbook): include wal-specspaces in the edition
docs(continue): PROP-035 complete — cold-resume for the finished mission
docs(wal): mark PROP-035 complete — the spec compiler shipped
feat(vibe-workspace): the inline-transitive link (PROP-035 §12)
feat(vibe-workspace): compile the inline boot lane (PROP-035)   [the payoff]
docs(spec-compiler): author the structural loader (PROP-035 §13)
feat(vibe-spec): reversible block markers and decompile (PROP-035)
feat(vibe-spec): build directive link tables (PROP-035 §10)
feat(vibe-spec): wire the source-fold into compile_static (PROP-035)
feat(vibe-spec): fold source into contract document (PROP-035)
feat(vibe-spec): admit contract-only #use cycles (PROP-035)
feat(vibe-spec): compile the pipeline / topo #use / expand #embed (PROP-035)
feat(vibe-spec): the spec:// router — address, doctree, resolver (PROP-035)
```

## Quick-start

```sh
cargo test -p vibe-spec                 # the compiler crate
bash tools/self-check.sh                # the full gate — expect "self-check: all green"
cargo build -p vibe-cli                 # the working-tree binary (never the PATH vibe)
./target/debug/vibe.exe install --registry packages --assume-yes   # regenerate boot (MCP off; revert vibedeps CRLF noise after)
```
