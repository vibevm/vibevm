/**
 * req-message.test.ts — pins the TypeScript port of the Class-F grammar
 * to the engine's semantics. Mirrors the Rust doctests on
 * `req_message` / `matches_req_grammar` verbatim, then exercises the
 * acceptor's edge behaviour so the port is shown to be faithful.
 */

import { describe, it } from "node:test";
import assert from "node:assert/strict";

import { matchesReqGrammar, reqMessage } from "../src/req-message.ts";

describe("reqMessage", () => {
  it("renders the Class-F grammar exactly (the engine twin)", () => {
    assert.equal(
      reqMessage("spec://p/d#a", "why", "where"),
      "violates REQ spec://p/d#a: why; fix surface: where",
    );
  });

  it("renders the TS rule's own URI", () => {
    const msg = reqMessage(
      "discipline://typescript-ai-native-lang/cards/scaffold-f-structured-diagnostics#ops",
      "free text",
      "use reqMessage",
    );
    assert.equal(
      msg,
      "violates REQ discipline://typescript-ai-native-lang/cards/scaffold-f-structured-diagnostics#ops: " +
        "free text; fix surface: use reqMessage",
    );
  });

  it("its own output passes the acceptor for every known scheme", () => {
    for (const uri of [
      "spec://org/p#x",
      "discipline://typescript-ai-native-lang/guide#scaffold-f",
      "misra://c-2012/r3.1",
    ]) {
      const msg = reqMessage(uri, "what went wrong", "where to fix");
      assert.ok(matchesReqGrammar(msg), `${uri} -> ${msg} should pass`);
    }
  });
});

describe("matchesReqGrammar (ported acceptor — faithful to the engine)", () => {
  // The engine's two doctest assertions, mirrored one-to-one.
  it("accepts the canonical engine example", () => {
    assert.ok(matchesReqGrammar(reqMessage("spec://p/d#a", "why", "where")));
  });

  it("rejects bare free text (engine doctest)", () => {
    assert.equal(matchesReqGrammar("Error: invalid configuration"), false);
  });

  // Acceptor edge behaviour — the port's exact semantics, recorded.
  it("rejects a message missing the `violates REQ ` prefix", () => {
    assert.equal(
      matchesReqGrammar("spec://p/d#a: why; fix surface: where"),
      false,
    );
  });

  it("rejects an unknown scheme after the prefix", () => {
    assert.equal(
      matchesReqGrammar("violates REQ http://x: why; fix surface: where"),
      false,
    );
  });

  it("rejects a grammar missing the `; fix surface:` half", () => {
    assert.equal(matchesReqGrammar("violates REQ spec://p/d#a: why"), false);
  });

  it("accepts the empty-string fix surface is NOT a pass without the marker", () => {
    assert.equal(
      matchesReqGrammar("violates REQ spec://p/d#a: why; fix surface:"),
      false,
    );
  });

  it("accepts a message whose `: ` is supplied by the fix marker (port quirk, faithful)", () => {
    // The acceptor only checks `: ` appears *somewhere* in the
    // remainder — here it comes from `fix surface: `, not after the
    // URI. The engine behaves identically; this test pins that.
    assert.ok(
      matchesReqGrammar(
        "violates REQ spec://p/d#a; fix surface: where to fix",
      ),
    );
  });
});
