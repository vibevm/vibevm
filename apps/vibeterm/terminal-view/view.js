/**
 * vibeterm — terminal-view (the lean per-tab renderer).
 *
 * spec://vibeterm/PROP-044#d3-split-two / PROP-047 — each tab is its own WebContentsView running
 * this page: xterm.js ONLY (no chrome framework). node-pty lives in main; bytes arrive on `pty`,
 * keystrokes go back on `input`, the fitted grid reports on `ready`/`resize`. Plain classic script
 * under nodeIntegration:true (a terminal view, not the chrome).
 */
"use strict";

const { Terminal } = require("@xterm/xterm");
const { FitAddon } = require("@xterm/addon-fit");
const { ipcRenderer } = require("electron");

const term = new Terminal({
  fontFamily: "Consolas, 'Cascadia Mono', monospace",
  fontSize: 14,
  cursorBlink: true,
  theme: { background: "#191724", foreground: "#e0def4" },
});

const fit = new FitAddon();
term.loadAddon(fit);
term.open(document.getElementById("term"));
term.focus();

const termEl = document.getElementById("term");

// Report the fitted grid so main spawns/resizes the pty to exactly what xterm shows.
function report(channel) {
  try {
    fit.fit();
    const size = { cols: term.cols, rows: term.rows };
    if (size.cols > 0 && size.rows > 0) ipcRenderer.send(channel, size);
  } catch {
    /* container not laid out yet */
  }
}

const ro = new ResizeObserver(() => report("resize"));
ro.observe(termEl);
// The first good report is `ready` (main spawns the pty at this size).
setTimeout(() => report("ready"), 0);

// main -> renderer: raw pty output.
ipcRenderer.on("pty", (_event, data) => term.write(data));

// renderer -> main: keystrokes / pastes.
term.onData((data) => ipcRenderer.send("input", data));

// Clicking anywhere returns focus to the grid.
window.addEventListener("click", () => term.focus());
