# Authenticating against private registries

vibevm supports four authentication regimes per `[[registry]]` block. The default (`auth = "none"`) covers public registries — every git host vibevm has shipped against by default, including the canonical `vibespecs` orgs on GitHub and GitVerse. The other three (`token-env`, `credential-helper`, `ssh`) cover private registries with progressively-different credential sources.

This document is the operator's reference. The architectural decisions live in [`PROP-002 §2.2.1`](../spec/modules/vibe-registry/PROP-002-decentralized-registry.md#registry-auth) (the auth axis itself) and [`§2.3.1`](../spec/modules/vibe-registry/PROP-002-decentralized-registry.md#failure-discriminator) (failure-mode classifier — `auth`-aware 401 / 403 handling).

## TL;DR

If your registry is public, you don't need to do anything: the default `auth = "none"` is correct. vibevm silences GUI credential prompts in non-interactive runs (CI, opencode harness, `--unattended`) so a 401 from a host like GitVerse — its policy on missing public repos — never triggers a popup; the resolver walks to the next registry instead.

If your registry is private, pick one of three regimes:

| Regime | Setup | Best fit |
| --- | --- | --- |
| `token-env` | Set `VIBEVM_REGISTRY_TOKEN_<HOST>` (or `--token-env <NAME>`); URL stays HTTPS | CI, scripts, agent harnesses, anything non-interactive |
| `credential-helper` | Have GCM / osxkeychain / libsecret already wired in `~/.gitconfig` | Local dev with corporate SSO |
| `ssh` | Have an ssh key on the host's deploy keys / personal keys | Local dev with ssh-agent |

## The four regimes

### `auth = "none"` (default)

Public read-only. **No credentials are ever sent.** If the registry returns 401 or 403, vibevm classifies it as "no public answer here" and walks to the next registry in the priority list — the same fall-through behaviour as a 404. This is what makes the default `vibespecs` + `vibespecs-gitverse` pair survive GitVerse's policy of returning 401 for missing public repos; vibevm does not get confused.

In non-TTY runs (CI, scripts, `--unattended`), vibevm also silences git's credential machinery — terminal prompts, `core.askPass`, `credential.helper`, GCM popups — so a 401 cannot become a blocking GUI window. On an interactive TTY without `--unattended`, the system credential helpers are left untouched (an operator running `vibe install` at a real terminal can still type a password if they really want to, though on `auth = "none"` that input would not be used by the resolver).

```toml
[[registry]]
name = "vibespecs"
url  = "https://github.com/vibespecs"
# auth = "none"   # default; skip-on-serialise
```

### `auth = "token-env"`

Read a personal access token from an environment variable and inject it into the per-package URL on every git invocation. The token is read once at registry-open time, held in memory only, never written to the lockfile, never logged in vibevm's own output, and (after a clone) immediately scrubbed out of the local `.git/config` so it does not persist on disk.

```toml
[[registry]]
name      = "internal"
url       = "https://gitlab.company.com/vibespecs"
auth      = "token-env"
token_env = "VIBEVM_REGISTRY_TOKEN_INTERNAL"
```

`token_env` is optional. When omitted, vibevm derives the env-var name from the registry's host — lowercase host with `.` and `-` mapped to `_`, prefixed by `VIBEVM_REGISTRY_TOKEN_` and uppercased. So `https://gitlab.company.com/vibespecs` defaults to `VIBEVM_REGISTRY_TOKEN_GITLAB_COMPANY_COM`. Operators who want a stable env-var across host migrations set `token_env` explicitly; everyone else gets the host-derived default.

#### Bash / zsh

```bash
export VIBEVM_REGISTRY_TOKEN_GITLAB_COMPANY_COM=ghp_yourTokenHere
vibe install flow:internal-helper --assume-yes
```

#### PowerShell

```powershell
$env:VIBEVM_REGISTRY_TOKEN_GITLAB_COMPANY_COM = 'ghp_yourTokenHere'
vibe install flow:internal-helper --assume-yes
```

#### CI (GitHub Actions example)

```yaml
- run: vibe install --unattended
  env:
    VIBEVM_REGISTRY_TOKEN_GITLAB_COMPANY_COM: ${{ secrets.INTERNAL_REGISTRY_TOKEN }}
```

#### Token shape

Whatever your host accepts as a Personal Access Token. vibevm injects it as `https://x-access-token:<TOKEN>@<host>/...`, the same shape `vibe-publish` uses on the push side and the same shape GitHub / GitLab / Gitea expect for token-based HTTPS auth.

Required scopes:
- **Read access** to the registry org and to each per-package repo.
- For most hosts that's `read:packages` / `repo:read` / similar — nothing else.

vibevm does not need write scopes for installation. (Publishing is a separate tool path with its own `VIBEVM_PUBLISH_TOKEN_<HOST>` env-var; see [`PROP-002 §2.10`](../spec/modules/vibe-registry/PROP-002-decentralized-registry.md#publish).)

#### Token discipline

Treat the env-var value as a surface secret:

- Do not echo it into shell history (use `read -s`, paste-from-secrets-manager, or set it in your shell rc).
- Do not commit it. `vibe.toml` carries the env-var **name**, never the value.
- Do not paste it into chat / issue trackers / vibe.lock — none of vibevm's output channels carry the token. Modern git (≥ 2.31) auto-redacts passwords from its own stderr; vibevm relies on that as a second line of defence.
- Do not check `.git/config` after install expecting to find the token — vibevm scrubs it via `git remote set-url origin <plain-url>` immediately after the clone completes. The clone state on disk carries only the credential-free URL.

#### What happens on a missing or wrong token

If the env-var is unset or empty when `vibe install` walks the registry, vibevm raises `MissingToken` *before spawning git* with a message naming the env-var to set:

```
error: registry `internal` declares `auth = "token-env"` but env-var
       `VIBEVM_REGISTRY_TOKEN_INTERNAL` is empty or unset; set it to a
       personal access token with read access to the registry org
```

The resolver does **not** silently walk past the missing token to the next registry — silencing this would mask the operator's setup mistake. Set the env-var, re-run.

If the token is set but rejected by the host (wrong, expired, insufficient scopes), git returns 401 / 403, vibevm classifies the failure as `AuthFailed` against an authenticated registry, and halts with the host's error message. Same policy: walk-past would mask a real configuration problem.

### `auth = "credential-helper"`

Opt in to the system git credential machinery — `credential.helper = manager` (Git Credential Manager Core on Windows), `osxkeychain` on macOS, `libsecret` on Linux, or whatever else the operator has already wired in `~/.gitconfig`. On an interactive TTY without `--unattended`, GUI prompts are allowed; the helper does its thing.

```toml
[[registry]]
name = "corporate"
url  = "https://corp.example.com/vibespecs"
auth = "credential-helper"
```

In non-TTY / `--unattended` runs, vibevm still silences these helpers — the contract on `--unattended` is "no prompt, no popup, period." `credential-helper` registries in CI behave identically to `none` registries: a 401 becomes `AuthFailed`, the resolver halts, the operator gets an actionable error.

Use this when:
- You already have GCM / keychain / libsecret working for your daily git workflow.
- You don't want to manage a token in env-vars.
- You're working interactively, not in CI.

### `auth = "ssh"`

ssh-form URL. Authentication is delegated to your ssh-agent and keys. vibevm does not touch ssh config, does not ask for passphrases, does not interact with the keys at all — if a passphrase prompt appears, that's your ssh-agent's decision.

```toml
[[registry]]
name = "internal-ssh"
url  = "git@gitlab.company.com:vibespecs"
auth = "ssh"
```

The URL **must** be ssh-form. vibevm does not rewrite an HTTPS URL to ssh under `auth = "ssh"` — that would be too magical; an explicit URL is the contract.

Use this when:
- Your daily workflow is already ssh-key-based.
- You're on a personal machine with ssh-agent running.
- The host accepts ssh better than HTTPS-with-token (some self-hosted Gitea / Forgejo instances).

## Setting up via the CLI

`vibe registry add` takes `--auth` and `--token-env` flags so you don't have to hand-edit `vibe.toml`:

```bash
# Token-env, derived env-var name (`VIBEVM_REGISTRY_TOKEN_GITLAB_COMPANY_COM`).
vibe registry add internal "https://gitlab.company.com/vibespecs" --auth token-env

# Token-env, explicit env-var name.
vibe registry add internal "https://gitlab.company.com/vibespecs" \
                  --auth token-env --token-env CORP_REG_TOKEN

# SSH-based (URL must be ssh-form).
vibe registry add internal-ssh "git@gitlab.company.com:vibespecs" --auth ssh

# Credential-helper (interactive only; OK to fall back to none in CI).
vibe registry add corporate "https://corp.example.com/vibespecs" --auth credential-helper
```

`--token-env` paired with anything other than `--auth token-env` is rejected — that combination has no meaning.

## How vibevm walks the registry list under different `auth`

PROP-002 §2.3.1 lays out the failure-mode classifier; here is the operator-facing summary.

| Failure on this registry | `auth = "none"` | `auth = "token-env"` (token loaded) | `auth = "token-env"` (token absent) | `auth = "credential-helper"` | `auth = "ssh"` |
| --- | --- | --- | --- | --- | --- |
| 404 / repo not found | Walk to next | Walk to next | (precheck halts at `MissingToken`) | Walk to next | Walk to next |
| 401 / 403 | **Walk to next** (treated as "not public here") | Halt with `AuthFailed` | (precheck halts at `MissingToken`) | Halt with `AuthFailed` | Halt with `AuthFailed` |
| Network unreachable | Halt | Halt | (precheck halts at `MissingToken`) | Halt | Halt |
| Server error (500) | Halt | Halt | (precheck halts at `MissingToken`) | Halt | Halt |

The walk-on-public-401 rule is what unblocks the original opencode regression: GitVerse returning 401 for `vibespecs/rust-cli` (a missing public repo) no longer halts the install — vibevm walks to GitHub which returns a clean 404, and the resolver finishes with `UnknownPackage` cleanly.

## Token never on disk — verifying

After a successful install of a `auth = "token-env"` package, you can verify that no token bytes are present in the local clone:

```bash
# Clone bucket lives at:
#   ~/.vibe/registries/<canonical-url-hash>/packages/<kind>-<name>/clone/
# (path layout: PROP-002 §2.6)

# Inspect the recorded origin URL — must NOT contain `x-access-token`.
git -C ~/.vibe/registries/*/packages/flow-internal-helper/clone remote -v

# Search the local config for the token verbatim — must come up empty.
grep -r x-access-token ~/.vibe/registries/
```

If the second command finds anything, that's a vibevm bug — please file an issue.

## Troubleshooting

### "GUI popup keeps appearing in CI"

You're not in a non-TTY environment from vibevm's perspective. Either the harness allocates a fake TTY, or you're running with a real terminal attached. Force the silencing on:

```bash
export VIBE_UNATTENDED=1     # or `vibe --unattended ...`
# or, the lower-level explicit override:
export VIBEVM_GIT_SILENCE_HELPERS=1
```

`VIBE_UNATTENDED=1` also makes every `vibe install` / `vibe mcp install` / etc. skip its own apply-confirm prompts. `VIBEVM_GIT_SILENCE_HELPERS=1` is narrower — it only suppresses the git credential machinery.

### "MissingToken even though I set the env-var"

Check the env-var name. If your `vibe.toml` carries `token_env = "MY_TOKEN"` then vibevm consults `MY_TOKEN`, NOT `VIBEVM_REGISTRY_TOKEN_<HOST>`. The two are mutually exclusive.

```bash
# Confirm the name vibevm expects:
vibe registry list --json | jq '.registries[].token_env'
```

### "401 against my private registry, token IS set"

Most likely your token has insufficient scopes. Required: read access to the org + each per-package repo. For GitHub, `repo` (or fine-grained "Contents: Read") on the registry org. For GitLab, `read_repository` + `read_registry` on the group.

A second possibility: the host returns 403 (not 401) for scope-insufficient tokens. vibevm treats both as `AuthFailed`; the host's error message in the stderr will name what's wrong.

### "401 against my public registry, but it's only a missing repo"

That is the exact case `auth = "none"`'s walk-past-401 rule handles. Confirm the registry is `auth = "none"`:

```bash
vibe registry list --json | jq '.registries[] | {name, url, auth}'
```

If it shows `auth = "token-env"` or `"credential-helper"`, the resolver is correctly halting because you declared the registry as authenticated. Either fix the auth (set the token / wire the helper) or change `auth` to `"none"` if the registry really is public.

### "I want to override `auth = "none"` for one package without changing the registry"

Use `[[override]]` (PROP-002 §2.4) to point the package at a different source URL — the override flow has its own auth handling (delegated to whatever auth the source URL uses; typically the operator's ssh-agent for ssh URLs).

```toml
[[override]]
pkgref     = "flow:wal"
source_url = "git@gitlab.company.com:my-fork/wal"
ref        = "my-fix"
reason     = "while I wait for upstream PR #42"
```

## See also

- [`commands/registry-add.md`](commands/registry-add.md) — full reference for the `vibe registry add --auth --token-env` flags.
- [`commands/install.md`](commands/install.md) — what happens during `vibe install` against various registry shapes.
- [`PROP-002 §2.2.1`](../spec/modules/vibe-registry/PROP-002-decentralized-registry.md#registry-auth) — the architectural decision and the four-cell silencing matrix.
- [`PROP-002 §2.3.1`](../spec/modules/vibe-registry/PROP-002-decentralized-registry.md#failure-discriminator) — the failure-mode classifier and the walk-vs-halt rules.
- [`PROP-000 §20`](../spec/common/PROP-000.md#token-secrecy) — the token-discipline contract that applies to every secret across vibevm.
