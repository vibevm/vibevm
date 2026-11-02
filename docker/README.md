# Official container images

The release publishes three Linux/amd64 images under the `vibevm` Docker Hub
organization. Every image has the mutable product tags of the release (for
example `1.0.9`) and `latest`.

- `vibevm/vibevm`: Alpine, Git, SSH tools, and the verified musl Vibe binary.
- `vibevm/vibevm-zap`: the base image plus the verified binary Zap application;
  its headless web UI is exposed on port 8080. It is not built by the release
  workflow, because the Zap source is a private repository; build it by hand
  from `docker/vibevm-zap/Dockerfile` with access to that repository.
- `vibevm/vibevm-dev`: the base image plus Rust 1.93, Node 24, pnpm, TypeScript,
  and native build tools.
- `vibevm/vibevm-doc`: the base image plus Node and nginx, carrying a complete
  pre-rendered vibevm.org documentation catalogue on port 80.

`NPM_CONFIG_REGISTRY` can be supplied as a Docker build argument to the dev and
documentation images. The GitHub workflow also accepts `npm_registry`, and
falls back to the repository variable `NPM_CONFIG_REGISTRY` before using the
public npm registry.
