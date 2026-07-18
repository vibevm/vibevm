import { writeFileSync } from 'node:fs';

// Combined icons: a hero four-point sparkle (from vibeterm-4, at 372,160)
// layered onto four sky effects. Shares the geometry helpers of gen.mjs.

const CORAL = '#D97757';
const WHITE = '#EAE3D8';
const LCORAL = '#E8A085';
const SPARK = [372, 160]; // hero sparkle centre — the key element to keep clear

const open = (extraDefs = '') =>
  `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 512 512" fill="none">
<defs><linearGradient id="g" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="#26262E"/><stop offset="1" stop-color="#161619"/></linearGradient>${extraDefs}</defs>
<rect x="24" y="24" width="464" height="464" rx="104" fill="url(#g)"/>`;
const PROMPT = `<path d="M 160 190 L 252 250 L 160 310" fill="none" stroke="${CORAL}" stroke-width="30" stroke-linecap="round" stroke-linejoin="round"/>
<rect x="250" y="300" width="96" height="26" rx="8" fill="${CORAL}"/>`;

function rng(seed) {
  return function () {
    seed |= 0; seed = (seed + 0x6d2b79f5) | 0;
    let t = Math.imul(seed ^ (seed >>> 15), 1 | seed);
    t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t;
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}
const r2 = (n) => Math.round(n * 100) / 100;
const pt = (P, R, d) => { const t = (d * Math.PI) / 180; return [P[0] + R * Math.cos(t), P[1] + R * Math.sin(t)]; };
const inBand = ([x, y]) => x >= 46 && x <= 466 && y >= 50 && y <= 184;
const dist = (ax, ay, bx, by) => Math.hypot(ax - bx, ay - by);
const nearSpark = (x, y, r = 46) => dist(x, y, SPARK[0], SPARK[1]) < r;

function bandRun(P, R, step = 1.5) {
  let best = null, cur = null;
  for (let th = -180; th <= 180; th += step) {
    if (inBand(pt(P, R, th))) { if (!cur) cur = [th, th]; else cur[1] = th; }
    else if (cur) { if (!best || cur[1] - cur[0] > best[1] - best[0]) best = cur; cur = null; }
  }
  if (cur && (!best || cur[1] - cur[0] > best[1] - best[0])) best = cur;
  return best;
}
function arc(P, R, th1, th2, stroke, width, opacity) {
  const [sx, sy] = pt(P, R, th1); const [ex, ey] = pt(P, R, th2);
  const large = Math.abs(th2 - th1) > 180 ? 1 : 0;
  return `<path d="M ${r2(sx)} ${r2(sy)} A ${r2(R)} ${r2(R)} 0 ${large} 1 ${r2(ex)} ${r2(ey)}" stroke="${stroke}" stroke-width="${width}" stroke-linecap="round" opacity="${opacity}" fill="none"/>`;
}
const dot = (x, y, r, fill, o = 1) => `<circle cx="${r2(x)}" cy="${r2(y)}" r="${r}" fill="${fill}" opacity="${o}"/>`;

// hero four-point sparkle with concave edges (exactly vibeterm-4's geometry)
function sparkle(cx = SPARK[0], cy = SPARK[1], R = 46, q = 8, fill = CORAL) {
  const T = `${cx} ${cy - R}`, RR = `${cx + R} ${cy}`, B = `${cx} ${cy + R}`, L = `${cx - R} ${cy}`;
  const cTR = `${cx + q} ${cy - q}`, cRB = `${cx + q} ${cy + q}`, cBL = `${cx - q} ${cy + q}`, cLT = `${cx - q} ${cy - q}`;
  return `<path d="M ${T} Q ${cTR} ${RR} Q ${cRB} ${B} Q ${cBL} ${L} Q ${cLT} ${T} Z" fill="${fill}"/>`;
}

const wrap = (body, defs = '') => `${open(defs)}\n${body}\n${sparkle()}\n${PROMPT}\n</svg>\n`;
const out = {};

// ---- c1: big two-tone starfield + hero sparkle ----------------------------
{
  const stars = [
    [96, 96, 5, WHITE, 1], [142, 72, 3.4, CORAL, 0.9], [180, 112, 6.5, WHITE, 1],
    [150, 150, 3, LCORAL, 0.85], [228, 82, 4.4, WHITE, 0.95], [276, 120, 3.2, CORAL, 0.8],
    [320, 92, 7, WHITE, 1], [300, 158, 3, LCORAL, 0.8],
    [406, 88, 4, WHITE, 0.9], [440, 150, 5.5, WHITE, 1],
    [112, 140, 4, WHITE, 0.85], [250, 150, 3.2, WHITE, 0.8], [70, 150, 3.4, CORAL, 0.7],
  ].filter(([x, y]) => !nearSpark(x, y, 52));
  out['vibeterm-c1-bigstars-sparkle'] = wrap(stars.map((s) => dot(...s)).join('\n'));
}

// ---- c2: large coral stars + hero sparkle ---------------------------------
{
  const stars = [
    [104, 104, 11], [176, 84, 8], [250, 108, 10], [326, 82, 9],
    [148, 156, 6.5], [300, 150, 7], [72, 150, 6], [438, 150, 7.5],
  ].filter(([x, y]) => !nearSpark(x, y, 54));
  out['vibeterm-c2-coralstars-sparkle'] = wrap(stars.map(([x, y, r]) => dot(x, y, r, CORAL)).join('\n'));
}

// ---- faint sweeping-trail field (shared by c3a/c3b), pole far lower-left ---
function sweepFaint() {
  const P = [-260, 820];
  const rnd = rng(9);
  const parts = [];
  const radii = [];
  for (let R = 690; R <= 1030; R += 12 + Math.floor(rnd() * 14)) radii.push(R);
  radii.forEach((R) => {
    const run = bandRun(P, R); if (!run) return;
    const [a, b] = run; const span = b - a;
    const len = Math.min(span, 6 + rnd() * 16);
    const s = a + rnd() * (span - len); const e = s + len;
    const [ex, ey] = pt(P, R, e);
    const faint = 0.22 + rnd() * 0.2;
    parts.push(arc(P, R, s, e, CORAL, 2, r2(faint)));
    if (!nearSpark(ex, ey, 40)) parts.push(dot(ex, ey, 1.6, LCORAL, r2(faint + 0.25)));
  });
  return parts.join('\n');
}

// ---- c3a: sweep + sparkle, NO bright streaks ------------------------------
out['vibeterm-c3a-sweep-sparkle-dim'] = wrap(sweepFaint());

// ---- c3b: sweep + sparkle, bright streaks only in the clear top-centre -----
{
  const P = [-260, 820];
  // two short bright arcs, hand-placed high & centre: clear of the > arm
  // (whose slope they must not continue) and left of / above the sparkle.
  const bright = [];
  const heroes = [
    [890, -58.1, -52.0], // (210,64) -> (288,119)
    [920, -56.5, -52.0], // (248,53) -> (306,95)
  ];
  heroes.forEach(([R, t1, t2]) => {
    const [sx, sy] = pt(P, R, t1); const [ex, ey] = pt(P, R, t2);
    console.log(`hero R=${R}: (${r2(sx)},${r2(sy)}) -> (${r2(ex)},${r2(ey)})  distToSpark=${r2(dist(ex, ey, ...SPARK))}`);
    bright.push(arc(P, R, t1, t2, CORAL, 7, 0.2));
    bright.push(arc(P, R, t1, t2, WHITE, 3.4, 0.95));
    bright.push(dot(ex, ey, 3, '#FFFFFF', 1));
  });
  out['vibeterm-c3b-sweep-sparkle-bright'] = wrap(sweepFaint() + '\n' + bright.join('\n'));
}

// ---- c4: star-rain (dense top -> thin centre) + sparkle, clear pocket -------
{
  const DROP_DEF = `<linearGradient id="drop" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="${CORAL}" stop-opacity="0"/><stop offset="0.55" stop-color="${CORAL}" stop-opacity="0.35"/><stop offset="1" stop-color="${CORAL}" stop-opacity="0.95"/></linearGradient>`;
  function drop(x, headY, len, hot) {
    const w = hot ? 3.6 : 3; const top = headY - len;
    const rect = `<rect x="${r2(x - w / 2)}" y="${r2(top)}" width="${w}" height="${r2(len)}" rx="${w / 2}" fill="url(#drop)"/>`;
    const head = dot(x, headY, hot ? 3.2 : 2.6, hot ? '#FFFFFF' : CORAL, 1);
    const halo = hot ? dot(x, headY, 5, CORAL, 0.3) : '';
    return halo + rect + head;
  }
  const rnd = rng(14);
  const parts = [];
  for (let k = 0; k < 26; k++) {
    const x = 50 + rnd() * 412, y = 50 + rnd() * 120;
    if (!nearSpark(x, y, 40)) parts.push(dot(x, y, 1, WHITE, 0.2 + rnd() * 0.35));
  }
  const xs = []; let n = 0, guard = 0;
  while (n < 22 && guard < 400) {
    guard++;
    const x = 54 + rnd() * 404; const headY = 58 + rnd() * 118;
    const p = 1 - (headY - 58) / 118;
    if (rnd() > 0.15 + 0.85 * p) continue;
    if (xs.some((q) => Math.abs(q[0] - x) < 16 && Math.abs(q[1] - headY) < 26)) continue;
    const len = (18 + rnd() * 22) * (0.5 + 0.5 * p);
    // keep a clean pocket around the sparkle: skip drops whose head or body enters it
    if (nearSpark(x, headY, 46) || nearSpark(x, headY - len, 40)) continue;
    xs.push([x, headY]); n++;
    parts.push(drop(x, headY, len, rnd() < 0.2 && p > 0.5));
  }
  out['vibeterm-c4-starrain-sparkle'] = wrap(parts.join('\n'), DROP_DEF);
}

for (const [name, svg] of Object.entries(out)) {
  writeFileSync(new URL(`./${name}.svg`, import.meta.url), svg);
  console.log('wrote', name + '.svg');
}
