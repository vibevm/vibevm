/**
 * vibeterm — renderer.
 *
 * xterm.js ONLY: no node-pty here (the renderer has no worker_threads). PTY
 * output arrives on the `pty` channel; keystrokes go back on `input`. This
 * runs under `nodeIntegration: true`, so Electron injects `require` into the
 * renderer's global scope — a classic `<script>`, not a Node ES module.
 */

const { Terminal } = require('@xterm/xterm');
const { FitAddon } = require('@xterm/addon-fit');
const { ipcRenderer } = require('electron');

const term = new Terminal({
  fontFamily: "Consolas, 'Cascadia Mono', monospace",
  fontSize: 14,
  cursorBlink: true,
  theme: {
    background: '#191724',
    foreground: '#e0def4',
  },
});

const fit = new FitAddon();
term.loadAddon(fit);
term.open(document.getElementById('term'));
term.focus();

// Fit the grid to the window so the hosted program renders for exactly the
// visible grid (no overlap / wrapping). We report the fitted size to main via a
// ResizeObserver rather than a one-shot rAF: the observer fires only once the
// container has a settled, non-zero layout, so `fit.fit()` measures the real
// width — a fit against a not-yet-laid-out (0-box) container yields a bogus
// column count, and spawning the pty at it is exactly the "everything skews
// diagonally" bug. The first good report is `ready` (main spawns the pty at
// this size); every later one is `resize` (main resizes the live pty).
const termEl = document.getElementById('term');
let sentReady = false;
let reportPending = false;

function reportSize() {
  reportPending = false;
  // Skip until the container is actually laid out — a hidden (headless) window
  // stays 0-box forever, so this simply never fires `ready`, which is correct:
  // headless spawns at the requested size on the main side.
  if (!termEl || termEl.clientWidth < 1 || termEl.clientHeight < 1) return;
  let size;
  try {
    fit.fit();
    size = { cols: term.cols, rows: term.rows };
  } catch {
    return; /* container not laid out yet */
  }
  if (!(size.cols > 0 && size.rows > 0)) return;
  if (!sentReady) {
    sentReady = true;
    ipcRenderer.send('ready', size);
  } else {
    ipcRenderer.send('resize', size);
  }
}

// Coalesce bursts (a drag-resize fires many times per second) into one fit per
// animation frame.
function scheduleReport() {
  if (reportPending) return;
  reportPending = true;
  requestAnimationFrame(reportSize);
}

const resizeObserver = new ResizeObserver(scheduleReport);
resizeObserver.observe(termEl);
// The observer delivers an initial callback for the observed element, but tick
// once too in case the very first (0 -> N) transition is coalesced away.
scheduleReport();

// main -> renderer: raw pty output.
ipcRenderer.on('pty', (_event, data) => {
  term.write(data);
});

// renderer -> main: every keystroke / paste.
term.onData((data) => {
  ipcRenderer.send('input', data);
});

// Clicking anywhere in the window returns focus to the grid.
window.addEventListener('click', () => term.focus());
