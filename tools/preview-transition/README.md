# Preview transition checker

`python -B tools/preview-transition/check.py --transition
docs/releases/developer-preview-1-to-2/transition.json --mode capture --format json`
checks the registered transition and capture shapes, their semantic invariants,
the retained raw capture hash, and every recorded Git tree, package manifest,
and source-file identity. It reads the selected repository and writes only a
machine report to stdout.

The Python entrypoint only parses CLI arguments and sends a versioned request
to `preview-transition-check`, a `vibe-wire` binary that uses generated JTD
readers. Exit `0` means the selected stage passed; `1` means a failed request
or claim; `2` means the selected stage remains pending. A passing capture
still reports later documentation and final-product proof as pending.

The request grammar also names `contribution`, `bindings` (`roles` or `linked`),
`documentation` (`intake`, `updated`, or `verified`), `release` (`reviewed` or
`verified`), and `retired`. Their domain checks remain pending until the
corresponding implementation work lands; the checker never promotes a
currently unimplemented stage to passed.
