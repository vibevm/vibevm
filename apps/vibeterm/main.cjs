/**
 * vibeterm — Electron main process.
 *
 * node-pty runs HERE, in the main process: the renderer has no
 * worker_threads, so constructing a pty there throws
 * "Failed to construct 'Worker'". PTY bytes flow main -> renderer over
 * `webContents.send('pty', …)`; keystrokes flow renderer -> main over the
 * `input` channel and into `pty.write`.
 *
 * With `--control`, main additionally stands up a loopback HTTP+JSON control
 * server (the "AIUI" surface) so an agent can drive and observe the session:
 * the same PTY bytes are mirrored into a headless xterm to expose a text
 * snapshot of the grid, symbolic key names are translated to PTY escape
 * sequences, and liveness is reported. Without `--control` none of that is
 * constructed — `@xterm/headless` is not even required — and vibeterm behaves
 * exactly as a plain terminal.
 */

'use strict';

const path = require('node:path');
const http = require('node:http');
const crypto = require('node:crypto');
const os = require('node:os');
const fs = require('node:fs');
const { app, BrowserWindow, ipcMain } = require('electron');
const pty = require('node-pty');

// A text grid needs no GPU compositing, and hardware acceleration is a
// frequent source of blank-window / driver faults on headless or remote boxes.
app.disableHardwareAcceleration();

// Chrome DevTools Protocol (CDP): in `--control` mode, open a remote-debugging
// endpoint so an external agent (vibe-cli via `chromiumoxide`) can attach to
// the live page and read its REAL state — the xterm grid's cols/cell metrics,
// the DOM layout, the scrollbar box — straight from the runtime, with no
// screenshot OCR. The loopback port is chosen by the launcher (Rust, which can
// bind synchronously) and passed as `--cdp-port`; we publish it in the
// discovery file. `app.commandLine.appendSwitch` MUST run before
// `app.whenReady()`, so the flags are read from `process.argv` here directly.
let cdpPort = 0;
{
  const idx = process.argv.indexOf('--cdp-port');
  const v = idx >= 0 ? Number(process.argv[idx + 1]) : NaN;
  if (Number.isInteger(v) && v > 0 && v < 65536 && process.argv.includes('--control')) {
    cdpPort = v;
    app.commandLine.appendSwitch('remote-debugging-port', String(v));
    app.commandLine.appendSwitch('remote-allow-origins', '*');
  }
}

// Backstop: never let a stray throw surface Electron's error dialog.
process.on('uncaughtException', (err) => {
  console.error('[vibeterm] uncaughtException:', err);
});

/** @type {import('electron').BrowserWindow | null} */
let win = null;
/** @type {import('node-pty').IPty | null} */
let ptyProc = null;
/** @type {import('node-pty').IDisposable | null} */
let ptyDataSub = null;
let quitting = false;

// --- control-mode (AIUI) state; all null/zero unless `--control` is passed ---
/** The loopback control server. @type {import('node:http').Server | null} */
let controlServer = null;
/** Headless xterm mirror of the pty, read for text snapshots. @type {any} */
let headless = null;
/** Bearer token guarding every control endpoint. @type {string | null} */
let controlToken = null;
/** Discovery files to remove on quit. @type {{pidFile:string, latestFile:string} | null} */
let discovery = null;
/** key-name → pty escape sequence (lib/keymap.mjs); set in control mode. */
let keyToSeq = null;
/**
 * Live-preview hooks (set in `createWindow`): `stopPty` tears the hosted
 * program down without touching Electron (freeing its binary for a rebuild);
 * `startPty` spawns it again at the last grid size. Both stay `null` until the
 * window is up.
 */
let stopPty = null;
let startPty = null;
/** Epoch ms of the last pty byte; drives POST /wait. */
let lastDataAt = 0;
/**
 * True once the pty has emitted its first byte. POST /wait must not report the
 * terminal "idle" before the hosted program has rendered anything — otherwise a
 * key sent right after `open` (the control server is up and discovery is written
 * before the pty even spawns) races a not-yet-started TUI and is dropped.
 */
let sawData = false;
/**
 * Epoch ms of the last POST /input write (0 if none). POST /wait treats the
 * terminal as settled only once output has arrived *after* this — so a wait
 * issued right after a keypress does not return on the stale pre-key idle
 * window, before the program has reacted and redrawn.
 */
let lastInputAt = 0;
/** The child's exit code once it has exited, else null; drives GET /state. */
let ptyExitCode = null;
/** Current pty / grid size, tracked for GET /state and the headless mirror. */
let curCols = 0;
let curRows = 0;

/**
 * Dispose the pty cleanly. Idempotent, safe from any teardown path.
 *
 * The onData subscription is disposed FIRST so no byte can be sent to a
 * destroyed window. The child is then stopped inside a try/catch: on Windows
 * `pty.kill()` throws ConPTY "AttachConsole failed" outside a real console —
 * swallowed here, and `app.quit()` reaps the child regardless.
 */
function disposePty() {
  if (ptyDataSub) {
    try {
      ptyDataSub.dispose();
    } catch {
      /* already disposed */
    }
    ptyDataSub = null;
  }
  if (ptyProc) {
    try {
      ptyProc.kill();
    } catch {
      /* ConPTY on Windows, or the child already exited */
    }
    ptyProc = null;
  }
}

/**
 * Tear down the control server: force-close sockets, stop listening, dispose
 * the headless mirror, and remove the discovery files. Idempotent.
 */
function disposeControl() {
  if (controlServer) {
    try {
      // Node ≥18.2: drop idle keep-alive sockets so close() completes promptly.
      controlServer.closeAllConnections?.();
    } catch {
      /* older runtime */
    }
    try {
      controlServer.close();
    } catch {
      /* not listening */
    }
    controlServer = null;
  }
  if (headless) {
    try {
      headless.dispose();
    } catch {
      /* already disposed */
    }
    headless = null;
  }
  removeDiscoveryFiles();
}

/** Tear down once and quit. */
function quit() {
  if (quitting) return;
  quitting = true;
  disposeControl();
  disposePty();
  app.quit();
}

// ---------------------------------------------------------------------------
// Control server (AIUI) — only constructed when `--control` is passed.
// ---------------------------------------------------------------------------

/** The discovery directory: `<home>/.vibevm/aiui`. */
function aiuiDir() {
  return path.join(os.homedir(), '.vibevm', 'aiui');
}

/**
 * Write the `<pid>.json` discovery file and the `latest.json` pointer, both
 * carrying `{ port, token, pid, startedAt }`.
 */
function writeDiscovery(port) {
  const dir = aiuiDir();
  fs.mkdirSync(dir, { recursive: true, mode: 0o700 });
  const info = {
    port,
    token: controlToken,
    pid: process.pid,
    startedAt: Date.now(),
  };
  if (cdpPort > 0) info.cdpPort = cdpPort;
  const json = JSON.stringify(info, null, 2);
  const pidFile = path.join(dir, `${process.pid}.json`);
  const latestFile = path.join(dir, 'latest.json');
  // 0o600: the token is a secret. Mode is honoured on POSIX, ignored on Windows.
  fs.writeFileSync(pidFile, json, { mode: 0o600 });
  fs.writeFileSync(latestFile, json, { mode: 0o600 });
  discovery = { pidFile, latestFile };
}

/**
 * Remove this pid's discovery file, and `latest.json` too but only while it
 * still points at this pid (a newer vibeterm may have overwritten it).
 */
function removeDiscoveryFiles() {
  if (!discovery) return;
  const { pidFile, latestFile } = discovery;
  discovery = null;
  try {
    fs.rmSync(pidFile, { force: true });
  } catch {
    /* already gone */
  }
  try {
    const parsed = JSON.parse(fs.readFileSync(latestFile, 'utf8'));
    if (parsed && parsed.pid === process.pid) {
      fs.rmSync(latestFile, { force: true });
    }
  } catch {
    /* latest.json absent, unreadable, or not ours — leave it */
  }
}

/** Write a JSON response with an explicit status and Content-Length. */
function sendJson(res, status, obj) {
  const body = JSON.stringify(obj);
  res.writeHead(status, {
    'Content-Type': 'application/json; charset=utf-8',
    'Content-Length': Buffer.byteLength(body),
  });
  res.end(body);
}

/** Constant-time `Authorization: Bearer <token>` check. */
function authorized(req) {
  const header = req.headers.authorization;
  if (typeof header !== 'string' || controlToken === null) return false;
  const got = Buffer.from(header);
  const want = Buffer.from(`Bearer ${controlToken}`);
  return got.length === want.length && crypto.timingSafeEqual(got, want);
}

/** Read and JSON-parse a request body (empty body → {}), capped at 1 MiB. */
function readJsonBody(req) {
  return new Promise((resolve, reject) => {
    const chunks = [];
    let size = 0;
    req.on('data', (c) => {
      size += c.length;
      if (size > 1 << 20) {
        reject(new Error('request body too large'));
        req.destroy();
        return;
      }
      chunks.push(c);
    });
    req.on('end', () => {
      const raw = Buffer.concat(chunks).toString('utf8').trim();
      if (raw === '') {
        resolve({});
        return;
      }
      try {
        resolve(JSON.parse(raw));
      } catch {
        reject(new Error('invalid JSON body'));
      }
    });
    req.on('error', reject);
  });
}

/** A finite number ≥ 0 (else `fallback`), clamped to `[0, max]`. */
function clampMs(value, fallback, max) {
  const n =
    typeof value === 'number' && Number.isFinite(value) && value >= 0
      ? value
      : fallback;
  return Math.min(n, max);
}

/**
 * Read the visible grid from the headless mirror: one right-trimmed line per
 * row, offset by `baseY` so scrollback is excluded and the current viewport
 * (or the alternate screen a TUI draws on, where baseY is 0) is returned.
 */
function snapshotNow() {
  const buf = headless.buffer.active;
  const rows = headless.rows;
  const cols = headless.cols;
  const base = buf.baseY;
  const lines = [];
  for (let y = 0; y < rows; y++) {
    const line = buf.getLine(base + y);
    lines.push(line ? line.translateToString(true) : '');
  }
  return { cols, rows, text: lines.join('\n') };
}

/**
 * Flush any queued headless writes, then snapshot. xterm parses writes off the
 * event loop, so an empty write's callback fires once the queue is drained; a
 * 50 ms safety timer guarantees the request never hangs.
 */
function readSnapshot() {
  return new Promise((resolve) => {
    let settled = false;
    const done = () => {
      if (settled) return;
      settled = true;
      resolve(snapshotNow());
    };
    try {
      headless.write('', done);
    } catch {
      done();
      return;
    }
    setTimeout(done, 50);
  });
}

/**
 * Resolve once the pty has emitted no bytes for `idleMs`, or `timeoutMs`
 * elapses. `stable` is true iff the idle window was reached before the timeout.
 */
function waitStable(idleMs, timeoutMs) {
  const start = Date.now();
  return new Promise((resolve) => {
    const tick = () => {
      const now = Date.now();
      const sinceData = now - lastDataAt;
      const waited = now - start;
      // "Idle" only counts once the program has rendered at least once (so a
      // wait before the pty starts does not return early) AND any keypress since
      // the last settle has been answered by output (so a wait right after a key
      // does not snapshot the screen before the program reacts).
      if (sawData && lastDataAt >= lastInputAt && sinceData >= idleMs) {
        resolve({ stable: true, waitedMs: waited });
        return;
      }
      if (waited >= timeoutMs) {
        resolve({ stable: false, waitedMs: waited });
        return;
      }
      // Once actually counting down the idle window, sleep until it would close
      // or the timeout hits, whichever is sooner — no busy-polling. While still
      // waiting for the first byte or for a keypress to be answered, the idle
      // math is meaningless, so poll on a short fixed interval instead.
      const settling = sawData && lastDataAt >= lastInputAt;
      const delay = settling
        ? Math.max(1, Math.min(idleMs - sinceData, timeoutMs - waited))
        : Math.min(25, Math.max(1, timeoutMs - waited));
      setTimeout(tick, delay);
    };
    tick();
  });
}

/** Route one authenticated request. Every endpoint requires a valid token. */
async function handleControlRequest(req, res) {
  if (!authorized(req)) {
    sendJson(res, 401, { error: 'unauthorized' });
    return;
  }

  const url = new URL(req.url || '/', 'http://127.0.0.1');
  const route = `${req.method} ${url.pathname}`;

  switch (route) {
    case 'GET /state': {
      sendJson(res, 200, {
        alive: ptyProc !== null,
        cols: curCols,
        rows: curRows,
        exitCode: ptyExitCode,
      });
      return;
    }

    case 'GET /snapshot': {
      const format = url.searchParams.get('format') ?? 'text';
      if (format !== 'text') {
        sendJson(res, 400, { error: 'unsupported_format', format });
        return;
      }
      if (!headless) {
        sendJson(res, 503, { error: 'unavailable' });
        return;
      }
      sendJson(res, 200, await readSnapshot());
      return;
    }

    case 'POST /capture': {
      let body;
      try {
        body = await readJsonBody(req);
      } catch {
        sendJson(res, 400, { error: 'invalid_body' });
        return;
      }
      const savePath = typeof body.path === 'string' ? body.path : null;
      if (!savePath) {
        sendJson(res, 400, { error: 'path_required' });
        return;
      }
      if (!win || win.isDestroyed()) {
        sendJson(res, 503, { error: 'no_window' });
        return;
      }
      try {
        const image = await win.webContents.capturePage();
        const png = image.toPNG();
        const { width, height } = image.getSize();
        fs.writeFileSync(savePath, png);
        sendJson(res, 200, { ok: true, path: savePath, width, height, bytes: png.length });
      } catch (err) {
        sendJson(res, 500, { error: 'capture_failed', detail: String(err) });
      }
      return;
    }

    case 'POST /resize': {
      let body;
      try {
        body = await readJsonBody(req);
      } catch {
        sendJson(res, 400, { error: 'invalid_body' });
        return;
      }
      const w = Number(body.width);
      const h = Number(body.height);
      if (!(w > 0 && h > 0)) {
        sendJson(res, 400, { error: 'width_height_required' });
        return;
      }
      if (!win || win.isDestroyed()) {
        sendJson(res, 503, { error: 'no_window' });
        return;
      }
      // Resize the window in CSS pixels; the renderer's ResizeObserver refits the
      // grid and reports back, and applySize resizes the pty + mirror — exactly
      // the fluid reflow a mouse drag triggers, but driven for tests/automation.
      win.setContentSize(Math.round(w), Math.round(h));
      sendJson(res, 200, { ok: true, width: Math.round(w), height: Math.round(h) });
      return;
    }

    case 'POST /input': {
      let body;
      try {
        body = await readJsonBody(req);
      } catch {
        sendJson(res, 400, { error: 'invalid_body' });
        return;
      }
      const keys = body.keys ?? [];
      if (!Array.isArray(keys)) {
        sendJson(res, 400, { error: 'keys_must_be_array' });
        return;
      }
      // Translate every key up front, so an unknown key writes nothing at all.
      // On Windows this yields win32-input-mode records; elsewhere, VT bytes.
      const seqs = [];
      for (const name of keys) {
        try {
          seqs.push(keyToSeq(name, process.platform));
        } catch {
          sendJson(res, 400, { error: 'unknown_key', key: String(name) });
          return;
        }
      }
      let wrote = false;
      try {
        for (const seq of seqs) {
          if (ptyProc) {
            ptyProc.write(seq);
            wrote = true;
          }
        }
        if (typeof body.text === 'string' && ptyProc) {
          ptyProc.write(body.text);
          wrote = true;
        }
      } catch {
        /* pty gone mid-write */
      }
      // Stamp the input so a following POST /wait blocks until the program has
      // reacted (produced output after this write) and then settled — not on the
      // stale idle window from before the key.
      if (wrote) lastInputAt = Date.now();
      sendJson(res, 200, { ok: true });
      return;
    }

    case 'POST /wait': {
      let body;
      try {
        body = await readJsonBody(req);
      } catch {
        sendJson(res, 400, { error: 'invalid_body' });
        return;
      }
      const idleMs = clampMs(body.idleMs, 120, 60000);
      const timeoutMs = clampMs(body.timeoutMs, 3000, 600000);
      sendJson(res, 200, await waitStable(idleMs, timeoutMs));
      return;
    }

    case 'POST /close': {
      sendJson(res, 200, { ok: true });
      // Tear down only after the response is flushed to the socket.
      res.on('finish', () => quit());
      return;
    }

    case 'POST /pty-stop': {
      // Live preview: stop the hosted program only (NOT Electron) so its binary
      // is free to rebuild. The renderer + CDP endpoint + discovery stay live.
      if (stopPty) stopPty();
      sendJson(res, 200, { ok: true });
      return;
    }
    case 'POST /pty-start': {
      // Live preview: (re)spawn the hosted program at the current grid. Pair
      // with /pty-stop around a rebuild for a fast TUI preview loop.
      if (startPty) startPty();
      sendJson(res, 200, { ok: true, cols: curCols, rows: curRows });
      return;
    }

    case 'POST /scrollbar': {
      // Flip the scrollbar policy live: `auto` (bar hidden for a full-screen
      // TUI, shown for a shell), `on` (always), `off` (never). The renderer
      // hides/shows the bar and refits the grid; no Electron restart.
      let body;
      try {
        body = await readJsonBody(req);
      } catch {
        sendJson(res, 400, { error: 'invalid_body' });
        return;
      }
      const mode =
        body && ['auto', 'on', 'off'].includes(body.mode) ? body.mode : null;
      if (!mode) {
        sendJson(res, 400, { error: 'mode_must_be_auto_on_off' });
        return;
      }
      if (!win || win.isDestroyed()) {
        sendJson(res, 503, { error: 'no_window' });
        return;
      }
      try {
        const ok = await win.webContents.executeJavaScript(
          `window.setScrollbarMode && window.setScrollbarMode(${JSON.stringify(mode)})`,
        );
        sendJson(res, 200, { ok: !!ok, mode });
      } catch (err) {
        sendJson(res, 500, { error: 'scrollbar_failed', detail: String(err) });
      }
      return;
    }

    default:
      sendJson(res, 404, { error: 'not_found', route });
  }
}

/**
 * Construct the headless mirror + token and start listening on an ephemeral
 * loopback port, then publish the discovery files.
 *
 * `@xterm/headless` is required LAZILY here so a plain (non `--control`) launch
 * never touches it — vibeterm runs even when that dependency is absent.
 */
function startControlServer(initialCols, initialRows) {
  const { Terminal } = require('@xterm/headless');
  headless = new Terminal({
    cols: initialCols,
    rows: initialRows,
    allowProposedApi: true,
  });
  controlToken = crypto.randomBytes(24).toString('hex');
  lastDataAt = Date.now();

  const server = http.createServer((req, res) => {
    req.on('error', () => {});
    res.on('error', () => {});
    Promise.resolve()
      .then(() => handleControlRequest(req, res))
      .catch((err) => {
        console.error('[vibeterm] control request error:', err);
        try {
          if (!res.headersSent) sendJson(res, 500, { error: 'internal' });
          else res.end();
        } catch {
          /* response already gone */
        }
      });
  });
  server.on('error', (err) => {
    console.error('[vibeterm] control server error:', err);
  });
  controlServer = server;

  server.listen(0, '127.0.0.1', () => {
    const addr = server.address();
    const port = addr && typeof addr === 'object' ? addr.port : 0;
    try {
      writeDiscovery(port);
    } catch (err) {
      console.error('[vibeterm] failed to write discovery file:', err);
    }
    console.error(`[vibeterm] AIUI control server on http://127.0.0.1:${port}`);
  });
}

// Resolve the app-family window icon (PROP-043 #icon): `--icon <name>` picks
// `resources/icon-<name>.<ext>` (ext = ico on Windows, png elsewhere), falling
// back to the default `resources/icon.<ext>` when no name is given or the named
// file is missing — so an unknown name never leaves the window icon-less.
function resolveIconPath(name) {
  const ext = process.platform === 'win32' ? 'ico' : 'png';
  const dir = path.join(__dirname, 'resources');
  if (typeof name === 'string' && name.trim() !== '') {
    const named = path.join(dir, `icon-${name}.${ext}`);
    if (fs.existsSync(named)) return named;
  }
  return path.join(dir, `icon.${ext}`);
}

async function createWindow() {
  // Pure, dependency-free logic — the same module the unit tests import.
  const { parseArgs, defaultShell, splitCommand } = await import('./lib/args.mjs');

  // `headless: hideWindow` renames on destructure so it does not shadow the
  // module-level `headless` (the @xterm/headless mirror).
  const { exec, cols, rows, control, headless: hideWindow, icon: iconName } =
    parseArgs(process.argv);
  const commandLine = exec ?? defaultShell(process.platform, process.env);
  const { file, args } = splitCommand(commandLine);

  curCols = cols;
  curRows = rows;

  if (control) {
    // Load the pure key map and stand up the loopback control server before the
    // window, so an agent can attach as early as possible. The headless mirror
    // is created at the args size and resized to the real grid once the pty
    // spawns (below).
    ({ keyToSeq } = await import('./lib/keymap.mjs'));
    startControlServer(cols, rows);
  }

  // A headless (offscreen) window is sized to the requested grid so the capture
  // is a tight PNG of the terminal (generous per-cell metrics for 14px Consolas,
  // a little margin over clipping); a visible window keeps a comfortable default.
  const winW = hideWindow ? Math.round(cols * 9) + 24 : 900;
  const winH = hideWindow ? Math.round(rows * 19) + 24 : 640;
  win = new BrowserWindow({
    width: winW,
    height: winH,
    // Hidden for control/observation sessions (`vibe aiui open --headless`):
    // the terminal is driven over HTTP and read from the headless mirror, so no
    // OS window pops up. Visible for standalone `vibe term` / `vibe tree -t`.
    show: !hideWindow,
    backgroundColor: '#191724',
    // The app-family window icon, chosen by `--icon <name>` (default | vibetree)
    // so the window matches its launcher (PROP-043 #icon). Windows uses ICO
    // (taskbar); PNG elsewhere. resolveIconPath falls back to the default.
    icon: resolveIconPath(iconName),
    webPreferences: {
      nodeIntegration: true,
      contextIsolation: false,
      // A headless session renders **offscreen** (to a bitmap, no visible
      // window) so `webContents.capturePage()` can still return a faithful PNG
      // of the xterm.js grid for `vibe aiui snapshot --png`. A never-shown
      // ordinary window does not paint, so its capture would be blank.
      offscreen: hideWindow,
      // Never throttle. A backgrounded/occluded window otherwise has its timers
      // and repaint (requestAnimationFrame) suspended by Chromium, which stalls
      // the initial fit (the grid stays at the spawn fallback) and makes
      // capturePage return an empty 0x0 image. We drive and capture these
      // windows headlessly, so they must keep running when not in front.
      backgroundThrottling: false,
    },
  });

  await win.loadFile(path.join(__dirname, 'index.html'));

  // Spawn the pty once the renderer reports its fitted grid size, so the hosted
  // program is born at exactly the visible size (no initial resize race). A
  // fallback timer spawns at the args size if the renderer never reports.
  const spawnPty = (spawnCols, spawnRows) => {
    if (ptyProc) return;
    curCols = spawnCols;
    curRows = spawnRows;
    ptyExitCode = null; // a fresh child has not exited yet
    if (headless) {
      try {
        headless.resize(spawnCols, spawnRows);
      } catch {
        /* headless gone */
      }
    }
    ptyProc = pty.spawn(file, args, {
      name: 'xterm-256color',
      cols: spawnCols,
      rows: spawnRows,
      cwd: process.cwd(),
      // Advertise 24-bit colour: xterm.js renders truecolour, so the hosted TUI
      // (`vibe tree`) must take its Tier-3 path and emit exact RGB (Rosé Pine),
      // not degrade to the 256-colour cube — which paints e.g. a `Surface1`
      // unset flag the same gold as a `Gold` set flag. Warp sets this; we must
      // too, or our colours silently mismatch the real terminal.
      env: { ...process.env, COLORTERM: 'truecolor' },
    });
    // main -> renderer, plus the headless mirror (control mode) so /snapshot
    // sees exactly what the renderer sees. Guard every send against a destroyed
    // window.
    ptyDataSub = ptyProc.onData((data) => {
      sawData = true;
      lastDataAt = Date.now();
      if (headless) {
        try {
          headless.write(data);
        } catch {
          /* headless gone */
        }
      }
      if (!win || win.isDestroyed()) return;
      win.webContents.send('pty', data);
    });
    // The child exited on its own (e.g. the user typed `exit`).
    ptyProc.onExit((e) => {
      ptyExitCode = e && typeof e.exitCode === 'number' ? e.exitCode : 0;
      if (controlServer) {
        // Control mode: keep the window + server alive so an agent can still
        // read the final grid and the exit code, then drive teardown itself via
        // POST /close. Just stop mirroring and mark the pty not-alive.
        if (ptyDataSub) {
          try {
            ptyDataSub.dispose();
          } catch {
            /* already disposed */
          }
          ptyDataSub = null;
        }
        ptyProc = null;
        return;
      }
      if (win && !win.isDestroyed()) win.close();
      quit();
    });
  };
  // Expose PTY stop/start to the control plane: an agent restarts the hosted
  // program WITHOUT restarting Electron — a live-preview loop. `pty-stop`
  // frees the binary for a rebuild; `pty-start` spawns the fresh one at the
  // current grid. The renderer, the CDP endpoint, and discovery all stay live.
  stopPty = () => disposePty();
  startPty = () => {
    if (!ptyProc) spawnPty(curCols || cols, curRows || rows);
  };
  // Spawn-or-resize to a grid size. Idempotent: the first call spawns the pty;
  // a later call (a late 'ready' that lost the fallback race, or a real window
  // resize) just resizes the pty + headless mirror. This is what keeps the
  // pty's column count equal to what xterm displays — a mismatch is exactly the
  // "everything skews diagonally" bug.
  const applySize = (c, r) => {
    if (!(c > 0 && r > 0)) return;
    if (!ptyProc) {
      spawnPty(c, r);
      return;
    }
    try {
      ptyProc.resize(c, r);
      curCols = c;
      curRows = r;
      if (headless) headless.resize(c, r);
    } catch {
      /* pty gone or bad size */
    }
  };

  // Both visible and headless-offscreen windows have a real layout, so both take
  // the renderer's fitted grid: the pty is born (and later resized) to exactly
  // what xterm displays, so the two never disagree — no fit-vs-pty skew, on
  // screen or in a capture. The renderer reports once layout settles; a fallback
  // spawns at the args size if it never does; a late 'ready' after that fallback
  // is corrected by applySize (a resize), not dropped.
  // Spawn the pty at the renderer's FITTED grid, not the requested `cols×rows`.
  // We used to wait for a `ready` IPC from the renderer, but that raced: the
  // renderer fires `ready` during DOM load, BEFORE this listener was wired, so
  // it was silently dropped — the pty then fell back (4s) to the args size
  // (120×40) while xterm displayed the fitted size (113×33), and the grid
  // skewed / wrapped under the scrollbar. The visible window is born at a
  // fixed 900×640 regardless of `--cols`, so the args size is never what xterm
  // shows. Instead, after load, ask the renderer directly for its fitted grid
  // and spawn at THAT; the `resize` IPC below keeps the pty in sync as the
  // window changes. A late fallback still covers a renderer that won't report.
  const spawnAtFitted = async () => {
    try {
      const size = await win.webContents.executeJavaScript(
        '(() => { try { window.fit && window.fit.fit(); } catch (e) {} ' +
        'return { cols: term.cols, rows: term.rows }; })()',
      );
      if (size && size.cols > 0 && size.rows > 0) {
        applySize(size.cols, size.rows);
        return;
      }
    } catch {
      /* page not ready yet; the fallback below spawns at the args size */
    }
    if (!ptyProc) spawnPty(cols, rows);
  };
  spawnAtFitted();
  setTimeout(() => {
    if (!ptyProc) spawnPty(cols, rows);
  }, 4000);

  // renderer -> main: feed keystrokes / pastes to the pty.
  ipcMain.on('input', (_event, data) => {
    try {
      if (ptyProc) ptyProc.write(data);
    } catch {
      /* pty gone */
    }
  });

  // renderer -> main: keep the pty (and the headless mirror) sized to the grid
  // xterm fits to, so the hosted program always lays out for exactly what is
  // shown. This is the fluid layout: dragging the window (visible) reflows the
  // program live at a constant font size. Routes through applySize, so a resize
  // arriving before the pty is up spawns it rather than being dropped.
  ipcMain.on('resize', (_event, size) => {
    if (size && size.cols > 0 && size.rows > 0) applySize(size.cols, size.rows);
  });

  win.on('closed', () => {
    win = null;
    quit();
  });
}

app.whenReady().then(createWindow).catch((err) => {
  console.error('[vibeterm] failed to start:', err);
  quit();
});

app.on('window-all-closed', () => {
  quit();
});
