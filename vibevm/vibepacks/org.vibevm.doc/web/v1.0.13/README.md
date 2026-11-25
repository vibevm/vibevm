# org.vibevm.doc/web — the vibevm.org site

One Qwik application for the whole domain: the landing at `/` and `/ru/`, and
the documentation reader under `/doc/`. A pnpm workspace of two parts —
`design/` holds the palette, the semantic tokens, both themes, the self-hosted
fonts and the shell components; `site/` holds the routes and the island the
Rust documentation pipeline fills with HTML.

Build it with Node 24.18.0 and pnpm 10.33.2 (`corepack enable`):
`pnpm install --frozen-lockfile`, then `pnpm floor` for the discipline's seven
steps and `pnpm audit:contrast` for the APCA audit of both themes.

Two adapters, one code base. `pnpm build:static` prerenders every route at
`base: "/"` for the server. `pnpm build:embedded` builds only the
documentation routes at `base: "/doc/"` for the shell `vibe doc serve`
embeds. The package is published as source: no build output ever enters it.

The complete site, including every package and both editions of the VibeVM
manual, is a native lifecycle build of the main `vibevm` repository. Its normal
development entry point is one command from that repository root:

```text
vibe run vibevm-doc
```

The command invokes the native incremental `vibe doc build-site` pipeline and
the same Qwik adapter as the lifecycle contribution, without reinstalling the
repository's dependency closure on every preview. It writes the domain to the
host-owned `.vibe/site-build/site` directory outside this source-only package,
then serves it on `http://127.0.0.1:4322/`. `--port` and the explicit reuse
switch pass after `--`, for example
`vibe run vibevm-doc -- --no-build --port 4400`. From this package directory,
`vibe build --path ../../../../..` remains the complete lifecycle build.

The low-level `./vibevm-doc.sh` and `./vibevm-doc.ps1` launchers serve an
existing build of this checkout. They no longer silently replace the complete
catalogue with fixtures. The small package test catalogue is selected
explicitly with `--fixture`; it is not an end-user site.

The package is also a globally installable application with the id
`vibevm-doc`. Install it from the registry with `vibe install -g
org.vibevm.doc/web`, or from a checkout with `vibe install -g
org.vibevm.doc/web --registry <checkout>/vibevm/vibepacks`. Every platform row
is a `source_build` with no sources: the recipe (`tooling/local-site/install.sh`
on Linux and macOS, `install.ps1` on Windows) copies this package tree without
build output into the installation's `site/` and writes a relocatable
`bin/vibevm-doc` wrapper (`bin\vibevm-doc.cmd` on Windows) around
`tooling/local-site/run.mjs`. vibe then puts the `vibevm-doc` command on PATH;
it takes the same arguments as the launchers above (`vibevm-doc --help`) and
serves on `http://127.0.0.1:4322/`. The installed copy carries no build
output: run it from a vibevm checkout root to serve that checkout's
`.vibe/site-build/site`, or pass `--site DIR` to serve any built site.
`vibe uninstall -g org.vibevm.doc/web` removes it.

When the JavaScript closure is absent, the native build runs the pinned
`pnpm install`. It inherits the standard npm/pnpm configuration, including the
user `.npmrc`, so an existing command such as
`npm config set registry https://registry.npmmirror.com` applies without any
VibeVM-specific setup. A checkout can instead pin a build-only registry by
adding this field to the root `build-documentation-site` extension:

```toml
config = { npm_registry = "https://registry.npmmirror.com" }
```

The override is passed to pnpm as `NPM_CONFIG_REGISTRY`; omit it to keep using
the system configuration on Windows, macOS and Linux.
