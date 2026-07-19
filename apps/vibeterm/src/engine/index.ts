// spec://vibeterm/PROP-046#overview
// The render-free vibeterm engine core. No Solid/DOM/Electron imports anywhere in this tree
// (the dependency-boundary discipline + tsconfig.engine.json lib enforce it). This module is the
// single source the main process (main.cjs) imports and the conformance golden exercises.

export * from "./address";
export * from "./i18n";
export * from "./context";
export * from "./action";
export * from "./registry";
export * from "./modelview";
export * from "./protocol";
export * from "./tabs";
export * from "./aiui";
