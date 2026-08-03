/**
 * The composition root (R-001 / GUIDE-AI-NATIVE-TYPESCRIPT §7): the ONE
 * site that reads the exterior. `process.env` — pure untyped exterior —
 * is read exactly once here, narrowed to a typed mode, and a typed
 * `as const` registry dispatches to the cells. No domain cell reads the
 * environment; the `ts-flag-sites` rule (B-039) enforces that — this file
 * is named by `[typescript] composition_root`, so its own read is the one
 * legal site and stays quiet.
 */

import { greet, parseGuestName, type GuestName } from "./cells/greeting/index.ts";
import { sendOff } from "./cells/farewell/index.ts";

/**
 * The flag registry: typed data with provenance. Each mode names the cell
 * that handles it — the system's table of contents, switched exhaustively
 * rather than looked up stringly.
 */
const REGISTRY = {
  hello: { dispatch: (name: GuestName) => greet(name) },
  goodbye: { dispatch: (name: GuestName) => sendOff(name) },
} as const;

type Mode = keyof typeof REGISTRY;

/** Narrow the untyped exterior to a known mode. */
function narrowMode(raw: string | undefined): Mode {
  return raw === "goodbye" ? "goodbye" : "hello";
}

// The single exterior read — at the root, validated and typed here, then
// threaded through `compose` rather than re-read in any domain cell.
const mode: Mode = narrowMode(process.env.TS_DEMO_GREETING);

/**
 * Dispatch a guest through the registry. `process.env` appears nowhere
 * but the line above.
 *
 * @example
 * ```ts
 * compose("Ada Lovelace") === "hello, Ada Lovelace";
 * ```
 */
export function compose(input: unknown): string {
  const parsed = parseGuestName(input);
  if (!parsed.ok) {
    return parsed.error.reason;
  }
  return REGISTRY[mode].dispatch(parsed.value);
}
