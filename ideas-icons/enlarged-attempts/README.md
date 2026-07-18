# enlarged-attempts — icon scaling experiments (not adopted)

While making the vibeterm / vibetree taskbar icons fill more of their cell,
these variants scaled the **glyph** up (~1.5×), trimmed the tile margin, and
(in intermediate takes) nudged sky dots inward to avoid clipping — a real change
to the drawing's proportions.

The owner preferred the **original glyphs at their original proportions**. So the
shipped icons (`assets/icons/vibeterm.*`, `vibetree.*`) instead keep the original
drawing verbatim and apply a single **uniform** scale (×512/464 ≈ 1.1034) so the
rounded tile is **full-bleed** — its straight edges touch the canvas, no
transparent side margin (the rounded corners keep their small gaps). Proportions
are perfectly preserved; only the empty edge margin is removed.

These files are kept for reference only — not part of any build.

| File | What |
|------|------|
| `vibeterm-enlarged.*` | glyph scaled ~1.5×, full-bleed tile, clip-path rim |
| `vibetree-enlarged.*` | graph scaled ~1.55×, full-bleed tile |
