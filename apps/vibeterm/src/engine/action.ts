// spec://vibeterm/PROP-046#action-fields, #action-snapshot, #presentation, #invoke, #capabilities
// The Action value: address + presentation + param schema + enablement + invoke + capability + search meta.
// The resolved snapshot is immutable; change = re-resolution (#action-snapshot).

import type { ActionAddr } from "./address";
import type { Ctx, Enablement } from "./context";
import type { Msg } from "./i18n";

export type Capability = "Safe" | "Mutating" | "Dangerous";

export interface Presentation {
  readonly name: Msg;
  readonly description: Msg;
}

export interface SearchMeta {
  readonly synonyms?: readonly string[];
  readonly aliases?: readonly string[];
  readonly keywords?: readonly string[];
}

export interface ParamField {
  readonly name: string;
  readonly type: "string" | "number" | "boolean";
  readonly required: boolean;
  readonly default?: string | number | boolean;
}

export interface ParamSchema {
  readonly fields: readonly ParamField[];
}

export const NO_PARAMS: ParamSchema = { fields: [] };

export type ParamValue = string | number | boolean;
export type ParamValues = ReadonlyMap<string, ParamValue>;

export interface InvokeResult<T = unknown> {
  readonly ok: boolean;
  readonly value?: T;
  readonly error?: string;
  readonly confirmationRequired?: boolean;
}

export type CallerTrust = "local" | "peer" | "network";

export interface CallerScope {
  readonly identity: string;
  readonly trust: CallerTrust;
  readonly granted: ReadonlySet<Capability>;
}

// spec://vibeterm/PROP-046#caller-scope — a fully-trusted local caller (the chrome in the pre-MVP).
export const LOCAL_CALLER: CallerScope = {
  identity: "local-chrome",
  trust: "local",
  granted: new Set<Capability>(["Safe", "Mutating", "Dangerous"]),
};

export interface ActionCtx {
  readonly addr: ActionAddr;
  readonly params: ParamValues;
  readonly ctx: Ctx;
  readonly caller: CallerScope;
  readonly signal: AbortSignal;
}

export type InvokeFn<T = unknown> = (a: ActionCtx) => Promise<InvokeResult<T>>;

export interface Action<T = unknown> {
  readonly addr: ActionAddr;
  readonly presentation: Presentation;
  readonly capability: Capability;
  readonly params: ParamSchema;
  readonly search: SearchMeta;
  readonly enablement: (ctx: Ctx) => Enablement;
  readonly invoke: InvokeFn<T>;
}
