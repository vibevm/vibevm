# vibevm.org site package

This package is the Qwik application behind vibevm.org and the globally
installable `vibevm-doc` command. Its manifest, installer options and launch
points live in `vibe.toml`; `README.md` explains how the site is built, served
and installed. The local-site launcher and its install recipes live in
`tooling/local-site/`; `tools/floor.mjs` runs the package's discipline floor.

Before changing the package, read `vibe.toml` and `README.md`. Keep platform,
architecture, installer, launcher and public-command names package-authored.
Keep install recipes relocatable: they write into `$VIBE_OUT` and never bake an
absolute path in. Do not claim a Windows or macOS installation has passed
without running it on that host.
