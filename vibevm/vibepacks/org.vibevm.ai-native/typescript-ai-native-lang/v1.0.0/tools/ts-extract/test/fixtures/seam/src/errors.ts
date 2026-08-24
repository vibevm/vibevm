/** @implements spec://fixture/PROP-001#req-parse */
export type ParseError =
  | { readonly kind: "empty"; readonly reason: string }
  | { readonly kind: "too-long"; readonly reason: string };

export type RouteError =
  | {
      readonly kind: "missing";
      readonly message: "violates REQ spec://fixture/PROP-001#req-route";
    }
  | {
      readonly kind: "bad";
      readonly message: "violates REQ spec://fixture/PROP-001#req-route";
    };

export type PlanError =
  | { readonly kind: "overflow"; readonly reason: string }
  | { readonly kind: "stale"; readonly reason: string };

// A discriminated union NOT in error position — no ts_seam_error fact.
export type Mode =
  | { readonly kind: "read"; readonly port: number }
  | { readonly kind: "write"; readonly port: number };

// In error position but NOT discriminated (no kind/tag/_tag) — no fact.
export type BlobError = { readonly code: number } | { readonly code: number };
