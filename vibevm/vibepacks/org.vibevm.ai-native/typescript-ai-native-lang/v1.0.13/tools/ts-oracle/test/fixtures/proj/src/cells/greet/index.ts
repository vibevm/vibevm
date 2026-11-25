/** @scope spec://fixture/PROP-001#cell-greet */

/** The branded guest name — constructed only via parseGuestName. */
export type GuestName = string & { readonly __brand: "GuestName" };

export type ParseError = {
  readonly kind: "empty" | "too-long";
  readonly reason: string;
};

export type Result<T, E> =
  | { readonly ok: true; readonly value: T }
  | { readonly ok: false; readonly error: E };

/**
 * Parse an untrusted input into a GuestName.
 *
 * @implements spec://fixture/PROP-001#req-parse
 * @example
 * ```ts
 * const r = parseGuestName("Ada");
 * ```
 */
export function parseGuestName(input: unknown): Result<GuestName, ParseError> {
  if (typeof input !== "string" || input.trim().length === 0) {
    return { ok: false, error: { kind: "empty", reason: "empty input" } };
  }
  const cleaned = input.trim();
  if (cleaned.length > 80) {
    return { ok: false, error: { kind: "too-long", reason: "over 80 chars" } };
  }
  return { ok: true, value: cleaned as GuestName };
}

/** @implements spec://fixture/PROP-001#req-greet */
export function greet(name: GuestName): string {
  return `Hello, ${name}!`;
}
