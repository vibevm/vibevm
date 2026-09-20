#!/usr/bin/env node
// `node tools/visual-parity.mjs <astro-dist> [qwik-dist]` — the other half
// of the parity gate: what the two builds LOOK like, measured rather than
// judged by eye.
//
// `tools/parity.mjs` compares the two builds tag by tag and word by word,
// and a page can pass all of that while being laid out wrong. A column
// that became one, a band that lost its inversion, a drawing that clipped
// at the wrong breakpoint — none of it changes a byte of the head or a
// fragment of the text. So this drives both builds in one browser, at the
// three widths the design was drawn for, and compares the pixels.
//
// Three things make the numbers mean something.
//
// ONE BROWSER. Both sides are rendered by the same Chromium at the same
// device scale factor, from the same font files — the two builds serve
// byte-identical `woff2`. A difference in rasterization would otherwise
// swamp every real difference, and comparing a screenshot taken today
// against one taken on another machine in another week is comparing two
// font stacks.
//
// FROZEN MOTION. Both pages animate: the frieze draws itself, the orbits
// turn on a four-minute loop, a status dot breathes, a question pings.
// Two screenshots of the same page at two instants differ, and the
// difference is the clock rather than the design. So every animation on
// both sides is moved to the same fixed time and paused there. The entry
// animations are past their end and hold their final state; the endless
// ones stand at the same phase on both sides because their keyframes and
// durations are the same.
//
// PAIRED ADDRESSES. One page moved — the owner shortened the AI-Native
// slug — so the pairs are named rather than derived, exactly as in the
// textual gate.
//
// The report is deterministic: the same two builds produce the same
// numbers, so a diff of two reports is a diff of the site.

import {
  createReadStream,
  existsSync,
  mkdirSync,
  statSync,
  writeFileSync,
} from "node:fs";
import { createServer } from "node:http";
import { dirname, extname, join, normalize, resolve, sep } from "node:path";
import { fileURLToPath } from "node:url";

import { chromium } from "@playwright/test";

const PACKAGE_ROOT = resolve(dirname(fileURLToPath(import.meta.url)), "..");

/** Two ports nothing else on this machine is expected to hold. */
const PORTS = { reference: 47311, port: 47312 };

/** The widths the design was drawn for, named the way the review names them. */
const VIEWPORTS = [
  { name: "desktop", width: 1440, height: 900 },
  { name: "tablet", width: 834, height: 1112 },
  { name: "mobile", width: 390, height: 844 },
];

/**
 * The page pairs. The reference address first, then this build's.
 *
 * The AI-Native page is the one pair that is not two spellings of the
 * same string, and that is the owner's routing decision rather than an
 * accident of the port (parity rule D-29).
 */
const PAIRS = [
  { id: "home-en", reference: "/", port: "/" },
  { id: "home-ru", reference: "/ru/", port: "/ru/" },
  { id: "vibevm-en", reference: "/why/vibevm/", port: "/why/vibevm/" },
  { id: "vibevm-ru", reference: "/ru/why/vibevm/", port: "/ru/why/vibevm/" },
  { id: "zap-en", reference: "/why/zap/", port: "/why/zap/" },
  { id: "zap-ru", reference: "/ru/why/zap/", port: "/ru/why/zap/" },
  {
    id: "ai-native-en",
    reference: "/why/ai-native-language/",
    port: "/why/ai-native/",
  },
  {
    id: "ai-native-ru",
    reference: "/ru/why/ai-native-language/",
    port: "/ru/why/ai-native/",
  },
];

/**
 * This run is EVIDENCE, not a verdict, and that is a finding rather than
 * a design choice.
 *
 * The first version of this file carried a threshold and failed every one
 * of its fifty-six comparisons at nine to twenty-nine per cent — the
 * already-accepted landing included. Taking the number apart
 * (`tools/visual-classify.mjs`) showed what it was made of: this build
 * wears the documentation's header, which is seventeen pixels shorter
 * than the landing's own bar, so every element below it sits seventeen
 * pixels higher and every line of text differs from the line that used
 * to be at that height. A raw pixel count cannot tell that from a page
 * that was laid out wrong, and a gate that cannot tell them apart is a
 * gate that gets switched off.
 *
 * So the judgement moved to two files that can answer:
 * `tools/layout-parity.mjs` compares the composition — sections, row
 * shapes at each width, where each drawing sits and how big it is — and
 * `tools/visual-classify.mjs` compares the pixels with the known
 * decisions removed one at a time and gates on what is left. What this
 * file produces is the pair of pictures behind both of those numbers, at
 * three widths, hero and whole page, with and without motion.
 */
const THRESHOLDS = { hero: 0.02, full: 0.02, reduced: 0.02 };

/**
 * A pixel counts as different when any channel moves by more than this.
 *
 * Anti-aliasing on a curve moves a channel by a few units between two
 * renderings of the same geometry at the same size; a colour that
 * actually changed moves it by tens. Eight is above the first and well
 * below the second.
 */
const CHANNEL_TOLERANCE = 8;

// ---------------------------------------------------------------------
// The two servers
// ---------------------------------------------------------------------

const TYPES = {
  ".html": "text/html; charset=utf-8",
  ".js": "text/javascript; charset=utf-8",
  ".mjs": "text/javascript; charset=utf-8",
  ".css": "text/css; charset=utf-8",
  ".json": "application/json; charset=utf-8",
  ".svg": "image/svg+xml",
  ".png": "image/png",
  ".woff2": "font/woff2",
  ".txt": "text/plain; charset=utf-8",
  ".xml": "application/xml; charset=utf-8",
};

/** A static host for one build, bound to the loopback and nothing else. */
function serve(root, port) {
  const base = resolve(root);
  const server = createServer((request, response) => {
    const url = new URL(request.url ?? "/", `http://127.0.0.1:${port}`);
    const path = decodeURIComponent(url.pathname);
    const file = join(base, normalize(path));
    if (file !== base && !file.startsWith(base + sep)) {
      response.writeHead(403).end("no");
      return;
    }
    const target = path.endsWith("/") ? join(file, "index.html") : file;
    if (!existsSync(target) || statSync(target).isDirectory()) {
      response.writeHead(404).end("not found");
      return;
    }
    response.writeHead(200, {
      "content-type": TYPES[extname(target)] ?? "application/octet-stream",
      "cache-control": "no-store",
    });
    createReadStream(target).pipe(response);
  });
  return new Promise((done) => {
    server.listen(port, "127.0.0.1", () => done(server));
  });
}

// ---------------------------------------------------------------------
// Taking one shot
// ---------------------------------------------------------------------

/**
 * Move every animation on the page to one fixed instant and stop it
 * there.
 *
 * Three seconds, which is past the end of every entry animation on both
 * sites — the longest is the orbital trajectory at 1.3s after a 0.35s
 * delay — so those hold their final state. The endless ones stand at the
 * same phase on both sides, because the phase is a function of the time
 * and the duration and both are the same.
 */
const FREEZE = `
  document.getAnimations().forEach((animation) => {
    try {
      animation.currentTime = 3000;
      animation.pause();
    } catch {
      /* An animation that has already finished refuses a seek; it is
         already where this wanted to put it. */
    }
  });
`;

async function shoot(page, origin, path, fullPage) {
  await page.goto(`${origin}${path}`, { waitUntil: "load" });
  await page.evaluate(() => document.fonts.ready);
  await page.evaluate(FREEZE);
  /* One frame after the seek, so the compositor has drawn the state the
     seek put every element in rather than the one before it. */
  await page.evaluate(
    () => new Promise((done) => requestAnimationFrame(() => done(null))),
  );
  return page.screenshot({ fullPage, animations: "disabled", scale: "css" });
}

// ---------------------------------------------------------------------
// The comparison
// ---------------------------------------------------------------------

/**
 * Compare two PNGs, in the browser that produced them.
 *
 * The decoding is the browser's own and the arithmetic is a loop over
 * two `ImageData` buffers, so the gate carries no image library and no
 * second opinion about what a PNG is. Where the two differ in size, the
 * overlap is compared and the rest is counted as differing — a page that
 * got taller IS a difference, and hiding it behind a crop would be the
 * one failure this gate exists to catch.
 */
const compareInPage = async ([a, b, tolerance]) => {
  const load = async (dataUrl) => {
    const image = new Image();
    image.src = dataUrl;
    await image.decode();
    const canvas = document.createElement("canvas");
    canvas.width = image.naturalWidth;
    canvas.height = image.naturalHeight;
    const context = canvas.getContext("2d", { willReadFrequently: true });
    context.drawImage(image, 0, 0);
    return {
      width: canvas.width,
      height: canvas.height,
      data: context.getImageData(0, 0, canvas.width, canvas.height).data,
    };
  };
  const left = await load(a);
  const right = await load(b);
  const width = Math.min(left.width, right.width);
  const height = Math.min(left.height, right.height);
  let differing = 0;
  for (let y = 0; y < height; y += 1) {
    for (let x = 0; x < width; x += 1) {
      const i = (y * left.width + x) * 4;
      const j = (y * right.width + x) * 4;
      if (
        Math.abs(left.data[i] - right.data[j]) > tolerance ||
        Math.abs(left.data[i + 1] - right.data[j + 1]) > tolerance ||
        Math.abs(left.data[i + 2] - right.data[j + 2]) > tolerance
      ) {
        differing += 1;
      }
    }
  }
  const union =
    Math.max(left.width, right.width) * Math.max(left.height, right.height);
  const outside = union - width * height;
  return {
    reference: { width: left.width, height: left.height },
    port: { width: right.width, height: right.height },
    differing: differing + outside,
    compared: union,
    ratio: union === 0 ? 1 : (differing + outside) / union,
    outside,
  };
};

// ---------------------------------------------------------------------

async function main() {
  const referenceRoot = process.argv[2];
  if (referenceRoot === undefined) {
    process.stderr.write(
      "visual-parity: usage — node tools/visual-parity.mjs <astro-dist> [qwik-dist]\n",
    );
    process.exit(2);
  }
  const portRoot = process.argv[3] ?? join(PACKAGE_ROOT, "site", "dist");
  const out = join(PACKAGE_ROOT, "tmp", "visual-parity");
  mkdirSync(out, { recursive: true });

  const servers = [
    await serve(referenceRoot, PORTS.reference),
    await serve(portRoot, PORTS.port),
  ];
  const referenceOrigin = `http://127.0.0.1:${PORTS.reference}`;
  const portOrigin = `http://127.0.0.1:${PORTS.port}`;

  const browser = await chromium.launch();
  const rows = [];

  /** One comparison, with its two files written beside the report. */
  const compare = async (context, pair, viewport, kind, fullPage) => {
    const page = await context.newPage();
    await page.setViewportSize({
      width: viewport.width,
      height: viewport.height,
    });
    const a = await shoot(page, referenceOrigin, pair.reference, fullPage);
    const b = await shoot(page, portOrigin, pair.port, fullPage);
    const name = `${pair.id}-${viewport.name}-${kind}`;
    writeFileSync(join(out, `${name}.astro.png`), a);
    writeFileSync(join(out, `${name}.qwik.png`), b);

    const measured = await page.evaluate(compareInPage, [
      `data:image/png;base64,${a.toString("base64")}`,
      `data:image/png;base64,${b.toString("base64")}`,
      CHANNEL_TOLERANCE,
    ]);
    await page.close();

    rows.push({
      route: pair.port,
      reference: pair.reference,
      viewport: `${viewport.width}×${viewport.height}`,
      kind,
      astro: `tmp/visual-parity/${name}.astro.png`,
      qwik: `tmp/visual-parity/${name}.qwik.png`,
      referenceSize: `${measured.reference.width}×${measured.reference.height}`,
      portSize: `${measured.port.width}×${measured.port.height}`,
      ratio: measured.ratio,
      threshold: THRESHOLDS[kind],
      pass: measured.ratio <= THRESHOLDS[kind],
    });
  };

  const motion = await browser.newContext({ deviceScaleFactor: 1 });
  const still = await browser.newContext({
    deviceScaleFactor: 1,
    reducedMotion: "reduce",
  });

  for (const pair of PAIRS) {
    for (const viewport of VIEWPORTS) {
      await compare(motion, pair, viewport, "hero", false);
      await compare(motion, pair, viewport, "full", true);
    }
    /* Reduced motion is asked once per page, at the width the design is
       reviewed at: the question it answers — does the decoration stand
       still — has no width. */
    await compare(still, pair, VIEWPORTS[0], "reduced", false);
  }

  await browser.close();
  for (const server of servers) server.close();

  report(rows, referenceRoot, portRoot, out);
}

function report(rows, referenceRoot, portRoot, out) {
  const failed = rows.filter((row) => !row.pass);
  const pct = (value) => `${(value * 100).toFixed(3)}%`;

  const lines = [
    "# Visual parity — the screenshots, and the raw difference between them",
    "",
    `- reference: \`${referenceRoot}\``,
    `- this build: \`${portRoot}\``,
    `- browser: one headless Chromium, device scale 1, motion frozen at 3.000s`,
    `- a pixel differs when any channel moves by more than ${CHANNEL_TOLERANCE}/255`,
    `- screenshots: \`${out}\``,
    "",
    "**These numbers are evidence, not a verdict.** Every one of them is",
    "dominated by one accepted decision: this build wears the documentation's",
    "header, seventeen pixels shorter than the landing's own bar, so the whole",
    "page sits seventeen pixels higher and every line of text differs from the",
    "line that used to be at that height. The already-accepted landing scores",
    "the same way. The verdicts are in `layout.md` — the composition at three",
    "widths — and `classify.md` — the pixels with the header, the type scale",
    "and the minted tones removed one at a time.",
    "",
    "| route | viewport | shot | reference | this build | differing | threshold | verdict |",
    "| --- | --- | --- | --- | --- | ---: | ---: | --- |",
    ...rows.map(
      (row) =>
        `| \`${row.route}\` | ${row.viewport} | ${row.kind} | ${row.referenceSize} | ${row.portSize} | ${pct(row.ratio)} | ${pct(row.threshold)} | ${row.pass ? "pass" : "OVER"} |`,
    ),
    "",
    `${rows.length} screenshot pair(s) written; ${failed.length} carry a raw difference over ${pct(THRESHOLDS.hero)}, which is expected and explained above.`,
    "",
  ];

  const markdown = lines.join("\n");
  writeFileSync(join(out, "report.md"), markdown, "utf8");
  writeFileSync(
    join(out, "report.json"),
    `${JSON.stringify({ referenceRoot, portRoot, rows }, null, 2)}\n`,
    "utf8",
  );
  process.stdout.write(`${markdown}\n`);
}

await main();
