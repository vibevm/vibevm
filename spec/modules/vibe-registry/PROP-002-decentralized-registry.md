# PROP-002: Decentralized, mirror-friendly registry with capability-based depsolver {#root}

<status stage="impl" state="done" comment="B2 2026-07-25: shipped registry contract (M1.1-revision + M1.6 + the M1.13/M1.14 auth/git-source/redirect slices); status line still says accepted-only and the Phase A acceptance boxes stay unchecked - F-032"/>

@fact:milestone-line **Milestone:** M1.1-revision ([`ROADMAP.md`](../../../ROADMAP.md#m11-revision--decentralized-per-package-registry-active-started-2026-04-24)). Phase B lands in M1.6. @status:impl/done
@fact:status-line **Status:** accepted 2026-04-24. @status:impl/done
@fact:supersedes-line **Supersedes (partially):** [PROP-001](PROP-001-git-backend.md) §2.3 (`Registry` trait), §2.4 (cache layout), §2.6 (lockfile `source_uri` format). PROP-001 §2.1 (shell-out-to-git), §2.2 (`GitBackend` trait), §2.5 (freshness TTL), §2.7 (Windows UX) remain authoritative. @status:impl/done
@fact:related **Related:** [spec://org.vibevm.core/vibevm/common/PROP-000](../../common/PROP-000.md) (especially §15 — dep weight, §16 — JTD, §17 — production architecture, §18 — complexity ≥ RPM), [`VIBEVM-SPEC.md` §7](../../../VIBEVM-SPEC.md) (manifest / lockfile schemas), [`VIBEVM-SPEC.md` §8](../../../VIBEVM-SPEC.md) (registry). @status:spec/done

---

## 1. Motivation {#motivation}

@fact:m1-monorepo-state M1.1 shipped a monorepo-shaped registry: one git repository (`anarchic/vibespecs`) contained every package under `<kind>/<name>/v<ver>/` directories, `[registry]` in `vibe.toml` was a singleton URL, the lockfile recorded each package as `git+ssh://…#<kind>/<name>/v<ver>`. This was the cheapest shape to prove the end-to-end install loop worked. @status:impl/done

@fact:wrong-shape-verdict It is the **wrong** shape for v1 shipping. The failure mode, named: Nix. @status:spec/done

@fact:nix-failure-pattern **Nix's failure pattern, precisely.** Nix's flake URL grammar hard-codes hosts (`github:owner/repo`, `gitlab:owner/repo`, `sourcehut:~owner/repo`); every new hosting platform is a `nix` core PR away. Nix's flake registry — the global namespace that maps `nixpkgs` to `github:NixOS/nixpkgs` — is itself hosted on GitHub. Every `flake.nix` in the ecosystem pins an absolute `github:` URL, so migrating `nixpkgs` to a different host would require rewriting every downstream `flake.nix` in the world. Mirror mechanisms (`nix registry add`) are redirects, not true indirection. Identity is URL-tied: `flake.lock` pins URL + rev, so even a transparent mirror causes lockfile churn. The 2022 Russian-maintainer GitHub freeze illustrated the blast radius — a hosting-platform policy decision rippled into the resolve path of the entire ecosystem. @status:spec/done

@fact:nix-mistakes **Underlying mistakes:** URL scheme tied to host, central index on one host, lockfile identity = URL (not content), no indirection layer, no first-class mirror support, naming decentralization never considered. @status:spec/done

@fact:shipped-shape-lead The shape vibevm ships instead: @status:impl/done

- @fact:SHAPE-OWN-REPO Each package is its **own** git repository — no monorepo. Per-package maintainer permissions are hosting-native (a package repo's owner controls access); no central merge queue. @status:impl/done
- @fact:SHAPE-REGISTRY-ARRAY `[[registry]]` is an **array**, priority-ordered. `[[mirror]]` is a first-class fallback layer, transparent to the lockfile. `[[override]]` bypasses the resolver for pins. Schema and code path support all three from day one. @status:impl/done
- @fact:SHAPE-CONTENT-IDENTITY Package identity is `(kind, name, version, content_hash)`. `source_url` is informational. Switching mirrors, migrating between hosts, reconciling with a fork — none of these touch identity. @status:impl/done
- @fact:SHAPE-PLAIN-GIT-URL URL syntax is **just git URL** — `git@host:…`, `ssh://`, `https://`, `file://`. No `github:` / `gitverse:` shorthands. New hosts "just work" as long as `git` speaks to them. @status:impl/done

@fact:impl-phases-lead This PROP locks those decisions. Implementation lands in two phases: @status:impl/done

- @fact:PHASE-A-SCOPE **Phase A (M1.1-revision):** single live `[[registry]]`, structures support multi; mirror / override parsing present, runtime limited to one registry without mirrors; publish utility shipped. @status:impl/done
- @fact:PHASE-B-SCOPE **Phase B (M1.6):** real multi-registry exercised end-to-end, mirror fallback chain, `vibe vendor`, richer publish adapters. @status:impl/done

---

## 2. Decisions {#decisions}

### 2.1 Identity: content-addressed, URL-orthogonal {#identity}

@fact:identity-req `req r1` @status:impl/done

@fact:IDENTITY-TUPLE **Decision.** A package's identity is the tuple `(kind, name, version, content_hash)`. The `content_hash` is `sha256:<hex>` over the deterministically-ordered concatenation of `(rel_path_bytes || 0x00 || file_bytes || 0x00)` for every file in the package directory (the existing scheme from `compute_content_hash` in `vibe-registry`). [PROP-024 §2.2](../../common/PROP-024-code-bearing-packages.md#shippable-tree) re-scopes this to the package's **shippable tree** — its source, minus build output (`.git/`, `.vibe/`, `target/`, `node_modules/`, `.vibeignore` globs) — so a code-bearing package's identity is its source, not its build state; that exclusion lands with the code that implements it. The URL used to fetch the content is **informational** — recorded in the lockfile for debuggability, not for identity. @status:impl/done

@fact:IDENTITY-CONSEQUENCE **Consequence.** Fetching the same `(kind, name, version)` from two different URLs (canonical + mirror, original + fork, upstream + vendored copy) must produce the same `content_hash`. Mismatch is a fatal `IntegrityError`. The effect is: @status:impl/done

- @fact:EFF-LOCKFILE-STABLE mirror-switching, host-migration, and vendoring never change the lockfile; @status:impl/done
- @fact:EFF-MIRROR-SUBSTITUTION a compromised mirror cannot silently substitute content — the mismatch triggers hard fail before any write; @status:impl/done
- @fact:EFF-FORCE-PUSH-CAUGHT a force-pushed tag upstream is caught by the same machinery on the next install. @status:impl/done

@fact:TRUST-MIRROR-HATCH Escape hatch for legitimate mirror-vs-upstream divergence (e.g. during an upstream outage): a `--trust-mirror` flag on `vibe install` / `vibe update`. Never silent; always operator-initiated. **Specified, not shipped** — the CLI carries `--trust-redirect` (a different hatch, for redirect chains) and no mirror equivalent; an operator hitting mirror divergence today has no flag. @status:spec/done

### 2.2 Registry model: `[[registry]]` array, priority-ordered {#registry-model}

@fact:registry-model-req `req r1` @status:impl/done

@fact:REGISTRY-ARRAY **Decision.** `vibe.toml` carries an array of registries: @status:impl/done

```toml
[[registry]]
name   = "vibespecs"
url    = "git@gitverse.ru:vibespecs"
ref    = "main"                              # registry-level metadata ref (reserved; not used today)
naming = "fqdn"
```

- @fact:REG-FIELD-NAME `name` — local alias, used in lockfile `registry` field and in `[[override]]` / `[[mirror]]` targeting. @status:impl/done
- @fact:REG-FIELD-URL `url` — **organization root URL**, not a package repo URL. A registry is a hosting-org; packages are children of it. @status:impl/done
- @fact:REG-FIELD-REF `ref` — reserved for a future registry-level metadata branch (e.g. capability index, trust policy). Not consumed today. @status:impl/done
- @fact:REG-FIELD-NAMING `naming` — convention for mapping a pkgref to a package repo name under this org. Values: `"fqdn"` (**default** — `org.vibevm.world/wal` → `<org>/org.vibevm.world_wal`; introduced and made the default by [PROP-008 §2.5](PROP-008-qualified-naming.md#repo-naming), shipped M1.19), `"kind-name"` (legacy — `flow:wal` → `<org>/flow-wal`; the default this section originally declared, superseded by PROP-008), `"name"` (if name collisions are impossible in a given registry), `"kind/name"` (for hosts supporting nested repos). Other registries may ship with different conventions; the setting is per-registry, not global. @status:impl/done

@fact:REGISTRY-WALK-ORDER Resolution: the solver iterates registries in array order; the first that has a satisfying match for a pkgref wins. Versions of the same pkgref are **not** unioned across registries — this prevents a lower-trust registry from influencing resolve when a higher-trust one already has a valid answer. @status:impl/done

### 2.2.1 Per-registry authentication {#registry-auth}

@fact:registry-auth-req `req r1` @status:impl/done

@fact:AUTH-REGIMES **Decision.** Each `[[registry]]` declares its authentication regime via an `auth` field. Four variants: @status:impl/done

```toml
[[registry]]
name   = "vibespecs"
url    = "https://github.com/vibespecs"
auth   = "none"                                  # default; public read-only

[[registry]]
name      = "internal"
url       = "https://gitlab.company.com/vibespecs"
auth      = "token-env"
token_env = "VIBEVM_REGISTRY_TOKEN_INTERNAL"     # optional override; default = derived from host

[[registry]]
name = "corporate-sso"
url  = "https://corporate.example.com/vibespecs"
auth = "credential-helper"                       # opt in to system git credential.helper / GCM

[[registry]]
name = "ssh-mirror"
url  = "git@host.example.com:vibespecs"
auth = "ssh"                                     # ssh-agent / keys; URL must be ssh-form
```

| `auth` value | What vibevm does | When to use |
| --- | --- | --- |
| @fact:ROW-AUTH-NONE `none` (default) @status:impl/done | Read-only HTTPS or `file://`. **No** credentials are sent; `git ls-remote` / `clone` / `fetch` run with `credential.helper` and `core.askPass` reset to empty **unconditionally — regardless of TTY** (so GUI / GCM popups are never raised for a public registry). A public registry sends no credentials, so a 401 / 403 is not a login failure — it means "no public answer here": the resolver classifies the registry as absent and walks to the next entry, same fall-through as 404. @status:impl/done | Public registries (the default for both `vibespecs` and `vibespecs-gitverse`). @status:impl/done |
| @fact:ROW-AUTH-TOKEN-ENV `token-env` @status:impl/done | Reads a personal access token from `VIBEVM_REGISTRY_TOKEN_<HOST>` (or the explicit `token_env` override) and injects it into the URL as `https://x-access-token:<TOKEN>@<host>/...` for the duration of the git invocation. The token is never logged, never recorded in the lockfile, never appears in git's stderr (modern git redacts passwords). On 401 with the token set: hard error (the token is wrong / expired / scoped wrong). On token absent: hard error directing the operator at the env-var; resolver does not silently fall through. @status:impl/done | Private organisation registries; CI; agent harnesses. Symmetric with the publish-side `VIBEVM_PUBLISH_TOKEN_<HOST>` already specified in §2.10. @status:impl/done |
| @fact:ROW-AUTH-CREDENTIAL-HELPER `credential-helper` @status:impl/done | The opt-in mode. Vibe leaves `credential.helper` / `core.askPass` untouched; the system git falls through to whatever is configured in `~/.gitconfig` (Git Credential Manager on Windows, `osxkeychain` on macOS, `libsecret` on Linux). GUI prompts may appear; that is the point. Only consulted when an interactive TTY is attached *and* `--unattended` is not set; in non-TTY / scripted runs this collapses to the same behaviour as `none` (helpers silenced, 401 → walk). @status:impl/done | Operators with corporate SSO already wired through GCM and a working interactive workflow. @status:impl/done |
| @fact:ROW-AUTH-SSH `ssh` @status:impl/done | URL must be ssh-form (`git@host:org`, `ssh://...`). Authentication is delegated to the system ssh-agent and keys. Vibe does not touch ssh config, does not ask for passphrases — if a passphrase prompt appears, that is the operator's ssh-agent decision. @status:impl/done | The classic developer workflow on personal machines with ssh keys configured. @status:impl/done |

@fact:TOKEN-ENV-DEFAULTING **`token_env` defaulting.** When `auth = "token-env"` and `token_env` is omitted, the env-var name is derived from the registry's host: lowercase host, dots and hyphens to underscores, prefixed with `VIBEVM_REGISTRY_TOKEN_` and uppercased. For `https://gitlab.company.com/vibespecs` the default is `VIBEVM_REGISTRY_TOKEN_GITLAB_COMPANY_COM`. Operators who want stable env-var names across host migrations set `token_env` explicitly; everyone else gets a working default. @status:impl/done

@fact:TOKEN-NEVER-ON-DISK **Token never lands on disk via vibevm.** The token comes from the operator's environment. Vibe reads it, builds the credentialed URL in memory, hands it to the spawned git process, and discards. The lockfile's `source_url` field always carries the **canonical** URL (no embedded credentials) — symmetric with the `[[mirror]]` invariant in §2.3. Token discipline (PROP-000 §20) applies: the value is treated as surface-secret; it does not appear in any vibevm-emitted output. Modern git (≥2.31) auto-redacts passwords from its own stderr, so even on errors the token is not echoed. @status:impl/done

@fact:SILENCING-BY-REGIME **Credential-silencing by regime.** Whether git's interactive credential mechanisms (terminal prompt + GCM + system `credential.helper` + `core.askPass`) are suppressed is decided by the registry's `auth` regime, **not** by the TTY alone: @status:impl/done

- @fact:SIL-NONE **`auth = none` — silenced unconditionally.** A public registry sends no credentials, so there is nothing for git to prompt *for*. Raising a GCM / askPass dialog on a public-registry 401 is never correct — the operator cannot make a missing or private repo public by logging in — so the helpers are reset to empty on every invocation, interactive or not. The 401 returns immediately and classifies as walk-to-next (§2.3.1). *(This is the fix for the interactive-terminal GCM popup: previously `none` left the helpers untouched on a TTY, so a missing public repo — GitHub / GitVerse answer 401, not 404, to hide private repos — turned into a blocking login dialog before the walk could even see the 401.)* @status:impl/done
- @fact:SIL-TOKEN-ENV **`auth = token-env` — silenced regardless (token wins).** The token is injected into the URL; leaving helpers live would only let git second-guess a supplied credential. A 401 with the token set is a real, actionable failure (wrong / expired / mis-scoped) and halts. @status:impl/done
- @fact:SIL-CREDENTIAL-HELPER **`auth = credential-helper` — TTY-aware, by design.** This is the *opt-in* interactive regime: on an interactive TTY without `--unattended` the helpers are left alone so GCM / keychain *can* pop up (that is the point); in non-TTY / `--unattended` / `VIBE_UNATTENDED` runs it collapses to the same silenced behaviour as `none`. @status:impl/done
- @fact:SIL-SSH **`auth = ssh`** — delegated to ssh-agent / system keys; vibe touches neither credential helpers nor ssh config. @status:impl/done

@fact:UNATTENDED-ADDS-ONLY The global `--unattended` flag / `VIBE_UNATTENDED` (and the `VIBEVM_GIT_SILENCE_HELPERS` override) still force silencing on for *every* regime — they only ever *add* silencing, never remove the unconditional silencing `none` now carries. The matrix: @status:impl/done

| Mode | Interactive (TTY, no `--unattended`) | Non-interactive (no TTY OR `--unattended`) |
| --- | --- | --- |
| @fact:ROW-MTX-NONE `auth = none` @status:impl/done | **Helpers silenced; 401 → walk** @status:impl/done | Helpers silenced; 401 → walk @status:impl/done |
| @fact:ROW-MTX-TOKEN-ENV `auth = token-env` @status:impl/done | Token injected; helpers silenced regardless (token wins) @status:impl/done | Token injected; helpers silenced @status:impl/done |
| @fact:ROW-MTX-CREDENTIAL-HELPER `auth = credential-helper` @status:impl/done | Helpers untouched; GCM / keychain may pop up @status:impl/done | Helpers silenced; behaves like `none` @status:impl/done |
| @fact:ROW-MTX-SSH `auth = ssh` @status:impl/done | URL must be ssh-form; behaviour delegated to ssh-agent / system @status:impl/done | Same @status:impl/done |

@fact:NONE-DEFAULT-SAFE This is what makes the `none` default safe everywhere — CI / opencode harnesses **and** an interactive `vibe install` at a real terminal — with no GUI popup for a public-registry 401. An operator who genuinely needs to authenticate against a host declares `auth = "credential-helper"` (or `token-env`) for that registry; the public default never prompts. @status:impl/done

@fact:AUTH-MIGRATION **Migration.** Pre-this-decision `vibe.toml` files with no `auth` field on `[[registry]]` parse as `auth = "none"` (default), preserving every current behaviour for public registries. Operators who want token-based access add the field explicitly; nothing breaks for anyone else. @status:impl/done

### 2.2.2 Machine-global registry config: `~/.vibe/registry.toml` {#global-config}

@fact:global-config-req `req r1` @status:impl/done

@fact:GLOBAL-REGISTRY-FILE **Decision.** Registry settings may also live in a per-user file resolved through the settings chokepoint (`vibe_core::settings::registry_config_path` → `~/.vibe/registry.toml`, or `$VIBE_SETTINGS/registry.toml`). It carries the same `[[registry]]` / `[[mirror]]` / `[[override]]` sections as a project `vibe.toml` — **any** registry, not only local ones: a remote `https://` / `ssh://` / `git@` org (with `auth`) is merged and searched exactly like a `file://` / path repo. A common motivation is keeping **machine-local** registries (a `file://` checkout, a path repo) out of a team-shared `vibe.toml`, where a hard-coded local path would differ per teammate; but a whole extra remote registry can be added machine-wide the same way. (Locality matters only to `--offline`, §2.2.2.1.) @status:impl/done

@fact:MERGE-PROJECT-FIRST **Merge — project first, dedupe by name.** The effective registry list is the project's `[[registry]]` entries followed by the global file's, with a `name` collision resolved in the **project's** favour (the project entry wins; the global one is dropped). Mirrors are concatenated (project first). Overrides are project-first, deduped by `pkgref` (project wins). The merge is a pure function (`vibe_core::merge_effective`), verified in isolation. A project's explicit declaration always outranks a machine default, so a shared `vibe.toml` stays authoritative for the team while each machine supplements it with its own local repositories. @status:impl/done

#### 2.2.2.1 `--offline` resolves local registries only {#offline-local}

@fact:offline-local-req `req r1` @status:impl/done

@fact:OFFLINE-LOCAL-ONLY **Decision.** `--offline` narrows the effective set to **local** sources — `file://` and bare filesystem paths — and drops every **remote** one (`http(s)://`, `ssh://`, `git://`, scp `user@host:…`). A machine-local registry (project or global) still resolves offline, while a github/gitverse registry is simply absent — no network round-trip, no credential prompt. When the local-only set is empty and there is no embedded registry or explicit `--registry <dir>`, offline resolution fails with an actionable message rather than reaching the network. Locality is classified by URL scheme (`vibe_core::url_is_local`). @status:impl/done

### 2.2.3 Disabling a registry: `enabled` {#enabled}

@fact:enabled-req `req r1` @status:impl/done

@fact:ENABLED-FLAG **Decision.** Every `[[registry]]` carries an `enabled` flag, default `true`. Setting `enabled = false` switches a registry off **without deleting its entry** — it is skipped by **every** resolution path (`install` / `outdated` / `search` / `registry sync` / `vendor`), because the filter lives at the one resolver-construction point (`MultiRegistryResolver::from_manifest`): a disabled registry is never built, so nothing downstream can consult it. Re-enable by flipping the flag back; no re-add. The default `true` is skipped on serialize, so only an explicit `enabled = false` appears in a written `vibe.toml`. The flag applies uniformly to a project `vibe.toml` and the machine-global `~/.vibe/registry.toml`. @status:impl/done

### 2.3 Mirror layer: transparent, integrity-verified {#mirror}

@fact:mirror-req `req r1` @status:impl/done

@fact:MIRROR-LAYER **Decision.** `[[mirror]]` entries are parallel alternative URLs for a specific registry (or `*` for any). During fetch: @status:impl/done

1. @fact:MIR-PRIORITY-ORDER For the target registry, try mirrors in `priority` ascending order. @status:impl/done
2. @fact:MIR-CANONICAL-FALLTHROUGH Fall through to the canonical `[[registry]].url` if all mirrors fail or return content whose hash disagrees with an existing lockfile pin. @status:impl/done
3. @fact:MIR-CANONICAL-IN-LOCKFILE The canonical URL is always recorded as the `source_url` in the lockfile when the fetch produces a new pin. Mirror URLs do not appear there. @status:impl/done

```toml
[[mirror]]
of       = "vibespecs"      # or "*" to mirror any registry by default
url      = "https://mirror.internal/vibespecs"
priority = 1
```

@fact:MIRROR-INTEGRITY-MANDATORY Mirror integrity verification is **mandatory**, not optional. A mirror whose `content_hash` for `(kind, name, version)` differs from the lockfile pin fails the install with an actionable error. This closes the supply-chain hole where a hijacked mirror could substitute content. @status:impl/done

@fact:mirror-staging Phase A: `[[mirror]]` parser and lockfile-canonical-URL invariant ship. Runtime fallback chain lands in Phase B, because we want to exercise it against a second *real* mirror, not just a constructed test fixture. @status:impl/done

#### 2.3.1 Failure-mode discriminator: registry-walk vs mirror-walk {#failure-discriminator}

@fact:failure-discriminator-req `req r1` @status:impl/done

@fact:discriminator-motivation `[[registry]]` and `[[mirror]]` mean different things, and the resolver treats their failures differently. Confusing them produces either silent mis-config (treating typos in a primary URL as transient) or broken offline workflows (failing fast on a mirror that is supposed to absorb outages). @status:spec/done

- @fact:REGISTRY-WALK-SEMANTICS A `[[registry]]` is a **distinct package source** — its own naming convention, its own publishing identity, its own trust scope. The priority-ordered registry walk falls through on **`UnknownPackage` only**: a registry that confidently answers "I don't have this package" is free to defer to the next one. Any other primary failure — connect-failure (DNS / TCP), auth-failure on a registry that explicitly requires authentication, server error, malformed manifest — halts the install with an actionable error. This is the same policy Cargo and npm apply to a registry that errors out: the operator wants to know about a typo or an outage, not paper over it with a different registry that may carry a different version. @status:impl/done

  - @fact:AUTH-AWARE-401 **`auth`-aware 401 classification (§2.2.1).** On `auth = "none"` a 401 / 403 response is an `UnknownPackage` signal, not an auth-failure: the registry is declared public, anything that responds with "you cannot read this without credentials" is — from this consumer's standpoint — equivalent to "this package does not have a public answer here." The walk falls through to the next registry, exactly like a 404. This is what unblocks the common case where one host (GitVerse) returns 401 for a missing repo while another (GitHub) returns 404 — the resolver treats both uniformly. On `auth = "token-env"` or `"credential-helper"` a 401 is a real `AuthFailed` and halts: the registry was declared as authenticated, the credentials presented were rejected, this is information the operator must see. @status:impl/done
- @fact:MIRROR-WALK-SEMANTICS A `[[mirror]]` is an **availability copy of the same source** — same naming, same identity, same `content_hash`. The mirror walk falls through on **any availability failure** (`NetworkUnreachable`, `AuthFailed` on the mirror, server error, `content_hash` mismatch). `RepoNotFound` from a mirror bubbles up to the registry-walk layer (same policy as if the canonical primary had said `UnknownPackage`), because absence-of-package is a registry-level fact, not a mirror-level one. @status:impl/done

@fact:discriminator-why-split **Why split.** A single uniform "fall through on any failure" rule maximises resilience but loses the ability to detect a misconfigured primary. A single uniform "fail-fast on any failure" rule preserves diagnostics but breaks the offline / vendor-mirror story Phase B v0 explicitly enables (`vibe registry vendor` → wire as `file://` `[[mirror]]` → install while the network primary is down). Splitting failure semantics by entry kind keeps both properties intact. @status:spec/done

@fact:DISCRIMINATOR-UX **Operator UX.** When a primary `[[registry]]` connect-fails, the error message points the operator at `vibe registry list` (typo check) and the network (outage check). When a `[[mirror]]` fails, the resolver logs `tracing::debug!` and tries the next mirror; the operator learns about it only if every source disappears, in which case the most informative diagnostic — the primary's own error — is what surfaces. @status:impl/done

@fact:DISCRIMINATOR-IMPL **Implementation.** The error classifier (`crates/vibe-registry/src/git_backend/shell.rs::classify_stderr_message`) maps git's free-form stderr into a typed `GitError` variant. `GitPackageRegistry`'s mirror walk pattern-matches on the variant (`NetworkUnreachable` / `AuthFailed` / `CommandFailed` / mirror-side `RepoNotFound`) and falls through; the canonical primary's `UnknownPackage` is what `MultiRegistryResolver` translates into a registry-walk fall-through. The split lives in code at the trait-method boundary, not in a single switch. @status:impl/done

### 2.4 Overrides: surgical pin of source location {#override}

@fact:override-req `req r1` @status:impl/done

@fact:OVERRIDE-SHORT-CIRCUIT **Decision.** `[[override]]` bypasses the registry layer for a named pkgref: @status:impl/done

```toml
[[override]]
pkgref     = "flow:wal"
source_url = "git@mycompany:forks/wal"
ref        = "my-fix-branch"                # tag, branch, or commit
reason     = "awaiting upstream PR #42"     # surfaces in `vibe list --overrides`
```

@fact:OVERRIDE-SEMANTICS The resolver short-circuits: it does not consult `[[registry]]` for this pkgref at all; it fetches directly from the given URL at the given ref. Content hash is still pinned in the lockfile and verified on each install — an override does not relax integrity. The lockfile records `overridden = true` on that entry. A `vibe list --overrides` flag is specified here and **not shipped** — the lockfile field is the only surface today. @status:impl/done

@fact:override-analogues This is the vibevm analogue of Cargo's `[patch]` and Go's `replace`. Same shape, same use case: pinning a fork during an in-flight upstream PR, emergency hotfixes, internal forks of public packages. @status:spec/done

### 2.4.1 Git-source declarations: `[requires.packages]` table-form {#git-source}

@fact:git-source-req `req r1` @status:impl/done

@fact:GIT-SOURCE-DECL **Decision.** A dependency may be declared as a first-class git-source in `[requires.packages]` — fetching the package from an arbitrary git repository instead of resolving it through `[[registry]]`. This is the vibevm analogue of Cargo's `[dependencies] foo = { git = "..." }`, npm's `"foo": "git+https://..."`, Poetry's `foo = { git = "..." }`, Bundler's `gem 'foo', git: '...'`, Go modules' baseline behaviour. The use cases are: @status:impl/done

- @fact:GS-USE-PRIVATE **Internal / private packages without a registry org.** A team has one private vibevm package they want to share across projects; standing up a multi-package `[[registry]]` org is overkill, and a single-repo declaration is the natural shape. @status:spec/done
- @fact:GS-USE-FORK **Active development against a fork.** The fork is the canonical source while the upstream PR is in flight (overlap with `[[override]]` — see "Comparison with override" below). @status:spec/done
- @fact:GS-USE-CROSS-ORG **Cross-organisation pulls.** A project consumes a package whose author lives in a different git org than any registered registry. @status:spec/done

@fact:GS-WIRE-FORM **Wire form.** `[requires.packages]` becomes a TOML table whose values are either a version-constraint **string** (registry-resolved, the M1.13 shape) or an inline-table (registry-resolved with options, or git-source). The legacy array-of-strings shape (`packages = ["flow:wal@^0.3"]`) parses transparently into table-form on read; on round-trip the manifest writes table-form. @status:impl/done

```toml
[requires.packages]
# Registry-resolved, simple constraint:
"flow:wal"      = "^0.3"
"feat:auth"     = "^0.5"

# Registry-resolved, table-form (reserved for future options like features):
"stack:rust-cli" = { version = "^0.2" }

# Git-source, immutable tag:
"flow:internal-helper" = { git = "git@gitlab.company.com:specs/internal-helper", tag = "v0.1.0" }

# Git-source, immutable commit SHA:
"flow:wal-fork" = { git = "https://github.com/me/flow-wal-fork", rev = "abc12345" }

# Git-source, mutable branch (HEAD on every resolve):
"flow:experimental" = { git = "https://github.com/me/flow-experimental", branch = "main" }

# Git-source on a private host with explicit auth:
"flow:internal-secret" = {
  git       = "https://gitlab.company.com/specs/internal-secret",
  tag       = "v1.0.0",
  auth      = "token-env",
  token_env = "VIBEVM_REGISTRY_TOKEN_GITLAB_COMPANY_COM",
}

# Git-source with verification version constraint:
"flow:checked" = {
  git     = "https://github.com/me/flow-checked",
  tag     = "v0.1.0",
  version = "^0.1",
}
```

@fact:gs-grammar-lead The wire grammar for the inline-table values: @status:impl/done

| Field | Required | Meaning |
|---|---|---|
| @fact:ROW-GS-VERSION `version` @status:impl/done | optional (registry) / optional (git) @status:impl/done | Version constraint. Registry-resolved: identical semantics to the bare-string form. Git-source: **verification only** — after resolving the package version from the git ref, the constraint must be satisfied; otherwise the install fails with `VersionMismatch`. @status:impl/done |
| @fact:ROW-GS-GIT `git` @status:impl/done | required for git-source @status:impl/done | Full git URL of the single-package repository. Same URL grammar as `[[registry]] url` and `[[override]] source_url` — `git@host:…`, `ssh://`, `https://`, `file://`. No host shorthands. @status:impl/done |
| @fact:ROW-GS-TAG `tag` @status:impl/done | one of `tag`/`rev`/`branch` is required @status:impl/done | Immutable tag. Resolved commit pinned in lockfile; force-pushed tag rewrite caught as `IntegrityError` per §2.1. @status:impl/done |
| @fact:ROW-GS-REV `rev` @status:impl/done | one of `tag`/`rev`/`branch` is required @status:impl/done | Commit SHA (full or short, ≥ 7 chars). Most strict; lockfile records the same SHA. @status:impl/done |
| @fact:ROW-GS-BRANCH `branch` @status:impl/done | one of `tag`/`rev`/`branch` is required @status:impl/done | Mutable branch. Lockfile records the resolved commit at install time; subsequent `vibe update` re-walks branch HEAD. **Mutable** — see "Mutability and `vibe update`" below. @status:impl/done |
| @fact:ROW-GS-AUTH `auth` @status:impl/done | optional @status:impl/done | Per-source auth regime. Same enum as `[[registry]] auth`: `"none" | "token-env" | "credential-helper" | "ssh"`. Default `"none"`. @status:impl/done |
| @fact:ROW-GS-TOKEN-ENV `token_env` @status:impl/done | optional @status:impl/done | Env-var name when `auth = "token-env"`. Default derived from URL host (same rule as `[[registry]]` per §2.2.1). @status:impl/done |

@fact:GS-EXACTLY-ONE-REF **Exactly one** of `tag` / `rev` / `branch` must be present in a git-source declaration. Zero is rejected at parse time with `MissingRef`. Two or more rejected with `ConflictingRefs`. There is no "default branch HEAD" fall-back — too magical for a security-sensitive surface; explicit > implicit. @status:impl/done

@fact:GS-RESOLUTION-ORDER **Resolution order.** When the resolver looks up a pkgref, the source is decided in this order: @status:impl/done

1. @fact:RES-ORDER-OVERRIDE **`[[override]]`** — if a matching override exists, it short-circuits everything (existing §2.4 semantics, unchanged). @status:impl/done
2. @fact:RES-ORDER-GIT-SOURCE **Git-source declaration** in `[requires.packages]` — if the value carries a `git` field, the resolver fetches directly from that URL at the declared ref. `[[registry]]` is not consulted for this pkgref. Same content-hash discipline as override. @status:impl/done
3. @fact:RES-ORDER-REGISTRY **Registry-resolved declaration** — bare string or `{ version = "..." }` table — falls through to the existing §2.2 priority-ordered registry walk. @status:impl/done

@fact:res-order-rationale Override > git-source-decl reflects the semantic "override is intentional patch on top of a declared dependency", same as Cargo's `[patch]` overriding `[dependencies] foo = { git = "..." }`. @status:spec/done

@fact:GS-IDENTITY **Identity.** Identical content-hash discipline as registry-resolved (§2.1): identity is `(kind, name, version, content_hash)`; the URL is informational. Two projects that pull the same git-source from different mirrors and produce the same `content_hash` are bit-identical installs. Force-pushed tag rewrite caught as `IntegrityError`. @status:impl/done

@fact:GS-IDENTITY-VERIFICATION The pkgref `<kind>:<name>` is read from the package's `vibe-package.toml` `[package]` section on the resolved git ref (same path as registry-resolved manifest fetch via `git archive`). The resolver verifies that the `(kind, name)` declared in `[requires.packages]` matches what the repo actually carries; mismatch = `PackageIdentityMismatch`. This means a malicious git-source cannot impersonate `flow:wal` if its `vibe-package.toml` declares it as `feat:auth`. @status:impl/done

@fact:GS-MUTABILITY **Mutability and `vibe update`.** Tags and revs are immutable by definition; force-push is detected via content-hash. Branches are explicitly mutable: `vibe install` against a branch resolves to the current branch HEAD and pins that commit in the lockfile. `vibe update` re-walks each branch-declared git-source, and if HEAD has moved, re-resolves and re-locks. `vibe install` (no flag) **does not** chase a branch's HEAD on subsequent runs — the lockfile's `resolved_commit` is authoritative until `update` is called. This matches Cargo's behaviour (`cargo build` does not bump branch deps; `cargo update` does). @status:impl/done

@fact:GS-AUTH-EXPLICIT **Auth.** Per-source `auth` is **explicit, not host-derived**. The resolver does not look at `[[registry]] auth` for the same host and apply it transitively to a git-source pointing at that host — too magical, creates implicit ordering dependencies between sections of the manifest. If a project has multiple packages from the same private host, the operator can either: @status:impl/done

- @fact:GS-AUTH-VIA-REGISTRY declare them through `[[registry]]` with shared auth (the DRY way; recommended for ≥ 3 packages from one host), or @status:impl/done
- @fact:GS-AUTH-PER-SOURCE declare each through `[requires.packages]` with explicit `auth` / `token_env` per source (verbose but transparent). @status:impl/done

@fact:GS-TOKEN-DISCIPLINE The token-discipline contract from §2.2.1 (read once, in-memory, scrubbed from `.git/config` after bootstrap) applies identically to git-source. @status:impl/done

@fact:GS-CACHE-SLOT **Cache layout.** Same as registry-resolved (§2.6), keyed by canonical URL hash. A git-source pointing at `https://github.com/me/flow-internal` lives at `~/.vibe/registries/<sha256(canonical-url)>/packages/<group>.<name>/clone/` — the qualified form since M1.19 (this line originally showed the earlier kind-name path `packages/flow-internal/clone/`). Multiple git-source declarations across different consumer projects pointing at the same URL share the same cache slot. @status:impl/done

@fact:GS-LOCKFILE-SOURCE-KIND **Lockfile schema.** A new `source_kind` field per `[[package]]` makes the resolution path explicit: @status:impl/done

```toml
[[package]]
kind            = "flow"
name            = "internal-helper"
version         = "0.1.0"
source_kind     = "git"                                                # NEW field; "registry" | "git" | "override"
registry        = ""                                                   # empty for git / override
source_url      = "git@gitlab.company.com:specs/internal-helper"
source_ref      = "v0.1.0"                                             # tag / branch name / rev as declared
resolved_commit = "abc123…def"
content_hash    = "sha256:…"
overridden      = false
```

@fact:SOURCE-KIND-VALUES `source_kind` is `"registry"` for the M1.13 default, `"git"` for git-source declarations, `"override"` for `[[override]]`-resolved (existing `overridden = true` is preserved as redundant marker for back-compat). Lockfile schema bumps to v3; v2 lockfiles read transparently and migrate to v3 on next install (everything that was `overridden = true` becomes `source_kind = "override"`; everything else `"registry"`). @status:impl/done

@fact:GS-TRANSITIVES **Transitive dependencies.** A git-source package's own `[requires]` declarations are resolved through the consuming project's `[[registry]]` (same path as override-resolved, §2.4). A git-source package may itself declare git-source dependencies — they recursively resolve through the same path, with cycle detection inheriting the existing solver's protection. There is no "git-source registry" concept; the consumer project's manifest is the authoritative resolution surface for transitives. @status:impl/done

@fact:gs-vs-override-lead **Comparison with `[[override]]`.** Both declare a git URL + ref for a pkgref. The difference is semantic: @status:impl/done

| | `[requires.packages]` git-source | `[[override]]` |
|---|---|---|
| @fact:ROW-CMP-ROLE Role @status:impl/done | Primary declaration @status:impl/done | Patch on top of an existing declaration @status:impl/done |
| @fact:ROW-CMP-PAIRS Pairs with @status:impl/done | A bare `[requires.packages]` entry? No — git-source IS the declaration @status:impl/done | A `[requires.packages]` entry — override patches it @status:impl/done |
| @fact:ROW-CMP-LOCKFILE Lockfile marker @status:impl/done | `source_kind = "git"` @status:impl/done | `source_kind = "override"`, `overridden = true` @status:impl/done |
| @fact:ROW-CMP-LIFETIME Typical lifetime @status:impl/done | Long-lived (project's normal architecture) @status:impl/done | Short-lived (awaiting upstream PR / hotfix) @status:impl/done |
| @fact:ROW-CMP-LIST `vibe list --overrides` (specified, not shipped) @status:spec/done | Not surfaced (it is a normal dependency) @status:impl/done | Surfaced via the lockfile's `overridden` field @status:impl/done |
| @fact:ROW-CMP-REMOVAL Removal @status:impl/done | `vibe uninstall <pkgref>` drops the entry @status:impl/done | Drop the `[[override]]` block; the underlying dependency comes back @status:impl/done |

@fact:GS-AND-OVERRIDE-COMPOSE A project may use both: declare `flow:internal` through `[requires.packages]` git-source (the architecture), and override `flow:wal` through `[[override]]` while waiting for an upstream fix (the patch). The override always wins. @status:impl/done

@fact:GS-ARRAY-MIGRATION **Migration: legacy array form parses as map form.** Existing manifests with `packages = ["flow:wal@^0.3", ...]` continue to parse — the deserializer accepts both shapes for a release window. `["flow:wal@^0.3"]` is canonically equivalent to `{ "flow:wal" = "^0.3" }`. On the next manifest write (any command that mutates `vibe.toml` — `vibe install`, `vibe uninstall`, `vibe registry add`), the array form is rewritten in map form. After that the array form is no longer present in the file. The parser keeps accepting the array form indefinitely — there is no version-fence; both shapes are equivalent forever, but only the map form is written. @status:impl/done

@fact:GS-SLICE-SCOPE **Out of scope for this slice.** Multiple `git-source` entries against the same `(kind, name)` with different URLs (i.e. parallel forks of the same package): rejected as `DuplicateDeclaration`. There is no "first-priority" fallback chain for git-source — the operator picks one URL. If they need failover, that's `[[mirror]]` territory, which is registry-only by design (§2.3). `vibe registry test` does not currently probe git-source declarations; the diagnostic is registry-scoped because git-source has no fall-through walk to validate. May add a `vibe deps test` (or extend `registry test`) in a follow-up if operators ask. @status:impl/done

### 2.4.2 Registry redirect: delegating package content to an external repo {#redirect}

@fact:redirect-req `req r1` @status:impl/done

@fact:REDIRECT-STUB **Decision.** A registry org may host a **stub repo** for a package — a normal `<org>/<kind>-<name>` repository whose content is **not** the package itself but a single file pointing at an external git repository where the package actually lives. The resolver, when fetching the package manifest, transparently follows the pointer; consumers `vibe install <pkgref>` see no difference from a direct registry-resolved package. The use case is **delegation**: an org owner wants the package to live in their namespace (so consumers find it via the org's `[[registry]]` walk without knowing about the external author) but offload the development, the PR queue, and the hosting platform's permission management to a different team or person who already has their own repo. @status:impl/done

@fact:redirect-analogues This is the vibevm analogue of Linux distro virtual packages with `Provides:` pointing at external SRPMs, DNS CNAME records, GitHub's repo-redirect feature, and Cargo's never-shipped `[workspace.metadata.redirect]` proposal. The closest direct analogue is **Bundler's `gem "foo", git: "..."` declared at the registry level rather than the consumer level** — and that is exactly what this is: registry-side declaration that "this package's content lives elsewhere". @status:spec/done

@fact:REDIRECT-MARKER-FILE **Marker file.** A stub repo carries `vibe-redirect.toml` at its root **instead of** `vibe-package.toml`. Both files in the same repo at the same ref is rejected at parse as `AmbiguousStub`. @status:impl/done

```toml
# vibe-redirect.toml at the root of <org>/<kind>-<name> stub repo

[redirect]
target_url = "git@gitlab.acme.example:flows/internal-helper"

# Tag policy. Default "pass-through-tag" — covered by the absence of these
# fields. Operator must declare `pinned_ref` if `ref_policy = "pinned"`.
# ref_policy = "pinned"
# pinned_ref = "v0.3.0"

# Auth-hint for the consumer's resolver when fetching from `target_url`.
# Same enum as [[registry]] auth (§2.2.1); same env-var conventions.
# Default "none".
# auth      = "token-env"
# token_env = "VIBEVM_TARGET_TOKEN_GITLAB_ACME_EXAMPLE"

# Optional human-readable note surfaced by `vibe show <pkgref>` and
# `vibe registry list`. Useful for "contact: …", "delegated to …", etc.
description = "Delegated to acme-corp; contact maintainers@acme.example"
```

@fact:redirect-grammar-lead **Wire grammar:** @status:impl/done

| Field | Required | Meaning |
|---|---|---|
| @fact:ROW-RD-TARGET-URL `target_url` @status:impl/done | required @status:impl/done | Full git URL of the package's actual content repository. Same URL grammar as `[[registry]] url`. @status:impl/done |
| @fact:ROW-RD-REF-POLICY `ref_policy` @status:impl/done | optional @status:impl/done | `"pass-through-tag"` (default) or `"pinned"`. @status:impl/done |
| @fact:ROW-RD-PINNED-REF `pinned_ref` @status:impl/done | required iff `ref_policy = "pinned"` @status:impl/done | Tag, branch, or commit on `target_url` that ALL consumers resolve to, regardless of which version they ask for. @status:impl/done |
| @fact:ROW-RD-AUTH `auth` @status:impl/done | optional @status:impl/done | Auth regime for `target_url`. Same enum as `[[registry]]` auth. Default `"none"`. @status:impl/done |
| @fact:ROW-RD-TOKEN-ENV `token_env` @status:impl/done | optional @status:impl/done | Env-var name when `auth = "token-env"`. Default derived from target host. @status:impl/done |
| @fact:ROW-RD-DESCRIPTION `description` @status:impl/done | optional @status:impl/done | Free-form text shown to operators. @status:impl/done |

@fact:REDIRECT-RESOLVER-FLOW **Resolver behaviour.** When fetching a package manifest at a ref `T` from a registry-level `<stub_url>`: @status:impl/done

1. @fact:RD-STEP-PROBE-PACKAGE Probe `git archive --remote=<stub_url> <T> vibe-package.toml`. If found — normal package, proceed as today (§2.5). @status:impl/done
2. @fact:RD-STEP-PROBE-REDIRECT If the file is missing, probe `vibe-redirect.toml` at the same ref. If found — this is a stub. @status:impl/done
3. @fact:RD-STEP-PARSE Parse the marker. Compute `target_ref`: @status:impl/done
   - @fact:RD-REF-PASS-THROUGH `ref_policy = "pass-through-tag"` (default): `target_ref = T` (same tag name on target). @status:impl/done
   - @fact:RD-REF-PINNED `ref_policy = "pinned"`: `target_ref = pinned_ref` (all stub tags collapse to this single target ref; `T` from step 1 is informational metadata only). @status:impl/done
4. @fact:RD-STEP-AUTH Apply `[redirect].auth` (or its host-derived default) to fetch from `target_url`. @status:impl/done
5. @fact:RD-STEP-REENTER Re-enter the standard resolution path against `target_url` at `target_ref`. Fetch `vibe-package.toml`, compute content-hash over target content, fetch package files for install via the same `GitPackageRegistry` machinery used elsewhere. @status:impl/done
6. @fact:RD-STEP-HOP-LIMIT **Hop limit: 1.** If `target_url`'s content-root is itself a stub (carries `vibe-redirect.toml`), reject with `RedirectChainNotAllowed`. There is no chain-following — stubs are flat indirection, not a redirect graph. @status:impl/done

@fact:REDIRECT-TAG-VISIBILITY **Tag visibility.** `list_versions(stub_url)` returns the tags of the **stub** repo, not the target. The org owner controls which versions surface in their namespace by managing stub tags — which is exactly the gating mechanism a registry already has via `vibe registry publish`. Adding a new version to the namespace = `git tag v<ver> && git push origin v<ver>` against the stub repo (or `vibe registry redirect-sync <pkgref>` if it auto-mirrors target tags — see "Sync helper" below). The stub itself need contain no actual code — only `vibe-redirect.toml`, optionally a `README.md` for humans browsing the repo. @status:impl/done

@fact:REDIRECT-VERSION-GATING The fact that stub-tags exist independently of target-tags is the key affordance: org owner certifies each version that enters their namespace. A target tag `v2.0.0` with a breaking change does NOT automatically appear in the org's namespace — the owner must `git tag v2.0.0 && git push origin v2.0.0` against the stub repo. Pass-through happens during a single resolve, not during version listing. @status:impl/done

@fact:REDIRECT-SYNC-HELPER **Sync helper (`vibe registry redirect-sync`).** Org owner can run `vibe registry redirect-sync <pkgref>` to copy target-side tags into the stub repo (with operator confirmation per tag, or `--all` for batch). This is opt-in convenience tooling; the stub repo is just a normal git repo and tags can equally be managed by hand or CI. @status:impl/done

@fact:REDIRECT-IDENTITY **Identity and content-hash.** Identity remains `(kind, name, version, content_hash)` per §2.1. The `content_hash` is computed over the **target's** content, not the stub's. The stub repo carries only `vibe-redirect.toml` and (optionally) human-readable companion files; nothing in the stub ships into the consumer project. A force-pushed target tag is detected exactly as it would be for a non-redirected package: hash mismatch on the next install raises `IntegrityError`. @status:impl/done

@fact:redirect-trust-lead **Trust model.** The stub is mutable — its owner can change `target_url` at any commit. Defence is layered: @status:impl/done

- @fact:RD-TRUST-CONTENT-HASH **Content-hash in lockfile** catches a target switch on the next install. The consumer sees `IntegrityError` and can investigate before any write happens. @status:impl/done
- @fact:RD-TRUST-FLAG **`--trust-redirect`** flag (parallel to `--trust-mirror` from §2.1) lets an operator accept a deliberate target switch — e.g. when the external maintainer migrates their hosting from GitLab to Forgejo. Never silent; always operator-initiated. @status:impl/done
- @fact:RD-TRUST-DESCRIPTION **Description field** lets the org owner publish contact / verification info for their delegate; consumers can manually verify out of band. @status:impl/done
- @fact:RD-TRUST-SIGNED-FUTURE **Future: signed redirects** (out of scope; tracked under §7 open questions). For v0, plain text + content-hash is the contract. @status:spec/done

@fact:REDIRECT-LOCKFILE-FIELD **Lockfile shape.** A new `via_redirect` field per `[[package]]` records the stub URL when a redirect was followed. `null` (or absent) for non-redirected packages. @status:impl/done

```toml
[[package]]
kind            = "flow"
name            = "internal-helper"
version         = "0.3.0"
source_kind     = "registry"                                     # registry-resolved (via stub)
registry        = "vibespecs"                                    # which [[registry]] hosted the stub
source_url      = "git@gitlab.acme.example:flows/internal-helper"  # target URL — actual content
via_redirect    = "git@github.com:vibespecs/flow-internal-helper"  # NEW; stub URL that delegated
source_ref      = "v0.3.0"                                       # target ref (= stub tag for pass-through, = pinned_ref for pinned)
resolved_commit = "abc123…def"                                   # target commit
content_hash    = "sha256:…"                                     # over target content
overridden      = false
```

@fact:VIA-REDIRECT-DIAGNOSTIC `via_redirect` is purely diagnostic / auditing — `vibe show <pkgref>` surfaces it; `vibe list --json` includes it; the resolver does not consult it on subsequent installs (lockfile is authoritative). Lockfile schema bumps to v3 (same bump as §2.4.1's `source_kind`); v2 lockfiles read transparently and migrate on next install with `via_redirect = null` for all entries. @status:impl/done

@fact:REDIRECT-CACHE **Cache layout.** Both stub and target appear in the per-user registry cache (§2.6) as separate entries keyed by their canonical URL hash. Stub cache holds the `vibe-redirect.toml` parse result for freshness window TTL (1 hour, per §2.6); target cache holds the actual package content. Consequence: the redirect file is not re-fetched on every consume within the freshness window, but the target's manifest is consulted per resolution call as today. @status:impl/done

@fact:REDIRECT-AUTH-LAYERS **Auth for the stub vs auth for the target.** Two independent layers: @status:impl/done

- @fact:RD-AUTH-STUB Stub auth = the registry's `[[registry]] auth` regime. The stub repo is a child of the registry org and inherits its auth. If the registry is `auth = "token-env"`, fetching the stub uses that token. @status:impl/done
- @fact:RD-AUTH-TARGET Target auth = the redirect's `[redirect].auth`. Independent. The stub may be in a public registry (`auth = "none"`) but point at a private target (`auth = "token-env"` against a different host). @status:impl/done

@fact:REDIRECT-TOKEN-PLUMBING Tokens flow through the same M1.14 plumbing: read once at resolver-open time, kept in memory, scrubbed from `.git/config` after any clone. The `inject_token` / `set_remote_url` discipline applies identically to both URLs. @status:impl/done

@fact:redirect-comparison-lead **Comparison with related mechanisms:** @status:impl/done

| Mechanism | Set by | Lifetime | Lockfile marker | Use case |
|---|---|---|---|---|
| @fact:ROW-MECH-DIRECT `[[registry]]` direct package @status:impl/done | Registry owner @status:impl/done | Long @status:impl/done | `source_kind = "registry"`, `via_redirect = null` @status:impl/done | Standard ownership @status:impl/done |
| @fact:ROW-MECH-STUB `[[registry]]` stub (this section) @status:impl/done | Registry owner @status:impl/done | Long @status:impl/done | `source_kind = "registry"`, `via_redirect = <stub_url>` @status:impl/done | Owner delegates content hosting to external party @status:impl/done |
| @fact:ROW-MECH-MIRROR `[[mirror]]` @status:impl/done | Consumer @status:impl/done | Long @status:impl/done | `source_kind = "registry"`; mirror URL not in lockfile @status:impl/done | Operator-side fallback URL for the same content @status:impl/done |
| @fact:ROW-MECH-OVERRIDE `[[override]]` @status:impl/done | Consumer @status:impl/done | Short @status:impl/done | `source_kind = "override"`, `overridden = true` @status:impl/done | Operator-side patch / fork pin @status:impl/done |
| @fact:ROW-MECH-GIT-SOURCE `[requires.packages]` git-source (§2.4.1) @status:impl/done | Consumer @status:impl/done | Long @status:impl/done | `source_kind = "git"` @status:impl/done | Consumer declares package not in any registry @status:impl/done |

@fact:STUB-VS-MIRROR-AXIS The key distinction between **stub** and **mirror** is *who controls the indirection*. Mirror is consumer-side (the operator's `vibe.toml` says "if vibespecs is slow, try this URL for the same content"). Stub is org-side (the registry owner's stub repo says "for this specific package, the content lives over there"). Same wire-level effect (redirected fetch), inverse control axis. @status:impl/done

@fact:REDIRECT-CLI **Publish helper (`vibe registry redirect`).** A new CLI command: @status:impl/done

```
vibe registry redirect <pkgref> --to <target-url>
                                [--ref-policy pass-through-tag|pinned]
                                [--pinned-ref <ref>]
                                [--auth <none|token-env|credential-helper|ssh>]
                                [--token-env <NAME>]
                                [--description "..."]
                                [--registry <name>]              # default = primary
```

@fact:REDIRECT-CLI-SEMANTICS Creates `<org>/<kind>-<name>` stub repo via the registry's `RepoCreator` (PROP-002 §2.10), commits a `vibe-redirect.toml`, pushes. Does not tag — operator runs `vibe registry redirect-sync <pkgref>` separately when ready to publish a version. Symmetric to `vibe registry publish` (which is the non-redirect path); the two commands are mutually exclusive on the same `<pkgref>` slot. @status:impl/done

@fact:REDIRECT-SLICE-SCOPE **Out of scope for this slice.** Redirect chains (stub → stub → real) — explicitly rejected at hop = 2. Signed redirect markers (cryptographic attestation that `target_url` is approved by org owner — `[redirect].signature = "..."` with org's pubkey). Auto-deprecation marker (a stub repo could carry `[redirect.deprecated] new_pkgref = "..."` to forward consumers to a renamed package — separate feature, separate PROP). `[[mirror]]` against a stub repo (mirror infrastructure pre-redirect) — undefined behaviour for v0; the resolver follows redirect first, mirror semantics apply to the target URL. @status:spec/done

### 2.5 Per-package layout: flat, tag-based {#layout}

@fact:layout-req `req r1` @status:impl/done

@fact:FLAT-LAYOUT **Decision.** A package repository contains the package content flat at the repository root: @status:impl/done

```
<org>/<kind>-<name>.git
├── vibe-package.toml
├── README.md
├── boot/<NN>-<kind>-<name>.md    # optional
├── spec/…                        # mirrored into consumer project
└── …

tags: v0.1.0, v0.2.0, v1.3.0-rc.1
```

- @fact:LAYOUT-TAG-VERSION Version = git tag with `v<semver>` prefix. Tag is immutable by convention. @status:impl/done
- @fact:LAYOUT-NO-SUBDIRS No versioned subdirectories inside the repo. A tagged checkout **is** the package content. @status:impl/done
- @fact:LAYOUT-TAG-INTEGRITY Integrity of the tag is verified per §2.1 on every install; a force-pushed tag rewrite is caught as `IntegrityError`. @status:impl/done

@fact:layout-rationale **Rationale for flat, not versioned subdirs:** tag-based is the idiomatic "git-as-package-source" shape (Go modules, Swift PM, many others). Consumers can browse a tag on GitVerse / GitHub and see exactly the package content, not navigate to a subdirectory. Authoring is natural — `main` branch is dev, tag `v0.x` is release. @status:spec/done

### 2.6 Cache layout: organized by canonical registry URL {#cache}

@fact:cache-req `req r1` @status:impl/done

@fact:CACHE-CANONICAL-ROOT **Decision.** The per-user registry cache is rooted at the **canonical registry URL**, not the mirror URL — a transparent mirror does not invalidate the cache: @status:impl/done

```
~/.vibe/registries/
└── <canonical-url-hash>/
    ├── meta.toml                 # { canonical_url, last_mirror_used?, last_synced_at }
    └── packages/
        └── <kind>-<name>/
            ├── clone/            # per-package git working tree
            └── meta.toml         # { source_url_last_used, last_synced_at, last_known_tags[] }
```

- @fact:CACHE-HASH-KEY `<canonical-url-hash>` = lowercase hex of the first 16 bytes of `sha256(normalize(canonical_registry_url))`. Normalization per PROP-001 §2.4 (lowercase, trailing `.git` and `/` stripped). @status:impl/done
- @fact:CACHE-OUTER-META Outer `meta.toml` carries the full hash, the canonical URL, and — for diagnostic purposes — the URL of the last mirror that actually answered. @status:impl/done
- @fact:CACHE-INNER-META Inner `meta.toml` (per package clone) carries the exact `source_url` that fetched this package last, its freshness timestamp, and the last set of tags observed by `ls-remote`. @status:impl/done
- @fact:CACHE-LAZY-FETCH Fetching is lazy per pkgref: a project that installs only `flow:wal` from an org of 50 packages clones exactly one of them. @status:impl/done

@fact:CACHE-TTL Freshness TTL per package repo is 1 hour (PROP-001 §2.5 carries over). `VIBE_REGISTRY_CACHE` env override applies. @status:impl/done

### 2.7 Lockfile schema v2 {#lockfile}

@fact:lockfile-req `req r1` @status:impl/done

@fact:LOCKFILE-V2 **Decision.** `vibe.lock` gains `schema_version = 2` and the following record shape per package: @status:impl/done

```toml
[meta]
generated_by      = "vibe 0.2.0"
generated_at      = "<RFC-3339 UTC>"
schema_version    = 2
solver            = "resolvo-<ver>"
root_dependencies = ["flow:wal", "stack:rust-cli"]

[[package]]
kind            = "flow"
name            = "wal"
version         = "0.3.0"
registry        = "vibespecs"                               # name from [[registry]]; "__override__" for override-resolved
source_url      = "git@gitverse.ru:vibespecs/flow-wal.git"  # canonical URL of the registry entry, NOT the mirror URL
source_ref      = "v0.3.0"
resolved_commit = "abc123…def"
content_hash    = "sha256:…"                                # identity per §2.1
boot_snippet    = "10-flow-wal.md"
files_written   = [ … ]
dependencies    = []                                        # resolved transitive deps, kind:name@=version
overridden      = false
```

- @fact:LF-V1-MIGRATION `schema_version = 1` (monorepo-era) is accepted read-only and auto-migrated to v2 on the next `vibe install` / `vibe update`, with a user-visible notice. @status:impl/done
- @fact:LF-ROOT-DEPENDENCIES `root_dependencies` is a **mirror of `vibe.toml` `[requires].packages`** — the lockfile keeps the user's declared roots inline so it remains a self-contained snapshot of the solve state (nothing inside `vibe.lock` requires reading `vibe.toml` to interpret). The source of truth for *what the user asked for* is the manifest's `[requires]` section; the lockfile carries a copy plus the resolved transitive closure. `vibe uninstall` of a root drops the entry from both files; `vibe uninstall` of a pure transitive is rejected with an explanation. @status:impl/done
- @fact:LF-REQUIRES-SEEDING A first-run migration path covers projects whose `vibe.toml` predates the `[requires]` section: when the manifest's `[requires]` is empty but the lockfile's `meta.root_dependencies` is non-empty, `vibe install` (no arguments) seeds `[requires].packages` from the lockfile snapshot before resolving. After that the manifest is authoritative. @status:impl/done
- @fact:LF-RESOLVED-DEPS `dependencies` field per package is **resolved** (exact version, not constraint) — the lockfile is the full resolved graph, not a constraint manifest. @status:impl/done

### 2.8 Depsolver: resolvo primary, DepSolver trait for fallback {#solver}

@fact:solver-req `req r1` @status:impl/done

@fact:RESOLVO-PRIMARY **Decision.** The primary depsolver is the [`resolvo`](https://crates.io/crates/resolvo) crate (pure Rust, BSD-3-Clause-or-Apache-2.0, used by Pixi and Rattler at conda scale). Chosen for: @status:impl/done

- @fact:RSV-FEATURE-COMPLETE **Feature completeness for complexity ≥ RPM (PROP-000 §18):** virtual packages, disjunctions, obsoletes-driven upgrades, boolean-style constraints, custom constraint operators. @status:impl/done
- @fact:RSV-RUST-NATIVE **Rust-native ergonomics.** Provider trait (`DependencyProvider` analog) maps cleanly onto our existing `Registry` / `MultiRegistryResolver` types; no FFI, no impedance mismatch, no C-toolchain dependency. @status:impl/done
- @fact:RSV-PRODUCTION-SCALE **Active upstream, production scale.** Pixi resolves over the conda ecosystem (hundreds of thousands of packages) in production; active development at prefix-dev. @status:impl/done

@fact:NOT-PUBGRUB **Not** `pubgrub` — the algorithm does not handle virtual packages or disjunctions, undershoot relative to PROP-000 §18. @status:impl/done

@fact:LIBSOLV-FALLBACK-SLOT **libsolv as explicit fallback.** A `DepSolver` trait in the new `vibe-resolver` crate mirrors the PROP-001 §2.2 `GitBackend` pattern: primary impl is `ResolvoSolver`; a future `LibsolvSolver` (FFI to C libsolv, BSD-3-Clause) drops in as a feature-gated alternative if resolvo ever hits a ceiling we can't raise. Swap cost: one impl block, one factory line. PROP-000 §15 (dep-weight not an argument) removes the size-based objection; PROP-000 §18 explicitly contemplates the switch if complexity demands. @status:impl/done

@fact:SOLVER-IDENTITY-FIELD A `lockfile.meta.solver = "resolvo-<ver>"` field would record the solver identity so a lockfile produced by a different engine is distinguishable, and one produced by an older resolvo can be re-verified by the same version when integrity investigation matters. **Specified, not shipped:** the live `[meta]` block has no `solver` key, and [PROP-017 §8](../vibe-resolver/PROP-017-resolvo-resolver.md) records the gap — adding it needs a lockfile schema bump. @status:spec/done

### 2.9 Capability-based deps: `[provides]` / `[requires]` / `[[requires_any]]` / `[obsoletes]` / `[conflicts]` {#capability}

@fact:capability-req `req r1` @status:impl/done

@fact:CAPABILITY-VOCABULARY **Decision.** The package manifest gains the full capability-based dependency vocabulary, pinned in [`VIBEVM-SPEC.md` §7.3](../../../VIBEVM-SPEC.md). Summary: @status:impl/done

- @fact:CAP-PROVIDES `[provides].capabilities = ["<namespace>:<name>[@<semver>]", …]` — abstract capabilities the package advertises. @status:impl/done
- @fact:CAP-REQUIRES-PACKAGES `[requires].packages = ["kind:name@<constraint>", …]` — concrete pkgref requirements. @status:impl/done
- @fact:CAP-REQUIRES-CAPABILITIES `[requires].capabilities = ["<namespace>:<name>[@<constraint>]", …]` — satisfied by any package that provides that capability. @status:impl/done
- @fact:CAP-REQUIRES-ANY `[[requires_any]] one_of = [ pkgrefs… ]` — disjunction; exactly one must be satisfied. Repeatable table for multiple independent disjunctions. @status:impl/done
- @fact:CAP-OBSOLETES `[obsoletes].packages = [ pkgrefs… ]` — the package supersedes these; the solver flags them for removal on upgrade. @status:impl/done
- @fact:CAP-CONFLICTS `[conflicts].packages = [ pkgrefs… ]` — mutually exclusive installs. @status:impl/done

@fact:LEGACY-COMPACT-MIGRATION **Legacy compact form** (`[dependencies] required = [...] conflicts = [...]`) remains parse-compatible — auto-migrated at read time to the new fields with a one-time deprecation warning. Three existing `vibe-package.toml`-s in the fixtures / live registry all declare empty `required` and `conflicts`, so no live data migration is forced. @status:impl/done

@fact:SOLVER-SEMANTIC Semantic: the solver computes a satisfying assignment over the declared constraints. Conflict-detection renders through resolvo's native conflict-explanation surface — produced as a human-readable chain of incompatibilities, not a stack trace. @status:impl/done

### 2.10 Publish utility: `vibe registry publish <path>` {#publish}

@fact:publish-req `req r1` @status:impl/done

@fact:PUBLISH-UTILITY **Decision.** Ship a maintainer utility in v1. Scope: mechanical-only publish — **create repo, push contents, tag version**. Semantic review (LLM-backed safety analysis per `VIBEVM-SPEC.md` §8.5) remains v2+. @status:impl/done

@fact:publish-architecture-lead Architecture: @status:impl/done

- @fact:PUB-CRATE New crate `vibe-publish` in the workspace. @status:impl/done
- @fact:PUB-REPO-CREATOR Core trait `RepoCreator` (host-specific operations) plus a Publisher orchestrator that drives the manifest read → repo presence/create → push → tag pipeline. Each supported host implements `RepoCreator` once; the rest is host-agnostic. The trait carries: @status:impl/done
  - @fact:PUB-HOST-NAME `host_name(&self) -> &str` — for error messages. @status:impl/done
  - @fact:PUB-REPO-EXISTS `repo_exists(org, name) -> Result<bool>` — distinguishes missing-token / missing-org / forbidden errors from a clean negative. @status:impl/done
  - @fact:PUB-CREATE-REPO `create_repo(org, name, opts) -> Result<RepoInfo>` — creates the repo in the org, returns metadata (HTML URL + clone URL). @status:impl/done
  - @fact:PUB-PUSH-URL `push_url(org, name) -> String` — produces the URL `git push` should target. SSH-auth hosts return the bare SSH URL (e.g. `git@gitverse.ru:vibespecs/flow-wal.git`); HTTPS-token-auth hosts return the URL with credentials embedded for the duration of the push (e.g. `https://x-access-token:<TOKEN>@github.com/vibespecs/flow-wal.git`). Modern git (≥ 2.31) redacts URL passwords in its own log output to `***`. @status:impl/done

@fact:pub-impls-lead **Concrete impls.** @status:impl/done

- @fact:PUB-GITVERSE-CREATOR **`GitVerseCreator`** (legacy, retained): hits `https://api.gitverse.ru` with `Authorization: Bearer <T>` and `Accept: application/vnd.gitverse.object+json;version=1`. **Operationally degraded** — `POST /orgs/{org}/repos` is documented as Gitea-canonical but is not exposed by the live host (verified 2026-04-26 by curl-probing); only `GET /repos/{owner}/{repo}` works for presence checks. Repo creation requires manual web-UI pre-create. The code path remains in tree so Gitea-shape forks of GitVerse, or any future GitVerse release that exposes the org-scoped POST, work transparently with no change. @status:impl/done
- @fact:PUB-GITHUB-CREATOR **`GitHubCreator`** (primary as of 2026-04-29): hits `https://api.github.com` with `Authorization: Bearer <T>`, `Accept: application/vnd.github+json`, `X-GitHub-Api-Version: 2022-11-28`. Both endpoints work natively — `POST /orgs/{org}/repos` returns 201 with the full repo metadata, no manual pre-create needed. SSH push is preferred when the operator has a configured GitHub SSH key; otherwise the publisher embeds the token into the HTTPS clone URL via `x-access-token:<TOKEN>@github.com/...` for that one push. The token never leaves the running process: not echoed to stdout/stderr, not logged at any level, redacted in `Token::Display`/`Debug`, and modern git redacts URL passwords in its own diagnostics. @status:impl/done
- @fact:PUB-ADAPTERS-ADDITIVE Adapters for Gitea / Forgejo / GitLab are additive — one new `impl RepoCreator` per host, no consumer-side changes. @status:impl/done

@fact:PUB-ADAPTER-SELECTION **Adapter selection.** The CLI picks an adapter from the registry URL's host segment. `github.com` (or any subdomain) → `GitHubCreator`; `gitverse.ru` → `GitVerseCreator`; unknown hosts surface a clean error pointing at PROP-002 §2.10 rather than guessing a Gitea-compatible shape that may not match the host's actual API. @status:impl/done

@fact:PUB-TOKEN-LOADING **Token loading.** The publish token loader (`crate::token::load_token(host)` → `load_token_for_host`) iterates these sources in order, returning the first non-empty value: @status:impl/done

1. @fact:TOK-HOST-ENV-VAR `VIBEVM_PUBLISH_TOKEN_<HOST>` environment variable — host-specific, and the **highest-precedence source**. The suffix is the uppercased first label of the host (`VIBEVM_PUBLISH_TOKEN_GITHUB` for `github.com`, `VIBEVM_PUBLISH_TOKEN_GITVERSE` for `gitverse.ru`); non-alphanumerics fold to `_` so the name stays a valid POSIX identifier. Lets CI hold tokens for several hosts in the same environment without one host-agnostic variable clobbering them all. @status:impl/done
2. @fact:TOK-ENV-VAR `VIBEVM_PUBLISH_TOKEN` environment variable — legacy host-agnostic form, kept so setups that already export it keep working without a rename. **Outranked by the host-specific variable above**; the host-specific form is preferred in new setups. @status:impl/done
3. @fact:TOK-PER-HOST-FILE `<settings-dir>/<host-prefix>.publish.token` — per-host file. The prefix is the first label of the host (`github` for `github.com`, `gitverse` for `gitverse.ru`, `gitlab` for `gitlab.com`). @status:impl/done
4. @fact:TOK-LEGACY-FALLBACK `<settings-dir>/git.publish.token` — legacy host-agnostic fallback. @status:impl/done

@fact:TOK-SETTINGS-DIR Both file legs hang off the one settings dir — `~/.vibe`, or `$VIBE_SETTINGS` when it is set — so a single variable relocates every on-disk credential read together, and an isolated run cannot reach the operator's real token file. The pre-consolidation config dir supplied two further legs until 2026-07-26; those reads were removed precisely because `$VIBE_SETTINGS` deliberately did not relocate that directory. @status:impl/done

@fact:tok-files-rationale The per-host file lets the operator hold tokens for several hosts simultaneously without juggling env vars. The legacy fallback covers the GitVerse-only era and keeps existing setups working. @status:spec/done

@fact:TOKEN-SECRECY-INVARIANT **Token secrecy invariant.** The token is a surface secret. It is **never** displayed in CLI output, log lines, error messages, JSON event payloads, the lockfile, or any committed file. The only sanctioned paths through which the value crosses a process boundary are: (a) the GitHub / GitVerse `Authorization: Bearer …` HTTP header, sent over TLS to the hosting API; (b) the `x-access-token:<TOKEN>@…` embed in the URL passed to a single `git remote add` / `git push` invocation; (c) the in-memory `Token` struct, which redacts on `Display` and `Debug`. The CLI prints the *source* of the token (explicit / env-var / file path) but never the value. Implementations must verify token redaction in unit tests (cf. `vibe_publish::token::tests::debug_redacts_value`). @status:impl/done

@fact:PUB-ERROR-SURFACE **Error surface** (tuned for non-admin contributors — PROP-000 §18 acknowledges this will hit routinely): @status:impl/done

- @fact:ERR-401-403 `401` / `403` from the API → `Publish refused: token lacks 'repo:create' permission in organization <org>. Contact an org owner or use a token with broader scope.` @status:impl/done
- @fact:ERR-PUSH-DENIED `git push` denied → `Publish refused: no push access to <repo>. Ask a maintainer of <repo> to grant you push access.` @status:impl/done
- @fact:ERR-TAG-EXISTS Tag already exists → `Publish refused: <repo> already has tag <tag>. Pick a new version — force-push is not automated.` @status:impl/done
- @fact:ERR-ORG-NETWORK Org does not exist / network unreachable → differentiated from auth errors so operators can tell a typo from a permissions issue. @status:impl/done
- @fact:ERR-UNSUPPORTED-HOST Unsupported host → `Publish refused: no RepoCreator adapter for host '<host>'. Add one in vibe-publish per PROP-002 §2.10.` @status:impl/done

@fact:PUBLISH-NEVER-RULES Never force-push. Never overwrite an existing tag. Never create a repo in a different org than the configured one unless `--org <other>` is passed explicitly. Never escalate scope: a publish run targets exactly the org named in the project's `[[registry]]` URL — adapters MUST refuse to create or modify anything outside that org. @status:impl/done

### 2.11 JTD + codegen for wire contracts {#jtd}

@fact:jtd-req `req r1` @status:impl/done

@fact:JTD-WIRE-CONTRACTS **Decision.** Per [PROP-000 §16](../../common/PROP-000.md#jtd), wire-format contracts — here, the GitVerse API request/response shapes, the `vibe --json` CLI output event schema, and future LLM provider wire shapes — are defined in JTD and codegen'd into Rust types via `jtd-codegen`. @status:impl/done

@fact:jtd-layout-lead Layout: @status:impl/done

- @fact:JTD-VENDORED-BINARY `tools/jtd-codegen/` — vendored `jtd-codegen` binary (gitignored; version pinned via README). @status:impl/done
- @fact:JTD-SCHEMAS-DIR `schemas/` — `.jtd.json` files at repo root, committed. One file per contract. @status:impl/done
- @fact:JTD-WIRE-CRATE `crates/vibe-wire/` — new crate housing `schemas/` → Rust codegen output in `src/generated/`, re-exports curated for downstream crates (`vibe-publish`, future `vibe-llm`). @status:impl/done
- @fact:JTD-XTASK-CODEGEN `cargo xtask codegen` — regenerates every schema. CI runs it and fails on diff. @status:impl/done

@fact:JTD-NOT-FOR-CONFIGS Manifests (`vibe.toml`, `vibe.lock`, `vibe-package.toml`) stay TOML and serde-driven — JTD is for wire, not for human configs. @status:impl/done

### 2.12 Performance and resolver I/O strategy {#perf}

@fact:perf-req `req r1` @status:impl/done

@fact:PROVIDER-ADAPTER **Decision.** The resolver is driven by a `DependencyProvider` adapter that sits on top of `MultiRegistryResolver` and exposes only what resolvo needs: @status:impl/done

- @fact:PERF-LIST-TAGS `list_versions(pkgref)` → backed by `ShellGit::list_tags(repo_url)` — a new method using `git ls-remote --tags <url>` to enumerate versions **without cloning**. @status:impl/done
- @fact:PERF-FETCH-FILE `get_dependencies(pkgref, version)` → backed by `ShellGit::fetch_file_at_ref(repo_url, tag, "vibe-package.toml")` — a new method using `git archive` to pull a single file from a tag **without a working tree**. @status:impl/done

@fact:PERF-IN-MEMORY-CACHE In-memory caching across one `vibe install` invocation: `(registry, kind, name) → Vec<Version>` and `(registry, kind, name, version) → Deps`. Same pkgref touched twice within a resolve pass hits memory, not network. @status:impl/done

@fact:PERF-RAYON-FANOUT Parallel I/O via `rayon` scoped thread pool: N parallel `ls-remote` subprocess invocations when resolving a multi-dep top-level install. No tokio introduction — our runtime is synchronous, `git` subprocesses are blocking, and a thread pool for blocking-I/O fan-out is exactly rayon's sweet spot. @status:spec/done

@fact:PERF-CLONE-ONCE A full clone happens only once the solver has committed to installing a specific version. The pathological case (resolver evaluates 50 versions of a package, then picks one) clones once, not 50 times. @status:impl/done

@fact:PERF-RESOLVE-CACHE-FUTURE Future (Phase B): resolved-graph cache keyed by `sha256((vibe.toml-registries) || all-requires || all-overrides)`. A second `vibe install` in the same project without any input change hits the cache, skipping resolve entirely. Listed here so the cache-key grammar is chosen once, not retrofitted. @status:spec/done

---

## 3. Rejected alternatives {#rejected}

### 3.1 libsolv as primary solver

@fact:REJ-LIBSOLV-PRIMARY Rejected for primary role — accepted as explicit fallback (§2.8). libsolv is battle-tested at RPM scale (DNF, zypper, SUSE) and its feature set is a superset of what we need. But its API is C-style Pool / Solvable / Transaction semantics, and modelling vibevm's pkgref / capability types in Solvable form is an ongoing impedance mismatch that would live in the codebase forever. `resolvo` is a generic Rust-native solver with the same feature coverage for our problem, so the Rust-native path wins on architectural consistency. The libsolv slot exists so we can switch if resolvo ever proves inadequate. @status:spec/done

### 3.2 pubgrub as primary solver

@fact:REJ-PUBGRUB Rejected. PubGrub is an excellent algorithm (first-class error messages, used by `uv` and others), but it does not handle virtual packages or disjunctions — and PROP-000 §18 expects both from day one. PubGrub may still be used in the future for explanatory rendering of conflicts in CLI output if its incompatibility-chain format proves superior; not as the authoritative solver. @status:spec/done

### 3.3 Registry union (merge versions across all registries)

@fact:REJ-REGISTRY-UNION Rejected. In a multi-registry setup, unioning versions from all `[[registry]]` entries into one candidate list requires trusting all of them equally — a lower-trust registry could influence resolution when a higher-trust one already had a valid answer. Priority-ordered resolution (§2.2) gives operators clear control: the registry at array index 0 is always preferred, subsequent ones are fallbacks. @status:spec/done

### 3.4 Per-registry identity (pkgref includes registry)

@fact:REJ-PER-REGISTRY-IDENTITY Rejected. Treating `vibespecs/flow:wal` and `corporate/flow:wal` as different identities makes mirror-switching impossible and forces every consumer's `vibe.toml` to pin a specific registry for every dep. Identity is package-level (kind, name, version, content_hash); the registry is a runtime resolution detail. @status:spec/done

### 3.5 `github:` / `gitverse:` / `gitlab:` URL shorthands

@fact:REJ-URL-SHORTHANDS Rejected. This is **the** Nix failure mode (§1). Every host-specific shorthand in the URL grammar is an invitation to tie the ecosystem to that host. vibevm accepts any URL that `git` accepts — full stop. Operators type full URLs; the CLI does not do host-specific magic. @status:spec/done

### 3.6 Central `vibevm/index` registry of registries

@fact:REJ-CENTRAL-INDEX Rejected. A global index would reintroduce the Nix centralization problem one level up. Registries are peer-level; operators configure their own `[[registry]]` array per project. Discovery of "what registries exist" is out of CLI scope (it's a search-engine / documentation problem, not a package-manager problem). @status:spec/done

### 3.7 Optional / recommended / supplemental deps in v1

@fact:REJ-WEAK-DEPS-V1 Rejected for v1 (not rejected forever). `required` + `conflicts` + capabilities + disjunctions + obsoletes cover every concrete use case we've identified. `recommends` / `suggests` / `supplements` (RPM's weak-deps) can be added later as extensions to `[requires]` — the fields slot in without breaking the schema. @status:spec/done

### 3.8 HTTPS-only hosted-registry API

@fact:REJ-HTTPS-ONLY Rejected for v1. Git-over-SSH is what works today on contributor machines with existing GitVerse keys; HTTPS with token auth is a Phase B / M2 add. The `GitBackend` abstraction makes this additive, not architectural. @status:spec/done

---

## 4. Out of scope for Phase A {#out-of-scope}

- @fact:OOS-REGISTRY-UX Polished multi-registry UX (`vibe registry add / list / remove / set-mirror` commands) — Phase B. @status:spec/done
- @fact:OOS-MULTI-LIVE Live multi-registry exercised end-to-end — Phase B. @status:spec/done
- @fact:OOS-MIRROR-CHAIN Real mirror fallback chain (code paths exist, tested with fixtures; live mirror comes in Phase B). @status:spec/done
- @fact:OOS-VENDOR `vibe vendor` — Phase B. @status:spec/done
- @fact:OOS-PUBLISH-ADAPTERS Publish adapters beyond GitVerse — added per adopter demand. @status:spec/done
- @fact:OOS-SIGNED Signed / attested packages (sigstore-style) — M2 design, noted as architectural allowance here. @status:spec/done
- @fact:OOS-LLM-REVIEW LLM-backed publish review — v2 (already spec'd in `VIBEVM-SPEC.md` §8.5). @status:spec/done
- @fact:OOS-OFFLINE `--offline` install mode — M2 polish. @status:spec/done

---

## 5. Acceptance (Phase A) {#acceptance}

@fact:acceptance-lead Phase A is code-complete when every item below is green: @status:impl/done

- @fact:ACC-SPEC-DOCS [ ] `VIBEVM-SPEC.md` §7.3 / §7.4 / §7.5 / §8.1 / §8.2 / §8.3 / §8.4 / §8.6 reflect the new design. PROP-001 carries its super-session marker. TASKS.md / ROADMAP / WAL updated. @status:impl/done
- @fact:ACC-JTD [ ] `jtd-codegen` vendored under `tools/`; `schemas/` seeded with at least the GitVerse publish-API contract and the `vibe --json` event shape; `cargo xtask codegen` regenerates; CI enforces zero drift. @status:impl/done
- @fact:ACC-CAPABILITY-PARSE [ ] `vibe-core` parses `[provides]` / `[requires]` / `[[requires_any]]` / `[obsoletes]` / `[conflicts]` into typed values; legacy compact form auto-migrates with a deprecation warning. @status:impl/done
- @fact:ACC-REGISTRY-PARSE [ ] `vibe.toml` parser accepts `[[registry]]` array, `[[mirror]]`, `[[override]]`; singleton legacy `[registry]` auto-migrates. @status:impl/done
- @fact:ACC-LOCKFILE-V2 [ ] `vibe.lock` schema v2 written by fresh installs; v1 lockfiles read and migrated on next write. @status:impl/done
- @fact:ACC-RESOLVER-CRATE [ ] `vibe-resolver` crate exists with `DepSolver` trait and `ResolvoSolver` impl; unit-tested against constructed dep graphs including a virtual-package and a disjunction case. @status:impl/done
- @fact:ACC-GIT-PACKAGE-REGISTRY [ ] `GitPackageRegistry` replaces `GitRegistry`; `MultiRegistryResolver` coordinates the array; `ShellGit::list_tags` and `ShellGit::fetch_file_at_ref` implemented. @status:impl/done
- @fact:ACC-CONTENT-HASH [ ] Content-hash integrity verified on every fetch; cross-source mismatch fails hard with actionable message. @status:impl/done
- @fact:ACC-TRANSITIVE-INSTALL [ ] `vibe install` runs transitive resolution; plan rendering shows the full subgraph with `(dep of flow:foo)` provenance tags; `--dry-run` surfaces the same. @status:impl/done
- @fact:ACC-PUBLISH-CRATE [ ] `vibe-publish` crate with `RepoCreator` trait and `GitVerseCreator` impl; `vibe registry publish <path>` subcommand; non-admin error paths render per §2.10. @status:impl/done
- @fact:ACC-FIXTURES [ ] Local fixtures relocated from `packages/` to `fixtures/registry/` in per-package layout; `cli_e2e.rs` updated. @status:impl/done
- @fact:ACC-DEMO-PACKAGES [ ] Three demo packages live in `vibespecs/flow-wal` / `vibespecs/flow-sync-from-code` / `vibespecs/flow-atomic-commits` via `vibe registry publish`, each tagged `v0.1.0`. @status:impl/done
- @fact:ACC-MANUAL-SMOKE [ ] `manual-tests/M1.5-gate-v2-per-package-smoke.md` written and passes against the live per-package registry. @status:impl/done
- @fact:ACC-WORKSPACE-GREEN [ ] `cargo test --workspace` green; `cargo clippy --workspace --all-targets -- -D warnings` clean. @status:impl/done

---

## 6. Phase B preview (M1.6) {#phase-b}

@fact:phase-b-preview-lead Pinned here so Phase A does not accidentally foreclose any of these options: @status:impl/done

- @fact:PBP-MULTI-LIVE Real multi-registry: a second live `[[registry]]` exercised end-to-end; priority ordering verified in smoke-test. @status:impl/done
- @fact:PBP-MIRROR-CHAIN Mirror fallback chain: the second-live-mirror URL and the integrity hard-fail both ship (the fallback loop in `git_package_registry/fetch.rs`, `fetch_with_expected_hash`, the M1.6 mirror-vendor smoke); the `--trust-mirror` escape hatch does not. @status:impl/done
- @fact:PBP-VENDOR `vibe vendor [--out <dir>]`: generates a local mirror directory, usable as `file://` `[[mirror]]`. @status:impl/done
- @fact:PBP-CLI-SURFACE CLI management surface: `vibe registry add / list / remove / set-mirror / status`. @status:impl/done
- @fact:PBP-PUBLISH-ADAPTERS Publish adapters: GitHub, Gitea, Forgejo on adopter demand — one new `RepoCreator` impl per host. @status:impl/done
- @fact:PBP-RESOLVE-CACHE Resolved-graph cache: incremental resolve skipped when inputs unchanged. @status:spec/done
- @fact:PBP-ATTESTATION Supply-chain attestation: sigstore-style signing of tags; consumer verification on install. Architectural slot only in Phase B. @status:spec/done

---

## 7. Open questions {#open}

@fact:open-lead None blocking Phase A. Parking lot: @status:spec/done

- @fact:OPEN-REGISTRY-META-REF **Registry-level metadata ref.** `[[registry]] ref = "main"` is reserved for a future capability index / trust policy branch at the org level. Design deferred until use case emerges (likely Phase B or M2). @status:spec/work
- @fact:OPEN-CROSS-REGISTRY-CACHE **Cross-registry content-hash cache.** Today each `(registry, kind, name, version)` gets its own cache entry. If two registries mirror the same content (same content_hash), the second fetch does redundant work. Optimization — Phase B. @status:spec/work
- @fact:OPEN-NAMING-TEMPLATES **Registry-level naming beyond `fqdn` / `kind-name` / `name` / `kind/name`.** Real-world adopters may want custom mappings (e.g. `pkg-<kind>-<name>`). If that arises, `naming` becomes a template string. Not speculative engineering today — react to first real request. @status:spec/work
- @fact:OPEN-VERIFY-SOLVER **Solver-level lockfile verification.** The lockfile records `solver = "resolvo-<ver>"`. Should `vibe install --verify-solver` re-run resolution and assert the graph matches the lockfile? Useful audit tool. Phase B. @status:spec/work
- @fact:OPEN-JTD-WINDOWS **JTD codegen ergonomics on Windows.** `jtd-codegen` is a Go binary; we vendor it project-local. Whether the experience is clean enough to not require PATH tinkering will be answered during the tooling commit. @status:spec/work
