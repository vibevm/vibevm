/**
 * vibeframe — pure key-name → PTY input-sequence map.
 *
 * Like `lib/args.mjs`, this module imports nothing from Electron or node-pty,
 * so it runs (and is unit-tested) under a plain `node --test`, with no native
 * build and no GUI. The control server (`main.cjs`) uses `keyToSeq(name,
 * platform)` to turn the symbolic key names in a `POST /input` request into the
 * bytes an application expects.
 *
 * Two encodings, chosen by platform:
 *
 *   - **Unix** (`darwin`, `linux`, …): the standard xterm VT sequences — the pty
 *     is a real terminal and the hosted app reads these bytes directly.
 *       - F1–F4 SS3 (`ESC O <final>`); F5–F12 CSI (`ESC [ <n> ~`);
 *       - arrows CSI cursor (`ESC [ <final>`); Shift+arrow CSI `1;2`;
 *       - `BackTab` = Shift+Tab (`ESC [ Z`).
 *
 *   - **Windows** (`win32`): **win32-input-mode** (`ESC [ Vk;Sc;Uc;Kd;Cs;Rc _`),
 *     the encoding Windows Terminal uses to pass full KEY_EVENTs through a
 *     ConPTY. Injecting the raw VT form instead relies on conhost synthesising a
 *     key record from the byte stream, which it does for a cooked reader (cmd)
 *     but NOT reliably for a raw reader (a crossterm TUI like `vibe tree`), so
 *     special keys were silently dropped. win32-input-mode encodes the key event
 *     explicitly, so conhost hands the child exactly the record it expects. Each
 *     key is a down record (`Kd=1`) followed by an up record (`Kd=0`).
 */

/**
 * The Unix VT sequence for each canonical key name. Names are matched
 * case-insensitively (see `keyToSeq`); this table holds the canonical spelling.
 *
 * @type {Readonly<Record<string, string>>}
 */
export const KEY_TO_SEQ = Object.freeze({
  Enter: '\r',
  Esc: '\x1b',
  Tab: '\t',
  BackTab: '\x1b[Z',
  Space: ' ',
  Backspace: '\x7f',

  Up: '\x1b[A',
  Down: '\x1b[B',
  Right: '\x1b[C',
  Left: '\x1b[D',

  'Shift+Left': '\x1b[1;2D',
  'Shift+Right': '\x1b[1;2C',
  'Shift+Up': '\x1b[1;2A',
  'Shift+Down': '\x1b[1;2B',

  F1: '\x1bOP',
  F2: '\x1bOQ',
  F3: '\x1bOR',
  F4: '\x1bOS',
  F5: '\x1b[15~',
  F6: '\x1b[17~',
  F7: '\x1b[18~',
  F8: '\x1b[19~',
  F9: '\x1b[20~',
  F10: '\x1b[21~',
  F11: '\x1b[23~',
  F12: '\x1b[24~',
});

/** SHIFT_PRESSED in a Windows console control-key state. */
const SHIFT = 0x0010;

/**
 * The Windows key descriptor for each canonical key name: virtual-key code
 * (`vk`), scan code (`sc`), unicode code point (`uc`, 0 for non-text keys), and
 * an optional control-key state (`cs`). Encoded to win32-input-mode by
 * {@link encodeWin32}.
 *
 * @type {Readonly<Record<string, { vk: number, sc: number, uc: number, cs?: number }>>}
 */
export const KEY_TO_WIN32 = Object.freeze({
  Enter: { vk: 0x0d, sc: 0x1c, uc: 0x0d },
  Esc: { vk: 0x1b, sc: 0x01, uc: 0x1b },
  Tab: { vk: 0x09, sc: 0x0f, uc: 0x09 },
  BackTab: { vk: 0x09, sc: 0x0f, uc: 0x09, cs: SHIFT },
  Space: { vk: 0x20, sc: 0x39, uc: 0x20 },
  Backspace: { vk: 0x08, sc: 0x0e, uc: 0x08 },

  Up: { vk: 0x26, sc: 0x48, uc: 0 },
  Down: { vk: 0x28, sc: 0x50, uc: 0 },
  Right: { vk: 0x27, sc: 0x4d, uc: 0 },
  Left: { vk: 0x25, sc: 0x4b, uc: 0 },

  'Shift+Up': { vk: 0x26, sc: 0x48, uc: 0, cs: SHIFT },
  'Shift+Down': { vk: 0x28, sc: 0x50, uc: 0, cs: SHIFT },
  'Shift+Right': { vk: 0x27, sc: 0x4d, uc: 0, cs: SHIFT },
  'Shift+Left': { vk: 0x25, sc: 0x4b, uc: 0, cs: SHIFT },

  F1: { vk: 0x70, sc: 0x3b, uc: 0 },
  F2: { vk: 0x71, sc: 0x3c, uc: 0 },
  F3: { vk: 0x72, sc: 0x3d, uc: 0 },
  F4: { vk: 0x73, sc: 0x3e, uc: 0 },
  F5: { vk: 0x74, sc: 0x3f, uc: 0 },
  F6: { vk: 0x75, sc: 0x40, uc: 0 },
  F7: { vk: 0x76, sc: 0x41, uc: 0 },
  F8: { vk: 0x77, sc: 0x42, uc: 0 },
  F9: { vk: 0x78, sc: 0x43, uc: 0 },
  F10: { vk: 0x79, sc: 0x44, uc: 0 },
  F11: { vk: 0x7a, sc: 0x57, uc: 0 },
  F12: { vk: 0x7b, sc: 0x58, uc: 0 },
});

// Case-insensitive lookups, built once from the canonical tables.
const VT_LOOKUP = new Map(
  Object.entries(KEY_TO_SEQ).map(([name, seq]) => [name.toLowerCase(), seq]),
);
const WIN32_LOOKUP = new Map(
  Object.entries(KEY_TO_WIN32).map(([name, desc]) => [name.toLowerCase(), desc]),
);

/**
 * Encode a key descriptor as a win32-input-mode down+up pair.
 *
 * @param {{ vk: number, sc: number, uc: number, cs?: number }} k
 * @returns {string}
 */
export function encodeWin32(k) {
  const cs = k.cs ?? 0;
  const ev = (kd) => `\x1b[${k.vk};${k.sc};${k.uc};${kd};${cs};1_`;
  return ev(1) + ev(0);
}

/**
 * Translate a symbolic key name into the bytes to write to the pty, in the
 * encoding the given platform's hosted app expects.
 *
 * Matching is case-insensitive and ignores surrounding whitespace, so `"F2"`,
 * `"f2"`, and `" f2 "` are equivalent, as are `"Shift+Left"` and `"shift+left"`.
 *
 * @param {string} name  a key name from {@link KEY_TO_SEQ}
 * @param {string} [platform]  e.g. `process.platform`; `win32` selects
 *   win32-input-mode, anything else selects the VT sequences.
 * @returns {string} the bytes to write to the pty
 * @throws {Error} if `name` is not a string, or is not a known key
 */
export function keyToSeq(name, platform) {
  if (typeof name !== 'string') {
    throw new Error(`unknown key: ${String(name)}`);
  }
  const key = name.trim().toLowerCase();
  if (platform === 'win32') {
    const desc = WIN32_LOOKUP.get(key);
    if (desc === undefined) {
      throw new Error(`unknown key: ${name}`);
    }
    return encodeWin32(desc);
  }
  const seq = VT_LOOKUP.get(key);
  if (seq === undefined) {
    throw new Error(`unknown key: ${name}`);
  }
  return seq;
}
