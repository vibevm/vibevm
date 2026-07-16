/**
 * vibeterm — pure argument + command helpers.
 *
 * This module is deliberately free of Electron and node-pty imports so it
 * runs (and is unit-tested) under a plain `node --test`, with no native
 * build and no GUI. `main.cjs` pulls it in via `await import('./lib/args.mjs')`.
 */

const DEFAULT_COLS = 84;
const DEFAULT_ROWS = 30;

/**
 * Parse a positive integer from a CLI value. Only an all-digit string
 * denoting a value > 0 is accepted; anything else yields null so the caller
 * keeps its default.
 *
 * @param {unknown} value
 * @returns {number | null}
 */
function toPositiveInt(value) {
  if (typeof value !== 'string' || !/^\d+$/u.test(value)) return null;
  const n = Number.parseInt(value, 10);
  return n > 0 ? n : null;
}

/**
 * Parse vibeterm's command-line flags out of an argv array.
 *
 * Recognises `--exec <cmd>`, `--cols <n>` and `--rows <n>` in either the
 * space-separated (`--cols 100`) or inline (`--cols=100`) form, anywhere in
 * the array. Positional tokens (the electron binary, `.`, a bare `--`) are
 * ignored, so passing the whole `process.argv` is safe. Missing, malformed,
 * out-of-range, or empty values fall back to the defaults.
 *
 * @param {string[]} argv
 * @returns {{ exec: string | null, cols: number, rows: number }}
 */
export function parseArgs(argv) {
  const out = { exec: null, cols: DEFAULT_COLS, rows: DEFAULT_ROWS };
  const args = Array.isArray(argv) ? argv : [];

  for (let i = 0; i < args.length; i++) {
    const token = args[i];
    if (typeof token !== 'string') continue;

    let flag = token;
    let inlineValue = null;
    if (token.startsWith('--')) {
      const eq = token.indexOf('=');
      if (eq !== -1) {
        flag = token.slice(0, eq);
        inlineValue = token.slice(eq + 1);
      }
    }

    // Take this flag's value: the inline `=value` if present, else the next
    // token — but never a following `--flag`, so a dangling flag cannot
    // swallow the next one.
    const takeValue = () => {
      if (inlineValue !== null) return inlineValue;
      const next = args[i + 1];
      if (typeof next === 'string' && !next.startsWith('--')) {
        i += 1;
        return next;
      }
      return null;
    };

    switch (flag) {
      case '--exec': {
        const value = takeValue();
        if (value !== null && value.trim() !== '') out.exec = value;
        break;
      }
      case '--cols': {
        const n = toPositiveInt(takeValue());
        if (n !== null) out.cols = n;
        break;
      }
      case '--rows': {
        const n = toPositiveInt(takeValue());
        if (n !== null) out.rows = n;
        break;
      }
      default:
        break;
    }
  }

  return out;
}

/**
 * The fallback command when `--exec` is absent — a plain per-platform shell.
 *
 * The smart pwsh-vs-powershell selection lives in the Rust `vibe term`
 * caller, which passes its choice through `--exec`; vibeterm's own default is
 * just this simple fallback, for standalone `electron .` use.
 *
 * @param {string} platform  e.g. `process.platform`
 * @param {Record<string, string | undefined>} env  e.g. `process.env`
 * @returns {string}
 */
export function defaultShell(platform, env) {
  const e = env ?? {};
  if (platform === 'win32') return e.COMSPEC || 'cmd.exe';
  return e.SHELL || '/bin/sh';
}

/**
 * Split a shell command line into an executable + argument vector for
 * `pty.spawn(file, args, …)`.
 *
 * The first token may be quoted (`"C:\\Program Files\\app.exe" -x`), which
 * lets the executable path carry spaces; the remainder is a simple whitespace
 * split. Quoting inside later arguments is NOT interpreted — the Rust caller
 * passes already-resolved arguments, and node-pty re-quotes as needed.
 *
 * @param {string} cmdline
 * @returns {{ file: string, args: string[] }}
 */
export function splitCommand(cmdline) {
  const line = typeof cmdline === 'string' ? cmdline.trim() : '';
  if (line === '') return { file: '', args: [] };

  let file;
  let rest;
  const quote = line[0];
  if (quote === '"' || quote === "'") {
    const end = line.indexOf(quote, 1);
    if (end === -1) {
      // Unterminated quote: take the remainder as the file, with no args.
      file = line.slice(1);
      rest = '';
    } else {
      file = line.slice(1, end);
      rest = line.slice(end + 1);
    }
  } else {
    const firstToken = line.match(/^\S+/u)[0];
    file = firstToken;
    rest = line.slice(firstToken.length);
  }

  const trimmedRest = rest.trim();
  const args = trimmedRest === '' ? [] : trimmedRest.split(/\s+/u);
  return { file, args };
}
