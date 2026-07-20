// CDP smoke for split view + tear-off. Usage: node scripts/cdp-split-smoke.mjs <port> <out-prefix>
// Drives the shell's bridge over CDP: opens tabs, splits a pane, closes a pane, tears a tab into a
// new window -- asserting the ModelView after each step. Uses the global WebSocket (node >= 22).
import { writeFileSync } from 'node:fs';

const port = Number(process.argv[2] || 9231);
const outPrefix = process.argv[3] || '/tmp/vibeterm';
const base = `http://127.0.0.1:${port}`;

const list = await (await fetch(`${base}/json/list`)).json();
const t = list.find((x) => x.url.includes('dist/chrome/index.html'));
if (!t) {
  console.error('no chrome target at', base);
  process.exit(1);
}
const ws = new WebSocket(t.webSocketDebuggerUrl);
await new Promise((r, e) => {
  ws.addEventListener('open', r);
  ws.addEventListener('error', e);
});
let id = 1;
const p = new Map();
ws.addEventListener('message', (ev) => {
  const m = JSON.parse(typeof ev.data === 'string' ? ev.data : ev.data.toString());
  if (m.id && p.has(m.id)) {
    const { resolve, reject } = p.get(m.id);
    p.delete(m.id);
    if (m.error) reject(new Error(JSON.stringify(m.error)));
    else resolve(m.result);
  }
});
const send = (m, pr = {}) =>
  new Promise((res, rej) => {
    const i = id++;
    p.set(i, { resolve: res, reject: rej });
    ws.send(JSON.stringify({ id: i, method: m, params: pr }));
  });
const evalJS = async (x) =>
  (await send('Runtime.evaluate', { expression: x, returnByValue: true, awaitPromise: true })).result
    ?.value;
const cmd = async (c) =>
  evalJS(`(async()=>JSON.stringify(await window.vibeterm.command(${JSON.stringify(c)})))()`);
const state = async () =>
  JSON.parse(
    await evalJS(
      `(async()=>{ const v=await window.vibeterm.state(); return JSON.stringify({wins:v.windows.map(w=>({id:w.id,tabs:w.tabs,panes:w.panes})), panes:[...v.panes.entries()].map(([id,p])=>({id,tabId:p.tabId,slot:p.slot})), activeTab:v.activeTab, activeWindow:v.activeWindow}); })()`,
    ),
  );
const shot = async (f) => {
  const r = await send('Page.captureScreenshot', { format: 'png' });
  writeFileSync(f, Buffer.from(r.data, 'base64'));
  console.log('shot ->', f);
};
const wait = (ms) => new Promise((r) => setTimeout(r, ms));

await send('Runtime.enable');
await send('Page.enable');

await cmd({ t: 'open' });
await wait(400);
let s = await state();
console.log('after open#1: windows=', s.wins.length, 'w0.tabs=', s.wins[0].tabs, 'w0.panes=', s.wins[0].panes);

const t1 = s.wins[0].tabs[0];
console.log('pane.split t1 ->', await cmd({ t: 'pane.split', tabId: t1, dir: 'right' }));
await wait(400);
s = await state();
console.log('after split: w0.panes =', s.wins[0].panes, '| panes:', JSON.stringify(s.panes));
await shot(`${outPrefix}-split.png`);

const pane2 = s.wins[0].panes[1];
console.log('pane.close', pane2, '->', await cmd({ t: 'pane.close', paneId: pane2 }));
await wait(300);
s = await state();
console.log('after pane.close: w0.panes =', s.wins[0].panes);

console.log('tear-off t1 ->', await cmd({ t: 'tab.move-to-window', tabId: t1, windowId: 'new' }));
await wait(400);
s = await state();
console.log(
  'after tear-off: windows=',
  s.wins.length,
  'w0.tabs=',
  s.wins[0]?.tabs,
  'w1.tabs=',
  s.wins[1]?.tabs,
  'activeWindow=',
  s.activeWindow,
);
await shot(`${outPrefix}-tearoff.png`);

ws.close();
process.exit(0);
