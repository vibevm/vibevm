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

async function createWindow() {
  // Pure, dependency-free logic — the same module the unit tests import.
  const { parseArgs, defaultShell, splitCommand } = await import('./lib/args.mjs');

  // `headless: hideWindow` renames on destructure so it does not shadow the
  // module-level `headless` (the @xterm/headless mirror).
  const { exec, cols, rows, control, headless: hideWindow } = parseArgs(process.argv);
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

  win = new BrowserWindow({
    width: 900,
    height: 640,
    // Hidden for control/observation sessions (`vibe aiui open --headless`):
    // the terminal is driven over HTTP and read from the headless mirror, so no
    // OS window pops up. Visible for standalone `vibe term` / `vibe tree -t`.
    show: !hideWindow,
    backgroundColor: '#191724',
    webPreferences: {
      nodeIntegration: true,
      contextIsolation: false,
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
      env: process.env,
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

  if (hideWindow) {
    // Headless: a hidden window has no settled layout to fit, so the pty and the
    // headless mirror are born at exactly the requested size and stay there.
    // The renderer still loads (hidden); its fit reports are ignored — snapshots
    // read from the headless mirror, which spawnPty sizes to match.
    spawnPty(cols, rows);
  } else {
    // Visible: born at the renderer's fitted grid (reported once layout settles),
    // so the hosted program lays out for exactly what is shown. A fallback spawns
    // at the args size only if the renderer never reports; a late 'ready' after
    // that fallback is corrected by applySize (resize), not dropped.
    ipcMain.once('ready', (_event, size) => {
      applySize(
        size && size.cols > 0 ? size.cols : cols,
        size && size.rows > 0 ? size.rows : rows,
      );
    });
    setTimeout(() => {
      if (!ptyProc) spawnPty(cols, rows);
    }, 4000);
  }

  // renderer -> main: feed keystrokes / pastes to the pty.
  ipcMain.on('input', (_event, data) => {
    try {
      if (ptyProc) ptyProc.write(data);
    } catch {
      /* pty gone */
    }
  });

  // renderer -> main: keep the pty (and the headless mirror) sized to the
  // visible grid (FitAddon), so the hosted program lays out for exactly what is
  // shown. Routes through applySize, so a resize arriving before the pty is up
  // spawns it rather than being dropped. Ignored in headless mode, where the
  // grid is fixed at spawn and a hidden window's phantom layout must not drift
  // the snapshot size.
  ipcMain.on('resize', (_event, size) => {
    if (hideWindow) return;
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
