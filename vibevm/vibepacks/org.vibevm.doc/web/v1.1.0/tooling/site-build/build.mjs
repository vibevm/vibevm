#!/usr/bin/env node
/**
 * The host repository's `phase:build` contribution for the complete site.
 * @scope spec://org.vibevm.core/vibevm/common/PROP-057#STACK-NATIVE-BUILD
 */
import { spawn } from "node:child_process";
import { createHash } from "node:crypto";
import {
  copyFileSync,
  existsSync,
  mkdirSync,
  readFileSync,
  readdirSync,
  rmSync,
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
const WEB_INPUTS = [
  ["package.json", join(PACKAGE_ROOT, "package.json")],
  ["pnpm-lock.yaml", join(PACKAGE_ROOT, "pnpm-lock.yaml")],
  ["pnpm-workspace.yaml", join(PACKAGE_ROOT, "pnpm-workspace.yaml")],
  ["design", join(PACKAGE_ROOT, "design")],
  ["site/package.json", join(PACKAGE_ROOT, "site", "package.json")],
  ["site/src", join(PACKAGE_ROOT, "site", "src")],
  ["site/adapters", join(PACKAGE_ROOT, "site", "adapters")],
  ["site/vite.config.ts", join(PACKAGE_ROOT, "site", "vite.config.ts")],
  ["tools", join(PACKAGE_ROOT, "tools")],
  [
    "tooling/site-build/build.mjs",
    join(PACKAGE_ROOT, "tooling", "site-build", "build.mjs"),
  ],
];

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

function filesUnder(root, directory = root, found = []) {
  if (!existsSync(directory)) return found;
  for (const name of readdirSync(directory).sort()) {
    const path = join(directory, name);
    if (statSync(path).isDirectory()) filesUnder(root, path, found);
    else found.push(relative(root, path));
  }
  return found;
}

export function reconcileTree(staging, destination) {
  const staged = filesUnder(staging);
  const wanted = new Set(staged);
  const report = { removed: 0, reused: 0, written: 0 };
  mkdirSync(destination, { recursive: true });
  for (const path of staged) {
    const source = join(staging, path);
    const target = join(destination, path);
    if (
      existsSync(target) &&
      readFileSync(source).equals(readFileSync(target))
    ) {
      report.reused += 1;
      continue;
    }
    mkdirSync(dirname(target), { recursive: true });
    copyFileSync(source, target);
    report.written += 1;
  }
  for (const path of filesUnder(destination)) {
    if (wanted.has(path)) continue;
    rmSync(join(destination, path), { force: true });
    report.removed += 1;
  }
  return report;
}

export function webInputFingerprint(trees) {
  const hash = createHash("sha256");
  const inputs = [
    ...WEB_INPUTS,
    ...trees.map((tree, index) => [`tree/${index}`, tree]),
  ];
  for (const [label, root] of inputs) {
    if (!existsSync(root)) failure(`web input is missing: ${root}`);
    const files = statSync(root).isDirectory() ? filesUnder(root) : [""];
    for (const path of files) {
      hash.update(label);
      hash.update("\0");
      hash.update(path.replaceAll("\\", "/"));
      hash.update("\0");
      hash.update(readFileSync(path === "" ? root : join(root, path)));
      hash.update("\0");
    }
  }
  return `sha256:${hash.digest("hex")}`;
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
  const context = contextPath
    ? parseContext(readFileSync(contextPath, "utf8"))
    : {
        envelope: 1,
        execution: { config: {} },
        project: { root: environment.VIBE_PROJECT_ROOT },
        run: { force: false },
      };
  if (typeof context.project.root !== "string")
    failure("VIBE_CONTEXT or VIBE_PROJECT_ROOT is required");
  if (contextPath && !replyPath)
    failure("VIBE_REPLY is required with VIBE_CONTEXT");
  const hostRoot = resolve(context.project.root);
  const outputRoot = join(hostRoot, ".vibe", "site-build", "catalogue");
  const siteRoot = siteOutput(hostRoot);
  const stagingRoot = `${siteRoot}.next`;
  const stagingServerRoot = `${stagingRoot}-server`;
  const webStatePath = join(hostRoot, ".vibe", "site-build", "web-input.json");
  const configPath = join(hostRoot, ".vibe", "site-build", "site.toml");
  mkdirSync(dirname(configPath), { recursive: true });
  writeFileSync(configPath, siteConfig(hostRoot), "utf8");

  const vibe = environment.VIBE_EXECUTABLE;
  if (!vibe || !isAbsolute(vibe))
    failure("lifecycle supplied no absolute VIBE_EXECUTABLE");
  const node = environment.VIBE_NODE_EXECUTABLE ?? process.execPath;
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
  const webFingerprint = webInputFingerprint(trees);
  let priorFingerprint;
  try {
    priorFingerprint = JSON.parse(
      readFileSync(webStatePath, "utf8"),
    ).fingerprint;
  } catch {
    priorFingerprint = undefined;
  }
  if (
    context.run?.force !== true &&
    priorFingerprint === webFingerprint &&
    existsSync(join(siteRoot, "index.html"))
  ) {
    const message = `reused the complete site from ${trees.length} unchanged trees`;
    console.log(`vibevm-doc build: ${message}`);
    if (replyPath)
      reply(replyPath, "ok", message, [
        {
          id: "vibevm-doc-site",
          kind: "directory",
          path: siteRoot.replaceAll("\\", "/"),
        },
      ]);
    return;
  }

  if (!existsSync(join(PACKAGE_ROOT, "node_modules", ".modules.yaml")))
    await command(
      "pnpm",
      ["install", "--frozen-lockfile", "--ignore-scripts"],
      PACKAGE_ROOT,
      toolEnvironment,
    );
  rmSync(stagingRoot, { recursive: true, force: true });
  rmSync(stagingServerRoot, { recursive: true, force: true });
  await command(
    node,
    [join(PACKAGE_ROOT, "tools", "build.mjs"), "static"],
    PACKAGE_ROOT,
    {
      ...toolEnvironment,
      VIBE_DOC_OUT: trees.join(delimiter),
      VIBE_SITE_DIST: relative(join(PACKAGE_ROOT, "site"), stagingRoot),
    },
  );
  const reconciled = reconcileTree(stagingRoot, siteRoot);
  rmSync(stagingRoot, { recursive: true, force: true });
  rmSync(stagingServerRoot, { recursive: true, force: true });
  console.log(
    `vibevm-doc build: ${reconciled.written} changed, ${reconciled.reused} reused, ${reconciled.removed} stale removed`,
  );
  writeFileSync(
    webStatePath,
    `${JSON.stringify({ fingerprint: webFingerprint, schema: 1 }, null, 2)}\n`,
    "utf8",
  );

  if (replyPath)
    reply(
      replyPath,
      "ok",
      `built the complete site from ${trees.length} trees; ${reconciled.written} changed and ${reconciled.reused} reused`,
      [
        {
          id: "vibevm-doc-site",
          kind: "directory",
          path: siteRoot.replaceAll("\\", "/"),
        },
      ],
    );
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
