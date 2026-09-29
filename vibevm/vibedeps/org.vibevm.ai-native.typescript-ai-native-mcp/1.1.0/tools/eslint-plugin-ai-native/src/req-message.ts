/**
 * req-message.ts — the TypeScript port of the engine's Class-F
 * diagnostic grammar (card scaffold-f-structured-diagnostics, Band 3):
 *
 *     violates REQ <uri>: <why>; fix surface: <where>
 *
 * The authoritative renderer is the Rust pair `req_message` /
 * `matches_req_grammar` in
 * `core-ai-native-conform/src/rules/mod.rs` (19 production call sites).
 * This file reproduces BOTH verbatim — a second spelling of the grammar
 * is precisely the bug this lint channel exists to prevent — so the TS
 * rule cannot drift from the engine. One renderer, one acceptor, kept
 * next to each other so they cannot diverge.
 *
 * Run with node >= 22.6 under type stripping (no build step), matching
 * the ts-extract / ts-oracle siblings.
 */

const PREFIX = "violates REQ ";
const FIX_MARKER = "; fix surface: ";
const KNOWN_SCHEMES = ["spec://", "discipline://", "misra://"] as const;

/**
 * Render a finding message in the Class-F diagnostic grammar — the
 * TypeScript twin of the engine's `req_message`. Every message this
 * plugin's rules emit goes through this one helper, so the grammar is
 * spelled in exactly one place.
 */
export function reqMessage(uri: string, why: string, fixSurface: string): string {
  return `violates REQ ${uri}: ${why}; fix surface: ${fixSurface}`;
}

/**
 * The grammar acceptor — the TypeScript twin of the engine's
 * `matches_req_grammar`. True iff `message` begins `violates REQ ` +
 * a known scheme (`spec://` | `discipline://` | `misra://`), and carries
 * both `: ` and `; fix surface: ` somewhere in the remainder. Kept next
 * to {@link reqMessage} so the two cannot drift.
 */
export function matchesReqGrammar(message: string): boolean {
  if (!message.startsWith(PREFIX)) {
    return false;
  }
  const rest = message.slice(PREFIX.length);
  const knownScheme = KNOWN_SCHEMES.some((scheme) => rest.startsWith(scheme));
  return knownScheme && rest.includes(": ") && rest.includes(FIX_MARKER);
}
