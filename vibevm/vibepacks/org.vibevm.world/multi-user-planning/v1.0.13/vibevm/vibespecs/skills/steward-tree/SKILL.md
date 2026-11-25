---
name: steward-tree
description: Render any schema-1 stewardship plan.toml as a static offline HTML tree with states, per-parent leaf counts, dependencies, acceptance and the validator's verdict. Use to inspect or review a plan's whole shape, or to keep a rendered view current while the plan changes; it reads the plan and writes only inside its output directory, by default the context's disposable view/ directory.
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

@fact:TREE-SKILL-RUN Run the packaged Python 3.11+ renderer on a steward
context. `--resolve`, run inside the checkout, finds the context through
`steward-common`; `--context` names it directly. Either reads
`<context>/plan.toml` and writes `<context>/view/` unless `--out` names another
directory. `--plan` with `--out` still renders any plan file. Nothing outside
the output directory is written and the plan is opened read-only:
@status:impl/done

```text
python -B scripts/render_tree.py --resolve [--home <steward home>] [--lang ru|en] [--watch]
python -B scripts/render_tree.py --context <steward home>/contexts/<context-id> [--out <directory>]
python -B scripts/render_tree.py --plan <plan.toml> --out <directory> [--lang ru|en] [--watch]
```

@fact:TREE-SKILL-VIEW-DISPOSABLE The context's `view/` is a disposable
projection: delete it at any time, regenerate it from the plan, never read it
back. It embeds no absolute path, so a copied steward home carries no stale
machine path. Nothing outside the steward home is a planning source of truth.
@status:impl/done

@fact:TREE-SKILL-LAUNCHER `bin/plan-tree` is a convenience command for people:
run in any checkout of a project that installs this package, it re-renders the
view through `--resolve` and prints the path of `index.html`; `--open` also
shows it in the default browser, `--repo` names another checkout and `--lang`
picks the language. It finds the newest installed copy of this package inside
the checkout, needs Git on PATH, runs itself on Python 3.8 or newer and starts
the renderer with Python 3.11 or newer, saying so when none is installed. To use
it, copy the files of `bin/` into a directory on PATH; on macOS and Linux make
`plan-tree` executable, on Windows `plan-tree.ps1` and `plan-tree.cmd` run the
same launcher from PowerShell and cmd. It writes nothing but the view.
@status:impl/done

@fact:TREE-SKILL-OUTPUT The output directory receives `index.html`, a self-contained offline
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
a layout that works at phone width. The tree uses the full document width with no
permanent reserved detail column. Details open in a separate closable fixed right
panel, never inserted into the tree; on desktop only while open, the tree uses all
remaining space to its left so badges are not obscured. Mobile uses an overlay.
Every task stays one nowrap row, titles ellipsize with full tooltip/details and
compact numeric status badges remain right-aligned. Escape closes the panel and
restores focus to its node.
Compact right-aligned colored count badges use the payload's leaf-descendant
buckets, with accessible status/count labels, not a recomputed history or progress.
Document scrolling, offline file access and mobile-width overlay remain supported.
The keyboard-accessible All / Unfinished / In progress control defaults to All.
Unfinished excludes only accepted/dropped/superseded (candidate, blocked and deferred
remain); In progress matches the node's own active state. Search intersects that
filter. Only matches and their ancestor context appear, with matching branches
initially expanded and unrelated siblings omitted. Filtered branches can then be
collapsed/reopened by twisty or ArrowLeft/ArrowRight; reopening reveals only
matching descendant/context rows. This filtered expansion is separate, so All
restores ordinary expansion. Search has the same collapsible behavior. Counts
always describe the full payload branch, not the filtered subset.
A node id in the URL fragment opens that
node, so a review can point at one node instead of describing where to click. Plan text is data and never markup: titles,
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
