# go-demo — the AI-Native Go pilot (a miniature reconciler)

The research pilot of `stack:org.vibevm.ai-native/go-ai-native-lang`
(GO-AI-NATIVE-PLAN D11): desired-vs-actual state reconciliation — the
Kubernetes-shaped rehearsal — small enough to read in one sitting,
exercising every scaffold class of GUIDE-AI-NATIVE-GO:

- **Seams** (`internal/seams`): defined-type brands (`ResourceID`,
  `Revision`), the closed `ActionOp` set, the `Planner`/`Store`/`Clock`
  contracts, and the REQ-citing `PlanError` (Class F).
- **Two planner cells** behind one seam: `naiveplanner` (the readable
  reference) and `batchplanner` (`replaces=naive`) — with the
  **differential fuzz oracle** (`FuzzPlannersAgree`) pinning their
  agreement (scaffold D). The oracle deliberately imports the sibling
  it replaces; that one `go-cell-isolation` finding is FROZEN in the
  conform baseline as the replacement window's recorded debt.
- **The steppable world model** (`internal/sim`): scaffold H — run the
  convergence instead of simulating it mentally; doubles as the store
  fake in every test.
- **The registry** (`internal/registry`): the only cell importer, the
  one flag switch; `cmd/reconcile` is the composition root (`PLANNER=batch`
  selects the replacement).
- **Executed Examples** with `// Output:` on every seam surface
  (scaffold G), a declared test matrix, a `testing/quick` property
  backing `#req-plan-total` (scaffold C).

## Drive it

```sh
go test ./...                # the whole corpus (fuzz seeds included)
go run ./cmd/reconcile       # watch a world converge (naive planner)
PLANNER=batch go run ./cmd/reconcile

# the discipline chain (the stack's binaries; see GUIDE §14):
go-ai-native floor           # gofmt→vet→test→staticcheck+exhaustive→conform→specmap→test-gate
go-ai-native health          # the collector's snapshot
printf '...' | go-ai-native-tcg validate internal/cells/<cell>/<f>.go --content-from -
```

`spec/PROP-001-reconciler.md` carries the anchored requirements the
`//spec:` directives cite; `specmap.json` is the committed index; the
conform baseline records exactly one frozen finding (see above). Note
the doc-id lesson: units mint under the document's own `# PROP-001 …`
heading id, not the filename.
