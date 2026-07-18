# App-family icons

Shared icons for the vibevm application family (the launcher binaries and the
desktop apps). All sit on the same **rounded-square gradient tile** and use the
same coral/emerald ink, in **two motifs**:

- **node-graph** — a hub, four nodes, four spokes (`default`, `vibetree`).
- **terminal-prompt** — a `>_` prompt with a starry sky (`vibeterm`).

| Name | Use | Motif | Ink |
|------|-----|-------|-----|
| `default`  | the default / library app | node-graph | coral `#D97757` |
| `vibetree` | the VibeTree launcher (`vibe tree -t`) + tree window | node-graph | muted emerald `#5FB584` |
| `vibeterm` | the VibeTerm launcher (`vibe term`) + the vibeterm window default | terminal-prompt `>_` + coral stars + sparkle | coral `#D97757` |

Each `*.svg` here is a standalone master (edit it directly). `vibeterm.svg` is
the winner of the [`../../ideas-icons/vibeterm/`](../../ideas-icons/vibeterm/)
exploration; the graph masters descend from vibeterm's original node-graph
`icon.svg`, since replaced by the prompt design.

## Formats

- `*.svg` — the master (edit this); infinitely scalable.
- `*.ico` — multi-resolution (256/128/64/48/32/16), rebuilt by downsampling a 1024px
  render for clean anti-aliasing. For Windows `.exe` embedding and the Start-menu tile
  (256 is the `.ico` format ceiling).
- `*.png` — 256×256, general use.
- `*-512.png` — 512×512, the high-resolution large icon (high-DPI / installer tiles /
  anywhere above the `.ico` 256 ceiling).

## Regenerate the raster formats from a master

```sh
# high-quality .ico: render 1024px first, then downsample to every layer
magick -background none -density 768 vibetree.svg -resize 1024x1024 vibetree-1024.png
magick vibetree-1024.png -define icon:auto-resize=256,128,64,48,32,16 vibetree.ico
# large + standard PNG
magick -background none -density 768 vibetree.svg -resize 512x512 vibetree-512.png
magick -background none -density 384 vibetree.svg -resize 256x256 vibetree.png
```

## Adding another app's icon

Copy a master, keep the tile background, change only the graph colour, and add a
row to the table above. Muted tones read best on the dark tile; avoid loud
saturation (it fatigues at frequent, small-size use).
