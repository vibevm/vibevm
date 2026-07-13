# Backlog — deferred items surfaced during the extraction

Owner-authorised deferrals. Move each to its proper home (a PROP / ROADMAP entry) when picked up.

## B1 — `transitive-inline` / `transitive-static` link types (owner, 2026-07-14)

**Problem found.** A family aggregator (e.g. `git-practices`) declaring a member `link = "inline"`
in its `[requires.packages]` does **not** propagate to the member's effective boot: `bootgen.rs`
resolves a dependency's `declared_link` from a **single** node's manifest (the consumer/root),
so an aggregator-declared link on a *transitive* member is not honoured — the member falls back
to its own `[boot_snippet]` suggestion or the `static` default (PROP-009 §2.4; `boot.rs`
`declared_link.or(suggested_link)`).

**Proposal (owner).** Add `link` variants that DO cross the transitive boundary — `transitive-inline`,
`transitive-static` (and the `-dynamic` twin) — meaning "apply this inclusion type to this package
**and everything it pulls transitively**". Then a consumer requiring `git-practices` with
`link = "transitive-inline"` makes the whole family load inline, without each member having to
self-suggest it. Same semantics as the existing `inline`/`static`/`dynamic`, only transitive.

**Home when built:** PROP-009 §2.4 (inclusion types) + `vibe-workspace` boot resolution + the
`vibe-core` manifest schema for the `link` enum.

**Interim (in force now).** The four `git-practices` members self-suggest `link = "inline"` in
their own `[boot_snippet]` so the commit rules land in `INLINE.md` today. When `transitive-inline`
lands, that per-member suggestion can be dropped and `git-practices` can declare it once.
