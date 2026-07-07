/** @scope spec://ts-demo/PROP-001#cell-greeting */

import { test } from "node:test";
import assert from "node:assert/strict";
import { greet, parseGuestName } from "./index.ts";
import type { GuestName, ParseError, Result } from "./index.ts";

/** Narrow a Result to its ok arm, failing the test loudly otherwise. */
export function expectOk<T, E>(result: Result<T, E>): T {
  if (!result.ok) {
    throw new Error(`expected ok, got: ${JSON.stringify(result.error)}`);
  }
  return result.value;
}

function expectErr(result: Result<GuestName, ParseError>): ParseError {
  if (result.ok) {
    throw new Error(`expected an error, got ok: ${result.value}`);
  }
  return result.error;
}

test("a messy but valid name normalises, brands, and greets", () => {
  const name = expectOk(parseGuestName("  Ada   Lovelace "));
  assert.equal(greet(name), "hello, Ada Lovelace");
});

test("non-strings, empties, and oversized names fail as values", () => {
  const cases: Array<[unknown, string]> = [
    [42, "unprintable"],
    ["   ", "empty"],
    ["x".repeat(81), "too-long"],
  ];
  for (const [input, kind] of cases) {
    const error = expectErr(parseGuestName(input));
    assert.equal(error.kind, kind);
    assert.match(error.reason, /spec:\/\/ts-demo\/PROP-001#req-guest-name/u);
  }
});
