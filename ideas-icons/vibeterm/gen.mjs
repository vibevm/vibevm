import { writeFileSync } from 'node:fs';

// ---- shared pieces ---------------------------------------------------------
const CORAL = '#D97757';
const WHITE = '#EAE3D8';
const GOLD = '#E7C98B';
const BLUE = '#9FB6D6';
const LCORAL = '#E8A085';

const open = (extraDefs = '') =>
  `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 512 512" fill="none">
<defs><linearGradient id="g" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="#26262E"/><stop offset="1" stop-color="#161619"/></linearGradient>${extraDefs}</defs>
<rect x="24" y="24" width="464" height="464" rx="104" fill="url(#g)"/>`;

// the >_ prompt, always drawn last so sky sits behind it
const PROMPT = `<path d="M 160 190 L 252 250 L 160 310" fill="none" stroke="${CORAL}" stroke-width="30" stroke-linecap="round" stroke-linejoin="round"/>
<rect x="250" y="300" width="96" height="26" rx="8" fill="${CORAL}"/>`;
const close = (body) => `${open()}\n${body}\n${PROMPT}\n</svg>\n`;

// deterministic PRNG (mulberry32) so re-runs are stable
function rng(seed) {
  return function () {
    seed |= 0; seed = (seed + 0x6d2b79f5) | 0;
    let t = Math.imul(seed ^ (seed >>> 15), 1 | seed);
    t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t;
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}

const r2 = (n) => Math.round(n * 100) / 100;
const pt = (P, R, thDeg) => {
  const t = (thDeg * Math.PI) / 180;
  return [P[0] + R * Math.cos(t), P[1] + R * Math.sin(t)];
};
// keep sky strictly above the prompt (chevron top is y=190)
const inBand = ([x, y]) => x >= 46 && x <= 466 && y >= 50 && y <= 184;

// longest contiguous in-band angular run for a circle of radius R around P
function bandRun(P, R, step = 1.5) {
  let best = null, cur = null;
  for (let th = -180; th <= 180; th += step) {
    if (inBand(pt(P, R, th))) {
      if (!cur) cur = [th, th]; else cur[1] = th;
    } else if (cur) { if (!best || cur[1] - cur[0] > best[1] - best[0]) best = cur; cur = null; }
  }
  if (cur && (!best || cur[1] - cur[0] > best[1] - best[0])) best = cur;
  return best; // [thStart, thEnd] or null
}

function arc(P, R, th1, th2, stroke, width, opacity) {
  const [sx, sy] = pt(P, R, th1);
  const [ex, ey] = pt(P, R, th2);
  const large = Math.abs(th2 - th1) > 180 ? 1 : 0;
  return `<path d="M ${r2(sx)} ${r2(sy)} A ${r2(R)} ${r2(R)} 0 ${large} 1 ${r2(ex)} ${r2(ey)}" stroke="${stroke}" stroke-width="${width}" stroke-linecap="round" opacity="${opacity}" fill="none"/>`;
}
const dot = (x, y, r, fill, opacity = 1) =>
  `<circle cx="${r2(x)}" cy="${r2(y)}" r="${r}" fill="${fill}" opacity="${opacity}"/>`;

const out = {};

// ---- 7: same starfield, larger & brighter ---------------------------------
{
  const stars = [
    [96, 96, 5, WHITE, 1], [142, 72, 3.4, CORAL, 0.9], [180, 112, 6.5, WHITE, 1],
    [150, 150, 3, LCORAL, 0.85], [228, 82, 4.4, WHITE, 0.95], [276, 120, 3.2, CORAL, 0.8],
    [320, 92, 7, WHITE, 1], [300, 158, 3, LCORAL, 0.8], [360, 128, 5, CORAL, 0.95],
    [406, 88, 4, WHITE, 0.9], [440, 150, 5.5, WHITE, 1], [392, 176, 3, LCORAL, 0.75],
    [112, 140, 4, WHITE, 0.85], [250, 150, 3.2, WHITE, 0.8], [70, 150, 3.4, CORAL, 0.7],
  ];
  out['vibeterm-7-bigstars'] = close(stars.map((s) => dot(...s)).join('\n'));
}

// ---- 8: large coral stars (echo the original dots), all coral -------------
{
  const stars = [
    [104, 104, 11], [176, 84, 8], [250, 108, 10], [326, 82, 9],
    [400, 112, 8], [148, 156, 6.5], [300, 150, 7], [392, 168, 6.5],
    [72, 150, 6], [438, 150, 7.5],
  ];
  out['vibeterm-8-coralstars'] = close(stars.map(([x, y, r]) => dot(x, y, r, CORAL)).join('\n'));
}

// ---- 9: sweeping trails, pole far off lower-left (#2 vibe) -----------------
{
  const P = [-260, 820];
  const rnd = rng(9);
  const parts = [];
  const radii = [];
  for (let R = 690; R <= 1030; R += 12 + Math.floor(rnd() * 14)) radii.push(R);
  radii.forEach((R) => {
    const run = bandRun(P, R);
    if (!run) return;
    const [a, b] = run;
    // shorten each trail to a fraction of the visible run, placed randomly in it
    const span = b - a;
    const len = Math.min(span, 6 + rnd() * 16);
    const s = a + rnd() * (span - len);
    const e = s + len;
    const faint = 0.22 + rnd() * 0.2;
    parts.push(arc(P, R, s, e, CORAL, 2, r2(faint)));
    const [ex, ey] = pt(P, R, e);
    parts.push(dot(ex, ey, 1.6, LCORAL, r2(faint + 0.25)));
  });
  // two bright hero streaks
  [[905, 8, 34], [770, 4, 26]].forEach(([R, s0, ln], i) => {
    const run = bandRun(P, R); if (!run) return;
    const [a, b] = run; const span = b - a;
    const len = Math.min(span - 2, ln); const s = a + Math.min(s0, span - len - 1);
    const e = s + len;
    parts.push(arc(P, R, s, e, CORAL, 8, 0.2));
    parts.push(arc(P, R, s, e, WHITE, 4, 0.95));
    const [ex, ey] = pt(P, R, e);
    parts.push(dot(ex, ey, 3.4, '#FFFFFF', 1));
  });
  out['vibeterm-9-trails-sweep'] = close(parts.join('\n'));
}

// ---- 10: compact circumpolar swirl, in-frame pole (#3 vibe) ----------------
{
  const P = [312, 98];
  const rnd = rng(10);
  const parts = [];
  [14, 26, 39, 53, 68, 82].forEach((R) => {
    // one long arc with a gap, clipped to band, plus a bright leading dot
    const run = bandRun(P, R);
    if (!run) return;
    let [a, b] = run;
    // leave a small gap so it reads as a trail, not a ring
    const gap = 18 + rnd() * 22;
    b = Math.max(a + 8, b - gap);
    const col = rnd() < 0.35 ? WHITE : CORAL;
    parts.push(arc(P, R, a, b, col, 2.4, r2(0.4 + rnd() * 0.35)));
    const [ex, ey] = pt(P, R, b);
    parts.push(dot(ex, ey, 2.4, col === WHITE ? '#FFFFFF' : LCORAL, 0.95));
  });
  parts.push(dot(P[0], P[1], 3.2, '#FFFFFF', 1)); // the pole star
  out['vibeterm-10-trails-circle'] = close(parts.join('\n'));
}

// ---- 11: dense multicolour concentric arcs around in-frame pole (#4 vibe) --
{
  const P = [338, 84];
  const rnd = rng(11);
  const palette = [CORAL, WHITE, GOLD, BLUE, CORAL, WHITE];
  const parts = [];
  let i = 0;
  for (let R = 16; R <= 180; R += 8 + rnd() * 4) {
    const run = bandRun(P, R);
    if (!run) { i++; continue; }
    let [a, b] = run;
    const span = b - a;
    // trim ends a little for an airy, trail-like feel
    a += span * (0.04 + rnd() * 0.12);
    b -= span * (0.04 + rnd() * 0.12);
    const col = palette[i % palette.length];
    const op = 0.28 + rnd() * 0.4;
    parts.push(arc(P, R, a, b, col, R < 60 ? 2.4 : 2, r2(op)));
    if (rnd() < 0.5) { const [ex, ey] = pt(P, R, b); parts.push(dot(ex, ey, 1.8, col, r2(op + 0.2))); }
    i++;
  }
  parts.push(dot(P[0], P[1], 2.6, '#FFFFFF', 1));
  out['vibeterm-11-trails-dense'] = close(parts.join('\n'));
}

// ---- 12: coral dots in place of the macOS lights (same spot, just coral) ---
{
  const body = [dot(332, 160, 13, CORAL), dot(372, 160, 13, CORAL), dot(412, 160, 13, CORAL)].join('\n');
  out['vibeterm-12-coral-lights'] = close(body);
}

// ---- star-rain drop helper -------------------------------------------------
const DROP_DEF = `<linearGradient id="drop" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="${CORAL}" stop-opacity="0"/><stop offset="0.55" stop-color="${CORAL}" stop-opacity="0.35"/><stop offset="1" stop-color="${CORAL}" stop-opacity="0.95"/></linearGradient>`;
function drop(x, headY, len, hot) {
  const w = hot ? 3.6 : 3;
  const top = headY - len;
  const rect = `<rect x="${r2(x - w / 2)}" y="${r2(top)}" width="${w}" height="${r2(len)}" rx="${w / 2}" fill="url(#drop)"/>`;
  const head = dot(x, headY, hot ? 3.2 : 2.6, hot ? '#FFFFFF' : CORAL, 1);
  const halo = hot ? dot(x, headY, 5, CORAL, 0.3) : '';
  return halo + rect + head;
}

// ---- 13: star rain, uniform (#5 vibe) -------------------------------------
{
  const rnd = rng(13);
  const parts = [];
  // faint background sparkle
  for (let k = 0; k < 22; k++) parts.push(dot(50 + rnd() * 412, 52 + rnd() * 130, 1, WHITE, 0.25 + rnd() * 0.35));
  const xs = [];
  let n = 0, guard = 0;
  while (n < 15 && guard < 600) {
    guard++;
    const x = 56 + rnd() * 400;
    if (xs.some((p) => Math.abs(p - x) < 20)) continue;
    xs.push(x); n++;
    const headY = 66 + rnd() * 108;
    const len = 22 + rnd() * 26;
    parts.push(drop(x, headY, len, rnd() < 0.22));
  }
  out['vibeterm-13-starrain'] = `${open(DROP_DEF)}\n${parts.join('\n')}\n${PROMPT}\n</svg>\n`;
}

// ---- 14: star rain, dense at top → thin toward centre ----------------------
{
  const rnd = rng(14);
  const parts = [];
  for (let k = 0; k < 26; k++) parts.push(dot(50 + rnd() * 412, 50 + rnd() * 120, 1, WHITE, 0.2 + rnd() * 0.35));
  const xs = [];
  let n = 0, guard = 0;
  while (n < 22 && guard < 400) {
    guard++;
    const x = 54 + rnd() * 404;
    const headY = 58 + rnd() * 118; // 58..176
    // keep probability high near the top, low toward the centre
    const p = 1 - (headY - 58) / 118; // 1 at top → 0 near centre
    if (rnd() > 0.15 + 0.85 * p) continue;
    if (xs.some((q) => Math.abs(q[0] - x) < 16 && Math.abs(q[1] - headY) < 26)) continue;
    xs.push([x, headY]); n++;
    const len = (18 + rnd() * 22) * (0.5 + 0.5 * p); // longer at top
    parts.push(drop(x, headY, len, rnd() < 0.2 && p > 0.5));
  }
  out['vibeterm-14-starrain-fade'] = `${open(DROP_DEF)}\n${parts.join('\n')}\n${PROMPT}\n</svg>\n`;
}

for (const [name, svg] of Object.entries(out)) {
  writeFileSync(new URL(`./${name}.svg`, import.meta.url), svg);
  console.log('wrote', name + '.svg');
}
