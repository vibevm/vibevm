// spec://vibeterm/PROP-046#registry-collision, #registry-integrity, #registry-enumeration
// Collision-erroring registry; referential integrity; full enumeration.

import type { Action } from "./action";
import type { ActionAddr } from "./address";
import { addrKey } from "./address";

export class CollisionError extends Error {
  constructor(readonly key: string) {
    super(`action collision: ${key} already registered`);
    this.name = "CollisionError";
  }
}

export class DanglingReferenceError extends Error {
  constructor(readonly ref: string) {
    super(`dangling action reference: ${ref}`);
    this.name = "DanglingReferenceError";
  }
}

export class Registry {
  private readonly map = new Map<string, Action>();

  register(action: Action): void {
    const key = addrKey(action.addr);
    if (this.map.has(key)) throw new CollisionError(key);
    this.map.set(key, action);
  }

  get(addr: ActionAddr): Action | undefined {
    return this.map.get(addrKey(addr));
  }

  require(addr: ActionAddr): Action {
    const a = this.get(addr);
    if (!a) throw new DanglingReferenceError(addrKey(addr));
    return a;
  }

  has(addr: ActionAddr): boolean {
    return this.map.has(addrKey(addr));
  }

  enumerate(): readonly Action[] {
    return [...this.map.values()];
  }

  get size(): number {
    return this.map.size;
  }
}
