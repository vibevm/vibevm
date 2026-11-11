#!/usr/bin/env node
// carrier.mjs — compose one launch's identity carrier for Qwen Code and
// prove, **on the installed package**, that it does what the launch needs.
//
// This file is vibe's own program and carries no word of anyone else's
// prompt. It is shipped inside the `vibe` binary and written to vibe's
// settings tree at launch, because the one thing it must do cannot be done
// from Rust: sampling Qwen Code's identity sentence means *calling* the
// installed program's own `getCoreSystemPrompt` (PROP-076 `#SYS-QWEN`).
//
//   node carrier.mjs <qwen-package-dir> --lane <AGENT-LANE.md>
//        --out-prefix <path-prefix> --mode interactive|headless|acp [--styled]
//
// SAMPLE   the identity sentence is the first paragraph of the base prompt
//          the installed package builds for this launch's session shape —
//          read exactly as upstream's own test reads it
//          (`getCoreSystemPrompt().split('\n\n', 1)[0]`). No copy of Qwen
//          Code's wording lives in vibe.
// COMPOSE  `<identity>\n\n<lane bytes>` into `<out-prefix><digest>.md`,
//          content-addressed on those bytes, written only when it is not
//          already there, through a temporary file and one rename. A running
//          Qwen Code session re-reads the file on every prompt rebuild and
//          throws when it is gone, so a later launch carrying a different
//          lane must write a DIFFERENT file rather than rewrite this one.
// PROVE    with `QWEN_SYSTEM_IDENTITY_MD=<carrier>` the same function
//          returns the plain prompt with exactly `<lane>\n` inserted right
//          after `<identity>\n\n`, and nothing else changed — for an ungated
//          and for a gated tool surface, so the claim does not rest on one
//          shape. A package where the variable is no longer honoured, or
//          replaces more than the first paragraph, fails here and the
//          launcher starts softly instead.
//
// It prints one JSON line and exits 0 only when every check held. It reads
// no network, starts no session and calls no model.
import fs from 'node:fs';
import path from 'node:path';
import crypto from 'node:crypto';
import { pathToFileURL } from 'node:url';

const args = process.argv.slice(2);
const pkgDir = args[0];
const opt = (name, fallback) => {
  const i = args.indexOf(`--${name}`);
  return i < 0 ? fallback : args[i + 1];
};
const flag = (name) => args.includes(`--${name}`);
const fail = (reason, extra = {}) => {
  console.log(JSON.stringify({ ok: false, reason, ...extra }));
  process.exit(1);
};

const lanePath = opt('lane');
const outPrefix = opt('out-prefix');
const mode = opt('mode', 'interactive');
const envName = opt('env-name', 'QWEN_SYSTEM_IDENTITY_MD'); // test hook only
if (!pkgDir || !lanePath || !outPrefix) {
  fail('usage: <package-dir> --lane F --out-prefix P --mode M [--styled]');
}
if (!['interactive', 'headless', 'acp'].includes(mode)) fail(`unknown mode ${mode}`);

// The three variables that change what the function returns. The launcher is
// about to set one of them; none may leak into the sampling, and
// `QWEN_WRITE_SYSTEM_MD` would make the call write a dump file besides.
for (const name of ['QWEN_SYSTEM_MD', 'QWEN_SYSTEM_IDENTITY_MD', 'QWEN_WRITE_SYSTEM_MD']) {
  delete process.env[name];
}

let version = 'unknown';
try {
  version = JSON.parse(fs.readFileSync(path.join(pkgDir, 'package.json'), 'utf8')).version;
} catch {
  /* reported as unknown; the caller has already matched it against --version */
}

// Locate the chunk by what it EXPORTS, because the bundle's file names are
// content hashes and change with every release.
const chunksDir = path.join(pkgDir, 'chunks');
let chunk = null;
try {
  for (const name of fs.readdirSync(chunksDir)) {
    if (!name.endsWith('.js')) continue;
    const text = fs.readFileSync(path.join(chunksDir, name), 'utf8');
    if (!text.includes('getCoreSystemPrompt')) continue;
    const lists = text.match(/export\{[^}]*\}/g) ?? [];
    if (lists.some((list) => /(^|[{,])getCoreSystemPrompt([,}]|$)/.test(list.slice(6)))) {
      if (chunk) fail('more than one chunk exports getCoreSystemPrompt', { version });
      chunk = name;
    }
  }
} catch (error) {
  fail(`cannot read the package's chunks: ${error.message}`, { version });
}
if (!chunk) fail('no chunk exports getCoreSystemPrompt', { version });

let build;
try {
  build = (await import(pathToFileURL(path.join(chunksDir, chunk)).href)).getCoreSystemPrompt;
} catch (error) {
  fail(`cannot import ${chunk}: ${error.message}`, { version });
}
if (typeof build !== 'function') fail('getCoreSystemPrompt is not callable', { version });

// A style object with no wording of ours in it: the sentence's styled form
// points AT the style and never quotes it, so any definition samples the
// same bytes.
const dummyStyle = {
  name: 'x',
  source: 'user',
  description: 'x',
  keepCodingInstructions: true,
  prompt: 'x',
};
const styled = flag('styled');
const style = styled ? dummyStyle : undefined;
const firstParagraph = (text) => text.split('\n\n', 1)[0];
// Two tool surfaces: none (every tool assumed declared) and a small declared
// set, so the proof covers both branches of the prompt's tool gating.
const surfaces = [
  undefined,
  { declaredTools: new Set(['read_file', 'glob', 'grep_search', 'skill']), agentReachable: false },
];
const promptFor = (surface) => build(undefined, '', undefined, mode, style, false, false, surface);

// ---- SAMPLE ---------------------------------------------------------------
const identity = firstParagraph(promptFor(undefined));
if (!identity || identity.includes('\n') || identity.length > 2000) {
  fail('the first paragraph of the base prompt is not a single short sentence', { version });
}
// A guard against a changed signature. If a later release moved or dropped
// the `interactionMode` / `outputStyle` parameters, every call would quietly
// fall back to one default sentence — so the three modes and the styled form
// must be four different single-line sentences, without this file knowing a
// word of any of them.
const sentences = new Set(
  ['interactive', 'headless', 'acp'].map((m) =>
    firstParagraph(build(undefined, '', undefined, m, undefined)),
  ),
);
sentences.add(firstParagraph(build(undefined, '', undefined, mode, dummyStyle)));
sentences.add(identity);
if (sentences.size !== 4 || [...sentences].some((s) => !s || s.includes('\n'))) {
  fail(
    'getCoreSystemPrompt no longer varies its first sentence by mode and style the way this carrier expects',
    { version },
  );
}

// ---- COMPOSE --------------------------------------------------------------
const lane = fs.readFileSync(lanePath, 'utf8');
if (lane !== lane.trim() + '\n') {
  fail('the lane must end with exactly one LF and carry no other edge whitespace', { version });
}
const carrier = `${identity}\n\n${lane}`;
const carrierSha = crypto.createHash('sha256').update(carrier).digest('hex');
const outPath = `${outPrefix}${carrierSha.slice(0, 16)}.md`;
let written = false;
let current = null;
try {
  current = fs.readFileSync(outPath, 'utf8');
} catch {
  /* absent */
}
if (current !== carrier) {
  fs.mkdirSync(path.dirname(outPath), { recursive: true });
  const temp = `${outPath}.${process.pid}.tmp`;
  fs.writeFileSync(temp, carrier);
  fs.renameSync(temp, outPath);
  written = true;
}

// ---- PROVE ----------------------------------------------------------------
const results = [];
for (const surface of surfaces) {
  delete process.env[envName];
  const plain = promptFor(surface);
  process.env[envName] = outPath;
  const withLane = promptFor(surface);
  delete process.env[envName];
  // The difference is ONE insertion of `<lane>␤` after the identity
  // paragraph, and nothing else anywhere in the prompt.
  const expected = `${identity}\n\n${lane}\n${plain.slice(identity.length + 2)}`;
  results.push({
    gated: surface !== undefined,
    plainBytes: Buffer.byteLength(plain),
    withLaneBytes: Buffer.byteLength(withLane),
    startsWithIdentity: plain.startsWith(`${identity}\n\n`),
    exact: withLane === expected,
    laneOnce: withLane.split(lane.trimEnd()).length - 1 === 1,
  });
}
const ok = results.every((r) => r.startsWithIdentity && r.exact && r.laneOnce);
console.log(
  JSON.stringify({
    ok,
    reason: ok
      ? undefined
      : `${envName} did not produce "the plain prompt plus the lane after its identity sentence"`,
    version,
    chunk,
    mode,
    styled,
    identityBytes: Buffer.byteLength(identity),
    laneBytes: Buffer.byteLength(lane),
    laneOffset: Buffer.byteLength(identity) + 2,
    carrier: outPath,
    carrierBytes: Buffer.byteLength(carrier),
    carrierSha256: carrierSha,
    written,
    results,
  }),
);
process.exit(ok ? 0 : 1);
