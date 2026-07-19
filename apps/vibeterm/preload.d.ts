// Types for the typed preload bridge exposed on `window.vibeterm` (preload.cjs).
// The contract is PROP-047 §3; the runtime types live in src/engine/protocol.ts.

export interface VibetermFramePayload {
  readonly t: string;
  readonly [k: string]: unknown;
}

export interface VibetermBridge {
  readonly protocolVersion: string;
  readonly command: (cmd: VibetermFramePayload) => Promise<{ ok: boolean; error?: string }>;
  readonly state: () => Promise<unknown>;
  readonly onEvent: (handler: (event: VibetermFramePayload) => void) => () => void;
}

declare global {
  interface Window {
    readonly vibeterm: VibetermBridge;
  }
}
