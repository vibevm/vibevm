# concepts.md — the EXHAUSTIVE package plan (redo v2)

The master checklist for the corrected model (see `memory/00-understanding.md`): **every
reusable idea → its own package**; content moves IN; host section deleted/thinned; package is
a **static dependency**; no loading prose; hierarchical topics become **families**. The owner's
named packages are only examples — this is the full enumeration, driven by verified traversal
so nothing is missed.

Legend: **NEW** = author the package · **EXISTS** = reuse the existing package (extend if
needed) · **FAMILY** = aggregator package whose members are sub-packages · **STAYS** = vibevm
implementation, no extraction (owner's PROP-011 example). `host fate`: **delete** (section/file
removed), **stub** (reduced to a link, no loading prose), **file-delete** (whole file gone).

---

## A. FAMILY — `org.vibevm.world/git-practices` (from PROP-000 §12 + CLAUDE.md Rules 1–4)

Aggregator (PROP-028 family); its members are separate sub-packages in ITS deps. §12 "Commit
and push discipline" heading → the family. Host §12 + CLAUDE.md rules → **deleted** (the family
is a host dep; the trio keeps at most a one-line pointer if a human index is wanted).

| member concept | package | new/exists | host source | host fate |
|---|---|---|---|---|
| Attribution — human-authored | `human-authored-packages` (owner-named) — reconcile w/ existing `attribution-policy` | NEW/EXISTS ⚠ | §12.1 · CLAUDE.md Rule 1 | delete (⚠ Rule-1 sensitivity) |
| Conventional Commits | `conventional-commits` — or reuse `atomic-commits` | EXISTS (atomic-commits) ⚠ | §12.2 · CLAUDE.md Rule 2 | delete |
| Group-by-meaning / atomicity | (in `atomic-commits`) or `group-by-meaning` | EXISTS ⚠ | §12.3 · CLAUDE.md Rule 3 | delete |
| Autonomy on routine changes | `autonomy` | NEW | §12.4 · CLAUDE.md Rule 4 | delete |
| Push discipline / pushed-history-frozen | (in atomic-commits or a `git-push-discipline`) | ⚠ | §12 push parts | delete |

⚠ **Family granularity decision:** `atomic-commits` already bundles conventional-commits +
grouping. Options: (a) `git-practices` aggregates {human-authored, atomic-commits, autonomy};
(b) split atomic-commits into fine-grained `conventional-commits` + `group-by-meaning` members.
Default: (a) reuse atomic-commits; create `autonomy` + `human-authored`(or reuse attribution-policy).

## B. NEW standalone packages (author + move content in)

| concept | package | host source | host fate |
|---|---|---|---|
| «move fast and break things» mode | `mfbt` (works beyond vibevm) | PROP-006 §2 | **file-delete** PROP-006 |
| Delegation-first directive (spend judgment, run on fractality) | `org.vibevm.fractality/delegation-first` | CLAUDE.md "Delegation-first" block | delete (⚠ fractality ledger: move or keep as host state?) |
| Setup-docs obligation | `org.vibevm.world/dev-runtime-docs` | PROP-000 §19 | delete |
| Memory discipline (project facts stay in the project) | `memory-discipline` (NEW?) or fold into two-process-model | CLAUDE.md "Memory discipline" | delete/stub |
| Package layout (mirror layout) | `package-layout` (NEW?) — reusable? | PROP-000 §13 | judgment |
| Dependency-weight-not-a-factor | fold into a `dependency-philosophy` or licensing | PROP-000 §15 | judgment |
| Production-lens / prototype-quality | `production-lens` (NEW?) | PROP-000 §17 | judgment |
| Complexity ≥ RPM | vibevm-specific? | PROP-000 §18 | likely STAYS |
| JTD wire contracts | vibevm tech choice | PROP-000 §16 | likely STAYS |

## C. EXISTING packages to reuse (move host content in, thin/delete host)

| concept | package (EXISTS) | host source | host fate |
|---|---|---|---|
| operating-modes framework | `operating-modes` | PROP-006 §1/§3 | file-delete PROP-006 (→ operating-modes + mfbt deps) |
| health-audit methodology | `health-audit` | PROP-013 | delete/stub (keep vibevm known-instances? or move to package as examples) |
| source-mirrors model | `source-mirrors` | PROP-016 | **stub**: "two mirrors: github + gitverse" + link (owner-specified) |
| token/secrets hygiene | `secrets-hygiene` | PROP-000 §20 | delete (⚠ code edge `#token-secrecy` → repoint to package) |
| managed-blocks | `managed-blocks` | PROP-012 | delete/stub (⚠ code edges `#markers` → repoint) |
| spec-genres | `spec-genres` | spec/design/README | stub |
| manual-tests | `manual-tests` | PROP-000 §14 | delete |
| licensing posture | `licensing` | PROP-000 §3 | stub (⚠ license state owner-governed; ⚠ audit EULA→UPL everywhere) |
| qualified-naming | `qualified-naming` | PROP-008 (⚠ many code edges) | stub (repoint code edges) |
| WAL discipline + session commands | `wal` | CLAUDE.md END/RESUME SESSION | delete (wal owns it) |
| specspaces | `wal-specspaces` | SPECSPACES.md · CLAUDE.md specspaces | already a dep; SPECSPACES stays as registry |
| addressable specs (spec:// URIs) | `addressable-specs` | PROP-029 | stub |
| conflict protocol / uncertainty | `conflict-protocol` | 00-core.md (RO!) · PROP-000 REVIEW markers | ⚠ source RO — repoint only |
| sync-from-code | `sync-from-code` | 00-core.md (RO) | note only |
| two-process-model | `two-process-model` | 00-core.md (RO) · book | note only |
| decision-records (ADR) | `decision-records` | PROP "Decision/Rejected" sections | genre cite |
| tool-design-lessons | `tool-design-lessons` | PROP-019/022/025/020 | companion cites (PROPs stay as impl) |
| delegation calculus (routing matrix) | `delegation-rules` | CLAUDE.md (in-place ledger) | ⚠ owner said "read in-place until fractality graduates" — reconcile with delegation-first |

## D. STAYS — vibevm implementation (owner's PROP-011 example: legit vibevm-only)

`spec/modules/vibe-registry/{PROP-001,002,010,021,023,030}`, `vibe-resolver/{PROP-003,017}`,
`vibe-index/PROP-005`, `vibe-workspace/{PROP-007,009,011,020,022,025}`, `vibe-mcp/{PROP-015,026,027}`,
`spec/common/{PROP-018,024,031,032,033}`. Analyze each for a hidden reusable seam (owner: "их
может быть МНОГО") but default to STAYS if it is genuinely vibevm machinery. Companion cites to
tool-design-lessons where lessons apply.

## E. Open decisions (surface with the plan; sensible defaults noted)

1. **git-practices granularity** — reuse atomic-commits vs split (default: reuse + add autonomy/human-authored).
2. **human-authored-packages vs existing attribution-policy** — attribution-policy exists and fits; owner named a new name. Default: reuse attribution-policy (or rename to the owner's name).
3. **delegation-first + the fractality ledger** — directive → package; ledger (live "keep current" state) default: stays host-side; delegation-rules already exists (calculus) → delegation-first is the DIRECTIVE layer above it.
4. **Rule-1 / attribution sensitivity** — extract per owner, but the operational "never attribute commits to AI" rule stays live.
5. **RO-cited anchors** — deleting an anchor cited by 00-core/90-user/VIBEVM-SPEC (RO) dangles their link → surface to owner (they own those files) or keep a redirect anchor.
6. **License audit** — sweep EULA→UPL state across the corpus (owner: "везде лицензия EULA уже давно" — verify/correct).

---

**Next:** validate this scope with the owner, then execute package-by-package (Section A family
first, then B new packages, then C existing-reuse), each via the v2 capsule + gate ladder. The
verified traversal over `allspecs.md` guarantees every in-scope unit is dispositioned.
