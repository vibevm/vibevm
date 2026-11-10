---
name: steward-tree
description: Render any schema-1 stewardship plan.toml as a static offline HTML tree with states, per-parent leaf counts, dependencies, acceptance and the validator's verdict. Use to inspect or review a plan's whole shape, or to keep a rendered view current while the plan changes; it reads the plan and writes only inside its output directory.
---

<status stage="impl" state="done"/>

# Steward tree {#root}

@fact:TREE-SKILL-SCOPE Render the plan a human can already read; change nothing.
The viewer is derived navigation under
[plan-hierarchy.xml#TREE-VIEWER-IS-DERIVED](../../flows/multi-user-planning/plan-hierarchy.xml),
never a second plan, a queue or an authority. It grants no execution or
acceptance authority and is not a status report: a central session still owes the
ordinary verified status, and a worker uses only its packet's named inputs.
@status:impl/done

@fact:TREE-SKILL-GENERIC The renderer is plan-shaped, not project-shaped: no
project ids, layers, tabs or inventories. Any conformant plan renders — the node
tree by `order`, the protocol's full state vocabulary, kinds, both dependency
kinds, mandates, acceptance and evidence. Unrecognised per-node or plan-level
fields are shown generically in the detail pane instead of being special-cased
or dropped. @status:impl/done

@fact:TREE-SKILL-EDGES Dependencies are typed edges, shown in both directions
and named by kind, never merged: `depends_on` reads as the inherited start gate
(«Предпосылка старта») and `completes_after` as this node's own completion gate
(«Завершается после»), per
[plan-hierarchy.xml#COMPLETION-GATES](../../flows/multi-user-planning/plan-hierarchy.xml).
A kind a plan does not use takes no space in the pane. @status:impl/done

@fact:TREE-SKILL-RUN Run the packaged Python 3.11+ renderer with the actual plan
and a directory it may own. Nothing outside `--out` is written and the plan is
opened read-only: @status:impl/done

```text
python -B scripts/render_tree.py --plan <plan.toml> --out <directory> [--lang ru|en] [--watch]
```

@fact:TREE-SKILL-OUTPUT `--out` receives `index.html`, a self-contained offline
page with no network, font or script dependency, and `plan-tree.json`, the same
model for other readers. Both are replaced atomically and only when their bytes
change, so a reload never sees a half-written page and an unchanged plan
re-renders byte-identically. `--watch` re-renders on changed plan bytes at
`--interval` seconds. Output carries no timestamp: the plan's own `updated_at`,
`revision` and sha256 identify what was rendered. @status:impl/done

@fact:TREE-SKILL-VOCABULARY States are displayed through the buckets fixed by
`plan-hierarchy.xml#TREE-DISPLAY-BUCKETS`, with `candidate` («Сделано · ждёт
приёмки») named distinctly from `accepted` («Готово · принято»), and `dropped`
/ `superseded` as «Закрыто без выполнения». Per-parent counts follow
`#TREE-LEAF-COUNTING` — leaf descendants only, each leaf once, so a parent's
buckets sum to its leaf total. Counts are not progress and not acceptance.
@status:impl/done

@fact:TREE-SKILL-VALIDATION The page always shows the verdict of the package's
own validator (`steward-parallel/scripts/parallel_frontier.py`, which runs the
goal renderer's structural checks first), so the picture cannot contradict the
tools. An invalid or unparsable plan still renders its nodes under a refusal
banner naming the code and message. Unknown states, orphans, parent cycles and
dangling dependency targets appear as per-node warnings; the viewer never
invents a readiness for them. @status:impl/done

@fact:TREE-SKILL-UI One page, no build step: search by id or title, expand and
collapse, a jump to `current_node`, light/dark/auto theme, a Russian/English
string table selectable in the page, keyboard navigation over an ARIA tree, and
a layout that works at phone width. Plan text is data and never markup: titles,
acceptance and evidence are injected as text, so a plan cannot script the page.
@status:impl/done

@fact:TREE-SKILL-REPORT Report the output path, plan id, revision, sha256, node
count, the validator verdict and the warning count; name the buckets a reviewer
should look at rather than restating the whole tree. A rendered view is evidence
about the plan's shape, never acceptance of its nodes. Select economical checks
under
[verification-selection.xml](../../flows/multi-user-planning/verification-selection.xml);
judge candidates with
[collaboration-and-acceptance.xml](../../flows/multi-user-planning/collaboration-and-acceptance.xml).
@status:impl/done
