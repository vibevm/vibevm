# Fixed static-file package owner seam

The lifecycle library exposes `submit_fixed_static_file_package` and
`inspect_fixed_static_file_package` for one builtin copy:
`source.bin` to `target/vibe-package/launcher/launcher.bin`, target `launcher`.
The API constructs the sole target and empty mechanism world internally. The
caller supplies a held lifecycle lease and host-issued C1 namespace/scope/key.
It invokes the existing package journal once, with no orchestrator reservation,
new result store, generic package route or public adapter.

For an existing key, the journal retains the first owner's `created_at` and
canonical plan. A later host clock reading is not a requested copy option:
same-key, same-source replay reads the retained plan without copying again.
Source/request drift refuses; a completed cold attach reads the incumbent
capsule, output and A2 record. A2 alone remains post-effect evidence.

Focused Linux checks: the new owner API family passed 3/3 in the worker and
primary targets. The previously existing package cold-attach family passed
4/4, covering completed cold result, Ready without result, damaged capsule
and replaced physical root. The worker's broader journal family was 13/15
under Cranelift; the two unchanged caught-panic crash-injection cases are the
known host backend limitation and were not weakened. Public examples passed
2/2. No full panel or mutation campaign was run.

A registered generated request/reply/error exchange and orchestrator route are
still required before any CLI, MCP or local frame can advertise this effect.
