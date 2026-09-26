# Chat icons

Avatars for the project's community chats. Both are drawn from the site's
mark — a hub, four nodes, four spokes, the same node-graph motif as the
[app-family icons](../icons/README.md) — and differ by ground, so that the
two chats are told apart at a glance in a chat list.

| Name | Use | Ground | Ink |
|------|-----|--------|-----|
| `telegram-news` | the Telegram news channel | the site's dark map: warm ink `#14120E` with a glow behind the hub and two dotted signal rings | terracotta `#D97757`, nodes filled with ink |
| `telegram-chat` | the Telegram discussion chat | a glowing terracotta: a light peach core that deepens to `#B24F30` at the rim | ink `#14120E`, nodes filled with ivory `#FCFBF8` |

`preview.png` shows both as Telegram shows them: cropped to a circle, large,
in a chat list and at the smallest size, on the light and the dark theme.

## Made for a circle

Telegram crops an avatar to the circle inscribed in the square. The ground
fills the whole square, so nothing is left bare at the rim, and the mark ends
at about 69 % of the radius, so no node touches the crop.

## Formats

- `*.svg` — the master (edit this); a 1024-unit square, infinitely scalable.
- `*.png` — 1024×1024, ready to upload (Telegram asks for at least 512×512).

## Regenerate a PNG from its master

Render through a browser engine: the PNGs here are a headless Chromium
screenshot of the SVG at 1024×1024, then recompressed losslessly.

```sh
magick telegram-news.png -strip -define png:compression-level=9 telegram-news.png
```

ImageMagick's own SVG renderer is close but not exact: it draws the dotted
signal rings of `telegram-news` as solid lines.
