/** @scope spec://ts-demo/PROP-001#cell-farewell */

import { test } from "node:test";
import assert from "node:assert/strict";
import { parseGuestName } from "../greeting/index.ts";
import { expectOk } from "../greeting/index.test.ts";
import { sendOff } from "./index.ts";

test("farewell composes greeting through the seam", () => {
  const name = expectOk(parseGuestName("Ada"));
  assert.equal(sendOff(name), "goodbye, Ada (hello, Ada)");
});
