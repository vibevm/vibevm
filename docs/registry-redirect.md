# Registry redirect — delegated package via stub repo

A registry org's package slot may carry a `vibe-redirect.toml` marker file pointing at an external git repo where the package's actual content lives, instead of carrying the package content directly. Consumers `vibe install <pkgref>` see no difference — the resolver follows the marker transparently. Spec: [PROP-002 §2.4.2](../spec/modules/vibe-registry/PROP-002-decentralized-registry.md#redirect).

## When to use

The use case is **delegation**. An org owner wants the package to live in their namespace (so consumers find it via the standard `[[registry]]` walk without knowing about the external author) but offload hosting / PR queue / hosting-platform permissions to a different team or external author. Closest analogues: Linux distro virtual packages with `Provides:` pointing at external SRPMs, DNS CNAME, GitHub repo-redirects, Bundler's `gem 'foo', git: '...'` declared at the registry level rather than the consumer level.

Distinct from `[requires.packages]` git-source ([git-source-dependencies.md](git-source-dependencies.md)) — git-source is **consumer-side** (the operator's `vibe.toml` declares "this dep lives at this URL"); registry redirect is **org-side** (the registry's stub repo declares "this package's content lives at this URL"). Same wire-level effect, inverse control axis.

## Marker file

A stub repo at `<org>/<kind>-<name>` carries `vibe-redirect.toml` at the repo root **instead of** `vibe-package.toml`. Both files in the same repo at the same ref is rejected by the resolver as `AmbiguousStub`.

```toml
# vibe-redirect.toml at the root of the stub repo

[redirect]
target_url = "git@gitlab.acme.example:flows/internal-helper"

# Default: ref_policy = "pass-through-tag" — covered by the absence
# of these fields. Stub tag v0.3.0 → target_url@v0.3.0.

# Opt in to pinning instead:
# ref_policy = "pinned"
# pinned_ref = "v0.3.0"

# Auth-hint for the consumer's resolver when fetching from the target.
# Same enum as [[registry]] auth (PROP-002 §2.2.1); same env-var
# conventions. Default "none".
# auth      = "token-env"
# token_env = "VIBEVM_TARGET_TOKEN_GITLAB_ACME_EXAMPLE"

# Optional human-readable note surfaced by `vibe show <pkgref>`.
description = "Delegated to acme-corp; contact maintainers@acme.example"
```

### Wire grammar

| Field | Required | Meaning |
|---|---|---|
| `target_url` | required | Full git URL of the package's actual content repo. Same URL grammar as `[[registry]] url`. |
| `ref_policy` | optional | `"pass-through-tag"` (default) or `"pinned"`. |
| `pinned_ref` | required iff `ref_policy = "pinned"` | Tag, branch, or commit on `target_url` that ALL consumers resolve to. |
| `auth` | optional | Auth regime for `target_url`. Default `"none"`. |
| `token_env` | optional | Env-var name when `auth = "token-env"`. Default derived from target URL host. |
| `description` | optional | Free-form text shown to operators. |

## Resolver behaviour

When fetching a package manifest at `<stub_url>@<tag>`:

1. The resolver first probes `vibe-package.toml`. If found, normal resolution proceeds.
2. If absent, it probes `vibe-redirect.toml` at the same ref. The file is small (a few hundred bytes), the probe is one extra `git archive` call only when the registry-walk leg succeeded.
3. Marker found → parse it, compute `target_ref`:
   - `ref_policy = "pass-through-tag"` (default): `target_ref = <tag>` (same tag name on target).
   - `ref_policy = "pinned"`: `target_ref = pinned_ref`.
4. Apply `[redirect].auth` to fetch from `target_url`.
5. Re-enter the standard resolution path against `target_url` at `target_ref`. Fetch `vibe-package.toml`, compute content-hash over target content, fetch package files for install via the same `GitPackageRegistry` machinery used elsewhere.
6. **Hop limit = 1.** If the target's content-root is itself a stub (carries `vibe-redirect.toml`), the resolver refuses with `RedirectChainNotAllowed`. Stubs are flat indirection, not a redirect graph.

## Tag visibility

`list_versions(stub_url)` returns the **stub** repo's tags, not the target's. The org owner controls which versions surface in their namespace by managing stub tags — same gating mechanism a registry already has via `vibe registry publish`.

The fact that stub-tags exist independently of target-tags is the key affordance: org owner certifies each version that enters their namespace. A target tag `v2.0.0` with a breaking change does NOT automatically appear in the org's namespace — the owner must `git tag v2.0.0 && git push origin v2.0.0` against the stub repo. Pass-through happens during a single resolve, not during version listing.

## Identity

`content_hash` is computed over the **target** package content, not over the stub. The stub repo carries only `vibe-redirect.toml` and (optionally) human-readable companion files; nothing in the stub ships into the consumer project. A force-pushed target tag is detected exactly as it would be for a non-redirected package: hash mismatch on the next install raises `IntegrityError`.

The pkgref `<kind>:<name>` declared on the consumer side and the stub URL slot must match what the target's `vibe-package.toml` declares. Mismatch — e.g. `<org>/flow-internal` redirects to a target whose manifest declares `feat:something-else` — is rejected as `PackageIdentityMismatch` ("refusing to install"). This means a malicious target cannot impersonate `flow:wal` if its manifest declares it as `feat:auth`.

## Lockfile

A redirect-resolved package surfaces in `vibe.lock` with the **target** URL in `source_url` and the **stub** URL in a new `via_redirect` field:

```toml
[[package]]
kind            = "flow"
name            = "internal-helper"
version         = "0.1.0"
source_kind     = "registry"                                          # same as a non-stub registry resolution
registry        = "vibespecs"                                         # the registry that hosted the stub
source_url      = "git@gitlab.acme.example:flows/internal-helper"     # target URL — actual content
via_redirect    = "git@github.com:vibespecs/flow-internal-helper"     # NEW; stub URL that delegated
source_ref      = "v0.1.0"                                            # target ref (pass-through tag or pinned_ref)
resolved_commit = "abc123…def"
content_hash    = "sha256:…"
overridden      = false
```

`via_redirect` is purely diagnostic / auditing — `vibe show <pkgref>` surfaces it; `vibe list --json` includes it. The resolver does not consult it on subsequent installs (lockfile is authoritative).

## Trust model

The stub is mutable — its owner can change `target_url` at any commit. Defence is layered:

- **Content-hash in lockfile** catches a target switch on the next install. The consumer sees `IntegrityError` and can investigate before any write happens.
- **`--trust-redirect`** flag on `vibe install` / `vibe update` (parallel to `--trust-mirror`) lets an operator accept a deliberate target switch — e.g. when the external maintainer migrates their hosting from GitLab to Forgejo. Never silent; always operator-initiated.
- **Description field** lets the org owner publish contact / verification info for their delegate; consumers can manually verify out of band.

For v0, plain text + content-hash is the contract. Signed redirect markers (`[redirect].signature = "..."` with org's pubkey) are tracked as a future enhancement; not implemented in M1.16.

## Auth — two independent layers

- **Stub auth** = the registry's `[[registry]] auth` regime. The stub repo is a child of the registry org and inherits its auth. If the registry is `auth = "token-env"`, fetching the stub uses that token.
- **Target auth** = the redirect's `[redirect].auth`. Independent. The stub may be in a public registry (`auth = "none"`) but point at a private target (`auth = "token-env"` against a different host).

Tokens flow through the same M1.14 plumbing: read once at resolver-open time, kept in memory, scrubbed from `.git/config` after any clone. The `inject_token` / `set_remote_url` discipline applies identically to both URLs.

## Creating a stub repo

### `vibe registry redirect` (recommended)

The CLI helper creates the stub repo automatically — analogous to `vibe registry publish`, but commits a `vibe-redirect.toml` marker instead of package content. It uses the same publish-token and host-adapter infrastructure (PROP-002 §2.10), so the same `~/.vibevm/<host>.publish.token` you already configured for publishing is consumed here.

```bash
# Bare minimum — pass-through-tag policy, no auth.
vibe registry redirect flow:internal-helper \
  --to git@gitlab.acme.example:flows/internal-helper \
  --description "Delegated to acme-corp; contact maintainers@acme.example"

# Pinned-policy stub: every consumer resolves to v1.0.0 on the target,
# regardless of which stub tag they probed.
vibe registry redirect flow:legacy-pinned \
  --to https://github.com/legacy-vendor/flow-pinned \
  --ref-policy pinned --pinned-ref v1.0.0

# Private target: redirect carries `[redirect].auth = "token-env"` so
# the consumer's resolver knows to inject `VIBEVM_TARGET_TOKEN_<HOST>`
# when fetching from `target_url`.
vibe registry redirect flow:internal-secret \
  --to https://gitlab.company.com/specs/internal-secret \
  --target-auth token-env \
  --target-token-env VIBEVM_TARGET_TOKEN_GITLAB_COMPANY_COM

# Create the stub AND immediately mirror current target tags.
# Equivalent to running `vibe registry redirect-sync <pkgref>` once
# the stub exists — useful when you already have v0.1.0 / v0.2.0 / ...
# sitting on the target side.
vibe registry redirect flow:internal-helper \
  --to git@gitlab.acme.example:flows/internal-helper \
  --sync
```

The command writes `vibe-redirect.toml` (and a small README explaining the delegation) into the stub repo, then pushes to the registry org's `<kind>-<name>` slot. By default no tags are added — surface a target version via `vibe registry redirect-sync` (below) or by hand.

### Surfacing target tags into the stub — `vibe registry redirect-sync`

In `pass-through-tag` policy, the stub's tags determine which target versions the org's namespace exposes. Mirror them across in one command:

```bash
vibe registry redirect-sync flow:internal-helper
```

What it does:

1. Shallow-clones the stub repo, reads `vibe-redirect.toml` to discover the target URL.
2. `git ls-remote --tags` against both the stub and the target.
3. For every target tag missing on the stub, creates an annotated tag on the stub's `main` commit (the marker-file commit) and pushes it.

Already-present tags are skipped quietly. `pinned`-policy stubs reject the sync command with a clear message — pinned-policy semantically ignores stub-side tags.

The command fits naturally into a periodic CI job: every Monday, `vibe registry redirect-sync flow:internal-helper` against every redirect stub the org owns; new target versions appear in the consumer-facing namespace within a week.

### Manual procedure (fallback)

The stub is just a git repo with a single marker file. If for any reason `vibe registry redirect` cannot run (offline / unsupported host / stub already partially exists), you can equivalently:

```bash
# Step 1: prepare the stub directory.
mkdir flow-internal-helper && cd flow-internal-helper
git init -b main

# Step 2: write the marker.
cat > vibe-redirect.toml <<'EOF'
[redirect]
target_url  = "git@gitlab.acme.example:flows/internal-helper"
description = "Delegated to acme-corp; contact maintainers@acme.example"
EOF

# Step 3 (optional): add a README explaining the delegation.

# Step 4: commit.
git add vibe-redirect.toml README.md
git commit -m "stub: delegate flow:internal-helper to acme-corp"

# Step 5: tag and push to the org's stub slot.
git remote add origin git@github.com:vibespecs/flow-internal-helper.git
git tag v0.1.0
git push -u origin main
git push --tags
```

After `git push --tags`, consumers running `vibe install flow:internal-helper@^0.1` resolve through the stub and fetch content from the acme-corp target.

To certify a new target version (e.g. acme-corp released `v0.2.0`):

```bash
git tag v0.2.0
git push origin v0.2.0
```

Pass-through ref policy means the new stub tag immediately routes resolves of `flow:internal-helper@^0.2` to the target's `v0.2.0`.

## Comparison with related mechanisms

| Mechanism | Set by | Lifetime | Lockfile marker | Use case |
|---|---|---|---|---|
| `[[registry]]` direct package | Registry owner | Long | `source_kind = "registry"`, `via_redirect = null` | Standard ownership |
| `[[registry]]` stub (this page) | Registry owner | Long | `source_kind = "registry"`, `via_redirect = <stub_url>` | Owner delegates content hosting to external party |
| `[[mirror]]` | Consumer | Long | `source_kind = "registry"`; mirror URL not in lockfile | Operator-side fallback URL for the same content |
| `[[override]]` | Consumer | Short | `source_kind = "override"`, `overridden = true` | Operator-side patch / fork pin |
| `[requires.packages]` git-source | Consumer | Long | `source_kind = "git"` | Consumer declares package not in any registry |

The key distinction between **stub** and **mirror** is *who controls the indirection*. Mirror is consumer-side ("if vibespecs is slow, try this URL for the same content"). Stub is org-side ("for this specific package, the content lives over there").

## Out of scope for v0

- **Redirect chains** (stub → stub → real). Rejected at hop = 2.
- **Signed redirect markers** — cryptographic attestation that `target_url` is approved by the org owner. Plain text + content-hash for v0.
- **Auto-deprecation forwarding** (`[redirect.deprecated] new_pkgref = "..."` to forward consumers to a renamed package). Separate feature, separate PROP.
- **`[[mirror]]` against a stub repo** — undefined behaviour for v0; the resolver follows redirect first, mirror semantics apply to the target URL.
- **Editing an existing stub via the CLI**. `vibe registry redirect` only creates fresh stubs; updating the marker file (e.g. to change `target_url`) is a manual `git clone` / edit / push procedure for v0.

## Related

- [`PROP-002 §2.4.2`](../spec/modules/vibe-registry/PROP-002-decentralized-registry.md#redirect) — the architectural decision and the wire-grammar contract.
- [`git-source-dependencies.md`](git-source-dependencies.md) — consumer-side counterpart (declare a dep as `{ git = "..." }` instead of through a registry stub).
- [`registry-auth.md`](registry-auth.md) — per-registry auth regime; the same enum is used by `[redirect].auth`.
