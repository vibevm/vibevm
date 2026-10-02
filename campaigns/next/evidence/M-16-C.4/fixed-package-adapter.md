# Fixed package application exchange

`FixedPackageRoute` is a library-callable adapter over a retained
`Arc<FixedPackageHost>`. It accepts only the exact generated fixed package
Input envelope or direct closed action request for `submit {key}` and
`inspect {key}`. It checks operation, domain, registered format, epoch,
bounded key and physical root before delegating to the host. The lifecycle
package journal remains the sole C1 reservation and result owner; the adapter
has no generic executor, cache or descriptor.

Owner-verified Completed maps to completed/complete with the retained run.
Uncertain carries exact partial output and A2 observations; Ready,
RecoveryRequired and Retired remain uncertain. Inspection of a truly absent
key is no-effects/complete for that key. After a possible owner admission,
`Err` triggers cold inspection for correlated evidence and cannot by itself
mean no effects. Failed projection/publication retains the original typed
owner observation in the error channel. The adapter validates the fixed A2
path, digest formats (plain output hex, prefixed A2 hash), exact 32-hex owner
run IDs, byte caps, UTF-8 diagnostic truncation and generated nested/outer
roundtrips before publication.

Focused Linux proof: `vibe-orchestrator --lib
machine_operation::fixed_package` passed 17/17 in the worker and primary
targets after parent review; five focused doctests passed in the worker.
Actual owner tests cover one C1-ready run/copy, direct-owner/route replay,
cold attach, changed source, six retained-evidence damage modes, A2-only
noncompletion, retired/expired scope, foreign root/grant, 29 malformed values
and publication-cap/error coherence. No full gate panel or mutation campaign
was run.

This is an application library route only. Local frame, MCP and CLI grants,
capability registration and cross-surface tests remain separate obligations.
