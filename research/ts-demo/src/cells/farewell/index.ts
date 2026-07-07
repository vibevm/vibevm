/** @scope spec://ts-demo/PROP-001#cell-farewell */

import { greet, type GuestName } from "../greeting/index.ts";

/**
 * Seam-only composition: this cell consumes `greeting` strictly
 * through its seam — `ts-cell-isolation` enforces exactly this.
 *
 * @implements spec://ts-demo/PROP-001#cell-farewell
 * @example
 * ```ts
 * sendOff(name) === "goodbye, Ada Lovelace (hello, Ada Lovelace)";
 * ```
 */
export function sendOff(name: GuestName): string {
  return `goodbye, ${name} (${greet(name)})`;
}
