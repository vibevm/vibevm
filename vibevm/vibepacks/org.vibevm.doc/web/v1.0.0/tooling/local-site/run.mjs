#!/usr/bin/env node
/**
 * Local loopback launcher for the complete Qwik site.
 * @scope spec://org.vibevm.core/vibevm/common/PROP-057#STACK-LOCAL-PREVIEW-LAUNCHER
 */
import { spawn } from "node:child_process";
import { createReadStream, existsSync } from "node:fs";
import { stat } from "node:fs/promises";
import { createServer } from "node:http";
import { dirname, extname, join, normalize, resolve, sep } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

const PACKAGE_ROOT = resolve(
  dirname(fileURLToPath(import.meta.url)),
  "..",
  "..",
);
const DEFAULT_PORT = 4322;

const CONTENT_TYPES = Object.freeze({
  ".css": "text/css; charset=utf-8",
  ".html": "text/html; charset=utf-8",
  ".ico": "image/x-icon",
  ".jpeg": "image/jpeg",
  ".jpg": "image/jpeg",
  ".js": "text/javascript; charset=utf-8",
  ".json": "application/json; charset=utf-8",
  ".map": "application/json; charset=utf-8",
  ".md": "text/markdown; charset=utf-8",
  ".png": "image/png",
  ".svg": "image/svg+xml; charset=utf-8",
  ".txt": "text/plain; charset=utf-8",
  ".webp": "image/webp",
  ".woff2": "font/woff2",
  ".xml": "application/xml; charset=utf-8",
});

export function parseArguments(argv) {
  const value = { build: true, install: true, port: DEFAULT_PORT };
  for (let index = 0; index < argv.length; index += 1) {
    const argument = argv[index];
    if (argument === "--no-build") value.build = false;
    else if (argument === "--no-install") value.install = false;
    else if (argument === "--port") {
      const spelling = argv[index + 1];
      if (spelling === undefined) throw new Error("--port requires a value");
      const port = Number.parseInt(spelling, 10);
      if (!/^\d+$/u.test(spelling) || port < 0 || port > 65_535)
        throw new Error("--port must be an integer from 0 through 65535");
      value.port = port;
      index += 1;
    } else if (argument === "--help" || argument === "-h") value.help = true;
    else throw new Error(`unknown argument: ${argument}`);
  }
  return value;
}

export function staticRoot(root = PACKAGE_ROOT, environment = process.env) {
  const named = (environment.VIBE_SITE_DIST ?? "").trim();
  return join(root, "site", named === "" ? "dist" : named);
}

export function requestFile(root, requestUrl) {
  let pathname;
  try {
    pathname = decodeURIComponent(
      new URL(requestUrl, "http://127.0.0.1").pathname,
    );
  } catch {
    return null;
  }
  if (pathname.includes("\\") || pathname.includes("\0")) return null;
  const relative = pathname.replace(/^\/+/, "");
  if (relative === "") return join(resolve(root), "index.html");
  const candidate = resolve(root, normalize(relative));
  const prefix = `${resolve(root)}${sep}`;
  if (candidate !== resolve(root) && !candidate.startsWith(prefix)) return null;
  if (pathname.endsWith("/")) return join(candidate, "index.html");
  return candidate;
}

function command(name, args, root) {
  return new Promise((accept, reject) => {
    const windows = process.platform === "win32";
    const executable = windows ? (process.env.ComSpec ?? "cmd.exe") : name;
    const commandArgs = windows ? ["/d", "/s", "/c", name, ...args] : args;
    const child = spawn(executable, commandArgs, {
      cwd: root,
      env: process.env,
      shell: false,
      stdio: "inherit",
    });
    child.once("error", reject);
    child.once("exit", (code, signal) => {
      if (code === 0) accept();
      else
        reject(
          new Error(
            `${name} ${args.join(" ")} exited ${code ?? signal ?? "unknown"}`,
          ),
        );
    });
  });
}

async function prepare(options, root) {
  if (
    options.install &&
    !existsSync(join(root, "node_modules", ".modules.yaml"))
  ) {
    await command("corepack", ["pnpm", "install", "--frozen-lockfile"], root);
  }
  if (options.build) await command("corepack", ["pnpm", "build:static"], root);
}

export function createStaticServer(root) {
  return createServer(async (request, response) => {
    const requested = requestFile(root, request.url ?? "/");
    if (requested === null || requested.includes(`${sep}.vibe-site${sep}`)) {
      response.writeHead(404, { "content-type": "text/plain; charset=utf-8" });
      response.end("not found\n");
      return;
    }
    let file = requested;
    try {
      const metadata = await stat(file);
      if (metadata.isDirectory()) file = join(file, "index.html");
      const found = await stat(file);
      if (!found.isFile()) throw new Error("not a file");
    } catch {
      const fallback = join(root, "404.html");
      if (existsSync(fallback)) {
        response.writeHead(404, { "content-type": "text/html; charset=utf-8" });
        createReadStream(fallback).pipe(response);
      } else {
        response.writeHead(404, {
          "content-type": "text/plain; charset=utf-8",
        });
        response.end("not found\n");
      }
      return;
    }
    response.writeHead(200, {
      "cache-control": "no-store",
      "content-type":
        CONTENT_TYPES[extname(file).toLowerCase()] ??
        "application/octet-stream",
      "x-content-type-options": "nosniff",
    });
    createReadStream(file).pipe(response);
  });
}

export async function runLocalSite(
  argv = process.argv.slice(2),
  root = PACKAGE_ROOT,
) {
  const options = parseArguments(argv);
  if (options.help) {
    process.stdout.write(
      "usage: vibevm-doc [--port PORT] [--no-build] [--no-install]\n" +
        "Build and serve the Qwik site on 127.0.0.1 (default port 4322).\n",
    );
    return null;
  }
  await prepare(options, root);
  const output = staticRoot(root);
  if (!existsSync(join(output, "index.html")))
    throw new Error(
      `${output}: no static site; remove --no-build or run pnpm build:static`,
    );
  const server = createStaticServer(output);
  await new Promise((accept, reject) => {
    server.once("error", reject);
    server.listen(options.port, "127.0.0.1", accept);
  });
  const address = server.address();
  const port =
    typeof address === "object" && address !== null
      ? address.port
      : options.port;
  process.stdout.write(`vibevm-doc: http://127.0.0.1:${port}/\n`);
  return server;
}

async function main() {
  try {
    await runLocalSite();
  } catch (error) {
    process.stderr.write(
      `${error instanceof Error ? error.message : "vibevm-doc failed"}\n`,
    );
    process.exitCode = 1;
  }
}

if (
  process.argv[1] !== undefined &&
  import.meta.url === pathToFileURL(resolve(process.argv[1])).href
)
  await main();
