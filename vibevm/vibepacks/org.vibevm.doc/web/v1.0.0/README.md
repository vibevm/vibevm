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

For local site testing, run `./vibevm-doc.sh` or
`./vibevm-doc.ps1`. Both install the pinned pnpm dependencies when absent,
rebuild the static adapter, and serve it on `http://127.0.0.1:4322/`.
`--no-build` reuses the last build; `--port <number>` selects another loopback
port. `vibe install -g org.vibevm.doc/web` installs the same entry point as
`vibevm-doc` (plus the native PowerShell and CMD shims on Windows); a local
registry can be selected with the ordinary `--registry <path>` flag.

The complete site, including every package and both editions of the VibeVM
manual, is a native lifecycle build of the host repository. From the repository
root run `vibe build`; its build contribution calls the documentation site
builder over the checkout, then hands every rendered tree to this package's
ordinary Qwik adapter. The finished domain is written to the host-owned
`.vibe/site-build/site` directory, outside the source-only package. From this
package directory the equivalent command is:

```text
vibe build --path ../../../../..
```

Serve that exact build without replacing it with the fixture catalogue by
running `./vibevm-doc.ps1 --no-build` or `./vibevm-doc.sh --no-build`. In a
source checkout the launcher finds the host-owned output automatically; an
installed package continues to use its ordinary `site/dist` output.

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
