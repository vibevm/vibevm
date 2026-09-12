#!/usr/bin/env node
// contrast.mjs — the eighth step of this package's floor: both themes are
// measured for readable contrast, and the run is red when a gated pair
// falls short.
//
// The formula is APCA (Accessible Perceptual Contrast Algorithm), the
// published 0.1.9 / 0.0.98G-4g form, implemented here rather than
// installed: the reference package `apca-w3` is AGPL and does not enter
// this tree (PROP-057 `##STACK-DESIGN-FLOOR`). The constants below are
// the published ones and are kept in one object so a reader can compare
// them against the specification line by line.
//
// What the number means. APCA is polarity-aware and asymmetric: the same
// two colours score differently depending on which one is the ink. A
// positive Lc is dark text on a light background, a negative Lc is light
// text on a dark one, and only the magnitude is compared to a threshold.
// The thresholds are by ROLE, because a caption and a paragraph are not
// held to the same standard:
//
//   --text                 |Lc| >= 75   the text a page is read for
//   --text-2, --text-3     |Lc| >= 60   captions, meta lines, eyebrows
//   --accent, --accent-hover |Lc| >= 45 interactive outlines on the page
//
// Separators (`--line`, `--line-strong`) and the paragraph numbers at
// half opacity are decorative — they carry no information a reader must
// read — so they are measured and printed but never gated. Printing them
// anyway is the point: a decorative tone that collapses to Lc 0 is worth
// knowing about even when nothing fails.
//
// Two more things this step proves, both cheap and both load-bearing:
// that the media-query dark map and the `[data-theme="dark"]` dark map
// are the same map, and that `tokens.css` resolves entirely through
// `palette.css` with no literal of its own.
//
// Usage:
//   node design/audit/contrast.mjs            gate, exit 1 on a failure
//   node design/audit/contrast.mjs --mint     print the minimal same-hue
//                                             replacement for every gated
//                                             pair that falls short

import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const DESIGN_ROOT = dirname(dirname(fileURLToPath(import.meta.url)));
const MINT = process.argv.includes("--mint");

/* ------------------------------------------------------------------ */
/* APCA 0.1.9 (0.0.98G-4g)                                            */
/* ------------------------------------------------------------------ */

const SA98G = {
  mainTRC: 2.4,
  sRco: 0.2126729,
  sGco: 0.7151522,
  sBco: 0.072175,
  normBG: 0.56,
  normTXT: 0.57,
  revTXT: 0.62,
  revBG: 0.65,
  blkThrs: 0.022,
  blkClmp: 1.414,
  scaleBoW: 1.14,
  scaleWoB: 1.14,
  loBoWoffset: 0.027,
  loWoBoffset: 0.027,
  deltaYmin: 0.0005,
  loClip: 0.1,
};

/** Screen luminance Y of an opaque sRGB triple, 0..1. */
function sRGBtoY([r, g, b]) {
  const lin = (c) => Math.pow(c / 255.0, SA98G.mainTRC);
  return SA98G.sRco * lin(r) + SA98G.sGco * lin(g) + SA98G.sBco * lin(b);
}

/** Lightness contrast of text over background, in Lc (-108 .. 106). */
function apcaContrast(txtY, bgY) {
  if (Math.min(txtY, bgY) < 0 || Math.max(txtY, bgY) > 1.1) return 0.0;
  const tY = txtY > SA98G.blkThrs ? txtY : txtY + Math.pow(SA98G.blkThrs - txtY, SA98G.blkClmp);
  const bY = bgY > SA98G.blkThrs ? bgY : bgY + Math.pow(SA98G.blkThrs - bgY, SA98G.blkClmp);
  if (Math.abs(bY - tY) < SA98G.deltaYmin) return 0.0;
  let sapc;
  let out;
  if (bY > tY) {
    sapc = (Math.pow(bY, SA98G.normBG) - Math.pow(tY, SA98G.normTXT)) * SA98G.scaleBoW;
    out = sapc < SA98G.loClip ? 0.0 : sapc - SA98G.loBoWoffset;
  } else {
    sapc = (Math.pow(bY, SA98G.revBG) - Math.pow(tY, SA98G.revTXT)) * SA98G.scaleWoB;
    out = sapc > -SA98G.loClip ? 0.0 : sapc + SA98G.loWoBoffset;
  }
  return out * 100.0;
}

/** Composite a translucent colour over an opaque one, sRGB, straight alpha. */
function alphaBlend([r, g, b, a], [br, bg, bb]) {
  const f = Math.max(0, Math.min(1, a));
  return [
    Math.round(r * f + br * (1 - f)),
    Math.round(g * f + bg * (1 - f)),
    Math.round(b * f + bb * (1 - f)),
  ];
}

/* ------------------------------------------------------------------ */
/* Colour parsing — only the two notations the palette is allowed      */
/* ------------------------------------------------------------------ */

/** `#rgb`, `#rrggbb`, `rgb(...)`, `rgba(...)` -> [r, g, b, a]. */
function parseColour(text) {
  const value = text.trim();
  const hex = /^#([0-9a-f]{3}|[0-9a-f]{6})$/i.exec(value);
  if (hex !== null) {
    const digits = hex[1];
    const wide = digits.length === 3 ? [...digits].map((c) => c + c).join("") : digits;
    return [
      parseInt(wide.slice(0, 2), 16),
      parseInt(wide.slice(2, 4), 16),
      parseInt(wide.slice(4, 6), 16),
      1,
    ];
  }
  const fn = /^rgba?\(([^)]*)\)$/i.exec(value);
  if (fn !== null) {
    const parts = fn[1].split(/[\s,/]+/).filter((p) => p.length > 0);
    if (parts.length < 3) return null;
    const chan = parts.slice(0, 3).map((p) => Number.parseFloat(p));
    const alpha = parts.length > 3 ? Number.parseFloat(parts[3]) : 1;
    if (chan.some((c) => Number.isNaN(c)) || Number.isNaN(alpha)) return null;
    return [chan[0], chan[1], chan[2], alpha];
  }
  return null;
}

function toHex([r, g, b]) {
  const two = (c) => Math.round(c).toString(16).padStart(2, "0");
  return `#${two(r)}${two(g)}${two(b)}`;
}

/* HSL is the minting space: it moves lightness and leaves hue and
   saturation exactly where the source put them, which is the whole
   requirement — the minted tone must still be the same colour. */

function rgbToHsl([r, g, b]) {
  const rn = r / 255;
  const gn = g / 255;
  const bn = b / 255;
  const max = Math.max(rn, gn, bn);
  const min = Math.min(rn, gn, bn);
  const l = (max + min) / 2;
  if (max === min) return [0, 0, l];
  const d = max - min;
  const s = l > 0.5 ? d / (2 - max - min) : d / (max + min);
  let h;
  if (max === rn) h = (gn - bn) / d + (gn < bn ? 6 : 0);
  else if (max === gn) h = (bn - rn) / d + 2;
  else h = (rn - gn) / d + 4;
  return [h / 6, s, l];
}

function hslToRgb([h, s, l]) {
  if (s === 0) {
    const v = Math.round(l * 255);
    return [v, v, v];
  }
  const q = l < 0.5 ? l * (1 + s) : l + s - l * s;
  const p = 2 * l - q;
  const channel = (t) => {
    let tt = t;
    if (tt < 0) tt += 1;
    if (tt > 1) tt -= 1;
    if (tt < 1 / 6) return p + (q - p) * 6 * tt;
    if (tt < 1 / 2) return q;
    if (tt < 2 / 3) return p + (q - p) * (2 / 3 - tt) * 6;
    return p;
  };
  return [
    Math.round(channel(h + 1 / 3) * 255),
    Math.round(channel(h) * 255),
    Math.round(channel(h - 1 / 3) * 255),
  ];
}

/* ------------------------------------------------------------------ */
/* Reading the two files                                               */
/* ------------------------------------------------------------------ */

function stripComments(css) {
  return css.replace(/\r\n/g, "\n").replace(/\/\*[\s\S]*?\*\//g, "");
}

/** Every `--name: value;` declaration of one brace-delimited body. */
function declarations(body) {
  const out = new Map();
  for (const m of body.matchAll(/(--[\w-]+)\s*:\s*([^;]+);/g)) {
    out.set(m[1], m[2].trim());
  }
  return out;
}

/**
 * The body of the first block whose selector matches `pattern`, brace
 * matched so a nested block cannot end the search early. A pattern
 * rather than a literal because a selector list may be wrapped over
 * lines, and the file's line endings are not the audit's business.
 */
function blockAfter(css, pattern) {
  const found = pattern.exec(css);
  if (found === null) return null;
  const at = found.index;
  const open = css.indexOf("{", at + found[0].length - 1);
  if (open < 0) return null;
  let depth = 0;
  for (let i = open; i < css.length; i += 1) {
    if (css[i] === "{") depth += 1;
    else if (css[i] === "}") {
      depth -= 1;
      if (depth === 0) return css.slice(open + 1, i);
    }
  }
  return null;
}

const paletteCss = stripComments(readFileSync(join(DESIGN_ROOT, "palette.css"), "utf8"));
const tokensCss = stripComments(readFileSync(join(DESIGN_ROOT, "tokens.css"), "utf8"));

const palette = declarations(blockAfter(paletteCss, /:root\s*\{/) ?? "");

const lightBody = blockAfter(tokensCss, /:root\s*,\s*\[data-theme="light"\]\s*\{/);
const mediaOuter = blockAfter(tokensCss, /@media\s*\(prefers-color-scheme:\s*dark\)\s*\{/);
const mediaBody =
  mediaOuter === null ? null : blockAfter(mediaOuter, /:root:not\(\[data-theme="light"\]\)\s*\{/);
const attrBody = blockAfter(tokensCss, /(?:^|\n)\[data-theme="dark"\]\s*\{/);

if (lightBody === null || mediaBody === null || attrBody === null) {
  process.stderr.write("contrast: tokens.css does not carry the three theme blocks.\n");
  process.exit(1);
}

const light = declarations(lightBody);
const darkMedia = declarations(mediaBody);
const darkAttr = declarations(attrBody);

/** Resolve a token through `var()` chains down to a literal colour. */
function resolve(map, name, seen = new Set()) {
  if (seen.has(name)) return null;
  seen.add(name);
  const raw = map.get(name) ?? palette.get(name);
  if (raw === undefined) return null;
  const ref = /^var\(\s*(--[\w-]+)\s*\)$/.exec(raw);
  if (ref !== null) return resolve(map, ref[1], seen);
  return parseColour(raw);
}

/* ------------------------------------------------------------------ */
/* The matrix                                                          */
/* ------------------------------------------------------------------ */

const BACKGROUNDS = ["--bg", "--bg-raise", "--bg-sink", "--code-bg", "--selection", "--accent-soft"];
const GATED_TEXT = [
  ["--text", 75],
  ["--text-2", 60],
  ["--text-3", 60],
];
const GATED_OUTLINE = [
  ["--accent", 45],
  ["--accent-hover", 45],
];
const DECORATIVE = ["--line", "--line-strong"];

function grade(abs, min) {
  if (abs >= min + 15) return "EXCELLENT";
  if (abs >= min + 5) return "GOOD";
  if (abs >= min) return "PASS";
  if (abs >= min - 10) return "MARGINAL";
  return "FAIL";
}

/** Lc of `ink` over `paper`, both resolved, `paper` flattened if translucent. */
function pairLc(map, inkName, paperName) {
  const ink = resolve(map, inkName);
  const paper = resolve(map, paperName);
  const page = resolve(map, "--bg");
  if (ink === null || paper === null || page === null) return null;
  const solidPaper = paper[3] < 1 ? alphaBlend(paper, page) : paper.slice(0, 3);
  const solidInk = ink[3] < 1 ? alphaBlend(ink, solidPaper) : ink.slice(0, 3);
  return apcaContrast(sRGBtoY(solidInk), sRGBtoY(solidPaper));
}

const themes = [
  ["light", light],
  ["dark", darkAttr],
];

let gatedPairs = 0;
let referencePairs = 0;
const failures = [];
const lines = [];

for (const [themeName, map] of themes) {
  lines.push(`\n--- ${themeName} ---`);
  for (const [ink, min] of [...GATED_TEXT, ...GATED_OUTLINE]) {
    const papers = GATED_TEXT.some(([n]) => n === ink) ? BACKGROUNDS : ["--bg"];
    for (const paper of papers) {
      const lc = pairLc(map, ink, paper);
      if (lc === null) {
        failures.push({ theme: themeName, ink, paper, lc: 0, min, missing: true });
        continue;
      }
      gatedPairs += 1;
      const abs = Math.abs(lc);
      const verdict = grade(abs, min);
      lines.push(
        `  ${ink.padEnd(14)} on ${paper.padEnd(14)} Lc=${lc.toFixed(2).padStart(7)}  min=${String(min).padStart(3)}  ${verdict}`,
      );
      if (abs < min) failures.push({ theme: themeName, ink, paper, lc, min });
    }
  }
  for (const ink of DECORATIVE) {
    const lc = pairLc(map, ink, "--bg");
    referencePairs += 1;
    lines.push(
      `  ${ink.padEnd(14)} on ${"--bg".padEnd(14)} Lc=${(lc ?? 0).toFixed(2).padStart(7)}  decorative, not gated`,
    );
  }
  const number = resolve(map, "--text-3");
  const page = resolve(map, "--bg");
  if (number !== null && page !== null) {
    const halved = alphaBlend([number[0], number[1], number[2], 0.5], page.slice(0, 3));
    const lc = apcaContrast(sRGBtoY(halved), sRGBtoY(page.slice(0, 3)));
    referencePairs += 1;
    lines.push(
      `  ${"block number".padEnd(14)} on ${"--bg".padEnd(14)} Lc=${lc.toFixed(2).padStart(7)}  decorative, not gated`,
    );
  }
}

/* ------------------------------------------------------------------ */
/* The two structural checks                                           */
/* ------------------------------------------------------------------ */

const divergences = [];
for (const key of new Set([...darkMedia.keys(), ...darkAttr.keys()])) {
  const a = darkMedia.get(key);
  const b = darkAttr.get(key);
  if (a !== b) divergences.push(`${key}: media=${a ?? "(absent)"} attr=${b ?? "(absent)"}`);
}

const literals = [];
for (const [scope, map] of [
  ["light", light],
  ["dark(media)", darkMedia],
  ["dark(attr)", darkAttr],
]) {
  for (const [name, value] of map) {
    if (!/^var\(\s*--[\w-]+\s*\)$/.test(value)) literals.push(`${scope} ${name}: ${value}`);
  }
}

/* ------------------------------------------------------------------ */
/* Minting                                                             */
/* ------------------------------------------------------------------ */

/**
 * The smallest lightness move on the same hue that clears `min + 2` on
 * every background the role is used over. Direction is the theme's:
 * light-theme ink darkens, dark-theme ink lightens.
 */
function mint(map, ink, min, papers, darker) {
  const start = resolve(map, ink);
  if (start === null) return null;
  const [h, s, l0] = rgbToHsl(start.slice(0, 3));
  const target = min + 2;
  for (let step = 0; step <= 1000; step += 1) {
    const l = darker ? l0 - step / 1000 : l0 + step / 1000;
    if (l < 0 || l > 1) return null;
    const rgb = hslToRgb([h, s, l]);
    const worst = Math.min(
      ...papers.map((paper) => {
        const p = resolve(map, paper);
        const page = resolve(map, "--bg");
        if (p === null || page === null) return 0;
        const solid = p[3] < 1 ? alphaBlend(p, page) : p.slice(0, 3);
        return Math.abs(apcaContrast(sRGBtoY(rgb), sRGBtoY(solid)));
      }),
    );
    if (worst >= target) return { hex: toHex(rgb), lc: worst, steps: step };
  }
  return null;
}

/* ------------------------------------------------------------------ */
/* Report                                                              */
/* ------------------------------------------------------------------ */

process.stdout.write("APCA audit — both themes, thresholds by role\n");
process.stdout.write(lines.join("\n"));
process.stdout.write("\n\n");

if (MINT) {
  process.stdout.write("--- minting candidates (minimal same-hue lightness shift) ---\n");
  const seen = new Set();
  for (const f of failures) {
    const key = `${f.theme}/${f.ink}`;
    if (seen.has(key)) continue;
    seen.add(key);
    const map = f.theme === "light" ? light : darkAttr;
    const papers = GATED_TEXT.some(([n]) => n === f.ink) ? BACKGROUNDS : ["--bg"];
    const from = resolve(map, f.ink);
    const candidate = mint(map, f.ink, f.min, papers, f.theme === "light");
    const worstBefore = Math.min(...papers.map((p) => Math.abs(pairLc(map, f.ink, p) ?? 0)));
    if (candidate === null) {
      process.stdout.write(`  ${f.theme} ${f.ink}: no candidate on this hue\n`);
      continue;
    }
    process.stdout.write(
      `  ${f.theme} ${f.ink}: ${toHex(from ?? [0, 0, 0])} -> ${candidate.hex}  ` +
        `worst |Lc| ${worstBefore.toFixed(2)} -> ${candidate.lc.toFixed(2)} ` +
        `(min ${f.min}, +2 margin, ${candidate.steps} steps of 0.1% L)\n`,
    );
  }
  process.stdout.write("\n");
}

process.stdout.write(
  `=== pairs: gated=${gatedPairs}, reference=${referencePairs}; below threshold=${failures.length} ===\n`,
);
process.stdout.write(
  divergences.length === 0
    ? "@media(prefers-color-scheme:dark) agrees with [data-theme=\"dark\"]: OK (0 divergences)\n"
    : `@media(prefers-color-scheme:dark) differs from [data-theme="dark"]:\n  ${divergences.join("\n  ")}\n`,
);
process.stdout.write(
  literals.length === 0
    ? "tokens.css writes no colour of its own: OK (every value is a var() into palette.css)\n"
    : `tokens.css carries literals, which belong in palette.css:\n  ${literals.join("\n  ")}\n`,
);

const red = failures.length > 0 || divergences.length > 0 || literals.length > 0;
if (red && !MINT) {
  process.stderr.write("contrast: the design floor is red — see the pairs above.\n");
}
process.exit(red && !MINT ? 1 : 0);
