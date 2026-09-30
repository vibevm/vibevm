# M-16-C.2.1 protected lifecycle foundation, still open

The current internal slice accepts only a single-node, untraced `validate`
command with no prior lifecycle state or install inputs. Under one workspace
lease it captures the selected `vibe.toml` and optional `vibe.lock`, derives a
deterministic pre-install plan (including the workspace-root witness), selects
a run ID without scratch, and uses the C.1 reservation. The lifecycle owner
writes and rereads the exact header before journal-ready. Only then does the
incumbent `PreparedDefaultLifecycle::run` execute. Its generated report is
retained and reread before the per-run marker; a same-key retry replays those
exact bytes. A missing marker returns recovery-required without another run.

Evidence: `575b6ba90` (pure leased ID), `d1582baea` and `ca5f68ea6`
(bounded witnessed selected source), `19aa8ae36` and `5da036fb7` (preselected
identity through the ordinary command path), `791c7d3d1`, `c570f59e2`, and
`9f0f5e39e` (bounded canonical pre-install plan), `63c87a22d` (marker builder)
and `df070ef46` (validate-only adapter). Focused tests passed: adapter 6/6,
marker 9/9, pre-install plan 7/7, and the affected legacy identity/prelude
cases. Source rereads are cooperative-writer drift detection, not hostile-writer
content CAS.

This is not M-16-C.2.1 acceptance. It does not dispatch install or handlers,
prove member or installed-package/handler executable bytes, retain an effectful
partial result, or expose a public machine operation. The prior-header case
refuses before execution; a new key may leave a pending C.1 reservation, which
is safe but requires later owner reconciliation. The generic C.1 fixture's
freshness row is not evidence that an effectful lifecycle run completed; only
the validated per-run report and marker provide that result.
