// spec://vibeterm/PROP-047#contract-union, #transport-adapter, #consistency
// The typed preload bridge. contextIsolation: true — the chrome renderer reaches the main engine
// ONLY through this narrow, typed surface. The contract is PROP-047 §3: a Command in, an Event out.

'use strict';

const { contextBridge, ipcRenderer } = require('electron');

const PROTOCOL_VERSION = '0.1.0';

contextBridge.exposeInMainWorld('vibeterm', {
  protocolVersion: PROTOCOL_VERSION,

  // command(Command) -> Promise<{ ok: boolean, error?: string }>
  // The Command union: open | select | close | set-compact | set-theme | set-locale.
  command: (cmd) => ipcRenderer.invoke('vibeterm:command', cmd),

  // state() -> Promise<ModelView>  (the AIUI `state()` peer-read of the serialisable snapshot)
  state: () => ipcRenderer.invoke('vibeterm:state'),

  // onEvent(handler: (event: Event) => void) -> unsubscribe
  // The Event union: ready | opened | closed | active-changed | modelview.
  onEvent: (handler) => {
    const listener = (_event, frame) => {
      try {
        handler(frame && frame.payload);
      } catch (err) {
        console.error('[vibeterm preload] event handler threw:', err);
      }
    };
    ipcRenderer.on('vibeterm:event', listener);
    return () => ipcRenderer.removeListener('vibeterm:event', listener);
  },
});
