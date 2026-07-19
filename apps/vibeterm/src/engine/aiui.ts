// spec://vibeterm/PROP-046#aiui, #plane-unification
// The reference surface: four verbs over the same engine, as a thin peer client. No rendering.

import type { ActionAddr } from "./address";
import { addrKey } from "./address";
import type { CallerScope, InvokeResult, ParamValues } from "./action";
import type { Ctx } from "./context";
import type { ModelView } from "./modelview";
import type { Registry } from "./registry";

export interface AiuiActionInfo {
  readonly addr: string;
  readonly name: string;
  readonly description: string;
  readonly enabled: boolean;
  readonly reason: string | null;
}

export type AiuiInvoke = (
  addr: ActionAddr,
  params: ParamValues,
  caller: CallerScope,
  signal: AbortSignal,
) => Promise<InvokeResult>;

export class Aiui {
  constructor(
    private readonly registry: Registry,
    private readonly ctx: () => Ctx,
    private readonly view: () => ModelView,
    private readonly invokeFn: AiuiInvoke,
  ) {}

  // list_actions(filter?) -- enumerate the registry with live enablement + reasons.
  list_actions(): AiuiActionInfo[] {
    const ctx = this.ctx();
    return this.registry.enumerate().map((a) => {
      const e = a.enablement(ctx);
      return {
        addr: addrKey(a.addr),
        name: a.presentation.name.default_en,
        description: a.presentation.description.default_en,
        enabled: e.visible && e.enabled,
        reason: e.reason ? e.reason.value : null,
      };
    });
  }

  // invoke(addr, args) -- the same invoke the chrome keymap calls.
  invoke(addr: ActionAddr, params: ParamValues, caller: CallerScope, signal: AbortSignal): Promise<InvokeResult> {
    return this.invokeFn(addr, params, caller, signal);
  }

  // state() -> ModelView -- the serialisable model snapshot.
  state(): ModelView {
    return this.view();
  }

  // search(query, tab?) -- sketch: a name/description substring match over the catalogue.
  // The full Search Everywhere engine lands later; this is the AIUI's read-only search verb.
  search(query: string): AiuiActionInfo[] {
    const q = query.toLowerCase();
    return this.list_actions().filter(
      (a) => a.name.toLowerCase().includes(q) || a.description.toLowerCase().includes(q),
    );
  }
}
