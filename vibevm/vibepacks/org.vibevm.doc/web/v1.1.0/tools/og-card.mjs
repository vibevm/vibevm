#!/usr/bin/env node
// `og.png` — the picture a link to this site unfurls into, drawn by the
// build rather than kept as a binary in the repository.
//
// The Astro site referenced `${SITE}/og.png` from four meta tags and the
// file did not exist anywhere in its tree (A0.26): every share of the
// landing for a year showed whatever the platform falls back to. The
// port keeps the four tags and finally answers them.
//
// What it draws is the site's own signature — the dependency graph from
// the hero, on the brand's warm near-black, under the same off-centre
// terracotta wash the page has. Not a word on it, and that is the
// decision: the card's words are `og:title` and `og:description`, which
// every platform renders beside the image in the reader's own type,
// while text baked into a picture cannot be selected, translated or
// read aloud. A drawing is what a picture is for.
//
// Every colour is read out of `design/palette.css`, so the card and the
// page cannot drift apart (R-27). No literal lives in this file.

import { deflateSync } from "node:zlib";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";

/** The card, at the size every platform crops from. */
const WIDTH = 1200;
const HEIGHT = 630;

/** Drawn at twice the size and averaged down: the cheapest antialiasing. */
const SUPERSAMPLE = 2;

/**
 * The constellation, in the coordinates the hero's SVG uses.
 *
 * Copied from `design/src/components/dep-graph/index.tsx` rather than
 * imported, because that file is JSX for a browser and this one is
 * arithmetic for a bitmap. They are the same drawing, and the parity
 * that matters is the one an eye checks.
 */
const GRAPH_VIEWBOX = { width: 420, height: 380 };

const EDGES = [
  [210, 196, 150, 120],
  [210, 196, 286, 128],
  [210, 196, 176, 300],
  [210, 196, 300, 256],
  [150, 120, 78, 72],
  [150, 120, 70, 196],
  [286, 128, 352, 80],
  [286, 128, 336, 196],
  [300, 256, 352, 322],
  [176, 300, 112, 330],
];

const NODES = [
  { x: 210, y: 196, r: 15, solid: true },
  { x: 150, y: 120, r: 9 },
  { x: 286, y: 128, r: 10 },
  { x: 176, y: 300, r: 8 },
  { x: 300, y: 256, r: 9 },
  { x: 78, y: 72, r: 6 },
  { x: 70, y: 196, r: 6 },
  { x: 352, y: 80, r: 7 },
  { x: 336, y: 196, r: 6 },
  { x: 352, y: 322, r: 6 },
  { x: 112, y: 330, r: 5 },
];

// ---------------------------------------------------------------------
// Colour, from the one file allowed to write it down
// ---------------------------------------------------------------------

function palette(packageRoot) {
  const css = readFileSync(join(packageRoot, "design", "palette.css"), "utf8");
  const read = (name) => {
    const found = new RegExp(`${name}\\s*:\\s*#([0-9a-fA-F]{6})\\s*;`).exec(
      css,
    );
    if (found === null) {
      throw new Error(`palette.css declares no six-digit ${name}`);
    }
    const hex = found[1];
    return [
      Number.parseInt(hex.slice(0, 2), 16),
      Number.parseInt(hex.slice(2, 4), 16),
      Number.parseInt(hex.slice(4, 6), 16),
    ];
  };
  return {
    ground: read("--ink"),
    surface: read("--ink-raise"),
    accent: read("--accent-raw"),
  };
}

// ---------------------------------------------------------------------
// A very small raster
// ---------------------------------------------------------------------

function canvas(width, height, background) {
  const pixels = new Uint8Array(width * height * 3);
  for (let index = 0; index < width * height; index += 1) {
    pixels[index * 3] = background[0];
    pixels[index * 3 + 1] = background[1];
    pixels[index * 3 + 2] = background[2];
  }
  return { width, height, pixels };
}

/** Paint one pixel, mixing `colour` in at `alpha`. */
function blend(target, x, y, colour, alpha) {
  if (alpha <= 0) return;
  const px = Math.round(x);
  const py = Math.round(y);
  if (px < 0 || py < 0 || px >= target.width || py >= target.height) return;
  const at = (py * target.width + px) * 3;
  const weight = Math.min(1, alpha);
  for (let channel = 0; channel < 3; channel += 1) {
    const was = target.pixels[at + channel];
    target.pixels[at + channel] = Math.round(
      was + (colour[channel] - was) * weight,
    );
  }
}

/**
 * The ambient wash: two soft radial glows, the same two the page paints
 * behind itself, at the same strengths.
 */
function wash(target, accent) {
  const glows = [
    { x: 0.78, y: 0.08, rx: 0.7, ry: 0.55, alpha: 0.1 },
    { x: 0.06, y: 1.0, rx: 0.6, ry: 0.5, alpha: 0.05 },
  ];
  for (const glow of glows) {
    const cx = glow.x * target.width;
    const cy = glow.y * target.height;
    const rx = glow.rx * target.width;
    const ry = glow.ry * target.height;
    for (let y = 0; y < target.height; y += 1) {
      for (let x = 0; x < target.width; x += 1) {
        const dx = (x - cx) / rx;
        const dy = (y - cy) / ry;
        const distance = Math.sqrt(dx * dx + dy * dy);
        if (distance >= 1) continue;
        blend(target, x, y, accent, glow.alpha * (1 - distance) ** 1.6);
      }
    }
  }
}

function line(target, from, to, colour, width, alpha) {
  const dx = to[0] - from[0];
  const dy = to[1] - from[1];
  const steps = Math.ceil(Math.hypot(dx, dy) * 2);
  const half = width / 2;
  for (let step = 0; step <= steps; step += 1) {
    const t = step / steps;
    const x = from[0] + dx * t;
    const y = from[1] + dy * t;
    for (let ox = -half; ox <= half; ox += 0.5) {
      for (let oy = -half; oy <= half; oy += 0.5) {
        if (Math.hypot(ox, oy) > half) continue;
        blend(target, x + ox, y + oy, colour, alpha);
      }
    }
  }
}

function disc(target, cx, cy, radius, colour, alpha) {
  for (let y = Math.floor(cy - radius); y <= Math.ceil(cy + radius); y += 1) {
    for (let x = Math.floor(cx - radius); x <= Math.ceil(cx + radius); x += 1) {
      if (Math.hypot(x - cx, y - cy) > radius) continue;
      blend(target, x, y, colour, alpha);
    }
  }
}

function ring(target, cx, cy, radius, width, colour, alpha) {
  const outer = radius + width / 2;
  const inner = radius - width / 2;
  for (let y = Math.floor(cy - outer); y <= Math.ceil(cy + outer); y += 1) {
    for (let x = Math.floor(cx - outer); x <= Math.ceil(cx + outer); x += 1) {
      const distance = Math.hypot(x - cx, y - cy);
      if (distance > outer || distance < inner) continue;
      blend(target, x, y, colour, alpha);
    }
  }
}

/** Average each 2×2 block down to one pixel. */
function downscale(source, factor) {
  const width = source.width / factor;
  const height = source.height / factor;
  const pixels = new Uint8Array(width * height * 3);
  for (let y = 0; y < height; y += 1) {
    for (let x = 0; x < width; x += 1) {
      for (let channel = 0; channel < 3; channel += 1) {
        let total = 0;
        for (let sy = 0; sy < factor; sy += 1) {
          for (let sx = 0; sx < factor; sx += 1) {
            const at =
              ((y * factor + sy) * source.width + (x * factor + sx)) * 3 +
              channel;
            total += source.pixels[at];
          }
        }
        pixels[(y * width + x) * 3 + channel] = Math.round(
          total / (factor * factor),
        );
      }
    }
  }
  return { width, height, pixels };
}

// ---------------------------------------------------------------------
// PNG
// ---------------------------------------------------------------------

const CRC_TABLE = (() => {
  const table = new Uint32Array(256);
  for (let n = 0; n < 256; n += 1) {
    let c = n;
    for (let k = 0; k < 8; k += 1) {
      c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1;
    }
    table[n] = c >>> 0;
  }
  return table;
})();

function crc32(bytes) {
  let crc = 0xffffffff;
  for (const byte of bytes) {
    crc = CRC_TABLE[(crc ^ byte) & 0xff] ^ (crc >>> 8);
  }
  return (crc ^ 0xffffffff) >>> 0;
}

function chunk(type, data) {
  const length = Buffer.alloc(4);
  length.writeUInt32BE(data.length, 0);
  const body = Buffer.concat([Buffer.from(type, "ascii"), data]);
  const crc = Buffer.alloc(4);
  crc.writeUInt32BE(crc32(body), 0);
  return Buffer.concat([length, body, crc]);
}

function encodePng(image) {
  const header = Buffer.alloc(13);
  header.writeUInt32BE(image.width, 0);
  header.writeUInt32BE(image.height, 4);
  header[8] = 8; // bit depth
  header[9] = 2; // truecolour
  header[10] = 0; // deflate
  header[11] = 0; // adaptive filtering
  header[12] = 0; // no interlace

  const stride = image.width * 3;
  const raw = Buffer.alloc((stride + 1) * image.height);
  for (let y = 0; y < image.height; y += 1) {
    raw[y * (stride + 1)] = 0; // filter: none
    raw.set(
      image.pixels.subarray(y * stride, (y + 1) * stride),
      y * (stride + 1) + 1,
    );
  }

  return Buffer.concat([
    Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]),
    chunk("IHDR", header),
    chunk("IDAT", deflateSync(raw, { level: 9 })),
    chunk("IEND", Buffer.alloc(0)),
  ]);
}

// ---------------------------------------------------------------------

/** Draw the card and write it; returns the file name for the build log. */
export function writeOgCard(file, packageRoot) {
  const colours = palette(packageRoot);
  const target = canvas(
    WIDTH * SUPERSAMPLE,
    HEIGHT * SUPERSAMPLE,
    colours.ground,
  );
  wash(target, colours.accent);

  /* The graph fills the card's height with a margin, centred. */
  const scale = ((HEIGHT * 0.74) / GRAPH_VIEWBOX.height) * SUPERSAMPLE;
  const offsetX =
    (target.width - GRAPH_VIEWBOX.width * scale) / 2 - 4 * SUPERSAMPLE;
  const offsetY = (target.height - GRAPH_VIEWBOX.height * scale) / 2;
  const at = (x, y) => [offsetX + x * scale, offsetY + y * scale];

  for (const [x1, y1, x2, y2] of EDGES) {
    line(target, at(x1, y1), at(x2, y2), colours.accent, 1.25 * scale, 0.32);
  }
  for (const node of NODES) {
    const [cx, cy] = at(node.x, node.y);
    disc(
      target,
      cx,
      cy,
      node.r * scale,
      node.solid === true ? colours.accent : colours.surface,
      1,
    );
    ring(target, cx, cy, node.r * scale, 1.5 * scale, colours.accent, 1);
  }
  /* The root's resting pulse, caught mid-breath. */
  const [rootX, rootY] = at(NODES[0].x, NODES[0].y);
  ring(
    target,
    rootX,
    rootY,
    NODES[0].r * scale * 2.1,
    1.5 * scale,
    colours.accent,
    0.22,
  );

  writeFileSync(file, encodePng(downscale(target, SUPERSAMPLE)));
  return "og.png";
}
