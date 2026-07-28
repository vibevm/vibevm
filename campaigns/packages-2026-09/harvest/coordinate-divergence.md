# one `name@version`, two contents

_Captured 2026-07-28 across `packages/` and `vibedeps/`._

`qualified-naming` states the law: *never reuse a `name@version` coordinate for
different content — a coordinate that meant one artifact must never mean another.*
`packages/<group>/<name>/v<ver>/` is the shipped artifact; `vibedeps/<kind>-<name>/<ver>/`
is what this consumer received.

```console
$ python campaigns/packages-2026-09/tasks/coordinate-divergence.py
markdown files at the same (package, version) in packages/ and vibedeps/: 425
  byte-identical                     : 252
  DIFFERENT content, same coordinate : 173
  packages affected                  : 33
  vibedeps slots with no package twin: none
    rust-ai-native-lang: 18
    typescript-ai-native-lang: 18
    core-ai-native: 16
    wal: 7
    health-audit: 6
    licensing: 6
    addressable-specs: 5
    campaign-plans: 5
    comparative-research: 5
    conflict-protocol: 5
    decision-records: 5
    git-attribution-policy: 5
    managed-blocks: 5
    manual-tests: 5
    operating-modes: 5
    qualified-naming: 5
    secrets-hygiene: 5
    source-mirrors: 5
    spec-genres: 5
    tool-design-lessons: 5
    two-process-model: 5
    sync-from-code: 4
    discovery-prompt: 3
    git-atomic-commits: 3
    wal-specspaces: 3
    dev-runtime-docs: 2
    git-autonomy: 2
    git-conventional-commits: 2
    redbook: 2
    rust-ai-native-mcp: 2
    typescript-ai-native-mcp: 2
    git-practices: 1
    typescript-ai-native: 1
EXIT=1
```

**Scope:** §3.1 source 3 for every package that ships markdown. The anchor list is not maintained here — a verdict cites this file in its `ev[]`.
