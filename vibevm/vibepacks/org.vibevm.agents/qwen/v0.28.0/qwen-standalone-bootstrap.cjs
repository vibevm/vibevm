// Package-owned adapter; supplier files are never patched.
// The supplied Node process becomes the supplier entry, preserving literal argv.
'use strict';
const fs = require('node:fs');
const path = require('node:path');
const { pathToFileURL } = require('node:url');

function ownedFile(root, parts) {
  let current = root;
  for (let i = 0; i < parts.length; i += 1) {
    current = path.join(current, parts[i]);
    const stat = fs.lstatSync(current);
    if (stat.isSymbolicLink() || (i === parts.length - 1 ? !stat.isFile() : !stat.isDirectory())) {
      throw new Error('invalid owned entry');
    }
  }
  if (fs.realpathSync(current) !== current) throw new Error('noncanonical owned entry');
  return current;
}

async function main() {
  const root = fs.realpathSync(__dirname);
  if (root !== __dirname) throw new Error('noncanonical payload root');
  const shim = ownedFile(root, ['bin', 'qwen.cmd']);
  const entry = ownedFile(root, ['lib', 'cli-entry.js']);
  process.env.QWEN_CODE_LAUNCHER_PATH = shim;
  process.argv = [process.execPath, entry, ...process.argv.slice(2)];
  await import(pathToFileURL(entry).href);
}

main().catch(() => {
  // Never relay arbitrary supplier exception data through this adapter.
  process.stderr.write('Qwen standalone entry could not be loaded.\n');
  process.exitCode = 1;
});
