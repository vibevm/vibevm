# ideas-icons — icon exploration archive

**Archive only — not part of any build.** A frozen snapshot of the icon
directions generated while choosing the vibevm app-family icons. The *shipping*
icons live in [`../assets/icons/`](../assets/icons/); this directory keeps the
alternatives so a future session can revisit a direction without re-deriving it.

Every entry is kept as **SVG (master) + PNG (256 px preview)**. All build on the
vibevm node-graph figure from `apps/vibeterm/resources/icon.svg`.

## The chosen pair (canonical copies in `assets/icons/`)

- `default` — coral graph on the rounded-square gradient tile.
- `vibetree` — muted emerald (`#5FB584`) graph, same tile.

## Green candidates for vibetree

`vibetree-neon` (too loud — rejected) · `vibetree-emerald` (**chosen**) ·
`-jade` · `-sage` · `-pine` · `-greysage` · `-forest` · `-moss`.

## Other graph hues (on a disc background)

`nofixcoral-1-mint` · `-2-amber` · `-3-greencrt` · `-4-cyan` · `-5-mono`.

## Creative / terminal-themed concepts

- `term-02-prompt-node` — one node replaced by a prompt chevron.
- `term-12-tree` — a branching tree + a prompt (plays on "VibeTree").
- `term-13-caret-hub` — the hub as a caret cursor.
- `term-20-ascii` — square, box-drawing nodes.

## Disc-background examples

`fixcoral-2-charcoal` · `fixcoral-4-plum` · `fixcoral-5-espresso`.

## Colour map

`palette.svg` / `palette.png` — every colour used across these icons, by name
and hex (graph hues, the shared tile gradient, disc backgrounds, ink).

## Regenerate a PNG from a master

```sh
magick -background none -density 384 <name>.svg -resize 256x256 <name>.png
```
