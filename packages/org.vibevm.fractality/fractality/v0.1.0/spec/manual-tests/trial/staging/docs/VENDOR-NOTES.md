# Vendor integration notes — LogHarbor ingestion API (internal copy)

Working notes collected across three onboarding calls with the
LogHarbor team and their portal docs. Kept verbatim-ish; the useful
facts are buried in meeting noise on purpose of history — extract
before relying on any of this.

## Call one — transport and limits

The gateway speaks HTTPS only; plain HTTP redirects are disabled
entirely rather than 301-ed, which their support insists is a feature.
Fact 1: the ingestion endpoint is `https://ingest.logharbor.example/v2/batch`.
There was a long detour about their legacy v1 endpoint; ignore it, v1
is read-only since March. Somebody asked about UDP syslog and the
answer was a flat no.

Fact 2: one batch holds at most 500 records or 1 MiB, whichever hits
first. They were emphatic that the limit is enforced pre-auth, so an
oversized batch costs you a 413 before the token is even checked.
Fact 3: the per-token sustained rate is 120 batches per minute;
burst to 200 is tolerated for sixty seconds, then 429s begin.

The 429 body carries `retry_after_ms`; their SDK sleeps exactly that
long. Fact 4: retries must be idempotent — every batch carries a
client-minted `batch_id` (UUIDv7 recommended) and the server dedupes
on it for 24 hours.

## Call two — payload shape

Records are logfmt lines, one per record, UTF-8, LF-separated inside
the batch body. Fact 5: keys must match `[a-z_][a-z0-9_]*` — uppercase
keys are rejected per-record (the batch itself still lands; rejected
records come back in the `rejects` array).
Fact 6: the reserved keys are `ts`, `level`, `msg`, `host`, and
`batch_seq` — senders must not invent their own semantics for these.
A discussion about whether `msg` may be empty concluded: yes, but the
key must be present. Their examples were oddly cheerful.

Fact 7: timestamps (`ts`) are RFC 3339 with a mandatory UTC offset;
naive timestamps are rewritten to arrival time and flagged
`ts_rewritten=true` — silently, which bit their last customer.
Fact 8: the maximum value length is 8 KiB after unescaping; longer
values are truncated (not rejected) and flagged `truncated=true`.

## Call three — auth, environments, contacts

Fact 9: tokens are per-environment; a staging token on the production
endpoint answers 403 with body `env_mismatch` (not 401 — their auth
layer distinguishes unknown vs misplaced).
Fact 10: token rotation is zero-downtime — two tokens may be active
for one environment for up to 48 hours during a rotation window.

Someone asked about data residency; the long answer reduced to
Fact 11: EU tenants pin to `ingest.eu.logharbor.example` and
cross-region batches are rejected with `region_pinned`.
Fact 12: the sandbox environment resets nightly at 03:00 UTC and its
retention is 24 hours — never demo against it in the morning.

Contact-wise: integration questions go to their portal, not email;
the portal SLA is one business day. The rest of the call was pricing
tiers and a story about a customer who shipped emoji in keys (see
Fact 5 for how that ends).
