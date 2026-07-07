/** @scope spec://ts-demo/PROP-001#cell-text */

/**
 * Collapse runs of whitespace and trim — the one normalisation every
 * inbound string gets before validation.
 *
 * @example
 * ```ts
 * normalise("  Ada   Lovelace ") === "Ada Lovelace";
 * ```
 */
export function normalise(input: string): string {
  return input.trim().replace(/\s+/gu, " ");
}
