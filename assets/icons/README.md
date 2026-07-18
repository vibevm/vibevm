# App-family icons

Shared icons for the vibevm application family (the launcher binaries and the
desktop apps). One visual system: the vibevm **node-graph** figure — a hub, four
nodes, four spokes — centered on a **rounded-square gradient tile** (charcoal
`#2A2A32` → `#161619`). Each app keeps the tile and varies the graph colour.

| Name | Use | Graph colour | Nodes |
|------|-----|--------------|-------|
| `default`  | the default / library app | coral `#D97757` | dark fill, coloured ring |
| `vibetree` | the VibeTree launcher (`vibe tree -t`) | muted emerald `#5FB584` | dark fill, coloured ring |

The figure derives from `apps/vibeterm/resources/icon.svg` (unchanged geometry).

## Formats

- `*.svg` — the master (edit this).
- `*.ico` — multi-resolution (256/64/48/32/16), for Windows `.exe` embedding.
- `*.png` — 256×256, for general use / other platforms.

## Regenerate the raster formats from a master

```sh
# .ico (multi-resolution)
magick -background none -density 384 vibetree.svg \
  -define icon:auto-resize=256,64,48,32,16 vibetree.ico
# .png (256)
magick -background none -density 384 vibetree.svg -resize 256x256 vibetree.png
```

## Adding another app's icon

Copy a master, keep the tile background, change only the graph colour, and add a
row to the table above. Muted tones read best on the dark tile; avoid loud
saturation (it fatigues at frequent, small-size use).
