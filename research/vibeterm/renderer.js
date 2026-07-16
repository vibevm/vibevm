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
// visible grid (no overlap / wrapping). Report the fitted size to main: the
// initial `ready` lets main spawn the pty at this size (no resize race); later
// window resizes send `resize` so main resizes the live pty.
function fitGrid() {
  try {
    fit.fit();
  } catch {
    /* container not laid out yet */
  }
  return { cols: term.cols, rows: term.rows };
}
requestAnimationFrame(() => ipcRenderer.send('ready', fitGrid()));
window.addEventListener('resize', () => ipcRenderer.send('resize', fitGrid()));

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
