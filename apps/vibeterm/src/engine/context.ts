// spec://vibeterm/PROP-046#context-snapshot
// A typed context snapshot: a branded-symbol-keyed Map. No stringly keys, no unchecked casts.
// Enablement is a pure, fast function over this snapshot.

import type { ResolvedLabel } from "./i18n";

export interface CtxKey<T> {
  readonly __key: symbol;
  readonly __t: T;
}

export function ctxKey<T>(description: string): CtxKey<T> {
  return { __key: Symbol(description), __t: undefined as unknown as T };
}

export class Ctx {
  private readonly map = new Map<symbol, unknown>();

  set<T>(key: CtxKey<T>, value: T): void {
    this.map.set(key.__key, value);
  }

  get<T>(key: CtxKey<T>): T | undefined {
    return this.map.get(key.__key) as T | undefined;
  }

  has<T>(key: CtxKey<T>): boolean {
    return this.map.has(key.__key);
  }

  keys(): readonly symbol[] {
    return [...this.map.keys()];
  }

  get size(): number {
    return this.map.size;
  }
}

export interface Enablement {
  readonly visible: boolean;
  readonly enabled: boolean;
  readonly reason: ResolvedLabel | null;
}

export const ENABLED: Enablement = { visible: true, enabled: true, reason: null };

export function disabledBy(reason: ResolvedLabel): Enablement {
  return { visible: true, enabled: false, reason };
}

export function hidden(): Enablement {
  return { visible: false, enabled: false, reason: null };
}
