import { test } from 'node:test';
import assert from 'node:assert/strict';

import { parseArgs, defaultShell, splitCommand } from '../lib/args.mjs';

// ---------------------------------------------------------------------------
// parseArgs
// ---------------------------------------------------------------------------

test('parseArgs: defaults when no flags are present', () => {
  assert.deepEqual(parseArgs([]), { exec: null, cols: 84, rows: 30 });
  assert.deepEqual(parseArgs(['electron', '.']), {
    exec: null,
    cols: 84,
    rows: 30,
  });
});

test('parseArgs: --exec, --cols, --rows in space-separated form', () => {
  assert.deepEqual(
    parseArgs(['--exec', 'vibe tree -c', '--cols', '100', '--rows', '40']),
    { exec: 'vibe tree -c', cols: 100, rows: 40 },
  );
});

test('parseArgs: inline --flag=value form (only the first = splits)', () => {
  assert.deepEqual(
    parseArgs(['--exec=cmd.exe /K echo hi', '--cols=120', '--rows=50']),
    { exec: 'cmd.exe /K echo hi', cols: 120, rows: 50 },
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
  assert.deepEqual(parseArgs(argv), { exec: '/bin/bash -l', cols: 90, rows: 30 });
});

test('parseArgs: malformed / missing values fall back to defaults', () => {
  // Non-numeric cols, zero rows (out of range), and a dangling --exec.
  assert.deepEqual(parseArgs(['--cols', 'abc', '--rows', '0']), {
    exec: null,
    cols: 84,
    rows: 30,
  });
  assert.deepEqual(parseArgs(['--exec']), { exec: null, cols: 84, rows: 30 });
  assert.deepEqual(parseArgs(['--cols', '100abc']), {
    exec: null,
    cols: 84,
    rows: 30,
  });
});

test('parseArgs: a dangling flag does not swallow the next flag', () => {
  // --exec has no value (next token is another flag) → exec stays null,
  // and --cols is still parsed.
  assert.deepEqual(parseArgs(['--exec', '--cols', '100']), {
    exec: null,
    cols: 100,
    rows: 30,
  });
});

test('parseArgs: an empty or whitespace-only --exec is treated as absent', () => {
  assert.equal(parseArgs(['--exec', '']).exec, null);
  assert.equal(parseArgs(['--exec=   ']).exec, null);
});

test('parseArgs: is robust to non-array / junk input', () => {
  assert.deepEqual(parseArgs(undefined), { exec: null, cols: 84, rows: 30 });
  assert.deepEqual(parseArgs(null), { exec: null, cols: 84, rows: 30 });
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

test('splitCommand: empty / whitespace / non-string input yields an empty file', () => {
  assert.deepEqual(splitCommand(''), { file: '', args: [] });
  assert.deepEqual(splitCommand('   '), { file: '', args: [] });
  assert.deepEqual(splitCommand(null), { file: '', args: [] });
});
