import { test } from 'node:test';
import assert from 'node:assert/strict';

import {
  keyToSeq,
  encodeWin32,
  KEY_TO_SEQ,
  KEY_TO_WIN32,
} from '../lib/keymap.mjs';

// ---------------------------------------------------------------------------
// keyToSeq — representative sequences (Unix / VT; the default when the platform
// is anything but win32)
// ---------------------------------------------------------------------------

test('keyToSeq: control + editing keys', () => {
  assert.equal(keyToSeq('Enter'), '\r');
  assert.equal(keyToSeq('Esc'), '\x1b');
  assert.equal(keyToSeq('Tab'), '\t');
  assert.equal(keyToSeq('BackTab'), '\x1b[Z');
  assert.equal(keyToSeq('Space'), ' ');
  assert.equal(keyToSeq('Backspace'), '\x7f');
});

test('keyToSeq: arrows use the CSI cursor form', () => {
  assert.equal(keyToSeq('Up'), '\x1b[A');
  assert.equal(keyToSeq('Down'), '\x1b[B');
  assert.equal(keyToSeq('Right'), '\x1b[C');
  assert.equal(keyToSeq('Left'), '\x1b[D');
});

test('keyToSeq: Shift+arrow uses the CSI 1;2 modified-cursor form', () => {
  assert.equal(keyToSeq('Shift+Left'), '\x1b[1;2D');
  assert.equal(keyToSeq('Shift+Right'), '\x1b[1;2C');
  assert.equal(keyToSeq('Shift+Up'), '\x1b[1;2A');
  assert.equal(keyToSeq('Shift+Down'), '\x1b[1;2B');
});

test('keyToSeq: function keys (SS3 for F1–F4, CSI ~ for F5–F12)', () => {
  assert.equal(keyToSeq('F1'), '\x1bOP');
  assert.equal(keyToSeq('F4'), '\x1bOS');
  assert.equal(keyToSeq('F5'), '\x1b[15~');
  assert.equal(keyToSeq('F12'), '\x1b[24~');
});

// ---------------------------------------------------------------------------
// keyToSeq — normalization
// ---------------------------------------------------------------------------

test('keyToSeq: names are case-insensitive', () => {
  assert.equal(keyToSeq('enter'), '\r');
  assert.equal(keyToSeq('f2'), '\x1bOQ');
  assert.equal(keyToSeq('F2'), '\x1bOQ');
  assert.equal(keyToSeq('shift+left'), '\x1b[1;2D');
  assert.equal(keyToSeq('SHIFT+DOWN'), '\x1b[1;2B');
});

test('keyToSeq: surrounding whitespace is ignored', () => {
  assert.equal(keyToSeq('  F5 '), '\x1b[15~');
  assert.equal(keyToSeq('\tEnter\n'), '\r');
});

test('keyToSeq: every canonical name maps in all cases', () => {
  for (const [name, seq] of Object.entries(KEY_TO_SEQ)) {
    assert.equal(keyToSeq(name), seq, name);
    assert.equal(keyToSeq(name.toLowerCase()), seq, name.toLowerCase());
    assert.equal(keyToSeq(name.toUpperCase()), seq, name.toUpperCase());
  }
});

// ---------------------------------------------------------------------------
// keyToSeq — errors
// ---------------------------------------------------------------------------

test('keyToSeq: an unknown key throws, naming the key', () => {
  assert.throws(() => keyToSeq('Nope'), (err) => {
    assert.ok(err instanceof Error);
    assert.match(err.message, /unknown key/iu);
    assert.match(err.message, /Nope/u);
    return true;
  });
  // Shift+Tab is NOT in the map — BackTab is the canonical spelling.
  assert.throws(() => keyToSeq('Shift+Tab'), /unknown key/iu);
  assert.throws(() => keyToSeq(''), /unknown key/iu);
});

test('keyToSeq: a non-string argument throws', () => {
  assert.throws(() => keyToSeq(null), /unknown key/iu);
  assert.throws(() => keyToSeq(undefined), /unknown key/iu);
  assert.throws(() => keyToSeq(42), /unknown key/iu);
});

// ---------------------------------------------------------------------------
// keyToSeq — Windows (win32-input-mode: ESC [ Vk;Sc;Uc;Kd;Cs;Rc _, down+up)
// ---------------------------------------------------------------------------

test('keyToSeq(win32): F2 encodes as a VK_F2 down+up pair', () => {
  assert.equal(
    keyToSeq('F2', 'win32'),
    '\x1b[113;60;0;1;0;1_\x1b[113;60;0;0;0;1_',
  );
});

test('keyToSeq(win32): text keys carry their unicode code point', () => {
  assert.equal(
    keyToSeq('Enter', 'win32'),
    '\x1b[13;28;13;1;0;1_\x1b[13;28;13;0;0;1_',
  );
  assert.equal(
    keyToSeq('Space', 'win32'),
    '\x1b[32;57;32;1;0;1_\x1b[32;57;32;0;0;1_',
  );
  assert.equal(
    keyToSeq('Esc', 'win32'),
    '\x1b[27;1;27;1;0;1_\x1b[27;1;27;0;0;1_',
  );
});

test('keyToSeq(win32): arrows have no unicode; Shift sets the control state', () => {
  assert.equal(
    keyToSeq('Up', 'win32'),
    '\x1b[38;72;0;1;0;1_\x1b[38;72;0;0;0;1_',
  );
  // Shift+Left: VK_LEFT=37 scan=75, cs = SHIFT_PRESSED (0x10 = 16).
  assert.equal(
    keyToSeq('Shift+Left', 'win32'),
    '\x1b[37;75;0;1;16;1_\x1b[37;75;0;0;16;1_',
  );
  // BackTab is Shift+Tab: VK_TAB=9 scan=15, cs = 16.
  assert.equal(
    keyToSeq('BackTab', 'win32'),
    '\x1b[9;15;9;1;16;1_\x1b[9;15;9;0;16;1_',
  );
});

test('keyToSeq(win32): case-insensitive and whitespace-trimmed', () => {
  const f2 = keyToSeq('F2', 'win32');
  assert.equal(keyToSeq('f2', 'win32'), f2);
  assert.equal(keyToSeq('  F2 ', 'win32'), f2);
});

test('keyToSeq(win32): every canonical name maps and matches the VT name set', () => {
  // The two tables cover exactly the same keys, so a script is portable.
  assert.deepEqual(
    Object.keys(KEY_TO_WIN32).sort(),
    Object.keys(KEY_TO_SEQ).sort(),
  );
  for (const [name, desc] of Object.entries(KEY_TO_WIN32)) {
    assert.equal(keyToSeq(name, 'win32'), encodeWin32(desc), name);
  }
});

test('keyToSeq(win32): an unknown key still throws, naming the key', () => {
  assert.throws(() => keyToSeq('Nope', 'win32'), /unknown key/iu);
  assert.throws(() => keyToSeq('Shift+Tab', 'win32'), /unknown key/iu);
});

test('encodeWin32: down record then up record, control state defaulting to 0', () => {
  assert.equal(encodeWin32({ vk: 1, sc: 2, uc: 3 }), '\x1b[1;2;3;1;0;1_\x1b[1;2;3;0;0;1_');
  assert.equal(encodeWin32({ vk: 1, sc: 2, uc: 3, cs: 16 }), '\x1b[1;2;3;1;16;1_\x1b[1;2;3;0;16;1_');
});
