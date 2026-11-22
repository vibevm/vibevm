import { greet } from "../greet/index.js";
import { normalise } from "../../core/text.js";

/** @implements spec://fixture/PROP-001#req-parse */
export function parseAndGreet(input: string): string {
  return greet(normalise(input));
}
