/**
 * diagnostic-cites-req.test.ts — the rule's valid/invalid battery and
 * the proof that every emitted message speaks the Class-F grammar.
 *
 * Uses `@typescript-eslint/rule-tester` wired to node's built-in test
 * runner (no jest/vitest). A second group drives the same rule through
 * ESLint's `Linter` to collect the raw emitted message strings and
 * assert each one passes the ported `matchesReqGrammar` acceptor — the
 * "your string passes the acceptor" check the packet requires, applied
 * to the rule's real output.
 */

import { after, describe, it } from "node:test";
import assert from "node:assert/strict";

import { Linter } from "eslint";
import parser from "@typescript-eslint/parser";
import { RuleTester } from "@typescript-eslint/rule-tester";

import { diagnosticCitesReq as rule } from "../src/diagnostic-cites-req.ts";
import { matchesReqGrammar } from "../src/req-message.ts";

// Wire the (jest-shaped) RuleTester onto node's built-in test runner so
// each valid/invalid case becomes a node:test subtest. RuleTester v8
// also needs `afterAll` for its end-of-suite fulfillment check, which
// node:test names `after`.
RuleTester.describe = describe;
RuleTester.it = it;
RuleTester.itOnly = it.only;
RuleTester.afterAll = after;

const RULE_NAME = "ai-native/diagnostic-cites-req";
// RuleTester's first arg is the rule's BARE name — it builds its own
// internal plugin namespace from it; a `plugin/rule` slash here makes it
// look for a plugin that is never registered.
const RULE_BARE_NAME = "diagnostic-cites-req";
const REQ_HEAD =
  "violates REQ discipline://typescript-ai-native-lang/cards/scaffold-f-structured-diagnostics#ops";

/** Lint one TS snippet under the rule; return the emitted messages.
 *  Flat-config form: the plugin is supplied inline in `plugins` (ESLint
 *  v9 forbids `Linter.defineRule` under flat config). */
function lint(code: string) {
  const linter = new Linter();
  return linter.verify(code, {
    plugins: {
      "ai-native": { rules: { "diagnostic-cites-req": rule } },
    },
    languageOptions: { parser },
    rules: { [RULE_NAME]: "error" },
  });
}

describe("diagnostic-cites-req (RuleTester battery)", () => {
  const ruleTester = new RuleTester({ languageOptions: { parser } });

  ruleTester.run(RULE_BARE_NAME, rule, {
    valid: [
      // A thrown error whose literal IS the full grammar — silent.
      `throw new Error("violates REQ spec://o/p#c: x is wrong; fix surface: do Y");`,
      // A plan-shaped error subclass message in the grammar — silent.
      `const e = new PlanError("violates REQ discipline://l/c#o: w; fix surface: f");`,
      // A thrown object literal whose message is in the grammar — silent.
      `throw { message: "violates REQ spec://a/b#c: w; fix surface: f" };`,
      // Not an Error-class callee (name does not end Error/Exception) — out of net.
      `const x = new Widget("anything goes here");`,
      // Error constructed WITHOUT a string literal (interpolated template) — unseen.
      "throw new Error(`expected ok, got: ${JSON.stringify(r)}`);",
      // Error constructed with no message at all — nothing to check.
      "throw new Error();",
      // First argument is a non-string literal — unseen.
      "throw new Error(42);",
    ],

    invalid: [
      {
        code: `throw new Error("boom");`,
        errors: [{ message: new RegExp(`^${escapeRegex(REQ_HEAD)}: `) }],
      },
      {
        code: `const e = new PlanError("just a plain failure");`,
        errors: [{ message: /custom diagnostic is free text/ }],
      },
      {
        code: `throw { message: "nope" };`,
        errors: [{ message: /thrown-object diagnostic is free text/ }],
      },
      {
        // A TypeError subclass message — matched by name shape.
        code: `throw new TypeError("bad type");`,
        errors: [{ message: new RegExp(`^${escapeRegex(REQ_HEAD)}: `) }],
      },
      {
        // Grammar with the fix-surface half lost — still a miss.
        code: `throw new Error("violates REQ spec://o/p#c: only why");`,
        errors: [{ message: new RegExp(`^${escapeRegex(REQ_HEAD)}: `) }],
      },
    ],
  });
});

describe("diagnostic-cites-req (emitted message passes the acceptor)", () => {
  for (const code of [
    `throw new Error("boom");`,
    `const e = new PlanError("plain failure");`,
    `throw { message: "nope" };`,
    `throw new TypeError("bad type");`,
  ]) {
    it(`emits a grammar-conformant message for: ${code}`, () => {
      const messages = lint(code);
      assert.equal(messages.length, 1, `expected one finding for ${code}`);
      assert.ok(
        matchesReqGrammar(messages[0].message),
        `emitted message must pass the Class-F acceptor, got: ${messages[0].message}`,
      );
    });
  }

  it("stays silent when the message already speaks the grammar", () => {
    const messages = lint(
      `throw new Error("violates REQ spec://o/p#c: w; fix surface: f");`,
    );
    assert.deepEqual(messages, []);
  });
});

function escapeRegex(value: string): string {
  return value.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}
