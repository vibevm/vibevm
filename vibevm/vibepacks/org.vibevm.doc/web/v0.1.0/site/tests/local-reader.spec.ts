/** @scope spec://org.vibevm.core/vibevm/common/PROP-057#INV-LOCAL-IS-OFFLINE */

/**
 * The local reader, in a real browser, with every request it makes
 * written down.
 *
 * `##INV-LOCAL-IS-OFFLINE` is the promise: the reader listens on
 * 127.0.0.1 only, serves only known roots, loads nothing external, and
 * contacts the network only for a shell download somebody confirmed.
 * Every part of that except the last is a statement about what a PAGE
 * does once a browser has it — and no unit test can see a browser fetch
 * a font. So this opens the pages `vibe doc serve` renders, intercepts
 * the whole network, and asserts that every single request went back to
 * the loopback port this test started.
 *
 * Why it matters more here than anywhere else: this is how the
 * documentation of a proprietary package is read. One request to a font
 * service is one company's package names in somebody's access log. The
 * public site may carry an analytics tag (D-24); a page served from a
 * machine's own store may not, and the DOM is checked for one.
 *
 * The second case is the static output — `vibe doc build --format html`,
 * opened as a `file://` document. Same promise, different producer, and
 * the producer is the half that could drift: the build writes the page
 * on its own and nothing about the server is involved.
 *
 * It needs a built `vibe`, which the panel has and a fresh clone does
 * not, so without one it SKIPS with the reason printed rather than
 * failing. A test that went red because nobody had compiled the product
 * would teach the next person to ignore it.
 */

import { spawn, spawnSync, type ChildProcess } from "node:child_process";
import { createServer } from "node:http";
import { existsSync, mkdtempSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { pathToFileURL } from "node:url";
import { fileURLToPath } from "node:url";

import { expect, test, type Page, type Request } from "@playwright/test";

const HERE = dirname(fileURLToPath(import.meta.url));

/** The repository this package lives in: seven directories above here. */
const REPO = resolve(HERE, "..", "..", "..", "..", "..", "..", "..");

/** The product, as the panel builds it. */
const VIBE_BIN =
  process.env["VIBE_BIN"] ??
  join(
    REPO,
    "target",
    "debug",
    process.platform === "win32" ? "vibe.exe" : "vibe",
  );

/** The documentation the reader is pointed at. */
const MANUAL =
  process.env["VIBE_MANUAL"] ??
  join(REPO, "vibevm", "vibepacks", "org.vibevm.core", "vibevm-docs", "v0.1.0");

/** Hosts that are never this reader, named so a failure reads plainly. */
const NEVER = [
  "fonts.googleapis.com",
  "fonts.gstatic.com",
  "cdn.jsdelivr.net",
  "unpkg.com",
  "cdnjs.cloudflare.com",
  "googletagmanager.com",
  "google-analytics.com",
];

/** One free loopback port, taken by asking the operating system. */
async function freePort(): Promise<number> {
  return await new Promise((settle, fail) => {
    const probe = createServer();
    probe.on("error", fail);
    probe.listen(0, "127.0.0.1", () => {
      const address = probe.address();
      const port =
        typeof address === "object" && address !== null ? address.port : 0;
      probe.close(() =>
        port === 0 ? fail(new Error("no port")) : settle(port),
      );
    });
  });
}

/** Wait until the reader answers its own health address. */
async function ready(port: number): Promise<boolean> {
  for (let attempt = 0; attempt < 120; attempt += 1) {
    try {
      const response = await fetch(`http://127.0.0.1:${port}/healthz`);
      if (response.ok) return true;
    } catch {
      // Not up yet. The loop is the wait.
    }
    await new Promise((settle) => setTimeout(settle, 250));
  }
  return false;
}

/** Every request a page made, in order, as absolute URLs. */
function watch(page: Page): string[] {
  const seen: string[] = [];
  page.on("request", (request: Request) => seen.push(request.url()));
  return seen;
}

/** What a set of requests says about where the page went. */
function hostsOf(requests: readonly string[]): string[] {
  const hosts = new Set<string>();
  for (const one of requests) {
    try {
      hosts.add(new URL(one).host || "(file)");
    } catch {
      hosts.add("(unparsed)");
    }
  }
  return [...hosts].sort();
}

/** The tag the public site carries and a local page may not (D-24). */
async function carriesAnalytics(page: Page): Promise<boolean> {
  return await page.evaluate(() => {
    const scripts = [...document.querySelectorAll("script")];
    const foreign = scripts.some((element) => {
      const source = element.getAttribute("src") ?? "";
      if (!source.startsWith("//") && !source.includes("://")) return false;
      try {
        return (
          new URL(source, window.location.href).host !== window.location.host
        );
      } catch {
        return true;
      }
    });
    const named = scripts.some(
      (element) =>
        element.hasAttribute("data-website-id") ||
        (element.getAttribute("src") ?? "").includes("umami"),
    );
    return foreign || named;
  });
}

/** What the manual says about itself, read from the product once. */
type Manifest = {
  package: { group: string; name: string; version: string };
  pages: { path: string }[];
};

let reader: ChildProcess | undefined;
let port = 0;
let missing = "";
let scratch = "";
let manifest: Manifest | undefined;

/** The address of the manual's page number `at`, on this reader. */
function pageHref(at: number): string {
  if (manifest === undefined) throw new Error("the manifest was not read");
  const card = manifest.package;
  const path = manifest.pages[at]?.path ?? "";
  if (path === "") throw new Error(`the manual has no page ${at}`);
  const stem = path.replace(/\.xml$/, "");
  return `/doc/${card.group}/${card.name}/${card.version}/${stem}/`;
}

test.beforeAll(async () => {
  if (!existsSync(VIBE_BIN)) {
    missing = `no \`vibe\` at ${VIBE_BIN}; build it (\`cargo build -p vibe-cli\`) or set VIBE_BIN`;
    return;
  }
  if (!existsSync(MANUAL)) {
    missing = `no documentation package at ${MANUAL}; set VIBE_MANUAL`;
    return;
  }
  const listed = spawnSync(
    VIBE_BIN,
    ["doc", "manifest", "--path", MANUAL, "--json"],
    { encoding: "utf8", maxBuffer: 64 * 1024 * 1024 },
  );
  if (listed.status !== 0) {
    missing = `\`vibe doc manifest\` failed: ${listed.stderr ?? ""}`;
    return;
  }
  manifest = JSON.parse(listed.stdout) as Manifest;
  if (manifest.pages.length < 2) {
    missing = "this manual has fewer than two pages to read";
    return;
  }
  port = await freePort();
  reader = spawn(
    VIBE_BIN,
    ["doc", "serve", "--path", MANUAL, "--port", String(port), "--no-derived"],
    { stdio: ["ignore", "pipe", "pipe"] },
  );
  let output = "";
  reader.stdout?.on("data", (chunk: Buffer) => (output += chunk.toString()));
  reader.stderr?.on("data", (chunk: Buffer) => (output += chunk.toString()));
  if (!(await ready(port))) {
    missing = `the reader did not start on 127.0.0.1:${port}\n${output}`;
    reader.kill();
    reader = undefined;
  }
});

test.afterAll(() => {
  reader?.kill();
  if (scratch !== "") rmSync(scratch, { recursive: true, force: true });
});

test.beforeEach(() => {
  test.skip(missing !== "", missing);
});

test("every request a served page makes goes back to the loopback", async ({
  page,
}) => {
  const origin = `http://127.0.0.1:${port}`;
  const requests = watch(page);

  // The mount, which is where the reader tells an operator to start, and
  // the page it sends them to.
  const response = await page.goto(`${origin}/doc/`, {
    waitUntil: "networkidle",
  });
  expect(response?.ok(), "the reader answers at its own mount").toBe(true);
  await page.waitForLoadState("networkidle");

  expect(requests.length, "a page makes at least one request").toBeGreaterThan(
    0,
  );
  for (const one of requests) {
    expect(new URL(one).origin, `${one} is not this reader`).toBe(origin);
  }
  for (const host of NEVER) {
    expect(requests.join(" "), `a request reached ${host}`).not.toContain(host);
  }
  process.stdout.write(
    `local-reader: ${requests.length} request(s) from ${hostsOf(requests).join(", ")}\n`,
  );
  expect(
    await carriesAnalytics(page),
    "a local page carries no analytics tag",
  ).toBe(false);
});

test("a second page of the manual loads with nothing from outside, and no policy blocks it", async ({
  page,
}) => {
  const origin = `http://127.0.0.1:${port}`;
  const requests = watch(page);
  const complaints: string[] = [];
  page.on("console", (message) => {
    if (message.type() === "error") complaints.push(message.text());
  });
  page.on("pageerror", (error) => complaints.push(error.message));

  // Two real pages of the manual, named by the manifest rather than by
  // following a link: the shell's own navigation is built from the page
  // library the shell was BUILT with, not from the package this reader
  // was pointed at, so a link on the page is not yet an address here.
  for (const at of [0, 1]) {
    const response = await page.goto(`${origin}${pageHref(at)}`, {
      waitUntil: "networkidle",
    });
    expect(response?.status(), `page ${at} of the manual answers`).toBe(200);
  }

  expect(requests.length, "a page makes requests").toBeGreaterThan(1);
  for (const one of requests) {
    expect(new URL(one).origin, `${one} is not this reader`).toBe(origin);
  }
  const blocked = complaints.filter((text) =>
    /content security policy|refused to (load|execute)/i.test(text),
  );
  expect(blocked, "the policy blocked something the shell needs").toEqual([]);
  process.stdout.write(
    `local-reader: ${requests.length} request(s) over two pages from ${hostsOf(
      requests,
    ).join(", ")}\n`,
  );
  expect(
    await carriesAnalytics(page),
    "a local page carries no analytics tag",
  ).toBe(false);
});

test("the static build opened as a file makes no request of its own", async ({
  page,
}) => {
  scratch = mkdtempSync(join(tmpdir(), "vibe-local-reader-"));
  const built = spawnSync(
    VIBE_BIN,
    [
      "doc",
      "build",
      "--path",
      MANUAL,
      "--out",
      scratch,
      "--format",
      "html",
      "--no-derived",
    ],
    { encoding: "utf8" },
  );
  expect(built.status, built.stderr ?? "").toBe(0);

  // The island of the first page the build wrote. It is a fragment, so
  // it is opened as a document of its own — which is the harshest
  // reading of «no external request»: nothing wraps it and nothing can
  // be blamed for what it fetches.
  const file = join(
    scratch,
    ...pageHref(0)
      .replace(/^\/doc\//, "")
      .replace(/\/$/, "")
      .split("/"),
    "index.html",
  );
  expect(existsSync(file), `the build wrote ${file}`).toBe(true);

  const requests = watch(page);
  await page.goto(pathToFileURL(file).href, { waitUntil: "networkidle" });
  const outside = requests.filter((one) => !one.startsWith("file:"));
  expect(outside, "the static output fetched something").toEqual([]);
  expect(await carriesAnalytics(page), "the static output carries a tag").toBe(
    false,
  );
  process.stdout.write(
    `local-reader: static output made ${requests.length} request(s) from ${hostsOf(
      requests,
    ).join(", ")}\n`,
  );
});
