/** @scope spec://org.vibevm.core/vibevm/common/PROP-057#SITE-MOUNT */

/**
 * A site of more than one documentation, in a real browser.
 *
 * Every other end-to-end test here reads the build the package makes of
 * its own fixture pair, which is ONE documentation in two languages —
 * and one library is the case where every grouping rule agrees with
 * every other by accident. What cannot be measured there is the thing
 * the registry render is made of: two source documentations that adapt
 * nothing of each other's, an adaptation that belongs to the second and
 * not to the first, and a door that has to list all of them.
 *
 * So this builds its own site. The fixture trees are named in
 * `VIBE_DOC_OUT` exactly as a deployment names its rendered ones, the
 * output goes beside the ordinary `dist` rather than over it
 * (`tools/out-dir.mjs`), and the same little static server the rest of
 * the suite reads `dist` through serves it on a port of its own. A build
 * that fails here fails the test with its own output, because a site
 * that cannot be built over two libraries is the defect this file exists
 * to catch.
 */

import { spawn, spawnSync, type ChildProcess } from "node:child_process";
import { createServer } from "node:http";
import { delimiter, dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

import { expect, test } from "@playwright/test";

const HERE = dirname(fileURLToPath(import.meta.url));
const PACKAGE_ROOT = resolve(HERE, "..", "..");
const FIXTURES = join(PACKAGE_ROOT, "site", "src", "fixtures");

/** Where this build writes, beside the one the other tests read. */
const OUT = "dist-libraries";

/** Two documentations and one adaptation of the second. */
const TREES = [
  join(FIXTURES, "doc-build"),
  join(FIXTURES, "doc-build-pair"),
  join(FIXTURES, "doc-build-pair-ru"),
];

/** The addresses the three fixture trees put on the site. */
const MANUAL = "/doc/com.example.docs/fixture-manual/0.1.0/guide/every-block/";
const PAIR = "/doc/com.example.docs/pair/0.1.0/guide/one/";
const PAIR_RU = "/doc/ru/com.example.docs/pair/0.1.0/guide/one/";

let server: ChildProcess | undefined;
let origin = "";
let failure = "";

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

/** Wait until the little server answers the door. */
async function ready(at: string): Promise<boolean> {
  for (let attempt = 0; attempt < 80; attempt += 1) {
    try {
      const response = await fetch(`${at}/doc/`);
      if (response.ok) return true;
    } catch {
      // Not up yet. The loop is the wait.
    }
    await new Promise((settle) => setTimeout(settle, 250));
  }
  return false;
}

test.beforeAll(async () => {
  /* A whole static build, which is minutes rather than the seconds a
     test is given: the hook's own limit is raised and no test's is. */
  test.setTimeout(600_000);
  const built = spawnSync(
    process.execPath,
    [join("tools", "build.mjs"), "static"],
    {
      cwd: PACKAGE_ROOT,
      encoding: "utf8",
      env: {
        ...process.env,
        VIBE_DOC_OUT: TREES.join(delimiter),
        VIBE_SITE_DIST: OUT,
      },
    },
  );
  if (built.status !== 0) {
    failure = `the two-library build failed (${built.status}):\n${built.stdout}\n${built.stderr}`;
    return;
  }
  const port = await freePort();
  origin = `http://127.0.0.1:${port}`;
  server = spawn(
    process.execPath,
    [join(HERE, "serve.mjs"), String(port), OUT],
    { cwd: PACKAGE_ROOT, stdio: "ignore" },
  );
  if (!(await ready(origin))) failure = `${origin} never answered /doc/`;
});

test.afterAll(() => {
  server?.kill();
});

test.beforeEach(() => {
  if (failure !== "") throw new Error(failure);
});

test("the door lists every documentation the build carries", async ({
  page,
}) => {
  await page.goto(`${origin}/doc/`);
  /* The heading carries the standing badge beside the name, so each is
     matched at its start: what is asserted is which documentations
     stand there and in which order. */
  const titles = page.locator(".shelf__items .card__title");
  await expect(titles).toHaveCount(3);
  await expect(titles).toHaveText([/^Fixture Manual/, /^The Pair/, /^Пара/]);

  // Each card leads to its own package page, the adaptation behind its
  // source's coordinate with the language in front (D-06).
  const links = page.locator(".shelf__items .card__link");
  await expect(links.nth(0)).toHaveAttribute(
    "href",
    "/doc/com.example.docs/fixture-manual/0.1.0/",
  );
  await expect(links.nth(1)).toHaveAttribute(
    "href",
    "/doc/com.example.docs/pair/0.1.0/",
  );
  await expect(links.nth(2)).toHaveAttribute(
    "href",
    "/doc/ru/com.example.docs/pair/0.1.0/",
  );
});

test("each documentation is served from its own bytes", async ({ page }) => {
  await page.goto(`${origin}${MANUAL}`);
  await expect(page.locator("[data-island] h1")).toHaveText("Every block once");

  await page.goto(`${origin}${PAIR}`);
  await expect(page.locator("[data-island] h1")).toHaveText("One");
  await expect(page.locator("[data-island]")).toContainText(
    "The first page of the pair",
  );

  await page.goto(`${origin}${PAIR_RU}`);
  await expect(page.locator("[data-island]")).toContainText("Первая страница");
});

test("a page names the pages of its own documentation and no other's", async ({
  page,
}) => {
  await page.goto(`${origin}${PAIR}`);
  const nav = page.locator(".contents a");
  await expect(nav).toHaveCount(2);
  for (const href of await nav.evaluateAll((items) =>
    items.map((item) => item.getAttribute("href") ?? ""),
  )) {
    expect(href).toContain("/com.example.docs/pair/");
  }
});

test("an adaptation is never served under its own coordinate", async ({
  page,
}) => {
  const response = await page.goto(
    `${origin}/doc/com.example.docs/pair-ru/0.1.0/guide/one/`,
  );
  expect(response?.status()).toBe(404);
});

test("the sitemap index carries a part for each documentation and language", async ({
  request,
}) => {
  const index = await (await request.get(`${origin}/doc/sitemap.xml`)).text();
  for (const part of [
    "/doc/sitemap/com.example.docs/fixture-manual/en.xml",
    "/doc/sitemap/com.example.docs/pair/en.xml",
    "/doc/sitemap/com.example.docs/pair/ru.xml",
  ]) {
    expect(index).toContain(`<loc>https://vibevm.org${part}</loc>`);
  }
  // One catalogue part for the whole site, never one per documentation.
  expect(
    [...index.matchAll(/sitemap\/catalogues\.xml/g)].length,
    "the catalogues stand in one part",
  ).toBe(1);
});

test("the catalogue for an agent names every edition", async ({ request }) => {
  const catalogue = await (await request.get(`${origin}/doc/llms.txt`)).text();
  expect(catalogue).toContain("### Fixture Manual — source");
  expect(catalogue).toContain("### The Pair — source");
  expect(catalogue).toContain("### Пара — ★ official");
});
