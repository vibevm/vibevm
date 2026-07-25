# Design-rationale specs

<status stage="doc" state="done" comment="pilot markup 2026-07-24: living genre guide + index; grows with each captured design session"/>

##genre-definition This directory holds vibevm's **design-rationale** documents: the *why* and the *lore* behind vibevm's own architectural decisions — the path of a design discussion, the forks weighed and rejected, the precedents studied, the owner's mental model, and the ideas parked for later. It is the **design-doc genre** of the `spec-genres` flow this project follows: `spec://org.vibevm.world/spec-genres/flows/spec-genres/SPEC-GENRES-PROTOCOL#root`. @doc/done

##PROP-WINS-PRECEDENCE These documents are **non-normative**. The contract — *what* the system does — lives in the PROP / FEAT documents under [`spec/modules/`](../modules/) and [`spec/common/`](../common/); a `spec/design/` document explains *why a PROP is shaped the way it is*. When a design document and its PROP disagree, **the PROP wins** and the design document is corrected — the flow's precedence law (`spec://org.vibevm.world/spec-genres/flows/spec-genres/SPEC-GENRES-PROTOCOL#precedence`): load-bearing rationale — the decision itself and the alternatives weighed, in each PROP's `Decision` / `Rejected alternatives` sections (the **decision-records** genre: `spec://org.vibevm.world/decision-records/flows/decision-records/DECISION-RECORDS-PROTOCOL#root`) — stays inside the PROP; the narrative lore moves out to here (`spec://org.vibevm.world/spec-genres/flows/spec-genres/SPEC-GENRES-PROTOCOL#contract-vs-lore`). @doc/done

## vibevm's spec/ genres

<status stage="doc" state="done"/>

##genre-table-lead vibevm's instance of the genre table — the general taxonomy (each genre's charter, mutability, reader, and authority-on-conflict) is the flow's `spec://org.vibevm.world/spec-genres/flows/spec-genres/SPEC-GENRES-PROTOCOL#genres`: @doc/done

| Directory | Holds | Normative? |
|---|---|---|
| ##ROW-BOOT [`boot/`](../boot/) @doc/done | Session-boot instructions read at the start of every session @doc/done | yes @doc/done |
| ##ROW-COMMON [`common/`](../common/) @doc/done | Foundation decisions crossing every crate (PROP-000, PROP-006) @doc/done | yes @doc/done |
| ##ROW-MODULES [`modules/`](../modules/) @doc/done | Per-crate PROP / FEAT — the implementation contract @doc/done | yes @doc/done |
| ##ROW-RESEARCH [`legacy-spec/research/`](../../legacy-spec/research/) — archived 2026-07-25 @doc/done | Backgrounders on **external** systems (Tessl, threat models, prior-art surveys) @doc/done | no @doc/done |
| ##ROW-DESIGN `design/` (this directory) @doc/done | Rationale for vibevm's **own** decisions — the why and the lore behind our PROPs @doc/done | no @doc/done |
| ##ROW-WAL [`WAL.md`](../WAL.md) @doc/done | Volatile current-state checkpoint, rewritten each session @doc/done | n/a @doc/done |

##research-vs-design `legacy-spec/research/` (archived) and `design/` are both non-normative, but they look in opposite directions: the archived research studies what *other* projects did; `design/` records why *we* chose what we chose. @doc/done

## Linking rule

<status stage="doc" state="done"/>

##TWO-WAY-LINKING Every `spec/design/` document names the PROP(s) it explains; every PROP it explains links back to it from its `Related` header — the flow's two-way linking law (`spec://org.vibevm.world/spec-genres/flows/spec-genres/SPEC-GENRES-PROTOCOL#linking`), so a session that reads a PROP during the boot sequence finds the rationale without being told it exists. A one-directional link is a latent break. @doc/done

## When to write a document here

<status stage="doc" state="done"/>

##when-to-write When a design discussion produces more reasoning than a PROP can absorb without losing its contract readability — a multi-fork design session, a large refactor weighed against several alternatives, a decision whose context would otherwise live only in one conversation and be lost at the next session boundary. (The general decision table is the flow's `when-to-write-what` document.) @doc/done

## Index

<status stage="doc" state="work" comment="living index — every new design doc adds a row; checked complete against the directory 2026-07-24"/>

- ##idx-workspace-naming [Workspace & qualified naming](workspace-and-qualified-naming.md) — rationale for [PROP-007](../modules/vibe-workspace/PROP-007-workspace.md) (workspace) and [PROP-008](../modules/vibe-registry/PROP-008-qualified-naming.md) (qualified naming): the owner's Maven-submodules + cargo mental model, the four-axis decomposition, the fork-by-fork decision record, the Cargo-vs-Maven precedent lore, the physical-publication model, and ideas parked for later. Captured 2026-05-20. @doc/done

- ##idx-loading-boot [Loading & boot composition model](loading-and-boot-model.md) — rationale for PROP-009 (loading model): why the flat boot model fails under a workspace, the static/dynamic linking spine, the two-trees + computed-index design, the three inclusion types (`inline` / `static` / `dynamic`) and the `STATIC.md` priority lane, and the fork-by-fork record. Captured 2026-05-21. @doc/done

- ##idx-action-system [The action system](action-system.md) — rationale + architecture for [PROP-039](../modules/vibe-actions/PROP-039-action-system.md) (the `vibe-actions` contract): the addressable, frontend-agnostic, programmatically-drivable behaviour layer (`action://`) — the behaviour-layer twin of `spec://`. The crate/module architecture, the core types, the MVC-plus data flow (the model is the real interface), the ten design decisions (URI address grammar, collision-erroring registry, typed pure enablement, primary programmatic invocation + the **headless AIUI reference surface**, the two-phase Search Everywhere provider seam, address-keyed i18n, …), the Search Everywhere architecture (packages + every card-field + actions now, structural/AI-Native later through one seam), and the AIUI surface. Derived clean-room from the [VSCode/IntelliJ study](../../legacy-spec/research/action-systems-vscode-idea.md). Captured 2026-07-15. @doc/done

- ##idx-tui-visual [TUI visual language](tui-visual-language.md) — the shared visual conventions of the `vibe` TUIs. @doc/done

- ##idx-structural-loader [Structural loader](structural-loader.md) — provisional loader instructions held for PROP-035; not yet wired into any live boot. @spec/hold
