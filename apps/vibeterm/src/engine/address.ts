// spec://vibeterm/PROP-046#address-grammar
// The action address: action://<group>/<name>[?params]. (group, name) is the identity;
// the query is not. Round-trips; rejects malformed input with a typed error, never a throw.

export type Group = string & { readonly __brand: "Group" };
export type Name = string & { readonly __brand: "Name" };

export interface ActionAddr {
  readonly group: Group;
  readonly name: Name;
  readonly query: ReadonlyMap<string, string>;
}

export class AddressParseError extends Error {
  constructor(
    message: string,
    readonly input: string,
  ) {
    super(message);
    this.name = "AddressParseError";
  }
}

const GROUP_RE = /^[a-z0-9]+(\.[a-z0-9]+)*$/;
const NAME_RE = /^[a-z0-9]+([.-][a-z0-9]+)*$/;

export function group(g: string): Group {
  if (!GROUP_RE.test(g)) throw new AddressParseError(`bad group: ${g}`, g);
  return g as Group;
}

export function name(n: string): Name {
  if (!NAME_RE.test(n)) throw new AddressParseError(`bad name: ${n}`, n);
  return n as Name;
}

export function makeAddr(g: string, n: string, query?: Record<string, string>): ActionAddr {
  const q = new Map<string, string>();
  if (query) for (const [k, v] of Object.entries(query)) q.set(k, v);
  return { group: group(g), name: name(n), query: q };
}

export function displayAddr(a: ActionAddr): string {
  const base = `action://${a.group}/${a.name}`;
  if (a.query.size === 0) return base;
  const qs = [...a.query.entries()].map(([k, v]) => `${k}=${v}`).join("&");
  return `${base}?${qs}`;
}

export function parseAddr(s: string): ActionAddr {
  if (!s.startsWith("action://")) throw new AddressParseError(`missing action:// prefix: ${s}`, s);
  const rest = s.slice("action://".length);
  const qIdx = rest.indexOf("?");
  const path = qIdx >= 0 ? rest.slice(0, qIdx) : rest;
  const qs = qIdx >= 0 ? rest.slice(qIdx + 1) : "";
  const slash = path.indexOf("/");
  if (slash < 0) throw new AddressParseError(`missing group/name: ${s}`, s);
  const g = path.slice(0, slash);
  const n = path.slice(slash + 1);
  if (!g || !n) throw new AddressParseError(`empty group or name: ${s}`, s);
  const query = new Map<string, string>();
  if (qs) {
    for (const pair of qs.split("&")) {
      const eq = pair.indexOf("=");
      if (eq < 0) throw new AddressParseError(`bad query pair: ${pair}`, s);
      query.set(pair.slice(0, eq), pair.slice(eq + 1));
    }
  }
  return { group: group(g), name: name(n), query };
}

export function addrKey(a: ActionAddr): string {
  return `${a.group}/${a.name}`;
}

export function addrEqual(a: ActionAddr, b: ActionAddr): boolean {
  return addrKey(a) === addrKey(b);
}
