// spec://vibeterm/PROP-046#overview, spec://vibeterm/PROP-047#mvc
// Engine cell tests (vitest). These double as the seed of the conformance golden (PROP-046 §9):
// the behaviours the identity-grammar promises, pinned in TS.

import { describe, expect, it } from "vitest";
import { addrKey, displayAddr, makeAddr, parseAddr } from "./address";
import { LOCAL_CALLER, NO_PARAMS, type Action } from "./action";
import { ENABLED } from "./context";
import { Registry, CollisionError, DanglingReferenceError } from "./registry";
import { msg } from "./i18n";
import { emptyModelView, apply, resetTabCounter } from "./tabs";
import { Aiui } from "./aiui";
import {
  checkVersion,
  commandFrame,
  decode,
  encode,
  eventFrame,
  PROTOCOL_VERSION,
  ProtocolVersionError,
} from "./protocol";

describe("address", () => {
  it("round-trips parse(display(a)) for the identity part", () => {
    const a = makeAddr("vibeterm", "tab.open");
    expect(parseAddr(displayAddr(a))).toEqual(a);
  });

  it("round-trips with a query", () => {
    const a = makeAddr("vibeterm", "pane.split", { dir: "right", target: "left" });
    const back = parseAddr(displayAddr(a));
    expect(addrKey(back)).toBe("vibeterm/pane.split");
    expect(back.query.get("dir")).toBe("right");
    expect(back.query.get("target")).toBe("left");
  });

  it("rejects a malformed address with a typed error", () => {
    expect(() => parseAddr("not-an-action")).toThrow();
    expect(() => parseAddr("action://BadGroup/name")).toThrow();
    expect(() => parseAddr("action://vibeterm/")).toThrow();
  });
});

describe("registry", () => {
  function action(name: string): Action {
    return {
      addr: makeAddr("vibeterm", name),
      presentation: { name: msg(name), description: msg(`the ${name} action`) },
      capability: "Safe",
      params: NO_PARAMS,
      search: {},
      enablement: () => ENABLED,
      invoke: async () => ({ ok: true }),
    };
  }

  it("errors on collision (hard, deterministic)", () => {
    const r = new Registry();
    r.register(action("tab.open"));
    expect(() => r.register(action("tab.open"))).toThrow(CollisionError);
  });

  it("require() throws a dangling reference for an unknown address", () => {
    const r = new Registry();
    expect(() => r.require(makeAddr("vibeterm", "missing"))).toThrow(DanglingReferenceError);
  });

  it("enumerates every registered action", () => {
    const r = new Registry();
    r.register(action("tab.open"));
    r.register(action("tab.select"));
    expect(r.size).toBe(2);
    expect(r.enumerate().map((a) => addrKey(a.addr)).sort()).toEqual([
      "vibeterm/tab.open",
      "vibeterm/tab.select",
    ]);
  });
});

describe("tabs cell (the ModelView single-writer)", () => {
  it("open creates a tab and activates it", () => {
    resetTabCounter();
    const start = emptyModelView("w1");
    const { view, events } = apply(start, { t: "open" });
    expect(view.tabs.size).toBe(1);
    expect(view.activeTab).not.toBeNull();
    const opened = view.tabs.get(view.activeTab as never);
    expect(opened?.active).toBe(true);
    expect(events.map((e) => e.type)).toEqual(["opened", "active-changed"]);
  });

  it("select switches the active tab without losing others", () => {
    resetTabCounter();
    let v = emptyModelView("w1");
    v = apply(v, { t: "open" }).view;
    v = apply(v, { t: "open" }).view;
    const first = v.windows[0]!.tabs[0]!;
    v = apply(v, { t: "select", tabId: first }).view;
    expect(v.activeTab).toBe(first);
    expect(v.tabs.get(first)?.active).toBe(true);
    expect(v.tabs.size).toBe(2);
  });

  it("close removes a tab and re-activates a survivor", () => {
    resetTabCounter();
    let v = emptyModelView("w1");
    v = apply(v, { t: "open" }).view;
    v = apply(v, { t: "open" }).view;
    const first = v.windows[0]!.tabs[0]!;
    v = apply(v, { t: "close", tabId: first }).view;
    expect(v.tabs.size).toBe(1);
    expect(v.tabs.has(first)).toBe(false);
    expect(v.activeTab).not.toBe(first);
  });
});

describe("protocol", () => {
  it("encodes and decodes a frame", () => {
    const f = commandFrame({ t: "open" });
    const back = decode(encode(f));
    expect(back).toEqual(f);
    expect(back.v).toBe(PROTOCOL_VERSION);
  });

  it("checkVersion throws on a mismatch", () => {
    expect(() => checkVersion({ v: "9.9.9", kind: "command", payload: { t: "open" } })).toThrow(
      ProtocolVersionError,
    );
  });

  it("rejects an invalid frame", () => {
    expect(() => decode('{"v":"0.1.0","kind":"nope","payload":{}}')).toThrow();
  });

  it("frames carry no Electron types (serialisable JSON)", () => {
    const f = eventFrame({ t: "ready" });
    expect(() => JSON.parse(encode(f))).not.toThrow();
  });
});

describe("aiui (the reference surface)", () => {
  it("state() returns the ModelView; list_actions() enumerates the registry", () => {
    resetTabCounter();
    const registry = new Registry();
    const v = apply(emptyModelView("w1"), { t: "open" }).view;
    const aiui = new Aiui(
      registry,
      () => ({ size: 0, keys: () => [], get: () => undefined, has: () => false, set: () => {} }) as never,
      () => v,
      async () => ({ ok: true }),
    );
    expect(aiui.state()).toBe(v);
    expect(aiui.list_actions()).toEqual([]);
  });
});

// The LOCAL_CALLER is the fully-trusted chrome caller in the pre-MVP (PROP-046 #caller-scope).
describe("caller scope", () => {
  it("the local chrome caller is granted every capability", () => {
    expect(LOCAL_CALLER.granted.has("Dangerous")).toBe(true);
    expect(LOCAL_CALLER.trust).toBe("local");
  });
});
