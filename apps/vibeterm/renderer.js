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

const IS_WIN = typeof process !== 'undefined' && process.platform === 'win32';
const DEFAULT_FONT_SIZE = 14;

const term = new Terminal({
  fontFamily: "Consolas, 'Cascadia Mono', monospace",
  fontSize: DEFAULT_FONT_SIZE,
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
// The scrollbar policy is a three-way switch the agent can flip at runtime
// (`window.setScrollbarMode`, driven by `POST /scrollbar`): `auto` hides it
// while the hosted program is in the ALTERNATE buffer (a full-screen TUI like
// `vibe tree` has no scrollback) and shows it for a normal shell; `on` always
// shows it; `off` always hides it. This single knob captures everything we
// learned about rendering with and without the bar — no scattered per-app hacks.
let scrollbarMode = 'auto'; // 'auto' | 'on' | 'off'
let lastScrollbarShown = null;
function scrollbarShown() {
  if (scrollbarMode === 'on') return true;
  if (scrollbarMode === 'off') return false;
  // auto: the normal buffer has scrollback (a shell) → show; the alternate
  // buffer (a full-screen TUI) does not → hide.
  return !(term.buffer.active && term.buffer.active.type === 'alternate');
}

// FitAddon reserves the scrollbar width unconditionally; when the bar is HIDDEN
// (per the policy above) that reservation leaves the grid ~1-2 columns short of
// the right edge. Wrap proposeDimensions to recompute cols over the FULL content
// width (parent - .xterm padding) with NO scrollbar reservation while the bar
// is hidden. Recomputing precisely (rather than `cols + floor(sbw/cell)`) avoids
// an off-by-one from rounding.
{
  const origPropose = fit.proposeDimensions.bind(fit);
  fit.proposeDimensions = function () {
    const d = origPropose();
    if (d && !scrollbarShown()) {
      const core = term._core;
      const dims = core && core._renderService && core._renderService.dimensions;
      const xEl = document.querySelector('.xterm');
      if (
        dims &&
        dims.css &&
        dims.css.cell &&
        dims.css.cell.width > 0 &&
        xEl &&
        xEl.parentElement
      ) {
        const st = getComputedStyle(xEl);
        const padL = parseFloat(st.paddingLeft) || 0;
        const padR = parseFloat(st.paddingRight) || 0;
        const availW = xEl.parentElement.clientWidth - padL - padR;
        d.cols = Math.max(2, Math.floor(availW / dims.css.cell.width));
      }
    }
    return d;
  };
}
// Expose the live terminal + fit addon + the scrollbar switch on `window` so an
// attached CDP client (`vibe aiui inspect`) can read the renderer's real state
// (PROP-042 §4), the main process can ask for the fitted size, and `POST
// /scrollbar` can flip the policy live.
window.term = term;
window.fit = fit;
window.getScrollbarMode = () => scrollbarMode;
window.setScrollbarMode = (m) => {
  if (m !== 'auto' && m !== 'on' && m !== 'off') return false;
  scrollbarMode = m;
  syncScrollMode();
  return true;
};

// Apply the scrollbar policy: tag the body `.scrollbar-hidden` so CSS HIDES the
// bar while it is off, and refit when the effective state flips — the refit is
// DELAYED (a `term.resize` fired on the buffer switch races the program's first
// alt-screen frame and blanks the grid, so let it draw first).
function syncScrollMode() {
  const shown = scrollbarShown();
  document.body.classList.toggle('scrollbar-hidden', !shown);
  if (shown !== lastScrollbarShown) {
    lastScrollbarShown = shown;
    setTimeout(scheduleReport, 400);
  }
}
term.onWriteParsed(syncScrollMode);
syncScrollMode();

// Fit the grid to the window so the hosted program renders for exactly the
// visible grid (no overlap / wrapping). Refit fires from a ResizeObserver
// (drag, and the initial layout) rather than a one-shot rAF — a fit against a
// not-yet-laid-out (0-box) container yields a bogus column count. The first
// good report is `ready` (main spawns the pty at this size); every later one is
// `resize` (main resizes the live pty). Dragging the window reflows the program
// live at a constant font size.
const termEl = document.getElementById('term');
let sentReady = false;
let reportPending = false;

function reportSize() {
  reportPending = false;
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

// Zoom is a terminal-level shortcut (never sent to the hosted program): change
// the font size and refit, so the grid reflows to any font size at any DPI —
// nothing pixel-hardcoded, the fit measures the new cell.
function zoom(delta) {
  const cur = term.options.fontSize || DEFAULT_FONT_SIZE;
  const next = delta === 0 ? DEFAULT_FONT_SIZE : Math.max(6, Math.min(40, cur + delta));
  if (next === cur) return;
  term.options.fontSize = next;
  scheduleReport();
}

// Encode a key event as win32-input-mode: `ESC [ Vk;Sc;Uc;Kd;Cs;Rc _`, a
// key-down record then a key-up. `Sc` (scan code) is left 0 — conhole accepts
// the record from Vk + the unicode char alone (verified: Esc closes a menu with
// Sc=0). Needed on Windows because a ConPTY does not synthesise a key record
// from the raw VT bytes xterm would otherwise send, for a raw reader (a crossterm
// TUI like `vibe tree`).
function win32KeySeq(ev) {
  const vk = ev.keyCode || 0;
  if (!vk) return null;
  let uc = 0;
  if (ev.key && ev.key.length === 1) uc = ev.key.codePointAt(0);
  else if (ev.key === 'Enter') uc = 13;
  else if (ev.key === 'Tab') uc = 9;
  else if (ev.key === 'Escape') uc = 27;
  else if (ev.key === 'Backspace') uc = 8;
  let cs = 0;
  if (ev.shiftKey) cs |= 0x0010; // SHIFT_PRESSED
  if (ev.ctrlKey) cs |= 0x0008; // LEFT_CTRL_PRESSED
  if (ev.altKey) cs |= 0x0002; // LEFT_ALT_PRESSED
  const rec = (kd) => `\x1b[${vk};0;${uc};${kd};${cs};1_`;
  return rec(1) + rec(0);
}

// renderer -> main: keystrokes. Zoom is intercepted on every platform. On
// Windows, keydowns become win32-input-mode (above) and xterm's default VT is
// suppressed; on other platforms xterm's VT (via onData) is correct.
term.attachCustomKeyEventHandler((ev) => {
  if (ev.type === 'keydown' && ev.ctrlKey && !ev.altKey && !ev.shiftKey) {
    if (ev.key === '=' || ev.key === '+') {
      zoom(1);
      return false;
    }
    if (ev.key === '-' || ev.key === '_') {
      zoom(-1);
      return false;
    }
    if (ev.key === '0') {
      zoom(0);
      return false;
    }
  }
  if (!IS_WIN) return true;
  if (ev.type !== 'keydown') return false; // keyup carries no VT; swallow it
  const seq = win32KeySeq(ev);
  if (seq) {
    ipcRenderer.send('input', seq);
    return false;
  }
  return true;
});

// renderer -> main: pasted text (and, off Windows, xterm's key VT) flows here.
term.onData((data) => {
  ipcRenderer.send('input', data);
});

// Clicking anywhere in the window returns focus to the grid.
window.addEventListener('click', () => term.focus());
