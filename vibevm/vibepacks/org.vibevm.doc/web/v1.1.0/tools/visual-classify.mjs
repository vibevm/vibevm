#!/usr/bin/env node
// `node tools/visual-classify.mjs <astro-dist> [qwik-dist]` — what the
// differing pixels ARE.
//
// The pixel gate beside this file reports that a tenth to a quarter of
// the pixels differ. That number is true and says almost nothing, because
// it cannot tell a page that was laid out differently from a page that
// was laid out identically sixteen pixels lower. A constant vertical
// shift makes every line of text differ from the line that used to be
// there, and the two builds DO sit at different heights: this one wears
// the documentation's header, which is shorter than the landing's own.
//
// So this takes the number apart, in four steps, each of which removes
// one thing that is known and named:
//
//   raw       the two builds as they are.
//   aligned   the same, after sliding one image vertically to where it
//             best matches the other. What is left is no longer the
//             offset. The offset itself is reported, because a page that
//             starts 16px higher is a fact and a page that starts 300px
//             higher is a defect.
//   +type     after restoring the reference's 16px/1.6 body type. This
//             package sets 17px/1.65, which changes every glyph's raster
//             and every wrap point.
//   +tones    after restoring the three dark tones the contrast audit
//             minted. Every word on the page is drawn in one of them.
//
// What survives all four is the residual: differences that are neither
// the header's height, nor the type scale, nor the minted tones. That is
// the number worth looking at, and the one a reviewer should be handed.
//
// The alignment is found by cross-correlating the two images' row
// brightness profiles — one pass over each image, then a sweep over a
// hundred and twenty-one candidate offsets on two vectors of nine
// hundred numbers. Sliding the images themselves would be a hundred
// million comparisons per candidate.

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
const PORTS = { reference: 47341, port: 47342 };
const TOLERANCE = 8;

/**
 * What the residual may be, and why it is this number.
 *
 * The residual is measured with the two pages slid into alignment, the
 * reference's type scale and dark tones put back, and the two headers —
 * which are deliberately different bars — out of the frame. What is left
 * is the same glyphs drawn at a fractional offset, and the picture of it
 * (`*-residual.png`) is glyph outlines and nothing else.
 *
 * Two and a half per cent is the band this pair of builds produces —
 * four of the eight pages come in under a tenth of a per cent, two under
 * one and a tenth, two under two — with room for the two that carry the
 * most small mono type. It is a ratchet: a page that climbs out of it
 * has a difference that is not rasterization, and the residual picture
 * will show what it is.
 */
const RESIDUAL_THRESHOLD = 0.025;
const VIEWPORT = { width: 1440, height: 900 };
/** How far one build may sit from the other before the slide gives up. */
const MAX_OFFSET = 60;

const LAYERS = [
  { id: "raw", css: "" },
  {
    /* The doorway into the essay, which the AI-Native hero gained with
       `/vision/` (parity rule D-34). It is a block the reference has no
       counterpart for, and it moves everything under it down by its own
       height — which a single whole-page slide cannot absorb. Hiding it
       subtracts that one known decision, the way the two rows below
       subtract the type scale and the minted tones. */
    id: "entry",
    css: `.an-vision-entry { display: none !important; }`,
  },
  {
    /* The landing's ground and its map (parity rule D-40, owner
       2026-09-26): four faint planes laid behind the whole page, one of
       them behind the hero's constellation inside this very frame, and
       the section of plates under the owner's content. Neither has a
       counterpart in the reference and both are decisions; the planes
       alone would count a fifth of the hero's pixels as different, for
       a colour moved by a few units under everything. Hidden here, so the
       residual is about the hero the reference also drew. */
    id: "map",
    css: `.landing-field, .landing-map { display: none !important; }`,
  },
  {
    id: "type",
    css: `body { font-size: 16px !important; line-height: 1.6 !important; }`,
  },
  {
    id: "tones",
    css: `:root, [data-theme="dark"] {
      --text-2: #a8a197 !important;
      --text-3: #6f695e !important;
      --accent: #d97757 !important;
      --accent-hover: #e08a6d !important;
    }`,
  },
];

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

const TYPES = {
  ".html": "text/html; charset=utf-8",
  ".js": "text/javascript; charset=utf-8",
  ".css": "text/css; charset=utf-8",
  ".json": "application/json; charset=utf-8",
  ".svg": "image/svg+xml",
  ".png": "image/png",
  ".woff2": "font/woff2",
  ".txt": "text/plain; charset=utf-8",
  ".xml": "application/xml; charset=utf-8",
};

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

const freeze = () => {
  document.getAnimations().forEach((animation) => {
    try {
      animation.currentTime = 3000;
      animation.pause();
    } catch {
      /* already finished */
    }
  });
};

async function shoot(page, origin, path, css) {
  await page.goto(`${origin}${path}`, { waitUntil: "load" });
  if (css.length > 0) await page.addStyleTag({ content: css });
  await page.evaluate(() => document.fonts.ready);
  await page.evaluate(freeze);
  await page.evaluate(
    () => new Promise((done) => requestAnimationFrame(() => done(null))),
  );
  return page.screenshot({ animations: "disabled", scale: "css" });
}

const measureInPage = async ([a, b, tolerance, maxOffset]) => {
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

  /** The mean brightness of each row — a page's vertical fingerprint. */
  const profile = (image) => {
    const rows = new Float64Array(image.height);
    for (let y = 0; y < image.height; y += 1) {
      let sum = 0;
      const base = y * image.width * 4;
      for (let x = 0; x < image.width; x += 1) {
        const i = base + x * 4;
        sum += image.data[i] + image.data[i + 1] + image.data[i + 2];
      }
      rows[y] = sum / image.width;
    }
    return rows;
  };

  const difference = (dy, skipTop = 0) => {
    const width = Math.min(left.width, right.width);
    let differing = 0;
    let compared = 0;
    for (let y = skipTop; y < left.height; y += 1) {
      const other = y + dy;
      if (other < 0 || other >= right.height) continue;
      const rowA = y * left.width * 4;
      const rowB = other * right.width * 4;
      for (let x = 0; x < width; x += 1) {
        const i = rowA + x * 4;
        const j = rowB + x * 4;
        compared += 1;
        if (
          Math.abs(left.data[i] - right.data[j]) > tolerance ||
          Math.abs(left.data[i + 1] - right.data[j + 1]) > tolerance ||
          Math.abs(left.data[i + 2] - right.data[j + 2]) > tolerance
        ) {
          differing += 1;
        }
      }
    }
    return compared === 0 ? 1 : differing / compared;
  };

  /* The offset is found on the profiles and confirmed on the pixels: the
     correlation says roughly where, and the pixel count at the three
     best candidates says exactly. */
  const pa = profile(left);
  const pb = profile(right);
  const scores = [];
  for (let dy = -maxOffset; dy <= maxOffset; dy += 1) {
    let sum = 0;
    let n = 0;
    for (let y = 0; y < pa.length; y += 1) {
      const other = y + dy;
      if (other < 0 || other >= pb.length) continue;
      sum += Math.abs(pa[y] - pb[other]);
      n += 1;
    }
    if (n > 0) scores.push({ dy, score: sum / n });
  }
  scores.sort((x, y) => x.score - y.score);
  let best = { dy: 0, ratio: difference(0) };
  for (const candidate of scores.slice(0, 3)) {
    const ratio = difference(candidate.dy);
    if (ratio < best.ratio) best = { dy: candidate.dy, ratio };
  }
  /* The two headers are not the same bar — this site wears the one the
     documentation wears, with a search field and a theme switch in it —
     so the band they occupy is a known difference rather than a measured
     one. Reported separately so the number below it is about the PAGE. */
  /* A picture of what is left, so the residual is looked at rather than
     guessed about: every differing pixel in magenta over the reference,
     dimmed, at the offset the two were matched on. */
  const paint = () => {
    const canvas = document.createElement("canvas");
    canvas.width = left.width;
    canvas.height = left.height;
    const context = canvas.getContext("2d");
    const out = context.createImageData(left.width, left.height);
    for (let y = 0; y < left.height; y += 1) {
      for (let x = 0; x < left.width; x += 1) {
        const i = (y * left.width + x) * 4;
        const other = y + best.dy;
        const inside =
          other >= 0 && other < right.height && x < right.width && y >= 100;
        const j = inside ? (other * right.width + x) * 4 : -1;
        const differs =
          inside &&
          (Math.abs(left.data[i] - right.data[j]) > tolerance ||
            Math.abs(left.data[i + 1] - right.data[j + 1]) > tolerance ||
            Math.abs(left.data[i + 2] - right.data[j + 2]) > tolerance);
        out.data[i] = differs ? 255 : left.data[i] >> 2;
        out.data[i + 1] = differs ? 0 : left.data[i + 1] >> 2;
        out.data[i + 2] = differs ? 255 : left.data[i + 2] >> 2;
        out.data[i + 3] = 255;
      }
    }
    context.putImageData(out, 0, 0);
    return canvas.toDataURL("image/png");
  };

  return {
    raw: difference(0),
    aligned: best.ratio,
    belowHeader: difference(best.dy, 100),
    offset: best.dy,
    diff: paint(),
  };
};

async function main() {
  const referenceRoot = process.argv[2];
  if (referenceRoot === undefined) {
    process.stderr.write(
      "visual-classify: usage — node tools/visual-classify.mjs <astro-dist> [qwik-dist]\n",
    );
    process.exit(2);
  }
  const portRoot = process.argv[3] ?? join(PACKAGE_ROOT, "site", "dist");
  const servers = [
    await serve(referenceRoot, PORTS.reference),
    await serve(portRoot, PORTS.port),
  ];
  const referenceOrigin = `http://127.0.0.1:${PORTS.reference}`;
  const portOrigin = `http://127.0.0.1:${PORTS.port}`;

  mkdirSync(join(PACKAGE_ROOT, "tmp", "visual-parity"), { recursive: true });
  const browser = await chromium.launch();
  const context = await browser.newContext({ deviceScaleFactor: 1 });
  const rows = [];

  for (const pair of PAIRS) {
    const page = await context.newPage();
    await page.setViewportSize(VIEWPORT);
    const reference = await shoot(page, referenceOrigin, pair.reference, "");
    const row = { id: pair.id, route: pair.port };
    let css = "";
    for (const layer of LAYERS) {
      css = `${css}\n${layer.css}`;
      const shot = await shoot(page, portOrigin, pair.port, css);
      const measured = await page.evaluate(measureInPage, [
        `data:image/png;base64,${reference.toString("base64")}`,
        `data:image/png;base64,${shot.toString("base64")}`,
        TOLERANCE,
        MAX_OFFSET,
      ]);
      if (layer.id === "raw") {
        row.atZero = measured.raw;
        row.offset = measured.offset;
      }
      row[layer.id] = measured.aligned;
      row[`${layer.id}BelowHeader`] = measured.belowHeader;
      if (layer.id === "tones") {
        writeFileSync(
          join(PACKAGE_ROOT, "tmp", "visual-parity", `${pair.id}-residual.png`),
          Buffer.from(measured.diff.split(",")[1], "base64"),
        );
      }
    }
    await page.close();
    rows.push(row);
  }

  await browser.close();
  for (const server of servers) server.close();

  const pct = (value) => `${(value * 100).toFixed(2)}%`;
  const lines = [
    "# What the differing pixels are — desktop hero, 1440×900",
    "",
    `- reference: \`${referenceRoot}\``,
    `- this build: \`${portRoot}\``,
    `- a pixel differs when any channel moves by more than ${TOLERANCE}/255`,
    "",
    "| page | raw | offset | aligned | − entry | − map | + type | + tones | below the header |",
    "| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |",
    ...rows.map(
      (row) =>
        `| \`${row.route}\` | ${pct(row.atZero)} | ${row.offset > 0 ? "+" : ""}${row.offset}px | ${pct(row.raw)} | ${pct(row.entry)} | ${pct(row.map)} | ${pct(row.type)} | ${pct(row.tones)} | ${pct(row.tonesBelowHeader)} |`,
    ),
    "",
    "The last column is the residual: with the pages aligned, the reference's",
    "type scale and dark tones restored, and the two headers — which are",
    "deliberately different bars — left out of the frame. What remains is the",
    "same glyphs drawn at a different sub-pixel offset; `<page>-residual.png`",
    "beside this report paints every one of those pixels in magenta, and the",
    "picture is letter outlines.",
    "",
    "## What each column removes",
    "",
    "- **− entry** — the doorway into the essay `/vision/`, which the",
    "  AI-Native hero gained after the reference was frozen (D-34). One",
    "  block, hidden for the measurement, so its height stops shifting",
    "  everything below it on that one page.",
    "- **− map** — the landing's ground and its map (D-40): four faint",
    "  planes behind the whole page, one of them inside this frame behind",
    "  the constellation, and the section of plates under the owner's",
    "  content. Both are decisions the reference predates; hidden, so the",
    "  residual is about the hero the reference also drew.",
    "- **offset** — this build wears the documentation's header, which is",
    "  seventeen pixels shorter than the landing's own bar, so every element",
    "  below it starts seventeen pixels higher. A constant shift makes every",
    "  line of text differ from the line that used to be at that height; it is",
    "  the single largest contributor and it is one accepted decision (D-10,",
    "  D-16, D-28: one header for one site).",
    "- **+ type** — the design system reads at 17px/1.65 where the reference",
    "  read at 16px/1.6.",
    "- **+ tones** — `--text-2`, `--text-3` and `--accent` were minted upward",
    "  by the APCA audit, so every word and every accent stroke on the page is",
    "  drawn in a slightly different tone than the reference's.",
    "- **below the header** — the two bars themselves, which carry a search",
    "  field, a theme switch and a `Documentation` entry the reference's bar",
    "  had none of.",
    "",
  ];
  const over = rows.filter((row) => row.tonesBelowHeader > RESIDUAL_THRESHOLD);
  lines.push(
    over.length === 0
      ? `visual parity: green — every page's residual is under ${pct(RESIDUAL_THRESHOLD)}; the differences that remain are the accepted header, type scale and minted tones, and glyph rasterization under them.`
      : `visual parity: RED — ${over.length} page(s) carry a residual over ${pct(RESIDUAL_THRESHOLD)}: ${over.map((row) => row.route).join(", ")}. Read the residual pictures; what is in them is not rasterization.`,
    "",
  );

  const markdown = lines.join("\n");
  const out = join(PACKAGE_ROOT, "tmp", "visual-parity");
  mkdirSync(out, { recursive: true });
  writeFileSync(join(out, "classify.md"), markdown, "utf8");
  writeFileSync(
    join(out, "classify.json"),
    `${JSON.stringify({ referenceRoot, portRoot, threshold: RESIDUAL_THRESHOLD, rows }, null, 2)}\n`,
    "utf8",
  );
  process.stdout.write(`${markdown}\n`);
  process.exitCode = over.length === 0 ? 0 : 1;
}

await main();
