# vibeterm icon explorations

Archive of the candidate icons for the **vibeterm** desktop terminal. Not part
of any build — this folder is a design record. The winner was promoted out of
here into the shipped tree:

> **Winner: `vibeterm-c2-coralstars-sparkle`** — large coral stars + a hero
> four-point sparkle over the `>_` prompt. Shipped as
> `assets/icons/vibeterm.{svg,ico,png,-512.png}` (canonical / `.exe` embed /
> Start-menu) and `apps/vibeterm/resources/icon.{svg,ico,png}` (the default
> window icon → the app's official identity).

Every candidate shares one base, so only the "sky" varies:

- **Tile** — rounded square `rect x24 y24 464×464 rx104`, vertical gradient
  `#26262E → #161619`.
- **Prompt `>_`** — chevron `path M160,190 L252,250 L160,310` (stroke `#D97757`,
  width 30, round caps) + dash `rect 250,300 96×26 rx8`. Drawn **last** so any
  sky sits behind it.

## Files

| Batch | How made | Members |
|-------|----------|---------|
| `vibeterm-1..6` | hand-written SVG | nodots · starfield · aurora · sparkle · lights · coral |
| `vibeterm-7..14` | `gen.mjs` | bigstars · coralstars · trails-sweep · trails-circle · trails-dense · coral-lights · starrain · starrain-fade |
| `vibeterm-c1..c4` | `gen-combo.mjs` | the four effects fused with the hero sparkle (`c3` split into `c3a` dim / `c3b` bright) |

`vibeterm-sheet*.png` are the contact sheets (`magick montage`). Each candidate
is **SVG + PNG**; the SVG is the master, the PNG a 256² render.

## Palette

| Token | Hex | Use |
|-------|-----|-----|
| coral | `#D97757` | prompt, primary stars, sparkle |
| white | `#EAE3D8` | warm-white stars, hot cores |
| lcoral | `#E8A085` | dim accent stars |
| gold | `#E7C98B` | multicolour trail hue |
| blue | `#9FB6D6` | multicolour trail hue |
| lights | `#F0645A` / `#F5BE4F` / `#54C15A` | macOS window dots (variant 5) |

## How the generators compute each sky

Both scripts are plain Node ESM (`node gen.mjs`, `node gen-combo.mjs`) and share
the same helpers:

- **PRNG** — `mulberry32`, **seeded by the variant number**, so re-runs are
  byte-stable (no `Math.random`).
- **`pt(P, R, θ)`** — a point at angle `θ` (degrees) and radius `R` around pole
  `P`, in SVG screen coords (**y grows downward**): `P + R·(cos θ, sin θ)`.
- **`inBand`** — the sky band `x∈[46,466], y∈[50,184]`; `y<190` keeps every sky
  element strictly above the chevron's top.
- **`bandRun(P,R)`** — samples a full circle and returns the longest angular run
  that stays in the band; that run is where a trail of radius `R` is visible.
- **`arc(P,R,θ1,θ2,…)`** — an SVG `A` (elliptical-arc) segment, `sweep=1`
  (increasing θ).

### Star fields (`7-bigstars`, `8-coralstars`)

Hand-placed circles in the band, larger/brighter than the `1..6` originals. `7`
mixes warm-white + coral; `8` is all coral at dot sizes echoing the original
term-10 dots (r 6–11).

### Star trails (`9`, `10`, `11`) — long-exposure "star circling"

Concentric arcs around a pole, each clipped to the band by `bandRun`:

- **`9-trails-sweep`** (ref: pole off-frame) — pole far to the lower-left
  `P=(-260,820)`, radii `690…1030`. Each star is a **short** sub-arc (6–22° of
  its band-run, placed randomly) → faint coral (opacity .22–.42) with a small
  `lcoral` end-dot. Plus **two bright hero streaks** (`R=905`, `770`): a white
  core (w4) over a coral glow (w8), tipped with a white point.
- **`10-trails-circle`** (ref: tight circumpolar) — in-frame pole `P=(312,98)`,
  radii `14/26/39/53/68/82`. Each ring is drawn as one long arc **minus an
  18–40° gap**, so it reads as a trail, not a solid circle; a bright white pole
  star sits at `P`.
- **`11-trails-dense`** (ref: dense multicolour) — pole `P=(338,84)`, radii
  `16…180` (step ≈8–12). Ends trimmed 4–16 % for an airy feel; hue cycles
  `coral → white → gold → blue …`; ~half get an end-dot; white pole star at `P`.

### Star rain (`13`, `14`) — falling meteors

A **drop** = a vertical rounded `rect` filled with the shared `drop` gradient
(coral, opacity `0 → .35 → .95` top→bottom) + a head circle at the bottom
(coral, or a white-hot core with a coral halo for "hot" drops), so the tail
fades **upward**.

- **`13-starrain`** — uniform: 15 drops, heads `y 66..174`, `x` spaced ≥20 apart,
  length 22–48, ~22 % hot; 22 faint background sparkles.
- **`14-starrain-fade`** — density graded by height. For a head at `y`,
  `p = 1 − (y−58)/118` (1 at the top → 0 near the centre); a drop is **kept with
  probability `0.15 + 0.85·p`** and its length scaled by `0.5 + 0.5·p`. Result:
  dense long drops at the top thinning to sparse short ones toward the centre.

### The hero sparkle ✦ (`vibeterm-4`, all `c*` combos)

A four-point star with **concave** edges, centred `(372,160)`, radius `R=46`,
pinch `q=8`. Four quadratic beziers between the tips `(cx,cy±R)`/`(cx±R,cy)` with
control points at `(cx±q, cy±q)` — small `q` pulls the edges inward:

```
M 372 114 Q 380 152 418 160 Q 380 168 372 206 Q 364 168 326 160 Q 364 152 372 114 Z
```

## Combos (`gen-combo.mjs`)

Each combo layers the sparkle onto one effect and **culls field stars within
~52–54 px of the sparkle** so ✦ stays the hero.

- **`c1`/`c2`** — bigstars / coralstars + ✦. `c2` (all coral) shipped.
- **`c3a-sweep-sparkle-dim`** — sweep **faint trails only** (no bright streaks) + ✦.
- **`c3b-sweep-sparkle-bright`** — sweep + ✦ + **two bright streaks placed only in
  the clear top-centre**. The original hero streaks were removed on purpose: the
  left one continued the `>` arm's slope (read as one line with the prompt) and
  the right one collided with ✦. The replacements (`R=890 → (210,64)→(288,119)`
  and `R=920 → (248,53)→(306,95)`) end ~92 px from ✦ and well above the chevron.
- **`c4-starrain-sparkle`** — rain-fade + ✦ with a **clean pocket**: a drop is
  skipped if its head or tail-top enters a 40–46 px radius of ✦.

## Regenerate

```sh
node gen.mjs          # writes vibeterm-7..14 .svg
node gen-combo.mjs    # writes vibeterm-c1..c4 .svg
# raster any master (256²); montage a sheet:
magick -background none -resize 256x256 vibeterm-c2-coralstars-sparkle.svg out.png
```

To rebuild the **shipped** icon from the winner master, see
`assets/icons/README.md` (1024 render → `.ico` downsample recipe).
