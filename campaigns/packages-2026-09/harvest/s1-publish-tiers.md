# S1 — двухъярусный publish: замер перед стройкой

> Замер ЧТЕНИЕМ кода. Ни `cargo`, ни `git`, ни `vibe`, ни сеть не запускались.
> Каждое утверждение о дереве несёт цитату `файл:строка`. Периметр записи —
> только этот файл (Worktree-отчёт — отдельно).

## 1. ВЕРДИКТ

**ДА ПОСЛЕ ДОРАБОТКИ** — небольшой и локальной.

Вопрос: различимы ли сегодня «репозитория нет» и «сеть/аутентификация
отвалились» на уровне, который увидит диспетчер publish?

На уровне диспетчера (`vibe-publish`, поверхность ошибок `PublishError`)
сегодня — **нет, не различимо чисто**: сетевой сбой и сбой аутентификации
уже разведены по разным вариантам (`HostUnreachable`, `PushDenied`), а вот
«репозитория нет» отдельного варианта не имеет и падает в общий
`PublishError::Git(String)` (`crates/vibe-publish/src/lib.rs:187`), куда
вложен сырой stderr. Диспетчер не может `match`-нуть «репо нет» — только
угадывать по подстроке.

Но — и это решающее — **нужный трёхсторонний дискриминатор уже существует в
дереве, причём дважды**: (а) тип `GitError` с чистыми `RepoNotFound` /
`AuthFailed` / `NetworkUnreachable` (`crates/vibe-registry/src/git_backend/mod.rs:47,54,61`);
(б) чистая функция `classify_stderr_message`, которая матчит
`"repository not found"` → `RepoNotFound`, семейство auth → `AuthFailed`,
семейство network → `NetworkUnreachable`
(`crates/vibe-registry/src/git_backend/shell.rs:492-555`). Она уже работает
на том же самом примитиве `git ls-remote --tags`
(`crates/vibe-registry/src/git_backend/shell.rs:212`), который нужен зонду
яруса 1. Доработка сводится к тому, чтобы дать publish-стороне вариант
`RepoNotFound` (перенести `classify_stderr_message` в `git_publish::ls_remote_tags`
или переиспользовать `ShellGit::list_tags` напрямую). Захват stderr, редация
кредов, `GIT_TERMINAL_PROMPT=0`, «анонимное» глушение credential-helper-ов —
всё уже построено и протестировано.

**Оговорка принципиальная (не чинится кодом):** GitHub на приватный репо, к
которому у токена нет доступа, отвечает тем же `Repository not found`, что и
на несуществующий — это безопасность хоста. Значит «не найден» — надёжный
сигнал «⇒ ярус 2» только для публичных / операторских репозиториев; в ином
случае диспетчер обязан остановиться и спросить — ровно как требует план
(«при неразличимости — остановка, а не угадывание»). Это свойство хоста, а
не пробел в нашем коде.

Вывод: **слайс строится**, доработка невелика и опирается на уже
оттестированный в дереве паттерн.

## 2. Сверка базы слайса (B1..B7)

| # | утверждение | вердикт | цитата file:line |
|---|---|---|---|
| B1 | оркестратор publish — `crates/vibe-publish/src/orchestrator.rs`; CLI-вход — `crates/vibe-cli/src/commands/registry/publish.rs` | ПОДТВЕРЖДЕНО (+ у оркестратора уже есть short-circuit прямого git-пути по `direct_repo_url()`) | `crates/vibe-publish/src/orchestrator.rs:127` (`Publisher`), `:159` (`publish`); `crates/vibe-cli/src/commands/registry/publish.rs:71` (`run_publish`); short-circuit `orchestrator.rs:189` |
| B2 | выбор адаптера по хосту URL; GitVerse — осознанный стаб с машинным конвертом (`publish.rs:121-141`, `extract_host_segment`, `creator_for_url`; `host_lower == "gitverse.ru"` → stub) | ПОДТВЕРЖДЕНО (диапазон строк сдвинут: база 121-141, по факту стаб-блок 132-170; `host_lower`-проверка на `:141`) | stub: `crates/vibe-cli/src/commands/registry/publish.rs:140-170`; конверт `PublishStubReport` `:44-52`; `extract_host_segment` ИСПОЛЬЗУЕТСЯ `:127`, ОПРЕДЕЛЕНА `crates/vibe-publish/src/lib.rs:257`; `creator_for_url` ИСПОЛЬЗУЕТСЯ `:193`, ОПРЕДЕЛЕНА `lib.rs:307` |
| B3 | прямой git-путь существует: `crates/vibe-publish/src/direct_git.rs` — движок `--repo-url`; `publish.rs:83` описывает его как «bypass registries, host adapters, tokens» | ПОДТВЕРЖДЕНО | `crates/vibe-publish/src/direct_git.rs:32` (`DirectRepoCreator`), `:124` (`direct_repo_url` → `Some`); комментарий `crates/vibe-cli/src/commands/registry/publish.rs:83` («bypass registries, host adapters, tokens») |
| B4 | композиция имени репо — `NamingConvention::repo_name`, `project.rs:313`, вариант `Fqdn` даёт `{group}.{name}` | ПОДТВЕРЖДЕНО (313 — строка `match`; ветвь `Fqdn` на `:314`; трейт несёт 4 варианта, не только `Fqdn`) | `crates/vibe-core/src/manifest/project.rs:307-331` (`repo_name`), `:314` (`Fqdn => format!("{group}.{name}")`); enum `:271-294` |
| B5 | трейт хоста в `creator.rs` несёт ровно `host_name`, `repo_exists(org,name)`, `create_repo`, `push_url` | ЧАСТИЧНО: эти 4 — единственные ОБЯЗАТЕЛЬНЫЕ методы (без default), но трейт дополнительно несёт 3 default-метода: `expected_org`, `validate_scope`, `direct_repo_url` | `crates/vibe-publish/src/creator.rs:170-238`; обязательные `:172` (`host_name`), `:177` (`repo_exists`), `:181` (`create_repo`), `:195` (`push_url`); default-методы `:202` (`expected_org`), `:216` (`validate_scope`), `:235` (`direct_repo_url`) |
| B6 | токен-лоадер `crates/vibe-publish/src/token.rs` знает ЧЕТЫРЕ источника | ПОДТВЕРЖДЕНО для загрузчика (ровно 4 шага в `load_token_for_host`); модуль-док перечисляет 5 — но 1-й (`explicit`) — тестовый конструктор, не часть загрузки | `crates/vibe-publish/src/token.rs:146-164` (4 шага: host-env `:147`, legacy-env `:151`, host-file+legacy-file через `token_file_candidates` `:155`); док-коммент 5 пунктов `:1-23`; `AuthMissing` `:161` |
| B7 | индекс-хук после publish — `crates/vibe-publish/src/post_hook.rs`, про ярусы ничего не знает | ПОДТВЕРЖДЕНО | `crates/vibe-publish/src/post_hook.rs:325` (`fire` / `fire_index_hook`); активация по env `:130`; «никогда не роняет publish» `:9-11`; о ярусах упоминаний нет |

## 3. Git-слой: тип ошибки и различимость отказов

### 3.1 В дереве ДВА независимых git-слоя

| слой | тип ошибки | где `ls-remote` | где живёт |
|---|---|---|---|
| **consume/registry** | `GitError` — с чистым `RepoNotFound`/`AuthFailed`/`NetworkUnreachable` | `ShellGit::list_tags` `shell.rs:212` | `crates/vibe-registry/src/git_backend/{mod.rs,shell.rs}` |
| **publish** | `PublishError` — `HostUnreachable`/`PushDenied` есть, `RepoNotFound` НЕТ | `git_publish::ls_remote_tags` `git_publish.rs:120` | `crates/vibe-publish/src/git_publish.rs` |

Диспетчер publish-ярусов будет жить рядом с `Publisher` / `run_publish`, т.е.
на стороне `PublishError`. Хинт пакета «начни с
`crates/vibe-registry/src/git_backend.rs`» ведёт по верному следу, но это
**каталог**, не файл: реальный файл — `git_backend/mod.rs` (+ `shell.rs`,
`shell/{query.rs,tar.rs}`).

### 3.2 Тип ошибки publish-слоя — дословно

`crates/vibe-publish/src/lib.rs:113-236`:

```rust
#[derive(Debug, Error)]
#[spec(implements = "spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#publish")]
pub enum PublishError {
    SourceInvalid { path: PathBuf, reason: String },          // :122
    OrgUrlInvalid { url: String },                             // :130
    AuthForbidden { host: String, org: String },              // :138
    AuthMissing { host: String },                             // :146
    OrgNotFound { host: String, org: String },                // :155
    TagCollision { repo: String, tag: String },               // :163
    PushDenied { repo: String },                              // :172
    HostUnreachable { host: String },                         // :180
    Git(String),                                              // :187  <-- сюда падает «репо нет»
    HttpFailed { host: String, message: String },             // :194
    UnexpectedResponse { host: String, status: u16, body: String }, // :202
    Io { path: PathBuf, message: String },                    // :213
    ScopeViolation { host: String, expected_org: String, attempted_org: String }, // :221
    UnsupportedHost { host: String },                         // :235
}
```

`Git(String)` (`lib.rs:187`) **несёт stderr дочернего процесса** — но как
свободную строку, а не как классифицированный тип. Сетевой сбой и сбой
auth на этом уровне уже разведены (`HostUnreachable :180`, `PushDenied :172`);
«репо нет» — нет.

### 3.3 Доказанный дискриминатор, который уже решает эту задачу — дословно

`crates/vibe-registry/src/git_backend/mod.rs:29-110` — `GitError` (тот самый
PROP-002 `#failure-discriminator`):

```rust
#[derive(Debug, Error)]
#[spec(implements = "spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#failure-discriminator")]
pub enum GitError {
    NotInstalled,                                              // :40
    RepoNotFound { url: String },                              // :47
    AuthFailed { url: String },                                // :54
    NetworkUnreachable { url: String },                        // :61
    RefNotFound { url: String, refname: String },              // :68
    FileNotFoundInRef { url: String, refname: String, path: String }, // :75
    ArchiveUnsupported { url: String },                        // :87
    CommandFailed { cmd: String, status: i32, stderr: String }, // :94  <-- несёт stderr
    Io { cmd: String, #[source] source: std::io::Error },      // :105
}
```

`crates/vibe-registry/src/git_backend/shell.rs:492-555` — чистая
сабстроковая классификация (порядок проверок значим):

```rust
fn classify_stderr_message(combined: &str, url: String, refname: String) -> Option<GitError> {
    let lc = combined.to_lowercase();

    if lc.contains("repository not found") || lc.contains("does not appear to be a git repository") {
        return Some(GitError::RepoNotFound { url });                       // :495-498
    }
    if lc.contains("permission denied (publickey)")
        || lc.contains("authentication failed")
        || lc.contains("could not read username")
        || lc.contains("could not read password")
        || lc.contains("user cancelled dialog")
        || lc.contains("http 401")
        || lc.contains("http 403")
        || lc.contains("401 unauthorized")
        || lc.contains("403 forbidden")
    {
        return Some(GitError::AuthFailed { url });                         // :517-528
    }
    if lc.contains("could not resolve host")
        || lc.contains("could not read from remote repository")
        || lc.contains("network is unreachable")
        || lc.contains("failed to connect")
        || lc.contains("could not connect to")
        || lc.contains("connection refused")
        || lc.contains("connection timed out")
        || lc.contains("operation timed out")
    {
        return Some(GitError::NetworkUnreachable { url });                 // :537-547
    }
    if lc.contains("remote branch") && lc.contains("not found")
        || lc.contains("couldn't find remote ref")
    {
        return Some(GitError::RefNotFound { url, refname });               // :548-552
    }
    None                                                                   // -> CommandFailed{stderr}
}
```

Вызывается из `classify_failure` (`shell.rs:454-480`), в которую `ShellGit`
направляет каждый ненулевой выход git (`shell.rs:110`). Незлассифицированное
падает в `GitError::CommandFailed { cmd, status, stderr }` (`shell.rs:475-479`)
— stderr сохраняется всегда.

### 3.4 Все места, где ошибка git разбирается по содержимому stderr

- `crates/vibe-registry/src/git_backend/shell.rs:492` — `classify_stderr_message`
  (главный дискриминатор; RepoNotFound/AuthFailed/NetworkUnreachable/RefNotFound).
- `crates/vibe-registry/src/git_backend/shell.rs:454` — `classify_failure`
  (обёртка: извлекает url/refname из argv, делегирует в `classify_stderr_message`).
- `crates/vibe-registry/src/git_backend/shell.rs:262-317` — `fetch_file_at_ref`:
  различает FileNotFoundInRef / ArchiveUnsupported (включая `HTTP 422` GitHub) /
  RefNotFound по combined stderr+stdout.
- `crates/vibe-publish/src/git_publish.rs:376` — `push_with_classification`:
  PushDenied / HostUnreachable / TagCollision / Git. **RepoNotFound не матчит.**
- `crates/vibe-publish/src/git_publish.rs:128-144` — классификатор внутри
  `ls_remote_tags`: HostUnreachable / PushDenied / Git. **RepoNotFound не матчит.**
- `crates/vibe-publish/src/git_publish.rs:255-272` — классификатор внутри
  `shallow_clone`: те же три ветви (HostUnreachable/PushDenied/Git).

### 3.5 Все места вызова `ls-remote` в дереве (runtime-код, не тесты/доки)

Ровно ДВА реальных вызова `git ls-remote`:

1. `crates/vibe-registry/src/git_backend/shell.rs:212` —
   `ShellGit::list_tags`: `self.run(&["ls-remote","--tags","--",url])`.
   Ошибка проходит `classify_failure` → **RepoNotFound различимо**.
2. `crates/vibe-publish/src/git_publish.rs:120` —
   `ls_remote_tags`: `git_command_in_temp(&["ls-remote","--tags","--",url])`.
   Ошибка проходит локальный классификатор (`:128-144`) → **RepoNotFound НЕ различимо**
   (падает в `PublishError::Git(String)` `:142-144`).

Остальные хиты grep по `ls-remote` — это: тестовые фейки `list_tags` (реализации
трейта в `tests/`), комментарии о fast-path индекса (`index_client/mod.rs:109`),
локальный `vibe-index`/`vvm` сканеры по локальным репо (не сетевые зонды), и
тесты CLI, проверяющие bare-репо через внешний `git ls-remote`
(`cli_registry_mgmt.rs:850`). Ни один из них — не зонд существования в publish.

### 3.6 Вывод по §3

На уровне, который увидит диспетчер publish (`PublishError`), «репо нет» и
«сеть/auth» **сегодня НЕ различимы чисто** — первое тонет в `Git(String)`.
Но дискриминатор для этого уже есть и работает в соседнем крейте
(`GitError` + `classify_stderr_message`, на том же `ls-remote`). Доработка —
см. §1 и §10.

## 4. Имя репозитория и сборка URL

### 4.1 `NamingConvention` целиком

`crates/vibe-core/src/manifest/project.rs:271-294`:

```rust
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum NamingConvention {
    #[default] #[serde(rename = "fqdn")]      Fqdn,         // :281  -> "{group}.{name}"
    #[serde(rename = "kind-name")]            KindName,     // :285  -> "{kind}-{name}"
    #[serde(rename = "name")]                 Name,         // :289  -> "{name}"
    #[serde(rename = "kind/name")]            KindSlashName,// :293  -> "{kind}/{name}"
}
```

`repo_name` (`project.rs:307-331`):

```rust
pub fn repo_name(&self, kind: Option<PackageKind>, group: &Group, name: &str) -> Result<String> {
    match self {
        NamingConvention::Fqdn => Ok(format!("{group}.{name}")),          // :314
        NamingConvention::KindName => { /* needs kind */ format!("{}-{name}", kind.as_str()) } // :315-321
        NamingConvention::Name => Ok(name.to_string()),                   // :322
        NamingConvention::KindSlashName => { /* needs kind */ format!("{}/{name}", kind.as_str()) } // :323-329
    }
}
```

`Fqdn` — default (`#[default]` `:279`), детерминирован из `(group, name)` без
участия kind, infallible. Это и есть «имя репо детерминированно из
идентичности» — база яруса 1. Тестируется: доктест `project.rs:265`,
`project/tests.rs:120`, `document/tests.rs:398`.

### 4.2 Все места сборки URL репозитория пакета

- **HTTPS + встроенный токен (push)**: `GithubRepoCreator::push_url`,
  `crates/vibe-publish/src/github.rs:156-171`:
  `format!("https://x-access-token:{TOKEN}@{host}/{org}/{name}.git")`.
- **SSH (push/clone)**: `GitverseRepoCreator::push_url`,
  `crates/vibe-publish/src/gitverse.rs:135-139`:
  `format!("git@{host}:{org}/{name}.git")`.
- **URL дословно (push)**: `DirectRepoCreator::push_url`,
  `crates/vibe-publish/src/direct_git.rs:117-122` — возвращает `self.repo_url` как есть.
- **Синтез clone_url/html_url из org_url + repo_name** (без хост-API):
  оркестратор `crates/vibe-publish/src/orchestrator.rs:245-247` и `:257-259`:
  `format!("{org_url}/{repo_name}")` и `format!("{org_url}/{repo_name}.git")`.
  Это ровно та форма, которой ярусу 1 достаточно, чтобы собрать push-URL из
  идентичности без токена.

## 5. Прямой git-путь сегодня

Файл: `crates/vibe-publish/src/direct_git.rs`. Адаптер `DirectRepoCreator`
(`direct_git.rs:32-41`) хранит одно поле `repo_url: String` (плюс `host_name`
для диагностики). Токенов, API-клиента, org-scoping — нет.

Публичная сигнатура входа в push — `git_publish::push_release`
(`crates/vibe-publish/src/git_publish.rs:30-36`):

```rust
pub fn push_release(
    source_dir: &Path,
    clone_url: &str,
    tag: &str,
    package_name: &str,
    version: &semver::Version,
) -> Result<(), PublishError>
```

Поведение:
- **Креды**: vibevm их НЕ загружает. Push-URL = `clone_url` как есть
  (`git_publish.rs:69` `remote add origin`), локальный git резолвит через
  SSH-agent / `credential.helper` / `~/.netrc` (см. док `direct_git.rs:117-122`,
  `publish.rs:83-89`).
- **Теги**: да. `push_release` делает `tag -a <tag>` (`git_publish.rs:63`),
  затем два пуша — ветка и тег (`git_publish.rs:71-72`).
- **Непустой удалённый**: **НЕ обрабатывается.** `push_release` всегда строит
  СВЕЖУЮ single-commit историю — `git init` (`:48`), copy source (`:44`),
  `commit` (`:58`), затем `git push -u origin main` без `--force` (`:71`;
  «publish never force-pushes» — `orchestrator.rs:149-152`). У нового корневого
  коммита нет общего предка с уже существующей на remote веткой `main` ⇒ push
  отклоняется как non-fast-forward. В `push_with_classification`
  (`git_publish.rs:376-422`) **нет ветви** под `rejected`/`non-fast-forward`/
  `fetch first` ⇒ отказ всплывает как общий `PublishError::Git(String)`.
  Вывод: сегодняшний прямой путь умеет «первую публикацию в пустой репо», но
  НЕ «обновление существующего репо». Это препятствие для яруса 1 (см. §10).

## 6. Токен: источники, приоритет, обязательность

`crates/vibe-publish/src/token.rs`. Загрузчик `load_token_for_host`
(`token.rs:146-164`) обходит ровно **ЧЕТЫРЕ** источника в порядке:

1. `VIBEVM_PUBLISH_TOKEN_<HOST>` env (host-specific; имя строит `host_env_var`
   `token.rs:111`, напр. `github.com`→`VIBEVM_PUBLISH_TOKEN_GITHUB`) —
   `read_host_env_token` `token.rs:147,173`.
2. `VIBEVM_PUBLISH_TOKEN` env (legacy host-agnostic, `LEGACY_ENV_VAR :102`) —
   `read_legacy_env_token` `token.rs:151,186`.
3. `<settings-dir>/<host-prefix>.publish.token` файл (host-specific, напр.
   `~/.vibe/github.publish.token`; `$VIBE_SETTINGS` переносит всё разом) —
   `per_host_token_path` `token.rs:155,239`.
4. `<settings-dir>/git.publish.token` файл (legacy host-agnostic) —
   `legacy_token_path` `token.rs:246`.

(Модуль-док `token.rs:1-23` перечисляет 5 пунктов, но пункт 1 —
`Token::from_explicit` — это тестовый конструктор, а не шаг загрузки; в
`load_token_for_host` его нет. Поэтому «четыре источника» из базы — верно для
загрузчика.)

### Обязательность токена — можно ли publish без него вовсе?

**Сегодня — нельзя на реестровом пути; можно только на прямом (`--repo-url`).**

- Реестровый путь: `run_publish` вызывает `load_token_for_host(&host)` в
  `crates/vibe-cli/src/commands/registry/publish.rs:180` **до** создания
  адаптера. Если ни один источник не дал значения, загрузчик возвращает
  `PublishError::AuthMissing { host }` (`token.rs:161-163`) — publish
  обрывается ещё до любого creator/push.
- Прямой путь: ветка `--repo-url` уходит в `run_publish_direct`
  (`publish.rs:90-92,280`), который **никогда** не вызывает
  `load_token_for_host` — токен не нужен по построению
  (доказано тестом-сигналом `cli_workspace_publish.rs:154-158`: ставятся
  заведомо фальшивые токены, и прямой путь их не читает).

Следствие для яруса 1: **ярус 1 бесплатен по кредам**, если диспетчер
маршрутизирует его через short-circuit `direct_repo_url` (как сегодня делает
`--repo-url`). Лоадер на этом пути просто не вызывается. Секреты в `~/.vibe/**`
в замере не открывались — только источники (пути/имена переменных).

## 7. Трейт хоста и его реализации

Трейт `RepoCreator` — `crates/vibe-publish/src/creator.rs:170-238`. Реализаций
**три** (плюс тестовые моки в doctest-ах):

| реализация | файл | `repo_exists` | `push_url` |
|---|---|---|---|
| `GithubRepoCreator` | `github.rs:67-293` | **реальный HTTP** `GET /repos/{org}/{repo}` (`:173-203`): 200→`Ok(true)`, **404→`Ok(false)`**, 401/403→`AuthForbidden`, сеть(connect/timeout)→`HostUnreachable`, иное→`UnexpectedResponse` | HTTPS+токен `github.rs:164` |
| `GitverseRepoCreator` | `gitverse.rs:51-256` | **реальный HTTP** `GET /repos/{org}/{repo}` (`:141-169`): 200→`Ok(true)`, **404→`Ok(false)`**, 401/403→`AuthForbidden`, иное→`UnexpectedResponse` | SSH `gitverse.rs:138` |
| `DirectRepoCreator` | `direct_git.rs:32-127` | **стаб** `Ok(true)` без запроса (`:89-96`; комментарий: «operator told us this repo exists») | URL дословно `:121` |

Важная подробность для §1: оба API-адаптера **уже различают** «репо нет»
(404→false) от auth (401/403) от сети — но это **токен-зависимый** зонд (GET
`/repos` требует `Authorization: Bearer`). Ярус 1 хочет зондиовать БЕЗ токена,
поэтому ему нужен `ls-remote`, а не `repo_exists` API. А у `ls-remote` на
publish-стороне варианта RepoNotFound нет (§3.5).

## 8. Интерактивность (TTY) и её выключатели

Механизм «интерактивно или нет» в дереве есть и многослойный:

- **Глобальный флаг `--unattended`**: `crates/vibe-cli/src/cli.rs:91-92`
  `#[arg(long, global = true)] pub unattended: bool`. Эквивалентен env
  `VIBE_UNATTENDED` (thruty: `1/true/yes/on`); «skip every confirmation
  prompt and refuse to open any interactive wizard» (`cli.rs:80-90`); штампует
  `"unattended": true` на каждый JSON-конверт.
- **Разрешение постуры**: `resolve_unattended(cli_flag)` и
  `Context::is_unattended()` — `crates/vibe-cli/src/output.rs:84-85,159-160`
  (CLI-флаг OR `VIBE_UNATTENDED`).
- **`--assume-yes`** — НЕ глобальный; локальный флаг отдельных подкоманд
  (build/run tool-runner) `crates/vibe-cli/src/cli.rs:317-318,332-333`. На
  `registry publish` его нет.
- **Доказанный шаблон confirm-or-abort**: `DestructiveGuard` —
  `crates/vibe-workspace/src/materialization.rs:45-50,87-92`. Три исхода:
  `Proceed` (opted-in через `--force`/`--assume-yes`/`--unattended`),
  `ConfirmInteractively` (интерактивно, без opt-in),
  `Abort` (неинтерактивно, без opt-in). На нём естественным образом встанет
  вопрос яруса 2 «создавать репо?».
- **Git-сторона TTY**: `should_silence_credential_helpers` —
  `crates/vibe-registry/src/git_backend/shell.rs:434-448`. Резолюция:
  `force_silence` > `VIBEVM_GIT_SILENCE_HELPERS` > `VIBE_UNATTENDED` >
  `!stdin().is_terminal()` (`IsTerminal` `:447`). Гарантирует, что 401/403
  никогда не станет блокирующим GCM/askPass-промптом — критично для
  бесклеточного зонда яруса 1.

`RegistryPublishArgs` (`crates/vibe-cli/src/cli/registry.rs:396-421`) несёт
только `source / registry / repo_url / path / dry_run` — никакого
per-command флага подтверждения. Подтверждение яруса 2 будет стоять на
глобальном `--unattended` + `DestructiveGuard`.

## 9. Тестовая опора для герметичного яруса 1

Образец герметичного теста реестра — `crates/vibe-cli/tests/cli_registry_mgmt.rs`,
функции `make_*`:

- `make_two_version_registry` (`cli_registry_mgmt.rs:44-108`): строит НЕ-bare
  репо с двумя версиями через `run_git` (`init --initial-branch=main` `:47`,
  `commit` `:74`, `tag v0.1.0` `:75`, … `tag v0.2.0` `:94`), затем
  `git clone --bare` в `org.vibevm.wal.git` (`:96-105`) и
  `symbolic-ref HEAD refs/heads/main` (`:106`).
- `make_features_fixture_registry` — `cli_registry_mgmt.rs:994`.
- Проверка bare-репо как git-источника — `vendor_produces_bare_repo_per_lockfile_entry`
  (`cli_registry_mgmt.rs:778-864`): внешний `git ls-remote --tags`
  (`:849-850`) + `git clone --branch` (`:871-878`).

**Решающий прецедент для «обновить пакет БЕЗ токена»** —
`publish_direct_repo_url_pushes_to_local_bare_repo`
(`crates/vibe-cli/tests/cli_workspace_publish.rs:109-198`):

```rust
let bare_dir = tempfile::tempdir().unwrap();
let bare = bare_dir.path().join("origin.git");
std::process::Command::new("git")
    .args(["init", "--bare", bare.to_str().unwrap()])      // :140-141
    .env("LC_ALL","C").status().unwrap();
let repo_url = format!("file:///{}", abs_bare.trim_start_matches('/')); // :148
let out = vibe()
    .env("VIBEVM_PUBLISH_TOKEN", "should-not-be-read-on-direct-path")          // :154
    .env("VIBEVM_PUBLISH_TOKEN_GITHUB", "should-not-be-read-on-direct-path")   // :156
    .arg("--json").arg("registry").arg("publish").arg(pkg_dir.path())
    .arg("--repo-url").arg(&repo_url) …                                       // :163-164
// asserts: payload["mode"] == "direct-git", tag "v0.0.1" present, main present // :178-198
```

Ключевые строки: `file:///<abs>/origin.git` — герметичный origin без сети и
кредов (`:138-148`); **фальшивые токены-сигналы** доказывают, что прямой путь
их не читает (`:154-158`); после push проверяются тег и ветка (`:184-198`).

**Годится ли форма для теста «обновить пакет БЕЗ токена»?** Да — как основа:
`file://` bare-репо + прямой push без токена + проверка через
`ls-remote`/`tag --list`. **Оговорка:** этот тест покрывает первую публикацию в
ПУСТОЙ bare-репо. Тест «обновление существующего репо» потребует
предзаполненного bare (второй push поверх истории) — а `push_release` сегодня
такой случай роняет (§5, §10). Значит тестовая опора есть, но семантика
обновления — часть работы яруса 1.

## 10. Что препятствие, а что работа

**Препятствия (надо решить, чтобы ярус 1 заработал):**

1. **Нет варианта `RepoNotFound` на publish-стороне.** Зонд существования яруса 1
   пойдёт через `git ls-remote` без токена; сегодня на этом уровне
   (`PublishError::Git`) «репо нет» неотличимо от прочего. Решение — добавить
   классификацию «not found» в `git_publish::ls_remote_tags` (или переиспользовать
   `ShellGit::list_tags`/`classify_stderr_message` из `vibe-registry`). Логика
   уже написана и оттестирована — перенос локален.
2. **`push_release` не умеет обновлять непустой remote.** Свежая single-commit
   история без `--force` отбрасывается как non-fast-forward (§5). Ярусу 1 нужна
   стратегия обновления (fetch+fast-forward / force-with-lease на теге /
   push только тега поверх существующего main) — сегодня её нет.
3. **Диспетчер ярусов ещё не существует.** Сегодня выбор «прямой git vs
   host-API» делает оператор флагом `--repo-url` (`publish.rs:90`), а не
   программа по результату `ls-remote`. Ярус 1 должен: собрать URL из
   идентичности (`NamingConvention::repo_name` + форма `orchestrator.rs:245`),
   прозондировать `ls-remote`, по verdict решить ярус — и при этом НЕ требовать
   `--repo-url` и НЕ грузить токен.

**Работа (не препятствие — есть и готово):** трёхсторонний дискриминатор
`GitError`/`classify_stderr_message`; редация кредов `redact_credentials`
(`git_publish.rs:431`); глушение интерактивных промптов `should_silence_credential_helpers`;
глобальный `--unattended` + `DestructiveGuard` для подтверждения яруса 2;
`NamingConvention::Fqdn` (детерминированное имя); `DirectRepoCreator` (ready
cell); герметичная тестовая форма `file://` bare + фальшивый токен-сигнал;
`PublishOutcome` и весь рендер CLI. Ничего из этого строить заново не нужно.

## 11. Как воспроизвести этот замер

Только чтение. Одна команда — один глагол. Пути — от корня worktree.

```
ls crates/vibe-publish/src crates/vibe-registry/src
Read crates/vibe-publish/src/orchestrator.rs
Read crates/vibe-cli/src/commands/registry/publish.rs
Read crates/vibe-publish/src/git_publish.rs
Read crates/vibe-registry/src/git_backend/mod.rs
Read crates/vibe-registry/src/git_backend/shell.rs
Read crates/vibe-publish/src/lib.rs
Read crates/vibe-publish/src/creator.rs
Read crates/vibe-publish/src/direct_git.rs
Read crates/vibe-publish/src/token.rs
Read crates/vibe-publish/src/github.rs
Read crates/vibe-publish/src/gitverse.rs
Read crates/vibe-publish/src/post_hook.rs
Read crates/vibe-core/src/manifest/project.rs
Grep "ls-remote|ls_remote_tags|fn list_tags" --glob crates/**/*.rs
Grep "struct RegistryPublishArgs" --glob crates/vibe-cli/src/**/*.rs
Read crates/vibe-cli/tests/cli_registry_mgmt.rs
Read crates/vibe-cli/tests/cli_workspace_publish.rs
Grep "unattended|assume_yes" --glob crates/vibe-cli/src/cli*.rs
```
