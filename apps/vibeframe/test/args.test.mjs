import { test } from 'node:test';
import assert from 'node:assert/strict';

import { parseArgs, defaultShell, splitCommand } from '../lib/args.mjs';

// ---------------------------------------------------------------------------
// parseArgs
// ---------------------------------------------------------------------------

test('parseArgs: defaults when no flags are present', () => {
  assert.deepEqual(parseArgs([]), {
    exec: null,
    cols: 84,
    rows: 30,
    control: false,
  });
  assert.deepEqual(parseArgs(['electron', '.']), {
    exec: null,
    cols: 84,
    rows: 30,
    control: false,
  });
});

test('parseArgs: --exec, --cols, --rows in space-separated form', () => {
  assert.deepEqual(
    parseArgs(['--exec', 'vibe tree -c', '--cols', '100', '--rows', '40']),
    { exec: 'vibe tree -c', cols: 100, rows: 40, control: false },
  );
});

test('parseArgs: inline --flag=value form (only the first = splits)', () => {
  assert.deepEqual(
    parseArgs(['--exec=cmd.exe /K echo hi', '--cols=120', '--rows=50']),
    { exec: 'cmd.exe /K echo hi', cols: 120, rows: 50, control: false },
  );
});

test('parseArgs: --icon selects the window icon by name (space and inline)', () => {
  assert.deepEqual(parseArgs(['--icon', 'vibetree']), {
    exec: null,
    cols: 84,
    rows: 30,
    control: false,
    icon: 'vibetree',
  });
  assert.deepEqual(parseArgs(['--icon=default']), {
    exec: null,
    cols: 84,
    rows: 30,
    control: false,
    icon: 'default',
  });
});

test('parseArgs: --icon is asymmetric — absent unless a non-empty value is given', () => {
  assert.ok(!('icon' in parseArgs([])));
  assert.ok(!('icon' in parseArgs(['--icon', ''])));
  assert.ok(!('icon' in parseArgs(['--icon'])));
});

test('parseArgs: --control is a boolean flag (present → true)', () => {
  assert.deepEqual(parseArgs(['--control']), {
    exec: null,
    cols: 84,
    rows: 30,
    control: true,
  });
  // It takes no value, so it does not swallow the following token.
  assert.deepEqual(
    parseArgs(['--exec', 'vibe tree', '--control', '--cols', '100']),
    { exec: 'vibe tree', cols: 100, rows: 30, control: true },
  );
});

test('parseArgs: --headless is asymmetric — key set only when present', () => {
  // Absent: the default parse shape carries no `headless` key at all.
  assert.equal('headless' in parseArgs([]), false);
  assert.equal(parseArgs([]).headless, undefined);
  // Present: a boolean flag that takes no value and reads truthy.
  assert.equal(parseArgs(['--headless']).headless, true);
  // It composes with the other flags and does not swallow a following token.
  assert.deepEqual(
    parseArgs(['--control', '--headless', '--cols', '100']),
    { exec: null, cols: 100, rows: 30, control: true, headless: true },
  );
});

test('parseArgs: ignores positional tokens (whole process.argv is safe)', () => {
  const argv = [
    '/usr/bin/electron',
    '.',
    '--',
    '--exec',
    '/bin/bash -l',
    '--cols',
    '90',
  ];
  assert.deepEqual(parseArgs(argv), {
    exec: '/bin/bash -l',
    cols: 90,
    rows: 30,
    control: false,
  });
});

test('parseArgs: malformed / missing values fall back to defaults', () => {
  // Non-numeric cols, zero rows (out of range), and a dangling --exec.
  assert.deepEqual(parseArgs(['--cols', 'abc', '--rows', '0']), {
    exec: null,
    cols: 84,
    rows: 30,
    control: false,
  });
  assert.deepEqual(parseArgs(['--exec']), {
    exec: null,
    cols: 84,
    rows: 30,
    control: false,
  });
  assert.deepEqual(parseArgs(['--cols', '100abc']), {
    exec: null,
    cols: 84,
    rows: 30,
    control: false,
  });
});

test('parseArgs: a dangling flag does not swallow the next flag', () => {
  // --exec has no value (next token is another flag) → exec stays null,
  // and --cols is still parsed.
  assert.deepEqual(parseArgs(['--exec', '--cols', '100']), {
    exec: null,
    cols: 100,
    rows: 30,
    control: false,
  });
});

test('parseArgs: an empty or whitespace-only --exec is treated as absent', () => {
  assert.equal(parseArgs(['--exec', '']).exec, null);
  assert.equal(parseArgs(['--exec=   ']).exec, null);
});

test('parseArgs: is robust to non-array / junk input', () => {
  assert.deepEqual(parseArgs(undefined), {
    exec: null,
    cols: 84,
    rows: 30,
    control: false,
  });
  assert.deepEqual(parseArgs(null), {
    exec: null,
    cols: 84,
    rows: 30,
    control: false,
  });
});

// ---------------------------------------------------------------------------
// defaultShell
// ---------------------------------------------------------------------------

test('defaultShell: win32 falls back to COMSPEC then cmd.exe', () => {
  assert.equal(defaultShell('win32', {}), 'cmd.exe');
  assert.equal(
    defaultShell('win32', { COMSPEC: 'C:\\pwsh\\pwsh.exe' }),
    'C:\\pwsh\\pwsh.exe',
  );
});

test('defaultShell: non-win32 falls back to SHELL then /bin/sh', () => {
  assert.equal(defaultShell('linux', {}), '/bin/sh');
  assert.equal(defaultShell('darwin', { SHELL: '/bin/zsh' }), '/bin/zsh');
});

test('defaultShell: tolerates a missing env object', () => {
  assert.equal(defaultShell('win32', undefined), 'cmd.exe');
  assert.equal(defaultShell('linux', undefined), '/bin/sh');
});

// ---------------------------------------------------------------------------
// splitCommand
// ---------------------------------------------------------------------------

test('splitCommand: simple whitespace split', () => {
  assert.deepEqual(splitCommand('cmd.exe /K echo hi'), {
    file: 'cmd.exe',
    args: ['/K', 'echo', 'hi'],
  });
});

test('splitCommand: collapses surrounding and repeated whitespace', () => {
  assert.deepEqual(splitCommand('   /bin/sh    -l   '), {
    file: '/bin/sh',
    args: ['-l'],
  });
});

test('splitCommand: a double-quoted first token may contain spaces', () => {
  assert.deepEqual(splitCommand('"C:\\Program Files\\app.exe" --flag x'), {
    file: 'C:\\Program Files\\app.exe',
    args: ['--flag', 'x'],
  });
});

test('splitCommand: a single-quoted first token may contain spaces', () => {
  assert.deepEqual(splitCommand("'/usr/local/bin/my shell' -l"), {
    file: '/usr/local/bin/my shell',
    args: ['-l'],
  });
});

test('splitCommand: an unterminated quote takes the rest as the file', () => {
  assert.deepEqual(splitCommand('"just-this'), { file: 'just-this', args: [] });
});

test('splitCommand: a quoted argument ANYWHERE keeps its spaces', () => {
  assert.deepEqual(
    splitCommand('"C:\\Program Files\\vibe.exe" tree --path "C:\\a b" -c'),
    {
      file: 'C:\\Program Files\\vibe.exe',
      args: ['tree', '--path', 'C:\\a b', '-c'],
    },
  );
});

test('splitCommand: a quote glued to an unquoted prefix joins into one word', () => {
  assert.deepEqual(splitCommand('vibe --path="C:\\a b" -c'), {
    file: 'vibe',
    args: ['--path=C:\\a b', '-c'],
  });
});

test('splitCommand: single and double quotes mix within one line', () => {
  assert.deepEqual(splitCommand('"a b" \'c d\' e'), {
    file: 'a b',
    args: ['c d', 'e'],
  });
});

test('splitCommand: an unterminated quote in a later arg takes the rest', () => {
  assert.deepEqual(splitCommand('vibe "unterminated arg'), {
    file: 'vibe',
    args: ['unterminated arg'],
  });
});

test('splitCommand: empty / whitespace / non-string input yields an empty file', () => {
  assert.deepEqual(splitCommand(''), { file: '', args: [] });
  assert.deepEqual(splitCommand('   '), { file: '', args: [] });
  assert.deepEqual(splitCommand(null), { file: '', args: [] });
});
