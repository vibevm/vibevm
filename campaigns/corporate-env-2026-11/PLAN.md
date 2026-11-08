# Campaign: VibeVM in a restricted corporate network

**Status:** planned 2026-11-08; written against `main` at `0ec3c96de` plus the
uncommitted VIBE-NEEDS completion pass. Starts after VIBE-NEEDS closes.
Phase 0 (spikes) may run in parallel with VIBE-NEEDS where files are disjoint.
Cold start: read this file, then `LOG.md` in this directory.

## 1. Mandate (owner, 2026-11-08, verbatim)

> Следующий этап: работа в корпоративной среде
>
> У нас есть крайне ограниченная локальная сеть, где нет доступа до GitHub и GitVerse, и даже качать файлы с релизами неоткуда. Все что у тебя есть - корпоративный репозиторий у которого есть доступ по git ssh ссылке, и возможность все свои обновления для vibe и пакетов сложить куда-то в директорию на своем же жестком диске и управлять ей вручную. Качать бинарные исполняемые файлики (и архивы с ними) из интернета вручную через браузер, и класть в нужное место. Не потому что это единственный реально способ, но это единственный который одобряет корпоративная политика. Это страшно убогая ситуация, но именно в такой ситуации живет значительная часть старых компаний.
>
> 1. В локальном Git репозитории может не быть API и может не быть возможности создавать организации. И работа может быть только через git ssh протокол (https и http нет). Нужна возможность все пакеты vibevm хранить в одном-единственном репозитории по SSH ссылке, и публиковать туда же. Этот репозиторий должен настраиваться в registry.toml и его локальных оверрайдах (если у нас такая функциональность локальных для проекта оверрайдов реализована).
> 2. Также может не быть доступа к Git вообще. В этом случае наши пакеты придется хранить в локальной директории. А шарить ее будут например, с помощью NFS шары или CEPH или еще чего-то - нам про это знать не нужно, мы будем считать директорию источником правды в том виде, в котором она есть, и в этом кейсе мы не будем заниматься например, гарантиями синхронности при загрузке пакета и публикации - это важно, потому что синхронизация поверх NFS/CEPH была бы огромной и сейчас бесполезной работой. Просто считаем, что люди работают с этой директорией осторожно и решают конфликты с помощью административных методов. В этом случае нам даже неважно какой бэкенд шаринга - его может не быть вообще и это может быть обычная локальная директория.
> 3. Вероятно в registry.toml и локальных оверрайдах должен появиться какой-то параметр типа type, который имеет значения типа github, gitverse, localdir, monorepo. Если домен github.com или gitverse.ru, параметры необязательные. Но в принципе, у нас может быть локальная on-prmises инсталляция GitVerse на другом домене, так что даже отдельный тип gitverse имеет смысл. Названия типов case insensitive, чтобы люди не гадали как писать - GitVerse или gitverse.
> 4. В локальной сети может не быть доступа к GitHub/GitVerse для скачивания релизов vibevm для vibe self update. Я хочу, чтобы у vibe self появилась переключалка между источниками релизов, например "vibe self source". Типы: Local Sources, GitHub, GitVerse, Nexus. Про последний тип Nexus - следующий пункт.
> 5. В локальной сети может быть невозможно хранить exe-файлы нигде кроме локального диска. Тогда нам нужна директория, куда мы будем складывать релизы vibe вручную! При указании источника релизов Local Directory, пользователю нужно указать путь где будут лежать релизы vibe. Имя релизов в директории, очевидно, должно подчиняться какому-то неймингу для автоматического поиска. Дальше vibe self update должен работать нормально. Попробуй опубликовать несколько фейковых релизов в эту директорию и посмотреть что будет. В идеале хотелось бы чтобы была команда, которая показывала бы какие релизы вообще там доступны на сервере (и для github и для gitverse тоже чтобы она работала) и занималась обновлениями.
> 6. Та же проблема с релизами пакетов, которым нужны исполняемые файлы. В особенности для vibe install -g. В гите может быть запрещено хранить исполняемые файлы. Поэтому нужно иметь систему, которая сможет оверрайдить пути из удаленного Git на локальную директорию с релизами.
> 7. Кроме того, желательно иметь систему для vibe install -g которая позволяет изначально указывать в качестве источника файлов локальную директорию. Если этого еще не сделано потому что возможно - это уже сделано. Например, у нас может быть одна директория для обновлений, в которой поддиректории как-то соответствуют пакетам по конвенции.
> 8. Возможно пункты 5 и 6 должны основываться на какой-то общем фундаменте. Я не знаю, насколько это необходимо. Это все сложно, я пока не знаю как это должно работать, и призываю тебя хорошо обдумать этот вопрос.
> 9. Очень сложно разработчикам рассказывать как настроить vibevm, когда это много команд в разных местах. Хочется чтобы была возможность вычитать вот эти настройки конкретной инсталляции vibevm из одного конкретного файла. Настройки: оверрайды https/http/git (эта фича уже есть), используемые репозитории, источник обновлений vibe. Команда например, vibe self apply <file> или что-то такое (слово apply взялось как аллюзия на kubectl apply -f, эту команду многие знают). Более конкретный набор настроек для корпоративной инсталляции можешь уточнить, название команды для настройки из файла можешь тоже придумать свое если vibe self apply почему-то плохое имя. После применения файлов разработчику должно быть написано, куда класть релизы vibe и релизы пакетов.
> 10. Часто из-за криворукости сисадминов, пользователям на корпоративных машинах Windows невозможно поменять user-level PATH из GUI. Если vibe нет в локальном PATH на Windows, нужно чтобы наш vibe self apply мог автоматически применить PATH. В интерактивном режиме стоит об этом спрашивать, в неинтерактивном должен быть флаг чтобы сделать это по-умолчанию (типа --fix-path). Вероятно, это должно работать не только для Windows, но и для Mac и Linux. Можно протестировать сам механизм на Linux (раз ты как агент работаешь на Linux прямо сейчас), а на Windows расширить сразу как только механизм будет обкатан. Тем не менее, приоритетная платформа для этой фичи - именно Windows, потому что именно там пользователи не умеют менять PATH. Изменять PATH я рекомендую, вызывая либо какие-то растовские библиотеки (есть ли кросплатформенные?), либо через cmd.exe, но никогда не через jshell (это важно, этот способ обычно заблокирован на корпоративных инсталляциях).
>
> Owner follow-up, same day: «если по этой новой кампании у тебя не будет каких-то ко мне больших вопросов, сразу можешь начинать ее реализовывать».

## 2. Baseline — verified facts at writing time (code, not specs)

Verified by a read-only survey of the tree on 2026-11-08; file references are
to that tree.

- **Registry config** (`vibe-core/src/manifest/project.rs` `RegistrySection`): `name, url, ref, naming, auth, token_env, enabled, index_url`. **No `type` field.** Backend is chosen only by URL prefix: `file:` → `LocalRegistry` (`<root>/<group>/<name>/v<ver>/`), everything else → `GitPerPackageRegistry` (`<org_url>/<repo>.git`, naming `fqdn|kind-name|name|kind/name`).
- **Single-repo registry exists but is unreachable:** `GitMonorepoRegistry` (`vibe-registry/src/git_registry.rs`) clones one repo and serves the local layout; only tests construct it.
- **Config layers for registries:** project `vibe.toml` `[[registry]]` > machine `~/.vibe/registry.toml` (`GlobalRegistryConfig`, also `[[mirror]] [[override]] [index] [source_policy]`), merged by `merge_effective`. **No project-local (uncommitted) registry override file.** Missing machine file is seeded with GitHub/GitVerse `vibespecs` (skipped by `VIBE_NO_DEFAULT_REGISTRY`/`CI`). No CLI edits `~/.vibe/registry.toml`.
- **Locked-source law (just delivered):** packages in `vibe.lock` replay their locked source; `vibe update --sources-only` moves locked sources to the current registry configuration keeping versions. This is the migration path when a corporate registry replaces GitHub.
- **Publish:** host chosen by domain (`github.com` → org repo creation via API; `gitverse.ru` → stub; else unsupported). `--repo-url <git-url>` pushes to an existing repo without API (any URL incl. `file:///` bare repo). Tag `v<version>`; payload commit on `main`; refs pushed with `--force-with-lease` (versions mutable). Reads only the project manifest's registries. **No publish into a monorepo or a local directory.**
- **Self releases:** URLs hardcoded to GitHub (`vvm/bundle.rs`: `.../releases/download`, `latest/download/DISTRIBUTIONS.json`). Only switch: remap `Distribution` rows / `$VIBEVM_RELEASE_ORIGIN`. Assets: `DISTRIBUTIONS.json`, `vibevm-<ver>-<target>.zip`, `.fragment.json`, `vibevm-source.zip`, installers. Validation accepts a remapped `file://` base, but **the bundle downloader is reqwest-only — `file://` likely cannot be read (unverified bug, Phase 0 spike S1).** No command lists available remote releases (`vibe self ls` = installed only). `vibe self import <exe> --tag X` imports a single binary offline.
- **App installer inputs (`install -g`):** `url + sha256 (+size)` or package-relative `path`; cache `<cache>/<sha256>/<file>`. Every input goes through remap `AppInput`; a `path = <dir>` remap target is served by `copy_verified` (`<dir>/<file>`, `<dir>/<sha256>/<file>`), digest always verified, works offline. **`url = "file://…"` targets are rejected.**
- **URL overrides that exist ("https/http/git overrides"):** PROP-066 `[[remap]]` rows (keys `sha256 > package > registry > prefix`; targets `url | path | deny`; artifact kinds `RegistryClone IndexFile RawFile EmbeddedSource GitSourceDep AppInput Distribution DocShell`) in `~/.vibe/settings.toml` (L1) and `<project>/.vibe/settings.local.toml` (L3); `[[mirror]]`/`[[override]]` lowered into rows; `--remap`, `VIBE_REMAP`, `VIBEVM_RELEASE_ORIGIN`. Registry clones honour only `url` targets.
- **PATH:** `EnvPersister`; Unix rc-file managed block; **Windows: direct Win32 API (`windows_sys` `RegSetValueExW` on `HKCU\Environment`, type preserved, `WM_SETTINGCHANGE` broadcast) — no `setx`, no PowerShell, no child process.** Consent: `--yes` or interactive (`vibe self doctor --fix`, `self import --use --yes`).
- **Settings:** `~/.vibe/config.toml` (`[env] [install] [init] [net] [llm] [proxy] [transfer]`), PROP-040 three-level store (`~/.vibe/settings.toml`, `<repo>/.vibe/settings.toml`, `.vibe/settings.local.toml`), `vibe prefs`. **No "apply configuration from a file" command.**
- **Offline:** `--offline`/`VIBE_OFFLINE`/`[net].offline`; machine store `~/.vibe/cache`; `vibe cache add|import`; `vibe registry vendor` writes bare per-package repos usable as `file://` mirrors.

## 3. Design

### 3.1 The one idea: a *site profile* layer plus two kinds of stores

The ten items reduce to three concerns:

1. **Where packages come from and are published to** — a registry *type*.
2. **Where binary artifacts come from** — vibe releases (item 5) and app payloads (items 6–7). Item 8 asks whether they share a foundation: **yes** — both are already routed by the remap engine (`Distribution`, `AppInput` artifact kinds) and both need the same thing in a closed network: *a directory a human fills by hand, with a naming convention, from which vibe serves verified bytes*. That is an **artifact drop**. We do not invent a second routing mechanism; a drop is configuration that lowers into remap `path` rows plus a lookup convention, and we fix the two places that cannot read local bytes today (bundle downloader `file://`, app input `file://`).
3. **How one file configures a machine** (item 9) — a **site profile**: a TOML document the administrator writes once. `vibe self apply` stores it as its own configuration layer (it does not rewrite the user's files) between built-in defaults and the user's own settings. Everything the profile declares (registries, mirrors/overrides/remap rows, source policy, release source, artifact drops, proxy, offline) is read through the existing owners from that layer. Removing the profile removes its effect; re-applying an edited profile replaces it. The user can still override any value in their own files.

Why a layer and not a writer: writing the profile's content into `registry.toml`, `config.toml` and `settings.toml` would mix administrator-owned and user-owned bytes, make re-apply and removal guesswork (managed-blocks law: never write outside your own block, and TOML files there carry no block markers), and lose provenance. A layer is idempotent, diffable (`vibe self apply --diff`), and every effective value can report its origin (`site-profile`).

### 3.2 Registry types (items 1–3)

New optional field on every registry declaration — `vibe.toml` `[[registry]]`, machine `registry.toml`, the site profile, and the new project-local override file:

```toml
[[registry]]
name = "corp"
type = "monorepo"          # github | gitverse | git | monorepo | localdir ; case-insensitive
url  = "ssh://git@git.corp.example/tools/vibe-packages.git"
ref  = "main"
```

- **Values (case-insensitive, stored lower-case):**
  - `github`, `gitverse` — per-package repositories in an organisation (today's behaviour) with host-specific publishing (API repo creation). `gitverse` on any domain (on-prem GitVerse).
  - `git` — generic per-package repositories on any Git server (today's behaviour for unknown hosts); publish only into existing repositories (`--repo-url` semantics), never creates repos.
  - `monorepo` — **one repository holds every package** in the local-registry layout `<group>/<name>/v<version>/` on `ref`. SSH works with the ordinary `~/.ssh` config (already implemented). Resolution reuses `GitMonorepoRegistry`; index = directory listing of the clone. Publish = clone/fetch, write `<group>/<name>/v<version>/`, commit, push (no API, no org). A version directory that already exists with different content is refused (`--replace` to overwrite deliberately); identical content is a no-op. Concurrent publishers are serialized by Git itself (non-fast-forward → refetch and retry once, then refuse with a clear message).
  - `localdir` — a directory (`url = "file:///…"` or `path = "…"`), possibly an NFS/CEPH mount. Resolution = today's `LocalRegistry`. Publish = copy into `<group>/<name>/v<version>/` through a temp directory and a rename inside the target (atomic on one filesystem, nothing more). **No locking, no synchronisation guarantees** (mandate item 2); same overwrite rule as monorepo.
- **Inference when `type` is absent** (backwards compatible): `file:` → `localdir`; host `github.com` → `github`; host `gitverse.ru` → `gitverse`; any other → `git`. `monorepo` is never inferred (a per-package org URL and a single repo URL look alike).
- **Lock rows:** monorepo rows record repository URL, package path and commit; replay fetches that commit and reads that path (fits the locked-source law). `localdir` rows record the exact package directory (already true).
- **Project-local override file:** `<project>/.vibe/registry.local.toml` (git-ignored, same grammar as `registry.toml`), highest precedence of the declaration layers. Final precedence, highest first: invocation flags → project-local override → project `vibe.toml` → user `~/.vibe/registry.toml` → site profile → built-in defaults (the GitHub/GitVerse seeds are not written when a site profile declares registries).
- **CLI:** `vibe registry add|remove|list` gain `--scope project|local|user` (today they edit `vibe.toml` only) and `--type`.

### 3.3 Artifact drops (items 5–8)

A drop is a directory plus a convention, declared in the site profile or user config:

```toml
[artifacts]
vibe     = "D:/corp/vibe-releases"     # item 5
packages = "D:/corp/vibe-artifacts"    # items 6-7
```

- **vibe releases** — `<vibe>/v<version>/` holding exactly the files of the GitHub release (`DISTRIBUTIONS.json`, `vibevm-<version>-<target>.zip`, …), so an administrator downloads a release's files in a browser into a folder named after its tag. A bare `vibevm-<version>-<target>.zip` without the manifest is also accepted (manifest synthesised from the archive's own `DISTRIBUTION.json`). Listing = directory scan.
- **package payloads** — lookup order for an input declared `url + sha256` by package `G/N@V`: `<packages>/<G>/<N>/<V>/<filename>`, then `<packages>/<G>/<N>/<filename>`, then `<packages>/<sha256>/<filename>`, then `<packages>/<filename>`. **The digest is always verified**, so a wrong file is a clear refusal, never silently used. The filename is the last path segment of the declared URL. This is the override of "paths from the remote Git to a local release directory" (item 6) without editing the package.
- **Lowering:** each drop becomes remap rows at the site/user layer — `Distribution` → `path` row with the release convention; `AppInput` → `path` row with the package convention — so `vibe remap explain` shows why a fetch went to the drop. Fixes required: bundle downloader reads `file://`/`path` targets; app inputs accept `file://` URLs equivalently to `path`.
- **Item 7 (author-side):** a package may declare an input as `path` relative to a configured drop (`drop = "packages"` + `file = "<name>"` + `sha256`) for internal packages built inside the closed network; resolved through the same lookup.
- **Diagnostics:** when a fetch is not satisfied, the error names the exact expected file names and the drop paths searched. `vibe artifacts where [<pkg>]` prints the expected location for each pending input of a package (and for vibe releases), so the developer knows where to put what.

### 3.4 Release sources for `vibe self` (items 4–5)

```
vibe self source                       # show current source and where it came from
vibe self source set github|gitverse|nexus|localdir [--url URL | --path DIR]
vibe self releases [--all]             # list available versions at the source, mark installed/active
vibe self update [<version>]           # existing, now through the selected source
```

- Types (case-insensitive): `github` (default, today), `gitverse` (`https://gitverse.ru/vibevm/vibevm/releases` or an on-prem GitVerse URL — release layout verified in spike S2), `nexus` (an HTTP(S) base URL of a *raw/hosted* repository, e.g. Nexus or Artifactory, holding the same `v<version>/<asset>` layout), `localdir` (= the `artifacts.vibe` drop).
- Listing: GitHub/GitVerse via their release-list endpoints; `localdir` via directory scan; `nexus` via an optional `RELEASES.json` at the base (written by `xtask` release tooling and copied by admins) and, if absent, the Nexus search REST API when it answers, else "listing unavailable, specify a version".
- Storage: `[self] source = …` in `~/.vibe/config.toml` or the site profile; lowered into the existing `Distribution` remap row (replacing the hard-wired `VIBEVM_RELEASE_ORIGIN` special case with one owner).
- Verification: archives are checked against the `DISTRIBUTIONS.json` digests exactly as today; a drop never weakens verification.

### 3.5 `vibe self apply` (item 9) and PATH (item 10)

```
vibe self apply <file> [--dry-run|--diff] [--fix-path] [--yes]
vibe self apply --show                  # applied profile, origin, hash
vibe self apply --remove
```

- **Profile content** (`vibe-site-profile/1`, TOML, schema-generated reader): `name`, `[[registry]]` (with `type`), `[[mirror]]`, `[[override]]`, `[[remap]]` rows, `[source_policy]` (`prefer_hosting`, `exclude_hosting` — e.g. exclude `github`/`gitverse` so nothing ever tries the internet), `[self] source`, `[artifacts] vibe/packages`, `[proxy]`, `[net] offline`, `[path] fix = "ask"|"always"|"never"`.
- **Apply:** validate everything first (refuse the whole file on any error, nothing written); store the profile at `~/.vibe/site-profile.toml` with a provenance header (source path, sha256, applied-at); print a report: effective registries and their types, release source, **where to put vibe releases and package payloads (exact paths and naming, with one example each)**, which hosts are now excluded, and the follow-up `vibe update --sources-only` for projects whose locks still point at the internet.
- **Bootstrap from a manually downloaded binary:** if the running `vibe` is not a managed instance (e.g. `D:\Downloads\vibe.exe`), apply first imports it (`self import --use` machinery) so shims and the store exist.
- **PATH (item 10):** if the shim directory is not on the durable PATH: interactive → ask (default yes on Windows, where this is the purpose of the command; default no elsewhere); non-interactive → only with `--fix-path` (or profile `[path] fix = "always"`). Implementation: the existing `EnvPersister` — on Windows the direct Win32 registry API with `WM_SETTINGCHANGE` (no PowerShell, no `setx`, no child process at all — satisfies "never through jshell"; `cmd.exe` is not needed); on Unix the managed rc block. Linux behaviour is tested end to end now; Windows is compiled and unit-tested via the registry abstraction, and executed on a Windows machine before the release (manual test MT).

### 3.6 Decisions with rejected options

| Decision | Rejected | Why |
|---|---|---|
| Site profile is a separate read-only layer | Writing profile values into `registry.toml`/`config.toml`/`settings.toml` | Mixes owners, breaks re-apply/remove, violates managed-blocks law, loses provenance |
| One artifact-drop foundation lowered into remap rows | A new "release mirror" subsystem per artifact type | Remap already routes `Distribution` and `AppInput`; two mechanisms would answer "where did this file come from" differently (omnichannel law) |
| Digest always verified for drop files | Trust any file with the right name | The drop is hand-filled; a wrong download must be a clear refusal |
| `monorepo` never inferred | Guess from URL shape | Org URL and repo URL are indistinguishable over SSH |
| Monorepo/localdir versions immutable by default (`--replace` explicit) | Today's mutable re-publish | A shared directory/repo without API has no other protection against accidental overwrite |
| No locking on `localdir` | Lock files / leases on NFS | Owner: synchronisation over NFS/CEPH is out of scope; administrative process resolves conflicts |
| PATH via Win32 API (existing) | `setx`, PowerShell, `cmd /c reg add` | `setx` truncates at 1024 chars; PowerShell is blocked in corporate setups; the API needs no child process |
| `vibe self apply` name | `vibe setup`, `vibe config apply` | `self` already owns installation-level state; `apply` matches the kubectl habit the owner cited |

### 3.7 Interaction with already-delivered behaviour

- Locked sources: after switching to a corporate registry, existing projects keep GitHub sources in `vibe.lock`; `apply` prints `vibe update --sources-only` (FMT-064) as the migration step.
- Source policy: the profile can `exclude_hosting = ["github", "gitverse"]`; with the controlled transport this guarantees no contact attempt (FMT-066 plain-HTTP admission keeps internal HTTP mirrors working).
- `--offline`/store/`vibe cache import` remain the fallback when even the corporate Git is unreachable.

## 3.8 Phase 0 findings (2026-11-08) and the plan changes they force

- **S1 (P1 confirmed):** three fake releases (1.0.11–1.0.13, all targets, correct digests; generator `spike-corp/scripts/make_releases.py`) install and update correctly over `http://` through a remap row; `file://` fails in `HttpDownloader` (reqwest-only, `bundle/download.rs:79-94`, cache-busting `?vvm_nonce` appended), and `path` remap targets are never suffix-rewritten (`remap/locator.rs` `resolve_target` rewrites only `Url`). Bootstrap accepts only `https`/`file` remapped bases, update/install accept `http`. → Phase 5 adds a local reader for `file`/`path` (nonce stripped, suffix preserved), one http/https rule for bootstrap/update/install, and the release-drop layout `<vibe>/v<version>/` with a directory-scan "latest" (the GitHub `latest/download` alias is lowered to it).
- **S1 side finding (P4 at risk):** the release downloader has its own reqwest client: it bypasses source policy, HTTP admission and `[proxy]`, so `exclude_hosting = ["github"]` does not stop `vibe self update` from contacting github.com. → Phase 5 routes release downloads through the controlled transport and proxy settings; Phase 9 asserts it.
- **S2:** GitHub's release list is anonymous and carries sha256 digests per asset. GitVerse publishes **no** vibevm release assets (release tooling uploads to GitHub only) and its release API requires a token even to list (web routes rate-limited with 429). → the `gitverse` release source needs release tooling that uploads assets to GitVerse plus token handling; deferred to an owner decision (R5). `nexus` and `localdir` cover the corporate case.
- **S3 (P2 partly confirmed):** `GitMonorepoRegistry` clones over SSH with the ordinary fixture and lists/resolves/fetches correctly, but records no registry name, ref or commit, and its clone bypasses the remap locator and the offline filter. Lock replay needs "repository + package subpath + commit", which no current `SourceKind` expresses — a lock-format change. → Phase 2 splits into **2a** resolution (new `RegistrySource` variant, 16 match sites in 9 files, provenance fields, clone through the locator) and **2b** lock row + replay + `--sources-only` (FORMAT-CHANGES entry).
- **S4 (P5 falsified):** the Windows target is installed, but `cargo check --target x86_64-pc-windows-gnu` fails at HEAD (`config_path` is `#[cfg(unix)]` while its SSH-config callers are not; introduced by `84f5a19d7`). Being fixed now as a release blocker for 1.0.12, outside this campaign. `WindowsEnvPersister` calls the registry functions directly, so Linux cannot exercise it. → the Windows cross-check joins every phase gate; Phase 8 adds a small user-registry injection seam so the persister logic is unit-tested on Linux.

## 4. Phases and gates

Every phase after 0 ends with the gate: targeted tests for the touched owners, `cargo fmt --check`, strict clippy, `cargo xtask conform check` (0 new), `cargo xtask check-codegen`, `cargo check --workspace --all-targets --target x86_64-pc-windows-gnu`; the final phase runs the full workspace suite. Each phase is one or a few atomic commits; FORMAT-CHANGES entries in this directory's `FORMAT-CHANGES.md`.

- **Phase 0 — spikes (no commits).**
  - S1: does `vibe self update` read a `file://`/`path` release base? (reproduce with fake releases built by `xtask dist` into a temp drop).
  - S2: GitVerse release download/list URLs (public API shape) for `vibevm/vibevm`.
  - S3: `GitMonorepoRegistry` resolve/fetch over SSH against the existing sshd fixture; lock/replay of a monorepo row.
  - S4: Windows PATH code path compiles for `x86_64-pc-windows-gnu` (if the target is installable) and its unit tests run on Linux through the registry abstraction.
- **Phase 1 — registry `type`**: field, case-insensitive parse, inference, validation errors with fix hints; `registry.local.toml` layer; `vibe registry add/list --scope/--type`. Tests: every type parses in every layer; precedence matrix; inference table.
- **Phase 2a — monorepo resolution**: `RegistrySource::Monorepo`, provenance (name, ref, commit), clone through the remap locator and offline filter.
- **Phase 2b — monorepo lock rows, replay, `--sources-only` onto it, publish into it** (SSH, no API, immutable versions, non-fast-forward retry). Lock-format change recorded. Tests with the loopback sshd fixture.
- **Phase 3 — localdir publish**: atomic copy, immutability, `--replace`; publish reads machine/site registries too (today project-only).
- **Phase 4 — artifact drops (shared foundation)**: config, lookup conventions, remap lowering, `file://` fixes, `vibe artifacts where`. Tests: digest mismatch refused; each lookup rung; offline.
- **Phase 5 — `vibe self source|releases|update`**: local `file`/`path` reader, release downloads through the controlled transport and proxy, one http/https rule, types `github|gitverse|nexus|localdir`, listing per type, update from a drop with the Phase-0 fake releases (mandate item 5 experiment), latest selection, verification failure path.
- **Phase 5b — GitVerse release publishing**: `xtask` release creates the GitVerse release and uploads assets (token file, never printed); a dry-run mode for tests; no publish without the owner's command.
- **Phase 6 — `install -g` from drops**: payload override without editing packages (item 6), author-side drop inputs (item 7).
- **Phase 7 — site profile + `vibe self apply`**: schema, layer wiring into every owner above, `--dry-run/--diff/--show/--remove`, the "where to put releases" report, bootstrap import.
- **Phase 8 — PATH in apply**: user-registry injection seam on `WindowsEnvPersister`; interactive/`--fix-path`/profile policy; Linux E2E; Windows logic unit-tested on Linux, cross-compiled, plus a manual test document for a real Windows machine.
- **Phase 9 — closed-network acceptance**: one scripted scenario in a network-isolated sandbox (`unshare -n` if permitted, else a fixture that fails any non-loopback connect): fresh HOME, a downloaded `vibe` binary, `vibe self apply corp.toml`, install project packages from an SSH monorepo, `install -g` an app whose payload lives in the drop, `vibe self releases`/`update` from the release drop, publish a package to the monorepo and to a localdir; assert zero non-loopback connection attempts. User guide page for administrators.

## 5. Predictions (falsifiable)

- P1: S1 shows `file://` release bases fail today. — **confirmed** (§3.8).
- P2: `GitMonorepoRegistry` needs only wiring, not a rewrite (≤ 400 lines including tests for Phase 2 resolution). — **holds for resolution; lock replay is a format change** (§3.8).
- P3: No new remap artifact kind is needed for drops.
- P4: The closed-network scenario passes with zero non-loopback connects once `exclude_hosting = ["github","gitverse"]` is in the profile.
- P5: Windows PATH needs no new code beyond wiring (the Win32 persister already exists). — **falsified**: the Windows build is broken at HEAD and the persister has no test seam (§3.8).

## 6. Quick start (cold)

```sh
cd /fast/git/vibevm
git log --oneline -3                     # expect VIBE-NEEDS commits on main
cat campaigns/corporate-env-2026-11/PLAN.md campaigns/corporate-env-2026-11/LOG.md
# Phase 0 spikes use the isolated runner:
python3 /fast/vibevm-test-tmp/claude-int/run_test.py <label> cargo test -p <crate> --test <t> --offline --locked
```

## 7. Acceptance script (whole campaign)

To be written in Phase 9 as `campaigns/corporate-env-2026-11/accept.sh`: runs the closed-network scenario of Phase 9 and exits non-zero on any non-loopback connect, any unverified artifact use, or any step's failure.

## 8. Open points (not blocking; decided as stated unless the owner objects)

- R1: "Nexus" is interpreted as a raw/hosted HTTP artifact repository with the GitHub-release-compatible `v<version>/<asset>` layout (Nexus, Artifactory, plain web server alike).
- R2: monorepo/localdir versions immutable by default, `--replace` to overwrite deliberately.
- R3: site profile sits below the user's own settings (the user may override it).
- R4: on Windows `apply` defaults the PATH question to *yes*; elsewhere to *no*.
- R5 — **resolved by the owner 2026-11-08: add GitVerse upload.** Release tooling (`xtask dist`/release) also creates the release and uploads assets to GitVerse with the per-host token file; the `gitverse` release source lists and downloads with the user's token when required, anonymously when assets are public. Releases are published only on the owner's command. Lands in Phase 5 (source type) and a Phase 5b (release tooling).

## 9. Log

See `LOG.md`.

## 10. Report

Empty until close.
