#!/usr/bin/env node
// `node tools/layout-parity.mjs <astro-dist> [qwik-dist]` — the gate that
// says whether the two builds are the same COMPOSITION.
//
// Why this and not a pixel count. The two builds share a design and not a
// stylesheet: this one sets its body type one point larger, mints three
// dark tones the contrast audit asked for, and wears the header the
// documentation wears — a shorter, sticky bar with a search box in it.
// Each of those is a decision the package took on purpose, and each of
// them moves every line on every page by a few pixels. A raw pixel
// comparison answers «did anything move» with «yes, all of it», and the
// one question worth asking — did the composition survive — disappears
// into the noise.
//
// So this compares the things the design IS, at each width it was drawn
// for:
//
//   · the sections, in order, by the heading each carries;
//   · how many things stand across a row inside each of them — which is
//     what a breakpoint actually does;
//   · where each drawing sits, how wide it is relative to its section,
//     and at what aspect ratio it is rendered;
//   · which colour family the page speaks in — terracotta, green, gold —
//     rather than which exact tone, because the tones were deliberately
//     moved and the families were not;
//   · that nothing makes the document scroll sideways.
//
// Everything here is measured in the browser on the built bytes, at the
// three widths, on both builds, and compared. A difference is a finding
// with a name, not a percentage.

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
const PORTS = { reference: 47331, port: 47332 };

const VIEWPORTS = [
  { name: "desktop", width: 1440, height: 900 },
  { name: "tablet", width: 834, height: 1112 },
  { name: "mobile", width: 390, height: 844 },
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

/**
 * How far a relative measurement may move and still be the same design.
 *
 * Three per cent of a section's own width or height. The type is one
 * point larger here, so a paragraph is a few pixels taller and every
 * block below it starts a few pixels lower; what must not change is
 * which fraction of the section a drawing occupies and where it sits
 * across it.
 */
const TOLERANCE = 0.03;

/**
 * The composition differences that are decisions, each with the decision
 * behind it.
 *
 * The same contract the textual gate keeps (F-73): a difference a rule
 * covers is recorded and counted, and a difference no rule covers fails
 * the run. A rule names a SHAPE and never a page — «anything on the
 * landing is fine» would hide the next real thing.
 */
const DIFFERENCES = [
  {
    id: "L-10",
    reason:
      "The install panel gained a copy button beside each command line, and the line became a scrolling box inside the chip so that the button stays put while the command scrolls. Both are the landing port's own addition — its string table marks `copyCommand` and `copied` as the port's rather than the owner's — and each adds one laid-out box the reference has no counterpart for. The rule fires only when the reference's sequence of boxes is still there in order, with boxes added and none lost or reordered.",
    where: "row shapes",
    matches: (observation) => observation.added && observation.subsequence,
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

// ---------------------------------------------------------------------
// The skeleton of a page, measured in the browser
// ---------------------------------------------------------------------

const skeletonInPage = () => {
  /** A colour, as the family it belongs to rather than as a tone. */
  const family = (css) => {
    const parts = /rgba?\(([^)]+)\)/.exec(css);
    if (parts === null) return "none";
    const [r, g, b, a = "1"] = parts[1].split(/[\s,/]+/).map(Number);
    if (Number(a) === 0) return "none";
    const max = Math.max(r, g, b);
    const min = Math.min(r, g, b);
    const light = (max + min) / 2 / 255;
    if (max - min < 24)
      return light > 0.6 ? "cream" : light > 0.3 ? "grey" : "ink";
    let hue;
    const d = max - min;
    if (max === r) hue = ((g - b) / d + (g < b ? 6 : 0)) * 60;
    else if (max === g) hue = ((b - r) / d + 2) * 60;
    else hue = ((r - g) / d + 4) * 60;
    if (hue < 45) return "terracotta";
    if (hue < 70) return "gold";
    if (hue < 190) return "green";
    if (hue < 260) return "slate";
    return "other";
  };

  /**
   * How many things stand across a row, and how many rows they make.
   *
   * Two children share a row when their vertical extents OVERLAP — not
   * when their tops round to the same number. The difference matters:
   * a marker with seven pixels of top margin beside its paragraph is one
   * row, and rounding the two tops to a grid puts them in the same row
   * or in two depending on where the pair happens to sit on the page.
   * That made the measurement a function of the scroll position rather
   * than of the layout, and reported a dozen differences that were the
   * quantisation and nothing else.
   */
  const rowsOf = (element) => {
    const boxes = [...element.children]
      .map((child) => child.getBoundingClientRect())
      .filter((box) => box.width > 0 && box.height > 0);
    if (boxes.length < 2) return null;
    const rows = [];
    for (const box of [...boxes].sort((a, b) => a.top - b.top)) {
      const row = rows.find((candidate) => {
        const overlap =
          Math.min(candidate.bottom, box.bottom) -
          Math.max(candidate.top, box.top);
        return (
          overlap > Math.min(candidate.bottom - candidate.top, box.height) / 2
        );
      });
      if (row === undefined) {
        rows.push({ top: box.top, bottom: box.bottom, count: 1 });
      } else {
        row.count += 1;
        row.bottom = Math.max(row.bottom, box.bottom);
      }
    }
    return {
      children: boxes.length,
      rows: rows.length,
      widest: Math.max(...rows.map((row) => row.count)),
    };
  };

  const main = document.querySelector("main") ?? document.body;
  const sections = [...main.querySelectorAll("section")].filter((section) => {
    const box = section.getBoundingClientRect();
    return box.width > 0 && box.height > 0;
  });

  const measured = sections.map((section) => {
    const box = section.getBoundingClientRect();
    const heading = section.querySelector("h1, h2, h3");
    const eyebrow = section.querySelector("p");

    /* Every laid-out container inside the section, in document order.
       The class names differ between the two builds — they are two
       stylesheets for one design — so the containers are matched by
       where they stand rather than by what they are called. */
    const layouts = [];
    for (const element of section.querySelectorAll("*")) {
      const display = getComputedStyle(element).display;
      /* The inline forms count too. A command chip that hugs its text is
         `inline-flex` and one that spans its column is `flex`, and that
         is a difference a reader sees — leaving the inline forms out of
         the scan hid it. */
      if (!/^(inline-)?(grid|flex)$/.test(display)) continue;
      const shape = rowsOf(element);
      if (shape === null) continue;
      layouts.push({ ...shape, inline: display.startsWith("inline-") });
    }

    const drawings = [...section.querySelectorAll("svg")]
      .map((svg) => {
        const svgBox = svg.getBoundingClientRect();
        if (svgBox.width < 24 || svgBox.height < 24) return null;
        return {
          viewBox: svg.getAttribute("viewBox") ?? "",
          /* Relative to the section, so the measurement survives a page
             that starts sixteen pixels higher. */
          left: (svgBox.left - box.left) / box.width,
          width: svgBox.width / box.width,
          aspect: svgBox.width / svgBox.height,
        };
      })
      .filter((one) => one !== null);

    return {
      heading: (heading?.textContent ?? "").replace(/\s+/g, " ").trim(),
      /* The fraction of the whole page this section occupies: a band
         that stopped being a band shows up here and nowhere else. */
      height: box.height / main.getBoundingClientRect().height,
      ground: family(getComputedStyle(section).backgroundColor),
      accent: family(getComputedStyle(eyebrow ?? section).color),
      layouts,
      drawings,
    };
  });

  return {
    sections: measured,
    overflow: document.documentElement.scrollWidth > window.innerWidth,
    scrollWidth: document.documentElement.scrollWidth,
    innerWidth: window.innerWidth,
  };
};

async function skeleton(page, origin, path) {
  await page.goto(`${origin}${path}`, { waitUntil: "load" });
  await page.evaluate(() => document.fonts.ready);
  return page.evaluate(skeletonInPage);
}

// ---------------------------------------------------------------------
// The comparison
// ---------------------------------------------------------------------

/** Is every item of `small`, in order, somewhere in `large`? */
function isSubsequence(small, large) {
  let at = 0;
  for (const item of small) {
    at = large.indexOf(item, at);
    if (at === -1) return false;
    at += 1;
  }
  return true;
}

function compare(pair, viewport, a, b, findings, explained) {
  const where = `${pair.port} @ ${viewport.name}`;
  const note = (text) => findings.push({ where, text });

  if (b.overflow) {
    note(
      `the document scrolls sideways: ${b.scrollWidth}px of content in a ${b.innerWidth}px window`,
    );
  }

  if (a.sections.length !== b.sections.length) {
    note(
      `${a.sections.length} section(s) in the reference, ${b.sections.length} here`,
    );
    return;
  }

  a.sections.forEach((left, index) => {
    const right = b.sections[index];
    const at = `section ${index + 1}${left.heading.length > 0 ? ` «${left.heading.slice(0, 40)}»` : ""}`;

    if (left.heading !== right.heading) {
      note(`${at}: heading «${left.heading}» became «${right.heading}»`);
    }
    if (left.ground !== right.ground) {
      note(
        `${at}: ground is ${left.ground} in the reference, ${right.ground} here`,
      );
    }
    if (left.accent !== right.accent) {
      note(
        `${at}: accent is ${left.accent} in the reference, ${right.accent} here`,
      );
    }
    if (Math.abs(left.height - right.height) > TOLERANCE) {
      note(
        `${at}: takes ${(left.height * 100).toFixed(1)}% of the page in the reference, ${(right.height * 100).toFixed(1)}% here`,
      );
    }

    /* The composition: how many things stand across a row. This is what
       a breakpoint does, and it is the measurement a stacked layout
       fails and a pixel count forgives. */
    const shape = (layout) =>
      `${layout.inline ? "i" : ""}${layout.children}/${layout.rows}/${layout.widest}`;
    const leftList = left.layouts.map(shape);
    const rightList = right.layouts.map(shape);
    if (leftList.join(" ") !== rightList.join(" ")) {
      const observation = {
        added: rightList.length > leftList.length,
        subsequence: isSubsequence(leftList, rightList),
      };
      const rule = DIFFERENCES.find(
        (candidate) =>
          candidate.where === "row shapes" && candidate.matches(observation),
      );
      const text = `${at}: row shapes (children/rows/widest) «${leftList.join(" ")}» became «${rightList.join(" ")}»`;
      if (rule === undefined) note(text);
      else explained.push({ id: rule.id, where, text });
    }

    if (left.drawings.length !== right.drawings.length) {
      note(
        `${at}: ${left.drawings.length} drawing(s) in the reference, ${right.drawings.length} here`,
      );
      return;
    }
    left.drawings.forEach((drawing, which) => {
      const other = right.drawings[which];
      if (drawing.viewBox !== other.viewBox) {
        note(
          `${at}: drawing ${which + 1} viewBox «${drawing.viewBox}» became «${other.viewBox}»`,
        );
      }
      if (Math.abs(drawing.left - other.left) > TOLERANCE) {
        note(
          `${at}: drawing ${which + 1} starts at ${(drawing.left * 100).toFixed(1)}% of the section in the reference, ${(other.left * 100).toFixed(1)}% here`,
        );
      }
      if (Math.abs(drawing.width - other.width) > TOLERANCE) {
        note(
          `${at}: drawing ${which + 1} spans ${(drawing.width * 100).toFixed(1)}% of the section in the reference, ${(other.width * 100).toFixed(1)}% here`,
        );
      }
      if (Math.abs(drawing.aspect - other.aspect) > 0.05 * drawing.aspect) {
        note(
          `${at}: drawing ${which + 1} is drawn at ${drawing.aspect.toFixed(3)} in the reference, ${other.aspect.toFixed(3)} here`,
        );
      }
    });
  });
}

// ---------------------------------------------------------------------

async function main() {
  const referenceRoot = process.argv[2];
  if (referenceRoot === undefined) {
    process.stderr.write(
      "layout-parity: usage — node tools/layout-parity.mjs <astro-dist> [qwik-dist]\n",
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

  const browser = await chromium.launch();
  const context = await browser.newContext({ deviceScaleFactor: 1 });
  const findings = [];
  const explained = [];
  let compared = 0;

  for (const pair of PAIRS) {
    for (const viewport of VIEWPORTS) {
      const page = await context.newPage();
      await page.setViewportSize({
        width: viewport.width,
        height: viewport.height,
      });
      const a = await skeleton(page, referenceOrigin, pair.reference);
      const b = await skeleton(page, portOrigin, pair.port);
      await page.close();
      compare(pair, viewport, a, b, findings, explained);
      compared += 1;
    }
  }

  /* The negative control. A gate that cannot go red is a green light
     with no lamp behind it, and this one compares structures that are
     meant to be identical — so the run that proves nothing moved looks
     exactly like the run where the measurement quietly stopped working.
     So it is asked, every time, to compare two pages that are NOT the
     same page, and the run fails if it finds them alike. */
  const control = await context.newPage();
  await control.setViewportSize({ width: 1440, height: 900 });
  const zap = await skeleton(control, portOrigin, "/why/zap/");
  const aiNative = await skeleton(control, portOrigin, "/why/ai-native/");
  await control.close();
  const controlFindings = [];
  compare(
    { port: "self-test" },
    { name: "desktop" },
    zap,
    aiNative,
    controlFindings,
    [],
  );

  await browser.close();
  for (const server of servers) server.close();

  const out = join(PACKAGE_ROOT, "tmp", "visual-parity");
  mkdirSync(out, { recursive: true });
  const lines = [
    "# Layout parity — the composition, at the three widths",
    "",
    `- reference: \`${referenceRoot}\``,
    `- this build: \`${portRoot}\``,
    `- ${compared} page/width comparison(s); relative measurements, ±${(TOLERANCE * 100).toFixed(0)}%`,
    "",
  ];
  if (findings.length === 0) {
    lines.push(
      "Every section stands in the same place, carries the same heading, keeps",
      "its ground and its accent family, puts the same number of things across",
      "a row at every width, and draws each figure at the same size in the same",
      "position. No page scrolls sideways.",
      "",
    );
  } else {
    for (const finding of findings) {
      lines.push(`- \`${finding.where}\` — ${finding.text}`);
    }
    lines.push("");
  }

  if (explained.length > 0) {
    lines.push("## Deliberate differences cited", "");
    for (const entry of explained) {
      lines.push(`- ${entry.id} \`${entry.where}\` — ${entry.text}`);
    }
    lines.push("");
    for (const rule of DIFFERENCES) {
      if (!explained.some((entry) => entry.id === rule.id)) continue;
      lines.push(`  ${rule.id}: ${rule.reason}`, "");
    }
  }

  const controlOk = controlFindings.length > 0;
  lines.push(
    controlOk
      ? `Negative control: comparing two different pages produces ${controlFindings.length} finding(s), so the measurement is awake.`
      : "Negative control: FAILED — comparing two different pages produced no finding at all, so this run's green means nothing.",
    "",
    findings.length === 0 && controlOk
      ? `layout parity: green — ${compared} comparison(s), ${explained.length} deliberate difference(s), 0 unexplained.`
      : `layout parity: RED — ${findings.length} unexplained finding(s) over ${compared} comparison(s).`,
    "",
  );
  const markdown = lines.join("\n");
  writeFileSync(join(out, "layout.md"), markdown, "utf8");
  process.stdout.write(`${markdown}\n`);
  process.exitCode = findings.length === 0 && controlOk ? 0 : 1;
}

await main();
