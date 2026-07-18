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
