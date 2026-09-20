#!/usr/bin/env node
/**
 * Global Vibe application adapter for the local site launcher.
 * @scope spec://org.vibevm.core/vibevm/common/PROP-057#STACK-LOCAL-PREVIEW-LAUNCHER
 */
import { createHash, randomBytes } from "node:crypto";
import {
  cp,
  mkdir,
  readFile,
  rename,
  rm,
  stat,
  writeFile,
} from "node:fs/promises";
import {
  basename,
  dirname,
  isAbsolute,
  join,
  relative,
  resolve,
  sep,
} from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

export const CONTEXT_PROTOCOL = "vibe-application-context/1";
export const RESULT_PROTOCOL = "vibe-application-result/1";
export const APPLICATION_ID = "vibevm-doc";
export const APPLICATION_COMMANDS = Object.freeze(["vibevm-doc"]);
const PACKAGE = Object.freeze({
  group: "org.vibevm.doc",
  name: "web",
  version: "1.0.0",
});
const OPERATIONS = new Set(["install", "update", "uninstall"]);
const PACKAGE_ROOT = resolve(
  dirname(fileURLToPath(import.meta.url)),
  "..",
  "..",
);
const OMITTED_DIRECTORIES = new Set([
  ".vite",
  "dist",
  "dist-embedded",
  "node_modules",
  "playwright-report",
  "server",
  "server-embedded",
  "target",
  "test-results",
  "tmp",
]);

export function parseApplicationContext(value) {
  const object = strictObject(value, "application context", [
    "protocol",
    "operation",
    "application",
    "settingsRoot",
    "hostRoot",
    "registryRoot",
    "vibeExecutable",
    "offline",
  ]);
  if (object.protocol !== CONTEXT_PROTOCOL)
    failure("context protocol is unsupported");
  if (!OPERATIONS.has(object.operation))
    failure("application operation is unsupported");
  const application = strictObject(object.application, "application identity", [
    "id",
    "package",
    "installerPackage",
    "commands",
  ]);
  if (application.id !== APPLICATION_ID)
    failure("application id differs from vibevm-doc");
  for (const [name, identity] of [
    ["application.package", application.package],
    ["application.installerPackage", application.installerPackage],
  ]) {
    const parsed = parsePackage(identity, name);
    if (Object.keys(PACKAGE).some((key) => parsed[key] !== PACKAGE[key]))
      failure(`${name} differs from org.vibevm.doc/web@1.0.0`);
  }
  if (
    !Array.isArray(application.commands) ||
    application.commands.length !== 1 ||
    application.commands[0] !== APPLICATION_COMMANDS[0]
  )
    failure("application commands differ from vibevm-doc");
  const settingsRoot = absolutePath(object.settingsRoot, "settingsRoot");
  const hostRoot = absolutePath(object.hostRoot, "hostRoot");
  if (hostRoot !== resolve(settingsRoot, "opt", "apps", APPLICATION_ID))
    failure("application host is not bound to the selected settings root");
  const registryRoot =
    object.registryRoot === null
      ? null
      : absolutePath(object.registryRoot, "registryRoot");
  if (
    object.operation === "uninstall"
      ? registryRoot !== null
      : registryRoot === null
  )
    failure("registryRoot does not match the application operation");
  if (typeof object.offline !== "boolean") failure("offline must be boolean");
  return {
    protocol: CONTEXT_PROTOCOL,
    operation: object.operation,
    application: {
      id: APPLICATION_ID,
      package: { ...PACKAGE },
      installerPackage: { ...PACKAGE },
      commands: [...APPLICATION_COMMANDS],
    },
    settingsRoot,
    hostRoot,
    registryRoot,
    vibeExecutable: absolutePath(object.vibeExecutable, "vibeExecutable"),
    offline: object.offline,
  };
}

export async function runApplicationAdapter(
  environment = process.env,
  options = {},
) {
  const contextPath = absoluteEnvironmentPath(
    environment.VIBE_APPLICATION_CONTEXT,
    "VIBE_APPLICATION_CONTEXT",
  );
  const replyPath = absoluteEnvironmentPath(
    environment.VIBE_APPLICATION_REPLY,
    "VIBE_APPLICATION_REPLY",
  );
  const context = parseApplicationContext(
    JSON.parse(await readFile(contextPath, "utf8")),
  );
  try {
    const value = await applyOperation(context, options);
    await writeReply(replyPath, value);
    return { ok: value.status !== "failed", replyPath, value };
  } catch (error) {
    const value = result(context, "failed", null, boundedMessage(error));
    await writeReply(replyPath, value);
    return { ok: false, replyPath, value };
  }
}

export async function applyOperation(context, options = {}) {
  if (context.operation === "uninstall") return uninstall(context);
  const sourceRoot = resolve(options.sourceRoot ?? PACKAGE_ROOT);
  await requireRuntimeClosure(sourceRoot);
  const generation = randomBytes(16).toString("hex");
  const generations = join(context.hostRoot, "generations");
  const staging = join(generations, `.pending-${generation}`);
  const runtime = join(generations, generation);
  await mkdir(generations, { recursive: true });
  await rm(staging, { recursive: true, force: true });
  await cp(sourceRoot, staging, {
    recursive: true,
    filter: copyFilter(sourceRoot),
  });
  await rename(staging, runtime);
  const launchers = await writeLaunchers(context.settingsRoot, runtime);
  const management = join(runtime, "tooling", "local-site", "application.mjs");
  return result(
    context,
    "ready",
    { runtime: "node", entry: management },
    `${context.operation === "install" ? "installed" : "updated"} local vibevm.org launcher`,
    launchers,
  );
}

async function uninstall(context) {
  const root = join(context.settingsRoot, "opt", "bin");
  for (const name of ["vibevm-doc", "vibevm-doc.ps1", "vibevm-doc.cmd"])
    await rm(join(root, name), { force: true });
  process.chdir(context.settingsRoot);
  await rm(context.hostRoot, { recursive: true, force: true });
  return result(
    context,
    "undeployed",
    null,
    "removed local vibevm.org launcher",
  );
}

function copyFilter(sourceRoot) {
  return (source) => {
    const relativePath = relative(sourceRoot, source);
    if (relativePath === "") return true;
    return !relativePath
      .split(sep)
      .some((part) => OMITTED_DIRECTORIES.has(part));
  };
}

async function requireRuntimeClosure(sourceRoot) {
  for (const path of [
    "package.json",
    "pnpm-lock.yaml",
    "vibevm-doc.sh",
    "vibevm-doc.ps1",
    "tooling/local-site/application.mjs",
    "tooling/local-site/run.mjs",
  ]) {
    const metadata = await stat(join(sourceRoot, ...path.split("/")));
    if (!metadata.isFile()) failure(`runtime closure is missing ${path}`);
  }
}

async function writeLaunchers(settingsRoot, runtime) {
  const root = join(settingsRoot, "opt", "bin");
  await mkdir(root, { recursive: true });
  const rows = [
    {
      destination: join(root, "vibevm-doc"),
      mode: 0o755,
      body:
        "#!/bin/sh\n" +
        "# vibe:vibevm-doc application launcher\n" +
        `exec ${shQuote(join(runtime, "vibevm-doc.sh"))} "$@"\n`,
    },
    {
      destination: join(root, "vibevm-doc.ps1"),
      mode: 0o644,
      body:
        "\uFEFF# vibe:vibevm-doc application launcher\r\n" +
        "$ErrorActionPreference = 'Stop'\r\n" +
        `& ${psQuote(join(runtime, "vibevm-doc.ps1"))} @args\r\n` +
        "$childExit = $LASTEXITCODE\r\n" +
        "if ($null -eq $childExit) { $childExit = 1 }\r\n" +
        "exit $childExit\r\n",
    },
    {
      destination: join(root, "vibevm-doc.cmd"),
      mode: 0o644,
      body:
        "@echo off\r\n" +
        "rem vibe:vibevm-doc application launcher\r\n" +
        'powershell.exe -NoLogo -NoProfile -ExecutionPolicy Bypass -File "%~dp0vibevm-doc.ps1" %*\r\n' +
        "exit /b %ERRORLEVEL%\r\n",
    },
  ];
  for (const row of rows)
    await writeFile(row.destination, row.body, {
      encoding: "utf8",
      mode: row.mode,
    });
  return Promise.all(
    rows.map(async (row) => ({
      destination: row.destination,
      sha256: await sha256(row.destination),
    })),
  );
}

function result(context, status, management, message, launchers = []) {
  return {
    protocol: RESULT_PROTOCOL,
    operation: context.operation,
    applicationId: APPLICATION_ID,
    status,
    hostRoot: context.hostRoot,
    management,
    commands: [...APPLICATION_COMMANDS],
    launchers,
    message,
  };
}

async function sha256(path) {
  return createHash("sha256")
    .update(await readFile(path))
    .digest("hex");
}

async function writeReply(path, value) {
  await mkdir(dirname(path), { recursive: true });
  await writeFile(path, JSON.stringify(value), { encoding: "utf8", flag: "w" });
}

function parsePackage(value, name) {
  const object = strictObject(value, name, ["group", "name", "version"]);
  return {
    group: boundedString(object.group, `${name}.group`, 160),
    name: boundedString(object.name, `${name}.name`, 160),
    version: boundedString(object.version, `${name}.version`, 80),
  };
}

function strictObject(value, name, fields) {
  if (value === null || typeof value !== "object" || Array.isArray(value))
    failure(`${name} must be an object`);
  if (Object.keys(value).sort().join(",") !== [...fields].sort().join(","))
    failure(`${name} has missing or unknown fields`);
  return value;
}

function absoluteEnvironmentPath(value, name) {
  if (typeof value !== "string" || value.length === 0)
    failure(`${name} is missing`);
  return absolutePath(value, name);
}

function absolutePath(value, name) {
  const text = boundedString(value, name, 32_768);
  if (!isAbsolute(text)) failure(`${name} must be absolute`);
  return resolve(text);
}

function boundedString(value, name, maximum) {
  if (typeof value !== "string" || value.length === 0 || value.length > maximum)
    failure(`${name} is invalid`);
  if ([...value].some((character) => character.charCodeAt(0) < 32))
    failure(`${name} is invalid`);
  return value;
}

function boundedMessage(error) {
  const value =
    error instanceof Error
      ? error.message
      : "vibevm-doc application operation failed";
  return value
    .replace(/[\r\n\t]+/gu, " ")
    .replace(/[\u0000-\u001f]/gu, "")
    .slice(0, 4_000);
}

function shQuote(value) {
  return `'${value.replaceAll("'", `'"'"'`)}'`;
}

function psQuote(value) {
  return `'${value.replaceAll("'", "''")}'`;
}

function failure(message) {
  throw new Error(`vibevm-doc application contract: ${message}`);
}

async function main() {
  let replyPath;
  let context;
  try {
    replyPath = absoluteEnvironmentPath(
      process.env.VIBE_APPLICATION_REPLY,
      "VIBE_APPLICATION_REPLY",
    );
    const contextPath = absoluteEnvironmentPath(
      process.env.VIBE_APPLICATION_CONTEXT,
      "VIBE_APPLICATION_CONTEXT",
    );
    context = parseApplicationContext(
      JSON.parse(await readFile(contextPath, "utf8")),
    );
    const outcome = await runApplicationAdapter(process.env);
    process.exitCode = outcome.ok ? 0 : 1;
  } catch (error) {
    if (replyPath !== undefined && context !== undefined) {
      try {
        await writeReply(
          replyPath,
          result(context, "failed", null, boundedMessage(error)),
        );
      } catch {
        // The nonzero process exit remains authoritative when no reply can be written.
      }
    }
    process.stderr.write(`${boundedMessage(error)}\n`);
    process.exitCode = 1;
  }
}

if (
  process.argv[1] !== undefined &&
  import.meta.url === pathToFileURL(resolve(process.argv[1])).href
)
  await main();
