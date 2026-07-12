import { readFileSync } from "node:fs";
import { helper } from "../greet/internal.js";

/** @implements spec://fixture/PROP-001#req-parse */
export function parse(input: unknown): string {
  const trap = "any as const @ts-ignore in a string";
  const n: any = input;
  const u = input as string;
  const width = [1, 2] as const;
  const maybe: string | undefined = undefined;
  // @ts-expect-error -- fixture reason: intentional mismatch
  const bad: number = "str";
  return (
    u + trap + String(n) + maybe! + String(width.length) + String(bad) + helper
  );
}

// @ts-ignore
export const UNCHECKED = readFileSync;

/** @deviates spec://fixture/PROP-001#req-parse fixture-recorded deviation */
export const DEVIANT = 1;
