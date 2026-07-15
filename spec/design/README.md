# Design-rationale specs

This directory holds vibevm's **design-rationale** documents: the *why* and the *lore* behind vibevm's own architectural decisions — the path of a design discussion, the forks weighed and rejected, the precedents studied, the owner's mental model, and the ideas parked for later. It is the **design-doc genre** of the `spec-genres` flow this project follows: `spec://org.vibevm.world/spec-genres/flows/spec-genres/SPEC-GENRES-PROTOCOL#root`.

These documents are **non-normative**. The contract — *what* the system does — lives in the PROP / FEAT documents under [`spec/modules/`](../modules/) and [`spec/common/`](../common/); a `spec/design/` document explains *why a PROP is shaped the way it is*. When a design document and its PROP disagree, **the PROP wins** and the design document is corrected — the flow's precedence law (`spec://org.vibevm.world/spec-genres/flows/spec-genres/SPEC-GENRES-PROTOCOL#precedence`): load-bearing rationale — the decision itself and the alternatives weighed, in each PROP's `Decision` / `Rejected alternatives` sections (the **decision-records** genre: `spec://org.vibevm.world/decision-records/flows/decision-records/DECISION-RECORDS-PROTOCOL#root`) — stays inside the PROP; the narrative lore moves out to here (`spec://org.vibevm.world/spec-genres/flows/spec-genres/SPEC-GENRES-PROTOCOL#contract-vs-lore`).

## vibevm's spec/ genres

vibevm's instance of the genre table — the general taxonomy (each genre's charter, mutability, reader, and authority-on-conflict) is the flow's `spec://org.vibevm.world/spec-genres/flows/spec-genres/SPEC-GENRES-PROTOCOL#genres`:

| Directory | Holds | Normative? |
|---|---|---|
| [`boot/`](../boot/) | Session-boot instructions read at the start of every session | yes |
| [`common/`](../common/) | Foundation decisions crossing every crate (PROP-000, PROP-006) | yes |
| [`modules/`](../modules/) | Per-crate PROP / FEAT — the implementation contract | yes |
| [`research/`](../research/) | Backgrounders on **external** systems (Tessl, threat models, prior-art surveys) | no |
| `design/` (this directory) | Rationale for vibevm's **own** decisions — the why and the lore behind our PROPs | no |
| [`WAL.md`](../WAL.md) | Volatile current-state checkpoint, rewritten each session | n/a |

`research/` and `design/` are both non-normative, but they look in opposite directions: `research/` studies what *other* projects did; `design/` records why *we* chose what we chose.

## Linking rule

Every `spec/design/` document names the PROP(s) it explains; every PROP it explains links back to it from its `Related` header — the flow's two-way linking law (`spec://org.vibevm.world/spec-genres/flows/spec-genres/SPEC-GENRES-PROTOCOL#linking`), so a session that reads a PROP during the boot sequence finds the rationale without being told it exists. A one-directional link is a latent break.

## When to write a document here

When a design discussion produces more reasoning than a PROP can absorb without losing its contract readability — a multi-fork design session, a large refactor weighed against several alternatives, a decision whose context would otherwise live only in one conversation and be lost at the next session boundary. (The general decision table is the flow's `when-to-write-what` document.)

## Index

- [Workspace & qualified naming](workspace-and-qualified-naming.md) — rationale for [PROP-007](../modules/vibe-workspace/PROP-007-workspace.md) (workspace) and [PROP-008](../modules/vibe-registry/PROP-008-qualified-naming.md) (qualified naming): the owner's Maven-submodules + cargo mental model, the four-axis decomposition, the fork-by-fork decision record, the Cargo-vs-Maven precedent lore, the physical-publication model, and ideas parked for later. Captured 2026-05-20.
- [Loading & boot composition model](loading-and-boot-model.md) — rationale for PROP-009 (loading model): why the flat boot model fails under a workspace, the static/dynamic linking spine, the two-trees + computed-index design, the three inclusion types (`inline` / `static` / `dynamic`) and the `STATIC.md` priority lane, and the fork-by-fork record. Captured 2026-05-21.
- [The action system](action-system.md) — rationale + architecture for [PROP-039](../modules/vibe-actions/PROP-039-action-system.md) (the `vibe-actions` contract): the addressable, frontend-agnostic, programmatically-drivable behaviour layer (`action://`) — the behaviour-layer twin of `spec://`. The crate/module architecture, the core types, the MVC-plus data flow (the model is the real interface), the ten design decisions (URI address grammar, collision-erroring registry, typed pure enablement, primary programmatic invocation + the **headless AIUI reference surface**, the two-phase Search Everywhere provider seam, address-keyed i18n, …), the Search Everywhere architecture (packages + every card-field + actions now, structural/AI-Native later through one seam), and the AIUI surface. Derived clean-room from the [VSCode/IntelliJ study](../research/action-systems-vscode-idea.md). Captured 2026-07-15.
