// CDP smoke for the vibeterm shell. Usage: node scripts/cdp-smoke.mjs <port> <out-prefix>
// Attaches to the shell's chrome CDP target, drives the bridge (state/command), and captures
// screenshots. Uses the global WebSocket (node >= 22). The shell must be running with
// --cdp-port <port> (offscreen --headless is fine).
import { writeFileSync } from 'node:fs';

const port = Number(process.argv[2] || 9223);
const outPrefix = process.argv[3] || '/tmp/vibeterm';
const base = `http://127.0.0.1:${port}`;

const list = await (await fetch(`${base}/json/list`)).json();
const target = list.find((t) => t.type === 'page' && t.url.includes('dist/chrome/index.html'));
if (!target) {
  console.error('no chrome target at', base);
  process.exit(1);
}
console.log('chrome target:', target.url);

const ws = new WebSocket(target.webSocketDebuggerUrl);
await new Promise((resolve, reject) => {
  ws.addEventListener('open', resolve);
  ws.addEventListener('error', reject);
});

let nextId = 1;
const pending = new Map();
ws.addEventListener('message', (ev) => {
  const msg = JSON.parse(typeof ev.data === 'string' ? ev.data : ev.data.toString());
  if (msg.id && pending.has(msg.id)) {
    const { resolve, reject } = pending.get(msg.id);
    pending.delete(msg.id);
    if (msg.error) reject(new Error(JSON.stringify(msg.error)));
    else resolve(msg.result);
  }
});

function send(method, params = {}) {
  const id = nextId++;
  return new Promise((resolve, reject) => {
    pending.set(id, { resolve, reject });
    ws.send(JSON.stringify({ id, method, params }));
  });
}

async function evalJS(expr) {
  const r = await send('Runtime.evaluate', {
    expression: expr,
    returnByValue: true,
    awaitPromise: true,
  });
  return r.result?.value;
}

async function screenshot(file) {
  const r = await send('Page.captureScreenshot', { format: 'png' });
  writeFileSync(file, Buffer.from(r.data, 'base64'));
  console.log('screenshot ->', file, `(${r.data.length} b64 chars)`);
}

await send('Runtime.enable');
await send('Page.enable');

const initial = await evalJS(
  `(async () => { try { return JSON.stringify(await window.vibeterm.state()); } catch (e) { return 'ERR ' + e.message; } })()`,
);
console.log('initial state:', initial);
await screenshot(`${outPrefix}-1-initial.png`);

// open three tabs in turn
for (let i = 1; i <= 3; i++) {
  console.log(`open#${i} ->`, await evalJS(
    `(async () => { try { return JSON.stringify(await window.vibeterm.command({ t: 'open' })); } catch (e) { return 'ERR ' + e.message; } })()`,
  ));
  await new Promise((r) => setTimeout(r, 600));
}
await screenshot(`${outPrefix}-2-three-tabs.png`);

// select the FIRST tab (the list's top item) and confirm the ModelView's activeTab moved
const beforeSelect = await evalJS(
  `(async () => { try { return JSON.stringify(await window.vibeterm.state()); } catch (e) { return 'ERR ' + e.message; } })()`,
);
const firstTab = JSON.parse(beforeSelect).windows[0].tabs[0];
console.log('select first tab:', firstTab, '->', await evalJS(
  `(async () => { try { return JSON.stringify(await window.vibeterm.command({ t: 'select', tabId: ${JSON.stringify(firstTab)} })); } catch (e) { return 'ERR ' + e.message; } })()`,
));
await new Promise((r) => setTimeout(r, 500));
const afterSelect = await evalJS(
  `(async () => { try { return JSON.stringify(await window.vibeterm.state()); } catch (e) { return 'ERR ' + e.message; } })()`,
);
console.log('active after select:', JSON.parse(afterSelect).activeTab);
await screenshot(`${outPrefix}-3-selected-first.png`);

ws.close();
process.exit(0);
