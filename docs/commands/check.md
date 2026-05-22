# `vibe check` — spec-consistency linter

Runs deterministic, pure-inspection checks against a project tree. No LLM, no network, no mutation. Output: a structured report (per-finding severity, originating check, file path, line) plus an exit code suitable for CI gating.

Spec: [`VIBEVM-SPEC.md` §12](../../VIBEVM-SPEC.md), [ROADMAP §M1.3](../../ROADMAP.md#m13--vibe-check-spec-linter).

## Usage

```
vibe check [--path <dir>]
           [--wal-max-age-hours <N>]
           [--review-max-age-days <N>]
           [--json | --quiet]
```

## Flags

| Flag | Description | Default |
| --- | --- | --- |
| `--path <dir>` | Project root with `vibe.toml`. | `.` |
| `--wal-max-age-hours` | WAL is "stale" past this age. | `24` |
| `--review-max-age-days` | `<!-- REVIEW: YYYY-MM-DD -->` marker age threshold. | `14` |
| `--json` | Structured payload — see [Output (JSON)](#output-json). | off |
| `--quiet` | One-line summary `vibe check: N errors, M warnings, K info`. | off |

## Checks (v0)

`vibe check` v0 covers six of the ten checks in `VIBEVM-SPEC.md` §12. The four deferred checks (`DeadSpecRefs`, `OrphanAnchors`, `AnchorUniqueness`, `ImplementationCoverage`) require markdown-anchor parsing or `vibe build` provenance and land later.

| Check id | Severity | What it does |
| --- | --- | --- |
| `manifest_validity` | error | `vibe.toml` is present and parses; `vibe.lock` (if present) parses. Both must match the current schema in `vibe-core::manifest` — the lockfile at schema v4. Missing `vibe.toml` is an error; missing `vibe.lock` is fine (empty project). |
| `wal_freshness` | warning | `spec/WAL.md` modification time is within `--wal-max-age-hours`. Older → warning. Future-dated mtime (clock skew) → info. |
| `wal_wellformed` | warning | WAL has the canonical `## current phase`, `## constraints`, `## done`, `## next`, `## known issues` top-level sections. Matching is case-insensitive and tolerates parenthetical suffixes (e.g. `## Constraints (do not violate without discussion)`). |
| `boot_directory` | error / warning | `spec/boot/` exists (error if a project's `vibe.toml` is present but the directory is gone) and holds markdown files. The two-digit `NN-` filename prefix is **retired** ([PROP-009 §2.5](../../spec/modules/vibe-workspace/PROP-009-loading-model.md#ordering)) — `vibe` owns boot ordering by `category`, so any markdown filename is valid and the generated `INDEX.md` / `INLINE.md` artifacts (no numeric prefix) are recognised. Only a non-markdown stray file in `spec/boot/` warns. |
| `lockfile_files` | error / warning | No orphan files in `spec/flows/`, `spec/feats/`, `spec/stacks/` (warning — likely a leftover from a manual edit). Under the loading model the lockfile carries no per-file `files_written` list, so there is nothing per-package to verify present on disk. |
| `review_aging` | warning | `<!-- REVIEW: YYYY-MM-DD ... -->` markers in `spec/**/*.md` whose date is older than `--review-max-age-days`. Markers with non-date prose (`<!-- REVIEW: ... -->`, `<!-- REVIEW: TODO -->`) are silently skipped — they're documentation, not work. |

## Exit codes

- `0` — no errors. Warnings and info findings still print but don't fail the run.
- `1` — at least one error finding.

This matches `VIBEVM-SPEC.md` §12 exactly: "Exit code: 0 if no errors, 1 if errors, 0 with warnings displayed if only warnings."

## Output

### Human-readable

```
vibe check: 3 findings in `/path/to/project`
  [E]  [boot_directory] spec/boot — spec/boot/ is missing — every project owns this directory; run `vibe init` if it disappeared.
  [W]  [wal_freshness] spec/WAL.md — WAL is 41 hours old (threshold: 24h). Consider an end-session checkpoint.
  [W]  [review_aging] spec/notes/old.md:7 — REVIEW marker dated 2026-04-01 is 33 days old (threshold: 14d) — resolve or refresh

1 error, 2 warnings, 0 info
```

The sigil legend is `[E]` (error), `[W]` (warning), `[i]` (info). The bracketed `[check_id]` matches what `--json` emits.

### Output (JSON)

```jsonc
{
  "ok": true,
  "command": "check",
  "project": "/path/to/project",
  "summary": { "error": 0, "warning": 2, "info": 0 },
  "findings": [
    {
      "check": "wal_freshness",
      "severity": "warning",
      "path": "spec/WAL.md",
      "message": "WAL is 41 hours old (threshold: 24h). Consider an end-session checkpoint."
    },
    {
      "check": "review_aging",
      "severity": "warning",
      "path": "spec/notes/old.md",
      "line": 7,
      "message": "REVIEW marker dated 2026-04-01 is 33 days old (threshold: 14d) — resolve or refresh"
    }
  ]
}
```

`ok = true` when `summary.error == 0`; `false` otherwise. The shape is stable enough to script against (e.g. `vibe --json check | jq '.summary.error'` gates a CI step).

## Examples

```bash
# Smoke-check the project before pushing.
vibe check

# Allow a longer review-marker window for a slow-moving project.
vibe check --review-max-age-days 30

# Pre-flight gate in CI: print findings, fail on errors only.
vibe check --quiet

# Programmatic check.
vibe --json check | jq -e '.summary.error == 0'
```

## Edge cases

- **No `vibe.toml`** — `manifest_validity` errors. Run `vibe init` or pass `--path <dir>` pointing at a real project root.
- **Empty lockfile.** `manifest_validity` accepts a missing `vibe.lock` (treated as "no installs yet"). `lockfile_files` short-circuits — there's nothing to check.
- **Project without a `spec/` tree.** `wal_wellformed` errors (WAL is missing); `boot_directory` errors (`spec/boot/` is missing) when `vibe.toml` exists; `review_aging` skips silently. Run `vibe init` to scaffold the standard layout.
- **Documentation references to the REVIEW convention.** `<!-- REVIEW: ... -->` and similar are detected as non-date prose and skipped — they aren't tracking real work.

## Limitations (v0)

- Four checks from `VIBEVM-SPEC.md` §12 are deferred (markdown-anchor parsing, `vibe build` provenance). Their slot exists in the report shape; v1+ extends the [`CheckId`](../../crates/vibe-check/src/lib.rs) enum.
- No `--fix` flag yet. The spec's "safe automatic fixes" landing is queued for v1+ once the deferred checks bring concrete fixable findings (dead anchor references, primarily).

## Related

- [`VIBEVM-SPEC.md` §12](../../VIBEVM-SPEC.md) — full ten-check catalogue (six implemented in v0).
- [PROP-002 §2.7](../../spec/modules/vibe-registry/PROP-002-decentralized-registry.md#lockfile) — lockfile schema v4; `manifest_validity` and `lockfile_files` consume the same parser.
- [`vibe install`](install.md) — produces the lockfile that `lockfile_files` validates.
- [`vibe update`](update.md) — touches the same files; running `vibe check` after a `vibe update` pass is a useful smoke step.
