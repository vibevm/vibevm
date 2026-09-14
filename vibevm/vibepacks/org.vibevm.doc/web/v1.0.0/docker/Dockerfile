# syntax=docker/dockerfile:1

# The two containers of the site, in one file
# (PROP-057 `##SITE-TWO-CONTAINERS`, `##SITE-DEPLOY-MODEL`).
#
# One of them renders and the other serves, and they meet at a directory:
# the renderer runs `vibe doc build-site` into it and exits, nginx serves
# it and knows nothing else. That is the whole deployment. A deploy is
# `git pull` in the checkout, `docker compose run --rm <renderer>` to
# refill the directory, and `docker compose up -d <serving service>` —
# the same three steps the landing has today, with a render in the middle
# instead of a build.
#
# The build context is the REPOSITORY ROOT, not this directory. The
# renderer needs three things that live there: the `vibe` binary built
# from this checkout's own source, the site package with its
# dependencies, and the checkout itself — which is a source the render
# reads, exactly as the registry is (`##SITE-HOST-CHECKOUT`). What of the
# root actually enters the image is decided by `Dockerfile.dockerignore`
# beside this file.
#
# Nothing is pushed anywhere. The image is built where it runs, and this
# file names no registry, no host and no port (R-24, R-25).

# ─────────────────────────────────────────────────────────────────────
# 1. The site package's dependencies.
#
# Its own layer, and the manifests are copied before the sources so that
# editing a page does not reinstall the toolchain. The version is pinned
# twice over: the image names the Node the package's `engines` names, and
# `corepack` takes pnpm from `packageManager` rather than from whatever
# is newest (`##STACK-BUILD-HYGIENE`).
# ─────────────────────────────────────────────────────────────────────
FROM node:24.18.0-bookworm-slim AS web-deps

ENV PNPM_HOME=/pnpm
ENV PATH=$PNPM_HOME:$PATH
WORKDIR /web

COPY vibevm/vibepacks/org.vibevm.doc/web/v1.0.0/package.json \
     vibevm/vibepacks/org.vibevm.doc/web/v1.0.0/pnpm-lock.yaml \
     vibevm/vibepacks/org.vibevm.doc/web/v1.0.0/pnpm-workspace.yaml ./
COPY vibevm/vibepacks/org.vibevm.doc/web/v1.0.0/design/package.json design/
COPY vibevm/vibepacks/org.vibevm.doc/web/v1.0.0/site/package.json site/

RUN corepack enable && pnpm install --frozen-lockfile

# ─────────────────────────────────────────────────────────────────────
# 2. The site package, and a build of it with nothing installed.
#
# The build over the package's own fixture library, run here because it
# is the cheapest possible answer to «does this image's Node, pnpm and
# optimizer produce a site at all» — and because the answer arrives
# while the image is being built rather than on the deploy host, half a
# minute before the domain was to be switched. Its output is removed:
# the fixture site is a smoke test, never something to serve.
# ─────────────────────────────────────────────────────────────────────
FROM web-deps AS web-build

COPY vibevm/vibepacks/org.vibevm.doc/web/v1.0.0/ ./
RUN pnpm build:static && rm -rf site/dist site/server

# ─────────────────────────────────────────────────────────────────────
# 3. `vibe`, built from this checkout.
#
# No Node in this stage and none in the image it produces: the pipeline
# is Rust and the shell is the other half (`##STACK-NODE-SERVER-ONLY`).
# The image is pinned at the `rust-version` the workspace manifest
# declares, so the compiler is the one the code says it needs and not
# whatever `stable` resolved to this week.
#
# What is copied is what the workspace resolves: every member, the
# templates and schemas its production code embeds, and the discipline
# packages the manifest path-depends on. The site package is NOT among
# them — editing a page must not recompile a compiler.
# ─────────────────────────────────────────────────────────────────────
FROM rust:1.93-bookworm AS vibe-build

WORKDIR /src
COPY Cargo.toml Cargo.lock ./
COPY crates crates
COPY xtask xtask
COPY formats formats
COPY schemas schemas
COPY vibevm/vibepacks/org.vibevm.ai-native vibevm/vibepacks/org.vibevm.ai-native

# The binary is copied out of the cached target directory rather than
# left in it: a cache mount is not part of the image, and a stage that
# ended with the binary only in the mount would hand the next stage
# nothing.
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/src/target \
    cargo build -p vibe-cli --release \
    && cp target/release/vibe /usr/local/bin/vibe

# ─────────────────────────────────────────────────────────────────────
# 4. The renderer.
#
# `vibe doc build-site` reads the two sources — the package registry and
# this checkout — renders every version that moved, and hands the trees
# to the site's static build, which turns them into the domain. One
# command, because the join between the two halves belongs in the
# product and not in a shell script on a server
# (`crates/vibe-cli/src/commands/doc/site/web.rs`).
#
# `git` is installed because warming a package from a registry is a
# clone. `--web /web` names the package built in stage 2 rather than the
# copy inside the checkout, which carries no dependencies.
# ─────────────────────────────────────────────────────────────────────
FROM web-build AS render

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates git \
    && rm -rf /var/lib/apt/lists/*

COPY --from=vibe-build /usr/local/bin/vibe /usr/local/bin/vibe
COPY . /checkout
COPY vibevm/vibepacks/org.vibevm.doc/web/v1.0.0/docker/site.toml /site.toml

# `/out` is the directory the serving container reads. It is a mount in
# every real run; the declaration is here so that a renderer started with
# no mount writes into a volume of its own rather than into the image's
# writable layer.
VOLUME ["/out"]

CMD ["vibe", "doc", "build-site", "--config", "/site.toml", "--out", "/out", "--web", "/web"]

# ─────────────────────────────────────────────────────────────────────
# 5. The serving container: stock nginx, one configuration file, and a
#    directory somebody else filled.
#
# It carries no content at all, and that is the point of the two-container
# model: a render is a new directory, never a new image, so the site is
# rebuilt hourly without rebuilding anything that serves it.
#
# The policy is the one thing the configuration cannot state by itself —
# it names a hash per inline script, computed from the bytes of the
# pages (X-035). So the image ships an empty policy, and the entrypoint
# takes the render's own from the directory it was handed; nginx's stock
# entrypoint runs everything in `/docker-entrypoint.d` before starting,
# which is what makes that possible without replacing it.
# ─────────────────────────────────────────────────────────────────────
FROM nginx:alpine AS serve

RUN rm /etc/nginx/conf.d/default.conf
COPY vibevm/vibepacks/org.vibevm.doc/web/v1.0.0/docker/nginx.conf /etc/nginx/conf.d/vibevm-site.conf
COPY vibevm/vibepacks/org.vibevm.doc/web/v1.0.0/docker/csp.default.conf /etc/nginx/csp.conf
COPY vibevm/vibepacks/org.vibevm.doc/web/v1.0.0/docker/csp-from-render.sh /docker-entrypoint.d/20-csp-from-render.sh
RUN chmod +x /docker-entrypoint.d/20-csp-from-render.sh \
    && nginx -t -c /etc/nginx/nginx.conf

EXPOSE 80
