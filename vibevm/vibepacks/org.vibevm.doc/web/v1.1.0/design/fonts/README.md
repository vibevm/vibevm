# Fonts

Three families, eight files, Latin and Cyrillic kept apart so a reader of
one script never downloads the other:

| Family | Files | Weights |
| --- | --- | --- |
| Spectral | `Spectral-latin-400`, `Spectral-cyrillic-400`, `Spectral-latin-600`, `Spectral-cyrillic-600` | 400, 600 |
| Inter | `Inter-latin`, `Inter-cyrillic` | variable, 100–900 |
| JetBrains Mono | `JetBrainsMono-latin`, `JetBrainsMono-cyrillic` | variable, 100–800 |

All three are licensed under the **SIL Open Font License, Version 1.1**,
not under this package's UPL-1.0: the `LICENSE.md` beside `vibe.toml`
covers the package's own source, and these bytes travel under the OFL
their authors chose. The OFL permits bundling and redistribution with
the reserved-name and same-licence conditions intact; the files are
unmodified subsets of the upstream releases.

They are an asset **source**, not build output: the site self-hosts them
and reaches no font service (PROP-057 `##STACK-DESIGN-FLOOR`). The
`@font-face` rules, the `unicode-range` split and `font-display: swap`
are in `design/fonts.css`.
