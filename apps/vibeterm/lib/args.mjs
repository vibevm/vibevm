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
 * space-separated (`--cols 100`) or inline (`--cols=100`) form, plus the
 * booleans `--control` and `--headless`, anywhere in the array. Positional
 * tokens (the electron binary, `.`, a bare `--`) are ignored, so passing the
 * whole `process.argv` is safe. Missing, malformed, out-of-range, or empty
 * values fall back to the defaults.
 *
 * `--headless` is asymmetric on purpose: the key is set only when the flag is
 * present, so the default parse shape (no `headless` key) is unchanged and a
 * caller reads `out.headless` as truthy-when-hidden, falsy otherwise.
 *
 * @param {string[]} argv
 * @returns {{ exec: string | null, cols: number, rows: number, control: boolean, headless?: boolean }}
 */
export function parseArgs(argv) {
  const out = {
    exec: null,
    cols: DEFAULT_COLS,
    rows: DEFAULT_ROWS,
    control: false,
  };
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
      case '--control': {
        // A boolean flag: presence enables the control server. It takes no
        // value, so a following token is left for the next iteration.
        out.control = true;
        break;
      }
      case '--headless': {
        // A boolean flag: presence hides the OS window (a control /
        // observation session is driven over HTTP and snapshotted, so it needs
        // no visible GUI). Set only when present — see the parse-shape note.
        out.headless = true;
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
 * Tokenize a shell-style command line into whitespace-separated words, with
 * double- and single-quoted spans allowed ANYWHERE in a word (not just at its
 * start). Quote characters are removed and their contents joined with any
 * adjacent unquoted characters, so `--path="C:\\a b"` yields the single word
 * `--path=C:\\a b`. An unterminated quote consumes to end-of-string.
 *
 * @param {string} line
 * @returns {string[]}
 */
function tokenize(line) {
  const tokens = [];
  /** @type {string | null} `null` = between words; a string = the word so far. */
  let current = null;
  /** @type {string | null} the open quote char, or `null` when unquoted. */
  let quote = null;

  for (const ch of line) {
    if (quote !== null) {
      // Inside a quoted span: the matching quote closes it; all else is literal.
      if (ch === quote) quote = null;
      else current += ch;
      continue;
    }
    if (ch === '"' || ch === "'") {
      quote = ch;
      if (current === null) current = '';
      continue;
    }
    if (ch === ' ' || ch === '\t' || ch === '\n' || ch === '\r') {
      if (current !== null) {
        tokens.push(current);
        current = null;
      }
      continue;
    }
    current = (current ?? '') + ch;
  }

  // A trailing word — including the content of an unterminated quote — is kept.
  if (current !== null) tokens.push(current);
  return tokens;
}

/**
 * Split a shell command line into an executable + argument vector for
 * `pty.spawn(file, args, …)`.
 *
 * A proper tokenizer (see {@link tokenize}) is used, so both the executable
 * and any argument may be double- or single-quoted to carry spaces:
 * `"C:\\Program Files\\vibe.exe" tree --path "C:\\a b" -c` splits into the
 * file `C:\\Program Files\\vibe.exe` and args `tree`, `--path`, `C:\\a b`,
 * `-c`. An unterminated quote takes the rest of the line as one word.
 *
 * @param {string} cmdline
 * @returns {{ file: string, args: string[] }}
 */
export function splitCommand(cmdline) {
  const line = typeof cmdline === 'string' ? cmdline : '';
  const tokens = tokenize(line);
  if (tokens.length === 0) return { file: '', args: [] };
  return { file: tokens[0], args: tokens.slice(1) };
}
