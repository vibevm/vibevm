# `vibe registry redirect-update` — rewrite an existing registry stub's marker

Maintainer-side command. Mutates the `vibe-redirect.toml` of an existing stub repo in-place: each flag is optional, so the command does a true partial update — fields not passed retain their current value. Closes the [v0 manual-procedure gap](../registry-redirect.md#manual-procedure-fallback) where retargeting a stub required a hand-driven `git clone` / edit / push.

Per [PROP-002 §2.4.2](../../spec/modules/vibe-registry/PROP-002-decentralized-registry.md#redirect); see [`docs/registry-redirect.md`](../registry-redirect.md) for the full operator reference.

## Usage

```
vibe registry redirect-update <pkgref>
                              [--to <new-target-url>]
                              [--ref-policy <policy>] [--pinned-ref <ref>]
                              [--target-auth <regime>] [--target-token-env <NAME>]
                              [--description <text>] [--clear-description]
                              [--registry <name>] [--trust-redirect] [--resync]
                              [--path <project>] [--dry-run]
                              [--json | --quiet]
```

## Arguments

- `<pkgref>` — `<kind>:<name>` of an existing stub to rewrite. The version segment of the pkgref is ignored; stubs live at the `(kind, name)` slot.

## Flags

Every flag is optional. Passing nothing surfaces a `no changes requested` error so trivial typo-runs fail fast.

| Flag | Description | Default |
| --- | --- | --- |
| `--to <url>` | Replace `[redirect].target_url`. Requires `--trust-redirect` because the change switches the content consumers receive. | keep current |
| `--ref-policy <pass-through-tag\|pinned>` | Switch the policy. Pass-through → pinned requires `--pinned-ref`; pinned → pass-through clears `pinned_ref` automatically. Either flip requires `--trust-redirect`. | keep current |
| `--pinned-ref <ref>` | Set or bump the pinned ref. Allowed only when the new (or unchanged) policy is `pinned`. Always requires `--trust-redirect` — bumping the pinned target changes resolution outcomes for every consumer. | keep current (pinned) / forbidden (pass-through) |
| `--target-auth <regime>` | Replace `[redirect].auth`. Same enum as `[[registry]] auth` — `none`, `token-env`, `credential-helper`, `ssh`. Switching away from `token-env` clears `token_env` automatically (otherwise the marker would fail to re-parse). | keep current |
| `--target-token-env <NAME>` | Override the env-var name when `--target-auth token-env`. Cleared when new auth regime is not `token-env`. | keep current |
| `--description <text>` | Replace `[redirect].description`. Mutually exclusive with `--clear-description`. | keep current |
| `--clear-description` | Drop the existing description field entirely. Mutually exclusive with `--description`. | off |
| `--registry <name>` | Which `[[registry]]` in `vibe.toml` hosts the stub. | the first registry |
| `--trust-redirect` | Acknowledge a deliberate switch of `target_url`, `ref_policy`, or `pinned_ref`. Per PROP-002 §2.4.2 trust model — never silent, always operator-initiated. | off |
| `--resync` | After pushing the updated marker, run `vibe registry redirect-sync <pkgref>` to mirror target tags into the stub. Most useful with `--to` migrations whose new target carries different tags. No-op for `--ref-policy pinned` stubs. | off |
| `--path <project>` | Project root with `vibe.toml`. | `.` |
| `--dry-run` | Describe what would happen but make no API calls or pushes. | off |
| `--json` | Structured payload (see below). | off |
| `--quiet` | One-line summary. | off |

## Trust model: which changes require `--trust-redirect`

Three fields alter the content consumers materialise:

1. `target_url` — points the stub at a different repo.
2. `ref_policy` — flips between pass-through-tag and pinned.
3. `pinned_ref` — bumps the pinned target ref under pinned policy.

Any change to these requires `--trust-redirect`. The flag is the operator's explicit "I know this changes what consumers receive." Without it, the command refuses with a pointer at the flag and the list of trust-required fields it detected.

Operator-side metadata changes (`auth`, `token_env`, `description`) do **not** require `--trust-redirect`. They affect how the resolver reaches the target (or how operators read the stub on a web UI) but never which content gets installed.

## Authentication

Same publish-token loading as [`vibe registry redirect`](registry-redirect.md): `VIBEVM_PUBLISH_TOKEN` env-var (highest), then `~/.vibe/<host-prefix>.publish.token`, then legacy `~/.vibe/git.publish.token`. The token must have `repo:write` permission for the stub repo in the registry organization.

Token secrecy invariants are identical to all publish commands (PROP-000 §20). The token is never logged, never recorded in any vibevm-produced output, and embedded into the push URL only at the moment of `git push`.

## Pipeline

1. Validate args-level invariants (`--description` / `--clear-description` mutual exclusion).
2. Resolve project root + `vibe.toml`, parse `<pkgref>`, resolve the target `[[registry]]`.
3. Load the publish token for the registry's host. Construct the host adapter (`RepoCreator`).
4. Probe `repo_exists` for the stub slot. Refuse with a clear pointer if the stub does not yet exist (suggests `vibe registry redirect <pkgref> --to ...`).
5. Shallow-clone the stub via the same `git_publish::shallow_clone` machinery `vibe registry redirect-sync` uses. The clone carries the existing `vibe-redirect.toml` at HEAD.
6. Read the existing marker. Bail if missing (not actually a stub) or unparseable.
7. Merge the CLI flags into a new `[redirect]` section. Validate cross-field invariants (pinned policy requires pinned_ref; token_env only meaningful with token-env auth).
8. If the computed marker is byte-identical to the current one, refuse with `no changes requested`.
9. If any change requires `--trust-redirect` and the flag is absent, refuse with the list of trust-required fields.
10. On `--dry-run`, render the per-field diff and exit.
11. Write the new `vibe-redirect.toml` + regenerated README into the clone working tree.
12. `git add -A && git commit -m "stub: …" && git push origin main` via `git_publish::commit_and_push`. The push is a fast-forward — the new marker is layered on top of existing history, not force-pushed.
13. Optional `--resync` invokes the same logic as `vibe registry redirect-sync`.

## JSON output (`--json`)

```jsonc
{
  "ok": true,
  "command": "registry:redirect-update",
  "registry": "default",
  "pkgref": "flow:internal-helper",
  "stub_url": "https://github.com/vibespecs/flow-internal-helper",
  "target_url": "https://forgejo.example/internal-helper",
  "ref_policy": "pass-through-tag",
  "target_auth": "none",
  "changes": [
    { "field": "target_url",  "before": "https://gitlab.acme.example/flows/internal-helper", "after": "https://forgejo.example/internal-helper" },
    { "field": "description", "before": "Delegated to acme-corp",                            "after": "Delegated to forgejo-corp" }
  ],
  "trust_required": true,
  "dry_run": false,
  "sync": {
    "ok": true,
    "command": "registry:redirect-sync",
    "registry": "default",
    "pkgref": "flow:internal-helper",
    "stub_url": "https://github.com/vibespecs/flow-internal-helper",
    "target_url": "https://forgejo.example/internal-helper",
    "pushed_tags": ["v0.2.0"],
    "already_present": ["v0.1.0"],
    "dry_run": false
  }
}
```

The `changes` array is the per-field before/after diff (canonical order: `target_url`, `ref_policy`, `pinned_ref`, `auth`, `token_env`, `description`). Each entry's `before` or `after` is absent when the corresponding side has no value (e.g. clearing a description renders `before: "old text"` and no `after`).

`trust_required` is `true` when at least one change targets `target_url`, `ref_policy`, or `pinned_ref`. Operators can use this for CI gating: a job that pipes `vibe registry redirect-update … --dry-run --json` into a policy check can decide whether to require manual review based on this flag alone.

`sync` is populated only when `--resync` ran (so absent on `--dry-run`, absent for pinned-policy stubs after update).

## Examples

```bash
# Description-only update. No trust flag needed — the change is
# operator-side metadata.
vibe registry redirect-update flow:internal-helper \
  --description "Delegated to forgejo-corp; contact ops@forgejo.example"

# Drop the description entirely.
vibe registry redirect-update flow:internal-helper --clear-description

# External author moved from GitLab to Forgejo. Requires --trust-redirect
# because target_url changes.
vibe registry redirect-update flow:internal-helper \
  --to https://forgejo.example/internal-helper \
  --trust-redirect \
  --resync

# Flip a pass-through-tag stub to pinned policy at v1.0.0.
vibe registry redirect-update flow:legacy \
  --ref-policy pinned --pinned-ref v1.0.0 \
  --trust-redirect

# Bump a pinned ref from v1.0.0 to v1.1.0 (within pinned policy).
vibe registry redirect-update flow:legacy \
  --pinned-ref v1.1.0 \
  --trust-redirect

# Switch target to a private host: change target URL + flip auth in one
# update. Trust required for target_url change; not required for the
# auth flip on its own.
vibe registry redirect-update flow:internal-secret \
  --to https://gitlab.company.com/specs/internal-secret \
  --target-auth token-env \
  --target-token-env VIBEVM_TARGET_TOKEN_GITLAB_COMPANY_COM \
  --trust-redirect

# Preview a change without pushing anything.
vibe registry redirect-update flow:internal-helper \
  --to https://forgejo.example/internal-helper \
  --trust-redirect --dry-run
```

## Error surface

- **`--description` paired with `--clear-description`** — refused immediately as an args-level error before any filesystem or network work.
- **No flags / no fields would change** — `no changes requested`. The command refuses to record an empty commit on the stub history.
- **`--ref-policy pinned` without an effective `--pinned-ref`** — refused. Either pass `--pinned-ref`, or keep the previous policy.
- **`--pinned-ref` against a pass-through-tag stub** — refused. Either pass `--ref-policy pinned` too, or drop the flag.
- **`--target-token-env` paired with an effective auth regime other than `token-env`** — refused.
- **`--to`, `--ref-policy`, or `--pinned-ref` requested without `--trust-redirect`** — refused with the list of trust-required fields detected and a pointer at the flag.
- **Stub does not exist on host** — refused with a pointer at `vibe registry redirect <pkgref> --to <url>` to create it.
- **Stub exists but carries no `vibe-redirect.toml` at HEAD** — refused. The command only operates on real stubs.
- **GitVerse `[[registry]]`** — refused early. Same gap as `vibe registry redirect` / `vibe registry publish`: the GitVerse public API does not expose org-scoped repo creation, so the host-adapter machinery cannot run. For GitVerse stubs, the manual `git clone` / edit / push procedure remains the workaround.

## Related

- [`vibe registry redirect`](registry-redirect.md) — create a fresh stub.
- [`vibe registry redirect-sync`](registry-redirect-sync.md) — mirror target tags into an existing stub.
- [`docs/registry-redirect.md`](../registry-redirect.md) — operator reference for the redirect protocol (wire grammar, resolver behaviour, identity rules, lockfile shape, trust model).
- [PROP-002 §2.4.2](../../spec/modules/vibe-registry/PROP-002-decentralized-registry.md#redirect) — the architectural decision and contract.
