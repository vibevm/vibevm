Task 9. Produce `manifest.json` at the repo root: a JSON object
`{"modules": [{"name": <module>, "public_fns": [<fn-name>, …]}, …]}`
listing every module under `src/` and its public functions. The result
MUST conform to the JSON Schema in `task-assets/manifest.schema.json`.
Because the return is structured and machine-checkable, delegate this with
the worker packet's `output.output_schema` set to that schema, so
mission-control validates the worker's return at the collection seam and
retries once on a violation (this is exactly what the schema gate is for).

Task 10. The crate's modules (`parse`, `render`, `filter`, `stats`, and —
if you added it — `dedup`) each make their own implicit assumption about
what a *valid record* is, and some of those assumptions conflict. Determine
the ONE consistent record-validity invariant the whole crate should agree
on, by cross-referencing ALL of the modules together, and write it to
`docs/RECORD-INVARIANT.md` — stating the invariant and naming the specific
cross-module conflicts it resolves. This is whole-crate judgment: it cannot
be answered from any single module in isolation, and splitting it across
independent workers (one per module) would destroy the cross-cutting
reasoning that is the whole point. Handle it accordingly — do not fan it
out into per-module children.
