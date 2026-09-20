#!/usr/bin/env node
/**
 * The host repository's `phase:build` contribution for the complete site.
 * @scope spec://org.vibevm.core/vibevm/common/PROP-057#STACK-NATIVE-BUILD
 */
import { spawn } from "node:child_process";
import {
  existsSync,
  mkdirSync,
  readFileSync,
  readdirSync,
  statSync,
  writeFileSync,
} from "node:fs";
import {
  delimiter,
  dirname,
  extname,
  isAbsolute,
  join,
  relative,
  resolve,
} from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

const PACKAGE_ROOT = resolve(
  dirname(fileURLToPath(import.meta.url)),
  "..",
  "..",
);
const FORMATS = new Set(["html", "md", "xml"]);

function failure(message) {
  throw new Error(
    `${message} (governed by ` +
      "spec://org.vibevm.core/vibevm/common/PROP-057#STACK-NATIVE-BUILD)",
  );
}

export function parseContext(bytes) {
  const value = JSON.parse(bytes);
  if (value === null || typeof value !== "object" || Array.isArray(value))
    failure("lifecycle context is not an object");
  if (value.envelope !== 1) failure("lifecycle context epoch is unsupported");
  if (value.project === null || typeof value.project !== "object")
    failure("lifecycle context has no project");
  if (typeof value.project.root !== "string")
    failure("lifecycle project has no root");
  return value;
}

export function siteConfig(hostRoot) {
  const checkout = resolve(hostRoot).replaceAll("\\", "/");
  return `schema = 1

[source.host]
git = "https://github.com/vibevm/vibevm"
ref = "main"
checkout = ${JSON.stringify(checkout)}
debounce_minutes = 0

[site]
base_path = "/doc"
origin = "https://vibevm.org"
default_theme = "dark"
featured = ["org.vibevm.core/vibevm", "org.vibevm.core/vibevm-docs"]
`;
}

export function siteOutput(hostRoot) {
  return join(resolve(hostRoot), ".vibe", "site-build", "site");
}

export function renderedTrees(outputRoot) {
  const root = join(outputRoot, ".vibe-site", "trees");
  if (!existsSync(root))
    failure(`documentation builder wrote no tree root at ${root}`);
  const trees = [];
  for (const slot of readdirSync(root).sort()) {
    const slotPath = join(root, slot);
    if (!statSync(slotPath).isDirectory()) continue;
    for (const format of readdirSync(slotPath).sort()) {
      const tree = join(slotPath, format);
      if (FORMATS.has(format) && statSync(tree).isDirectory()) trees.push(tree);
    }
  }
  if (trees.length === 0)
    failure("documentation builder produced no projections");
  return trees;
}

export function packageManagerEnvironment(
  context,
  environment = process.env,
  platform = process.platform,
) {
  const configuredRegistry = context.execution?.config?.npm_registry;
  if (
    configuredRegistry !== undefined &&
    (typeof configuredRegistry !== "string" ||
      configuredRegistry.trim().length === 0)
  )
    failure("npm_registry must be a non-empty string");

  const result = { ...environment };
  if (platform === "win32" && typeof environment.USERPROFILE === "string") {
    result.APPDATA ??= join(environment.USERPROFILE, "AppData", "Roaming");
    result.LOCALAPPDATA ??= join(environment.USERPROFILE, "AppData", "Local");
  }
  if (typeof configuredRegistry === "string")
    result.NPM_CONFIG_REGISTRY = configuredRegistry.trim();
  return result;
}

function command(name, args, cwd, environment = process.env) {
  return new Promise((accept, reject) => {
    const windows = process.platform === "win32";
    const direct = isAbsolute(name) && extname(name).toLowerCase() === ".exe";
    const executable =
      windows && !direct
        ? (environment.ComSpec ??
          join(environment.SystemRoot, "System32", "cmd.exe"))
        : name;
    const commandArgs =
      windows && !direct ? ["/d", "/s", "/c", name, ...args] : args;
    const child = spawn(executable, commandArgs, {
      cwd,
      env: environment,
      shell: false,
      stdio: ["ignore", "pipe", "pipe"],
    });
    let output = "";
    const remember = (chunk) => {
      output = `${output}${chunk}`.slice(-16_384);
    };
    child.stdout.on("data", (chunk) => {
      process.stdout.write(chunk);
      remember(chunk);
    });
    child.stderr.on("data", (chunk) => {
      process.stderr.write(chunk);
      remember(chunk);
    });
    child.once("error", reject);
    child.once("exit", (code, signal) => {
      if (code === 0) accept();
      else
        reject(
          new Error(
            `${name} exited ${code ?? signal ?? "unknown"}; output tail:\n${output}`,
          ),
        );
    });
  });
}

function reply(path, status, message, artifacts = []) {
  writeFileSync(
    path,
    JSON.stringify({ artifacts, envelope: 1, message, status, tasks: [] }),
    "utf8",
  );
}

export async function buildSite(environment = process.env) {
  const contextPath = environment.VIBE_CONTEXT;
  const replyPath = environment.VIBE_REPLY;
  if (!contextPath || !replyPath)
    failure("VIBE_CONTEXT and VIBE_REPLY are required");
  const context = parseContext(readFileSync(contextPath, "utf8"));
  const hostRoot = resolve(context.project.root);
  const outputRoot = join(hostRoot, ".vibe", "site-build", "catalogue");
  const siteRoot = siteOutput(hostRoot);
  const configPath = join(hostRoot, ".vibe", "site-build", "site.toml");
  mkdirSync(dirname(configPath), { recursive: true });
  writeFileSync(configPath, siteConfig(hostRoot), "utf8");

  const vibe = environment.VIBE_EXECUTABLE;
  if (!vibe || !isAbsolute(vibe))
    failure("lifecycle supplied no absolute VIBE_EXECUTABLE");
  const node = environment.VIBE_NODE_EXECUTABLE;
  if (!node || !isAbsolute(node))
    failure("launcher supplied no absolute VIBE_NODE_EXECUTABLE");
  const toolEnvironment = packageManagerEnvironment(context, environment);
  await command(
    vibe,
    [
      "doc",
      "build-site",
      "--config",
      configPath,
      "--out",
      outputRoot,
      "--no-web",
      "--offline",
      "--no-progress",
    ],
    hostRoot,
    environment,
  );
  const trees = renderedTrees(outputRoot);

  if (!existsSync(join(PACKAGE_ROOT, "node_modules", ".modules.yaml")))
    await command(
      "pnpm",
      ["install", "--frozen-lockfile", "--ignore-scripts"],
      PACKAGE_ROOT,
      toolEnvironment,
    );
  await command(
    node,
    [join(PACKAGE_ROOT, "tools", "build.mjs"), "static"],
    PACKAGE_ROOT,
    {
      ...toolEnvironment,
      VIBE_DOC_OUT: trees.join(delimiter),
      VIBE_SITE_DIST: relative(join(PACKAGE_ROOT, "site"), siteRoot),
    },
  );

  reply(replyPath, "ok", `built the complete site from ${trees.length} trees`, [
    {
      id: "vibevm-doc-site",
      kind: "directory",
      path: siteRoot.replaceAll("\\", "/"),
    },
  ]);
}

if (
  process.argv[1] !== undefined &&
  import.meta.url === pathToFileURL(resolve(process.argv[1])).href
) {
  buildSite().catch((error) => {
    const message = `vibevm-doc build: ${error instanceof Error ? (error.stack ?? error.message) : error}`;
    console.error(message);
    if (process.env.VIBE_PROJECT_ROOT)
      writeFileSync(
        join(
          process.env.VIBE_PROJECT_ROOT,
          ".vibe",
          "site-build",
          "failure.log",
        ),
        `${message}\n`,
        "utf8",
      );
    process.exitCode = 1;
  });
}
