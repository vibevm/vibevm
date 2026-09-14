/** @scope spec://org.vibevm.core/vibevm/common/PROP-057#SITE-TWO-CONTAINERS */

/**
 * The deployment, read off its own files.
 *
 * Two containers and a directory between them: one renders into it and
 * exits, the other serves it and knows nothing else. Everything that can
 * go wrong there goes wrong quietly — a version that drifted from the
 * one the package declares, a service that starts before the render it
 * depends on, a real host name where a placeholder belongs — and each of
 * those is visible in the text of a file long before it is visible in a
 * deploy.
 *
 * What this does NOT do is run Docker. The live probe belongs to the
 * deployment atom and needs a machine with a daemon on it; this runs in
 * the floor, in a clone with nothing installed, and fails on the edit
 * rather than on the next release.
 */

import assert from "node:assert/strict";
import { existsSync, readFileSync } from "node:fs";
import test from "node:test";

const read = (relative: string): string =>
  readFileSync(new URL(relative, import.meta.url), "utf8");

const COMPOSE = read("../../../docker/compose.yaml");
const DOCKERFILE = read("../../../docker/Dockerfile");
const IGNORE = read("../../../docker/Dockerfile.dockerignore");
const SITE_TOML = read("../../../docker/site.toml");
const PACKAGE = JSON.parse(read("../../../package.json")) as {
  engines?: Record<string, string>;
  packageManager?: string;
};

/**
 * One writes the directory, the other serves it, and the serving side
 * waits rather than races: a container that came up first would answer
 * 404 for the whole domain until somebody noticed.
 */
test("the compose file is the two containers and the volume between them", () => {
  assert.match(COMPOSE, /^\s{2}site:/m);
  assert.match(COMPOSE, /^\s{2}renderer:/m);
  assert.match(COMPOSE, /condition:\s*service_completed_successfully/);
  assert.match(COMPOSE, /rendered:\/usr\/share\/nginx\/html:ro/);
  assert.match(COMPOSE, /rendered:\/out\b/);
  assert.match(COMPOSE, /target:\s*serve/);
  assert.match(COMPOSE, /target:\s*render/);
});

/**
 * The service names and the published port are placeholders and say so
 * where they stand. The real ones belong to the server's arrangement,
 * which this repository carries no part of (R-25).
 */
test("the service names and the port are placeholders the owner replaces", () => {
  assert.match(COMPOSE, /"8080:80"/);
  assert.match(COMPOSE, /site:\s*#\s*placeholder/i);
  assert.match(COMPOSE, /renderer:\s*#\s*placeholder/i);
  assert.equal(/([0-9]{1,3}\.){3}[0-9]{1,3}/.test(COMPOSE), false);
  assert.equal(/ssh|certbot|letsencrypt|\.pem\b/i.test(COMPOSE), false);
});

/**
 * The version the image pins is the version the package declares. Two
 * places naming a toolchain is one place too many, and the way that
 * failure arrives is a build that works everywhere except in the
 * container.
 */
test("the image pins the Node the package's engines name", () => {
  const declared = PACKAGE.engines?.["node"];
  assert.equal(typeof declared, "string");
  assert.ok(
    DOCKERFILE.includes(`FROM node:${declared}-`),
    `the Dockerfile does not build on node:${declared}`,
  );
  /* pnpm is taken from `packageManager` through corepack rather than
     pinned a second time here, which is why no version of it appears in
     the file at all. */
  assert.match(DOCKERFILE, /corepack enable && pnpm install --frozen-lockfile/);
  assert.equal(DOCKERFILE.includes("pnpm@"), false);
});

/**
 * The compiler is pinned at the `rust-version` the workspace declares,
 * for the same reason and with the same failure. Skipped where the host
 * is not beside the package: this package is published as source and is
 * built outside a checkout too.
 */
test("the image pins the compiler the workspace manifest declares", () => {
  const manifest = new URL("../../../../../../../Cargo.toml", import.meta.url);
  if (!existsSync(manifest)) return;
  const declared = /^rust-version = "([^"]+)"$/m.exec(
    readFileSync(manifest, "utf8"),
  );
  assert.notEqual(declared, null);
  assert.ok(
    DOCKERFILE.includes(`FROM rust:${declared?.[1]}-`),
    `the Dockerfile does not build on rust:${declared?.[1]}`,
  );
});

/**
 * The renderer is one command over one configuration, and the serving
 * image carries no content at all — which is what makes an hourly render
 * a new directory rather than a new image.
 */
test("the renderer renders and the serving image holds nothing", () => {
  assert.match(
    DOCKERFILE,
    /CMD \["vibe", "doc", "build-site", "--config", "\/site\.toml", "--out", "\/out", "--web", "\/web"\]/,
  );
  assert.match(DOCKERFILE, /FROM nginx:alpine AS serve/);
  assert.match(DOCKERFILE, /docker\/nginx\.conf \/etc\/nginx\/conf\.d\//);
  assert.match(
    DOCKERFILE,
    /docker\/csp\.default\.conf \/etc\/nginx\/csp\.conf/,
  );
  assert.match(DOCKERFILE, /docker-entrypoint\.d\/20-csp-from-render\.sh/);
  /* Nothing of the rendered site is copied into the serving image: the
     only COPY into it is configuration. */
  const serve = DOCKERFILE.slice(DOCKERFILE.indexOf("FROM nginx:alpine"));
  assert.equal(/COPY --from=/.test(serve), false);
});

/**
 * The context is an allow-list because the alternative cannot be made
 * safe: a working tree's build directory is hundreds of gigabytes, and a
 * deny-list that forgot one entry would send all of it to the daemon
 * before the first instruction ran.
 */
test("the build context is named in, never filtered out", () => {
  assert.match(IGNORE, /^\*\*$/m);
  for (const needed of [
    "!Cargo.toml",
    "!crates/**",
    "!vibe.toml",
    "!vibevm/vibespecs/**",
  ]) {
    assert.ok(IGNORE.includes(needed), `the context drops ${needed}`);
  }
  assert.match(IGNORE, /^\*\*\/target$/m);
  assert.match(IGNORE, /^\*\*\/node_modules$/m);
});

/**
 * The configuration the container ships is public in every value, and
 * the one value that is not public is empty: an id that names a live
 * property belongs to the deployment, and a tag carrying an empty one
 * would load the script and report to nobody (`##SITE-ANALYTICS`).
 */
test("the shipped configuration carries the defaults and no live value", () => {
  assert.match(SITE_TOML, /^schema = 1$/m);
  assert.match(SITE_TOML, /url = "https:\/\/github\.com\/vibespecs"$/m);
  assert.match(SITE_TOML, /git = "https:\/\/github\.com\/vibevm\/vibevm"$/m);
  assert.match(SITE_TOML, /^ref = "main"$/m);
  assert.match(SITE_TOML, /^origin = "https:\/\/vibevm\.org"$/m);
  assert.match(SITE_TOML, /^base_path = "\/doc"$/m);
  /* The palette a first-time reader gets is a decision of the design
     review, not a build-time accident, so the file the container ships
     with says it out loud (F-48). */
  assert.match(SITE_TOML, /^default_theme = "dark"$/m);
  assert.match(SITE_TOML, /^website_id = ""$/m);
  /* Over the settings and not over the prose around them: the shape has
     no `auth`, no token and no environment name in it, and a comment
     saying so must not read as one. */
  const settings = SITE_TOML.split("\n").filter(
    (line) => !line.trimStart().startsWith("#"),
  );
  assert.equal(/token|secret|password|auth/i.test(settings.join("\n")), false);
});
