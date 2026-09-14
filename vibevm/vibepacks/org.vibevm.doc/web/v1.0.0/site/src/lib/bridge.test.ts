import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { describe, it } from "node:test";

import { bridgeOf } from "./bridge.ts";
import { parseDocManifest } from "./manifest.ts";

const FIXTURES = join(import.meta.dirname, "..", "fixtures");

function card(name: string) {
  const parsed = parseDocManifest(
    JSON.parse(readFileSync(join(FIXTURES, name), "utf8")),
  );
  if (!parsed.ok) throw new Error(`${name}: ${parsed.error.path}`);
  return parsed.value.package;
}

describe("a bridge's two authorships, as a page is handed them", () => {
  it("hands over both lists and the licence, under the names a page reads", () => {
    assert.deepEqual(bridgeOf(card("manifest.json")), {
      maintainers: ["The fixture's maintainer"],
      upstreamAuthors: ["The author of the bytes the fixture wraps"],
      upstreamLicense: "Apache-2.0",
    });
  });

  /**
   * A package that is not a bridge has one authorship, and the card it
   * stands on is the card it always was. «Not a bridge» is not an empty
   * bridge: an empty bridge is a bridge that named nobody.
   */
  it("answers nothing at all for a package that is not a bridge", () => {
    assert.equal(bridgeOf(card("manifest-ru.json")), undefined);
  });
});
