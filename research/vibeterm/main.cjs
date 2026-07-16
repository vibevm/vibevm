/**
 * vibeterm — Electron main process.
 *
 * node-pty runs HERE, in the main process: the renderer has no
 * worker_threads, so constructing a pty there throws
 * "Failed to construct 'Worker'". PTY bytes flow main -> renderer over
 * `webContents.send('pty', …)`; keystrokes flow renderer -> main over the
 * `input` channel and into `pty.write`.
 */

'use strict';

const path = require('node:path');
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

/** Tear down once and quit. */
function quit() {
  if (quitting) return;
  quitting = true;
  disposePty();
  app.quit();
}

async function createWindow() {
  // Pure, dependency-free logic — the same module the unit tests import.
  const { parseArgs, defaultShell, splitCommand } = await import('./lib/args.mjs');

  const { exec, cols, rows } = parseArgs(process.argv);
  const commandLine = exec ?? defaultShell(process.platform, process.env);
  const { file, args } = splitCommand(commandLine);

  win = new BrowserWindow({
    width: 900,
    height: 640,
    show: true,
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
    ptyProc = pty.spawn(file, args, {
      name: 'xterm-256color',
      cols: spawnCols,
      rows: spawnRows,
      cwd: process.cwd(),
      env: process.env,
    });
    // main -> renderer: guard every send against a destroyed window.
    ptyDataSub = ptyProc.onData((data) => {
      if (!win || win.isDestroyed()) return;
      win.webContents.send('pty', data);
    });
    // The child exited on its own (e.g. the user typed `exit`): close and quit.
    ptyProc.onExit(() => {
      if (win && !win.isDestroyed()) win.close();
      quit();
    });
  };
  ipcMain.once('ready', (_event, size) => {
    spawnPty(
      size && size.cols > 0 ? size.cols : cols,
      size && size.rows > 0 ? size.rows : rows,
    );
  });
  setTimeout(() => spawnPty(cols, rows), 1500);

  // renderer -> main: feed keystrokes / pastes to the pty.
  ipcMain.on('input', (_event, data) => {
    try {
      if (ptyProc) ptyProc.write(data);
    } catch {
      /* pty gone */
    }
  });

  // renderer -> main: keep the pty sized to the visible grid (FitAddon), so the
  // hosted program lays out for exactly what is shown.
  ipcMain.on('resize', (_event, size) => {
    try {
      if (ptyProc && size && size.cols > 0 && size.rows > 0) {
        ptyProc.resize(size.cols, size.rows);
      }
    } catch {
      /* pty gone or bad size */
    }
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
