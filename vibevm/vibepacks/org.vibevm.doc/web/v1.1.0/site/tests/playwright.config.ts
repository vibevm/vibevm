/** @scope spec://org.vibevm.core/vibevm/common/PROP-057#READER-SETTINGS */

/**
 * The end-to-end run: one browser, headless, over the BUILT site.
 *
 * It reads `site/dist` through the little static server beside this
 * file, so what is measured is the bytes a deployment serves — not a dev
 * server's transformed modules and not a component rendered in
 * isolation. The reader's behaviours are the only part of this package
 * that a type checker cannot judge at all: they exist to move a real
 * document in a real browser, and nothing short of one can say whether
 * a block ends up in the middle of the window.
 *
 * One worker and no retries. These tests read and write `localStorage`
 * under one origin and one of them is about what a reload restores, so
 * parallel workers would be two readers sharing one memory; and a retry
 * that turns a failure green is a failure that will come back.
 *
 * Two knobs, both from the environment and both with the defaults the
 * run always had: `VIBE_E2E_PORT` moves the server's port, and
 * `VIBE_SITE_DIST` — the same name the build reads (`tools/out-dir.mjs`)
 * — names the output it serves. They exist so that two checkouts of this
 * package can run their suites on one machine at the same time without
 * one of them measuring the other's build.
 */

import { defineConfig, devices } from "@playwright/test";

const PORT = Number.parseInt(process.env["VIBE_E2E_PORT"] ?? "4173", 10);
const DIST = (process.env["VIBE_SITE_DIST"] ?? "").trim() || "dist";
const ORIGIN = `http://127.0.0.1:${PORT}`;

export default defineConfig({
  testDir: ".",
  testMatch: /.*\.spec\.ts/,
  fullyParallel: false,
  workers: 1,
  retries: 0,
  reporter: [["list"]],
  use: {
    baseURL: ORIGIN,
    trace: "off",
  },
  projects: [{ name: "chromium", use: { ...devices["Desktop Chrome"] } }],
  webServer: {
    command: `node serve.mjs ${PORT} ${DIST}`,
    url: `${ORIGIN}/doc/`,
    reuseExistingServer: false,
    stdout: "ignore",
    stderr: "pipe",
  },
});
