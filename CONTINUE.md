# CONTINUE.md — cold-resume checkpoint

_Written 2026-07-13, session close. A long **analytical / design session** — the owner framed it as conversation, **no product code**. Starting from the owner's big cultural-pattern extraction plan, we designed the entire **refactoring-engine program** and captured it as committed specs + plans. Nothing was executed; the design is **provisional**. Everything is committed on `main` (head `e6c818f`, **8 commits ahead of origin** at write time — see push status at the bottom); tree clean._

> `spec/WAL.md` is the canonical living state; if this snapshot and the WAL disagree, the **WAL wins**. The **git log is the authoritative per-item record.** Boot first (`CLAUDE.md` → `spec/boot/INDEX.md` → its files → `spec/WAL.md`), then read this.

---

## TL;DR

This session designed vibevm's **refactoring engine** as a program and wrote its plans. The through-line: **refactoring is the largest and most expensive activity in AI-assisted development; making it algorithmic + gated is the highest-leverage investment** — it turns `O(files)` LLM file-walking into `O(decision)` tool calls (below even the cheap-model tier). **Nothing was implemented** — the design is **provisional input** to an OpenRewrite-research-driven redesign.

**Start here next session:** `spec/terraforms/REFACTORING-ENGINE-META-PLAN-v0.1.md` — the program map that indexes everything below.

## Where work stands
- **Branch `main`**, tree clean, **8 commits ahead of origin** at write time (see push status).
- **No product code changed.** Eight design/plan docs committed (`782752c` → `e6c818f`).
- **Nothing executed.** M1, the `EmbeddedPrecedence` orphan, and the specmap regen are **parked**.

## The design arc — the eight documents

| Document | Role |
|---|---|
| `spec/common/PROP-032` | **the model** — universal typed graph (spec + code nodes, edges); agent-first IDE substrate; `code://` node (§2.3); three-tier packaging (§2.8) |
| `spec/common/PROP-031` | **the mutations** — algorithmic refactoring / codemod engine; write-side of the model; typed commands, gated; three-tier stack; operation algebra |
| `spec/common/PROP-033` | **the registry** — package-declared (`[[refactoring]]`), discovered, precompiled refactorings; three kinds (algo/llm/hybrid) |
| PROP-014 *(grows in place)* | the code↔spec **projection** of the model; gains `code://` node + spec→spec / spec→code directions |
| `spec/terraforms/SPECMAP-UNIT-MOBILITY-PLAN` | the **first operation's** execution plan (move units across package boundaries, gated) |
| `spec/research/OPENREWRITE-RESEARCH-PLAN` | the **clean-room study** that precedes + informs everything |
| `spec/terraforms/REFACTORING-ENGINE-META-PLAN` | **the program map** — start here |
| `spec/terraforms/CULTURAL-EXTRACTION-PLAN` | the **executable bootstrap** (autonomous `/goal`) — the owner's original refactoring, hardened |

## The key decisions (condensed; full list in the meta-plan §3)
1. **The model is a symmetric typed graph** (spec ↔ code ↔ package); specmap is the read-side, refactoring the write-side of the *same* model.
2. **Agent-first.** The IDE is a headless model+operations server; the agent emits typed commands over MCP; the GUI is the last, optional client.
3. **Code is a first-class addressable node** — `code://<ns>/<id>`, minted on an item marker (per-language carrier), never external/location-based.
4. **A refactoring is done only when the model re-checks clean** — atomic, deterministic, dry-run, gated.
5. **Three-tier operation stack**; wrap permissive engines (rust-analyzer / ast-grep / ts-morph), never reimplement AST surgery.
6. **Refactorings are package-declared, discovered, precompiled, cached** (the `INDEX.md`/`.mcp.json` pattern); three kinds under one gated interface.
7. **Three-tier product model** — base vibevm / SDD substrate (`org.vibevm.world`) / ai-native discipline; dependency inverted (a legacy tree gets traceability + refactoring without conform).
8. **Prose `spec://` links + spec→code become graph edges** (refactorable, not just gated).
9. **PROP-014 grows in place** (owner decision).
10. **Clean-room OpenRewrite study; iterative essential-first; a three-session firewall.**
11. **The cultural refactoring does NOT delegate** (never-delegate set: architecture + merge judgment); boss-side, restart-on-overflow.

## Next — two independent tracks (either can go first)
1. **Bootstrap (the original goal):** launch `spec/terraforms/CULTURAL-EXTRACTION-PLAN-v0.1.md` under `/goal`. Needs **only the existing specmap gate — no engine.** Boss-side, no swarm. It cleans + layers vibevm's own specs, and its `report.md` hands the engine build a concrete requirements list from doing every move by hand — resolving the chicken-and-egg.
2. **Engine:** run `spec/research/OPENREWRITE-RESEARCH-PLAN-v0.1.md` in a **fresh clean session** (clean-room firewall) → redesign PROP-031/032/033 from the findings → implement essential-first (M1 → `rename-address` → `move-unit` → composition → search/find-fix).

## Parked (deferred; or handled as bootstrap Slice 0)
- **M1** — wire `cargo xtask specmap --check` into `tools/self-check.sh` (the gate that catches severed spec↔code edges).
- **The orphan** — `EmbeddedPrecedence` (`crates/vibe-resolver/src/embedded_provider.rs:18`), untagged `pub enum`; tag it to make specmap `--check` green.
- **The specmap regen** — the host `specmap.json` is silently drifted (editorial + code-tag evolution); regen when M1 lands. (Reverted the in-progress regen this session to keep the tree clean.)
- **Candidate tweak** — add a *stop-on-stuck-gate* rule to `CULTURAL-EXTRACTION-PLAN` §4.9 (offered, not applied).

## Non-obvious findings (this session)
- The specmap engine **already** does cross-package resolution (`external_specs`), revisions/suspects, dangling detection — all tested. The gap is narrow: it's **not gated** (M1).
- The host `specmap.json` was **silently drifted** — the proof that gating it (M1) is needed.
- The specmap graph is **code→spec only**; prose `spec://` links are **not** edges → need a grep gate now, or the M5 "prose-as-edges" extension.
- Cultural patterns are mostly **process** (commit discipline, WAL, delegation) with **few code edges** → the bootstrap's dominant safety is prose-links + boot-resolves + self-check, with specmap dangling-delta catching the `token.rs`-style cases.
- `| tail` masks the real exit code (90-user.md quirk).

## Repository map (the new + relevant)
```
spec/
├─ common/     PROP-031/032/033 — NEW, provisional (the engine design)
├─ terraforms/ REFACTORING-ENGINE-META-PLAN (start here), SPECMAP-UNIT-MOBILITY-PLAN,
│              CULTURAL-EXTRACTION-PLAN (the /goal bootstrap) — NEW
├─ research/   OPENREWRITE-RESEARCH-PLAN — NEW (clean-room)
└─ WAL.md, boot/*
packages/org.vibevm.ai-native/core-ai-native/.../mechanisms/PROP-014  — the specmap engine + spec (grows in place)
crates/vibe-resolver/src/embedded_provider.rs:18   — the parked orphan
specmap.toml / specmap.json (repo root)            — the host traceability index (drifted; regen parked)
tools/self-check.sh                                — the gate (M1 adds a specmap --check step)
```

## Recent commit chain (newest first)
```
e6c818f docs(terraform): cultural-extraction bootstrap plan (autonomous /goal)
53ba94c docs(terraform): refactoring-engine meta-plan (the program map)
6c27eea docs(research): OpenRewrite clean-room research plan
39b78b9 docs(spec): refactoring registry (PROP-033) + three-tier packaging (PROP-032 §2.8)
5d2c510 docs(terraform): SPECMAP unit-mobility plan under PROP-031/032
037de30 docs(spec): PROP-031 - algorithmic refactoring, the codemod engine
782752c docs(spec): PROP-032 - project model and agent-first IDE substrate
d64276a docs(wal): session-end checkpoint — PROP-030 5/5, refactoring next   (prior session)
```

## Quick-start
```sh
bash tools/self-check.sh; echo "EXIT=$?"                      # baseline (real exit code, not a | tail)
sed -n '1,40p' spec/terraforms/REFACTORING-ENGINE-META-PLAN-v0.1.md   # the map — start here
# Bootstrap track: launch CULTURAL-EXTRACTION-PLAN under /goal.
# Engine track:    run OPENREWRITE-RESEARCH-PLAN in a fresh clean session.
```

The WAL supersedes this snapshot wherever they diverge. Next session: **read the meta-plan first**, then pick a track (bootstrap `/goal` or the OpenRewrite research).
