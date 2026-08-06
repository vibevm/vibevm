# PROP-012: The managed redirect block — vibevm as a co-tenant of the agent instruction files {#root}

<status stage="impl" state="done" comment="B0 2026-07-24: IMPLEMENTED, M1.18 phase 7 block engine"/>

@fact:milestone-line **Milestone:** design proposal; it refines [PROP-009 §2.3](PROP-009-loading-model.md) (M1.18, Phases 1–6 shipped) and **corrects a destructive defect already in the shipped Phase-4 code** — so it is a prerequisite for PROP-009 §4 / the M1.18 Phase-7 redirect rewrite, not a far-future milestone. Owner to place in [`ROADMAP.md`](../../../ROADMAP.md). Not implementation-locked. @status:impl/done

@fact:status-line **Status:** IMPLEMENTED — shipped with M1.18 Phase 7 (the block engine in `vibe-workspace::boot_artifacts`, plan-time validation, the `vibe check` `RedirectBlock` finding, and the self-migration that put the `<vibevm>` block into this repository's own instruction files). Requirements were captured 2026-05-22 (drafts 1–2); decision units typed at REQ grain 2026-06-12 (the depth program). @status:impl/done

@fact:related **Related:** [PROP-009](PROP-009-loading-model.md) (the loading model — §2.3 the redirect, §4 migration; the Phase-4 implementation `vibe-workspace::boot_artifacts::write_boot_artifacts` / `render_redirect` / `REDIRECT_FILES` this PROP reworks); [PROP-007](PROP-007-workspace.md) (workspaces — every entry-point node carries its own instruction files). @status:spec/done

@fact:discipline-line **Discipline:** the general co-tenant / managed-block pattern — the co-tenant law, the marker design, the well-formedness state machine that classifies a file before mutation, and the create / update / remove verbs — is the `managed-blocks` flow: `spec://org.vibevm.world/managed-blocks/flows/managed-blocks/MANAGED-BLOCKS-PROTOCOL#root`. This PROP is vibevm's instance of it — the `<vibevm>` redirect block in the agent instruction files, verified by `vibe-workspace::boot_artifacts` and the `vibe check` `RedirectBlock` finding. @status:spec/done

@fact:OWNER-SANCTION **Owner sanction:** PROP-012 amends PROP-009 §2.3 (a PROP document — editable) and reshapes the redirect-file wording in `VIBEVM-SPEC.md` §6.1 / §4.2. Those `VIBEVM-SPEC.md` edits fall inside the M1.18 Phase-7 spec-consistency sanction already granted; PROP-012 adds no new owner-frozen surface. @status:impl/done

---

## 1. Motivation {#motivation}

- @fact:redirect-status-quo PROP-009 §2.3 has `vibe install` generate the `CLAUDE.md` / `AGENTS.md` / `GEMINI.md` **thin redirect** that points a session at the boot artifacts. @status:impl/done
- @fact:whole-file-write The Phase-4 implementation — `vibe-workspace::boot_artifacts::write_boot_artifacts` — does this with an unconditional whole-file `fs::write`: every `vibe install` / `vibe reinstall` / `vibe uninstall` / `vibe update` replaces the **entire file** with vibevm's generated content. @status:impl/done

- @fact:wrongness-claim This is wrong, and it is wrong in the most damaging possible place. @status:impl/done

@fact:SHARED-SURFACE `CLAUDE.md` / `AGENTS.md` / `GEMINI.md` are not vibevm's files. They are the **shared contact surface every coding agent reads at session start**, and many parties legitimately write there: @status:impl/done

- @fact:tenant-developer the developer's own hand-authored project instructions; @status:impl/done
- @fact:tenant-other-tools another tool, agent, framework, or convention the developer runs *alongside* vibevm — vibevm must assume it is never the only tenant of a repository; @status:impl/done
- @fact:tenant-vibevm-repo in the vibevm repository itself, `CLAUDE.md` is today ~200 hand-authored lines carrying the four non-negotiable rules and the session-end command. @status:impl/done

- @fact:silent-destruction A whole-file overwrite silently destroys every byte of that. @status:impl/done
- @fact:data-loss-event The first `vibe install` in any project with a non-trivial `CLAUDE.md` is a data-loss event — and it destroys precisely the file a developer is most likely to have invested in by hand. @status:impl/done

- @fact:FOUNDING-RULE-CONTRADICTION It also contradicts vibevm's own founding rule. PROP-009 §2.1 — the C++-`#include` rule — is that *installing a dependency must never modify a node's authored content*. @status:impl/done
- @fact:rule-violated The redirect write violates exactly that, for the most visible authored file in the project. @status:impl/done

- @fact:FIX-MANAGED-BLOCK The fix is the standard managed-block discipline (the general problem and the co-tenant law: the `managed-blocks` flow's `spec://org.vibevm.world/managed-blocks/flows/managed-blocks/MANAGED-BLOCKS-PROTOCOL#problem`) — the shape `ssh`, shell-rc installers, and countless config tools already use: vibevm owns one small, clearly-delimited, **machine-findable** region of each instruction file, and never touches a byte outside it. @status:impl/done
- @fact:good-co-tenant vibevm becomes a **good co-tenant** — it writes its redirect into its own pen and leaves the rest of the file to whoever else shares it. @status:impl/done

---

## 2. Decisions {#decisions}

### 2.1 vibevm owns one delimited block, nothing else {#co-tenant}

@fact:req-co-tenant `req r1` @status:impl/done

@fact:ONE-BLOCK-LAW **Decision.** `vibe` owns exactly one **managed block** inside each agent instruction file — a contiguous region bounded by an opening and a closing marker. @status:impl/done

- @fact:OUTSIDE-PRESERVED `vibe` reads and rewrites only the content *between* the markers; every byte outside the block is treated as another tenant's property and preserved verbatim across every `vibe` operation. @status:impl/done

- @fact:SHARED-FILE-REFRAME The instruction file ceases to be "a vibevm-generated file." It is a **shared file with a vibevm-managed block in it**. @status:impl/done
- @fact:REDIRECT-FILE-SET The set of instruction files is PROP-009's `REDIRECT_FILES` — today `CLAUDE.md`, `AGENTS.md`, `GEMINI.md`; the set is open. @status:impl/done

### 2.2 The markers, and well-formedness {#markers}

@fact:req-markers `req r1` @status:impl/done

@fact:BARE-TAGS **Decision.** The block is delimited by the literal **bare tags** `<vibevm>` and `</vibevm>` — opening and closing — each alone on its own line. @status:impl/done

- @fact:bare-tags-why Bare tags are chosen over HTML-comment delimiters (`<!-- vibevm:begin -->` …): they read unambiguously to an LLM, the file's primary consumer. @status:impl/done
- @fact:cosmetic-cost A markdown renderer may display a bare non-standard tag oddly — an accepted cosmetic cost, addressed separately if it ever matters. @status:impl/done

@fact:canonical-block-lead The canonical block vibevm writes: @status:impl/done

```
<vibevm>
<!-- Generated by vibe — do not edit inside this block. It is rewritten on `vibe install`. -->
... the boot redirect (§2.6) ...
</vibevm>
```

- @fact:PLAIN-TEXT-SCAN The markers are located by a **plain line-anchored text scan** — no markdown parse, no model in the loop. "Programmatically findable without an LLM" is a hard requirement: the markers gate a mutating write, so detection must be deterministic and trivial. @status:impl/done
- @fact:WELL-FORMED-DEF A **well-formed** instruction file contains either *zero* markers (the block is absent — §2.3) or *exactly one* `<vibevm>` line followed, later in the file, by *exactly one* `</vibevm>` line. @status:impl/done
- @fact:MALFORMED-DEF Anything else is **malformed**: two or more of either marker, an opener with no closer, a closer with no opener, or a closer preceding its opener. @status:impl/done

### 2.3 Malformed → hard stop; absent → create; present → splice {#create}

@fact:req-create `req r1` @status:impl/done

@fact:CLASSIFY-FIRST **Decision.** On every operation that would write the block, `vibe` first classifies each instruction file: @status:impl/done

- @fact:CLASS-MALFORMED **Malformed** (§2.2) — `vibe` **aborts the whole operation** with an error naming the file and the exact defect, and changes nothing. It does not proceed until the user repairs the file by hand. vibevm **never guesses** which of two blocks is canonical and **never auto-deletes** a stray marker — a malformed managed block is always a human's call. @status:impl/done
- @fact:CLASS-ABSENT **Absent** (zero markers) — `vibe` appends a fresh block at the end of the file, preceded by one blank line. If the file itself does not exist, `vibe` creates it containing only the block. @status:impl/done
- @fact:CLASS-PRESENT **Present** (one well-formed pair) — `vibe` replaces the content *between* the markers with freshly-generated content; the markers themselves and all text outside them are untouched. If the new inter-marker content is byte-identical to the old, `vibe` writes nothing — no git churn. @status:impl/done

### 2.4 Placement is the user's — vibevm never moves the block {#placement}

@fact:req-placement `req r1` @status:impl/done

@fact:PLACEMENT-ONCE **Decision.** vibevm decides the block's position **exactly once** — when it first creates the block (§2.3, absent → create) it is appended at the **end of the file**. @status:impl/done

@fact:POSITION-USERS From then on the position is the **user's**: `vibe` rewrites the content between the markers and **never relocates the markers**; whatever text precedes or follows the block stays exactly where it is, across every `vibe` operation. @status:impl/done

@fact:humble-default The end-of-file default is the humble, co-tenant choice — vibevm does not claim the attention-priority opening of a shared file, so the developer's own instructions and any other tool's content load first. @status:impl/done

@fact:POSITION-KNOB The position is then a deliberate, user-owned knob. Because vibevm honours wherever the block sits, the user controls how strongly its redirect weighs in a session: @status:impl/done

- @fact:position-top moved to the **top** of the file, vibevm's redirect reads as the **"First Prompt"** — a system-prompt-like instruction with maximum attention weight; @status:impl/done
- @fact:position-end left at the **end**, vibevm behaves as a **sidecar**, secondary to the user's own content. @status:impl/done

@fact:manual-once It is a manual configuration the user makes by hand, once; vibevm supplies the polite default and never overrides the choice. @status:impl/done

### 2.5 Validate before mutate {#plan-time}

@fact:req-plan-time `req r1` @status:impl/done

@fact:PLAN-TIME-VALIDATION **Decision.** The well-formedness classification of §2.3 runs at **plan time** — before any `vibedeps/` materialisation or boot-artifact write. @status:impl/done

@fact:FAIL-FAST A malformed block fails the operation **fast and clean**: an install never gets half-applied, with a materialised `vibedeps/` tree and an unwritable redirect. This is the same plan-time-not-apply-time discipline vibevm already applies to its user-owned-path guards. @status:impl/done

### 2.6 The block content is unchanged — this PROP changes the envelope, not the payload {#content}

@fact:design-content `design r1` @status:impl/done

@fact:PAYLOAD-UNCHANGED **Decision.** What goes *between* the markers is exactly the redirect content PROP-009 §2.3 already specifies — an instruction to read `spec/boot/STATIC.md` (when present) and then `spec/boot/INDEX.md`. @status:impl/done

- @fact:ENVELOPE-NOT-PAYLOAD PROP-012 changes how that content is *delivered* — a fenced region of a possibly-shared file — not what it *says*. @status:impl/done

- @fact:SUPERSEDES-WORDING Consequently this PROP **supersedes the "thin generated file" wording of PROP-009 §2.3**: the instruction file is no longer wholly generated; only its `<vibevm>` block is. @status:impl/done
- @fact:SESSION-ORDER-UNCHANGED PROP-009 §2.3's session-start order is otherwise unchanged — the agent reads the instruction file, then `STATIC.md`, then `INDEX.md`. @status:impl/done
- @fact:WARNING-SCOPED The "generated — do not edit" warning moves *inside* the block and is reworded to scope it to the block. @status:impl/done
- @fact:CHECK-FINDING A malformed block is also a `vibe check` finding (§3), so the user meets the problem in the linter rather than mid-install. @status:impl/done

---

## 3. Command and crate surface {#surface}

@fact:design-surface `design r1` @status:impl/done

- @fact:SURF-BOOT-ARTIFACTS **`vibe-workspace::boot_artifacts`** — the implementation site. `write_boot_artifacts`'s redirect loop changes from a whole-file `fs::write` to a locate-validate-splice. New helpers: locate the block, classify well-formedness, splice the body. `render_redirect` becomes the block-*body* renderer; `REDIRECT_BODY`'s "do not edit" framing moves inside the block. @status:impl/done
- @fact:SURF-ORCHESTRATION **The install orchestration** (`apply_resolution` and the `vibe reinstall` path) — gains the §2.5 plan-time validation pass over every node's instruction files before any mutation. @status:impl/done
- @fact:SURF-CLI **`vibe-cli`** — `vibe install` / `reinstall` / `uninstall` / `update` surface the malformed-block abort as a clear error; `vibe init` scaffolds each instruction file with a `<vibevm>` block instead of a one-line file. @status:impl/done
- @fact:SURF-CHECK **`vibe-check`** — `CheckId::RedirectBlock` reports a malformed `<vibevm>` block (a file with anything other than zero markers or one ordered pair), composing with the boot-directory check. @status:impl/done
- @fact:SURF-EXIT-CODE **Exit code** — the malformed-block abort is conflict-shaped; reusing exit code `3` (package conflict) is the working assumption (§5). @status:spec/work

---

## 4. Migration {#migration}

@fact:req-migration `req r1` @status:impl/done

@fact:MIGRATION-CASE The one non-trivial case is an instruction file that is *wholly* the **old whole-file generated redirect** — written by the Phase-4 `fs::write`, recognisable by its generated header, and carrying no `<vibevm>` markers. For a file with no markers, `vibe`: @status:impl/done

- @fact:MIG-RECLAIM if the **entire file** is recognisably the old whole-file generated redirect (it matches the known generated header) — replaces it with a clean file containing just the `<vibevm>` block. The old content *was* vibevm's; reclaiming it as a block loses nothing. @status:impl/done
- @fact:MIG-APPEND otherwise — takes the §2.3 **absent → create** path: appends a block, preserves everything else. @status:impl/done

- @fact:self-migration-path This is also the gentle path for the **vibevm repository's own self-migration** (PROP-009 §4 / M1.18 Phase 7). @status:impl/done
- @fact:SELF-MIGRATION-APPEND vibevm's `CLAUDE.md` / `AGENTS.md` / `GEMINI.md` are hand-authored — not the generated form — so they take the append path: every hand-authored line, including the four non-negotiable rules, **stays**, and a `<vibevm>` block is appended. @status:impl/done
- @fact:delicacy-removed The destructive whole-file write is what made the Phase-7 self-migration delicate; with PROP-012 it is not. @status:impl/done

@fact:PREREQUISITE-LAW Because the defect is in shipped code, PROP-012 **must land before any `vibe install` / `vibe reinstall` is run against a project with a real `CLAUDE.md`** — including the vibevm repository itself. It is therefore a prerequisite of the Phase-7 redirect rewrite, not a successor to it (§7). @status:impl/done

---

## 5. Open questions {#open}

1. @fact:OPEN-EXIT-CODE **Exit code.** Reuse `3` (package conflict — a malformed managed block is conflict-shaped) or mint a dedicated code? Leaning: reuse `3`. @status:spec/work
2. @fact:OPEN-OLD-REDIRECT-DETECTION **Old-redirect detection for migration (§4).** Match the exact Phase-4 generated-header string (precise; a false positive would wrongly destroy a hand-authored file) versus a looser heuristic. Leaning: exact string match. @status:spec/work

@fact:draft2-closed Closed in draft 2: the marker syntax — bare `<vibevm>` … `</vibevm>` tags (§2.2), chosen for legibility to an LLM; the markdown-rendering cosmetics are an accepted, deferred cost. @status:spec/done

---

## 6. Rejected / deferred alternatives {#rejected}

- @fact:REJ-WHOLE-FILE **The whole-file overwrite** (the shipped Phase-4 status quo). Rejected — it destroys co-tenant content; undoing it is the reason this PROP exists. @status:spec/done
- @fact:REJ-HEURISTIC-DETECTION **Heuristic or LLM-assisted block detection** ("find the part that looks like vibevm's"). Rejected — the region that gates a mutating write must be found by a deterministic byte scan, with no model and no guessing. @status:spec/done
- @fact:REJ-AUTO-REPAIR **Auto-repairing a malformed file** (delete the surplus marker, keep the first block). Rejected — vibevm never decides which of two regions is canonical; a malformed file is a hard stop for the human (§2.3). @status:spec/done
- @fact:REJ-SIDECAR **A sidecar file** (`.vibe/redirect`, a `vibe`-owned file the agent is pointed at) instead of a block inside `CLAUDE.md`. Rejected — the entire value of `CLAUDE.md` / `AGENTS.md` / `GEMINI.md` is that every agent already reads them with zero configuration (PROP-009 §6.1's cross-agent property). A sidecar the agent does not natively read forfeits that. @status:spec/done

---

## 7. Phase plan {#phases}

1. @fact:PHASE-1-BLOCK-ENGINE **The block engine** in `vibe-workspace::boot_artifacts` — locate / classify / splice; `write_boot_artifacts`'s redirect loop reworked; unit tests for every well-formedness class (absent, well-formed, each malformed shape, byte-identical no-op). @status:impl/done
2. @fact:PHASE-2-PLAN-TIME **Plan-time validation** wired into `apply_resolution` and the `vibe reinstall` path; the malformed-block abort surfaced by `vibe install` / `reinstall` / `uninstall` / `update`; `vibe init` scaffolds the block. @status:impl/done
3. @fact:PHASE-3-MIGRATION **Migration** of the old whole-file generated redirect (§4). @status:impl/done
4. @fact:PHASE-4-CHECK-DOCS **`vibe check` finding**, the `docs/` note, and the `VIBEVM-SPEC.md` §6.1 / §4.2 wording (inside the M1.18 Phase-7 spec-consistency pass). @status:impl/done

@fact:SEQUENCING **Sequencing.** PROP-012 corrects shipped Phase-1–6 behaviour and is a **prerequisite** for PROP-009 §4 / the M1.18 Phase-7 redirect rewrite — the vibevm self-migration must not be able to destroy `CLAUDE.md`. Recommended: implement PROP-012 **within M1.18 Phase 7**, ahead of the self-migration step, rather than as a later standalone milestone. Owner to confirm. @status:impl/done

---

## 8. Version history {#history}

- @fact:HISTORY-DRAFT-1 **2026-05-22 — draft 1.** Requirements captured in an owner discussion during M1.18 Phase-7 planning. The whole-file redirect overwrite shipped in PROP-009 Phase 4 is destructive to co-tenant content; vibevm must own only a `<vibevm>`-delimited block of each agent instruction file: exactly one block per file, a hard stop on a malformed file (the user repairs it by hand — vibevm never guesses), absent → create at end of file, present → splice, validated at plan time. The block's position is thereafter the user's — vibevm never relocates it, so the user can promote the block to a "First Prompt" or leave it a sidecar. Recorded as a separate PROP at the owner's request. @status:spec/done
- @fact:HISTORY-DRAFT-2 **2026-05-22 — draft 2.** Owner review settled the one substantive §5 question — the marker syntax: bare `<vibevm>` … `</vibevm>` tags (§2.2), chosen because they read unambiguously to an LLM, the file's primary consumer; the markdown-rendering cosmetics of a bare tag are an accepted, deferred cost. The two remaining §5 questions — the exit code and old-redirect detection — are implementation details with working answers. PROP-012 is ready for implementation within M1.18 Phase 7. @status:spec/done
- @fact:HISTORY-UNIT-TYPING **2026-06-12 — unit typing (the depth program).** §2.1–2.5 and §4 typed `req r1`, §2.6 and §3 typed `design r1`; the Status line updated to reflect the shipped M1.18 Phase-7 implementation (the audit's finding 2026-06-12-04 recorded PROP-012 as implemented-with-zero-edges; the affirmation sweep tags the implementation against these units). @status:spec/done
