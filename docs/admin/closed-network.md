# vibe в закрытой корпоративной сети — руководство администратора

Для кого: администратор, который разворачивает vibe там, где нет доступа к
GitHub и GitVerse, исполняемые файлы скачиваются только вручную через браузер,
а Git доступен (если вообще доступен) только по SSH.

Как проверено: каждая команда ниже выполнена на собранном `vibe 1.0.11`
(Linux, x86_64) в песочнице без сети (`bwrap --unshare-net`, только loopback);
сценарий целиком — `campaigns/corporate-env-2026-11/accept.sh`. Команды,
которые относятся только к Windows или требуют интернета, помечены
**«не выполнялось здесь»** с причиной. Все адреса в примерах — заглушки
(`git.example.corp`, `proxy.example.corp:8080`, `\\fileserver\vibe`).

---

## 1. Обзор: что где лежит

| Что | Где | Чем управляется |
|---|---|---|
| Пакеты vibe | Один Git-репозиторий на корпоративном SSH-сервере (тип `monorepo`) **или** общая папка (тип `localdir`: NFS, CEPH, SMB или просто локальный диск) | `vibe registry publish`, `vibe registry mirror` |
| Релизы самого vibe | Папка-«дроп» релизов, по подпапке на версию: `<drop>/v<версия>/` | Администратор кладёт файлы руками; `vibe self releases`, `vibe self update` |
| Большие бинарные файлы пакетов (payload для `vibe install -g`) | Папка-«дроп» пакетов | Администратор кладёт файлы руками; `vibe artifacts where` |
| Все настройки площадки | Один файл профиля площадки, например `corp.toml` | `vibe self apply corp.toml` |
| Прокси | Секция `[proxy]` профиля; по умолчанию прокси нет | `vibe dl --test-proxy`, `vibe self doctor --network` |

Разработчик выполняет одну команду — `vibe self apply <профиль>` — и получает:
список реестров, запрет обращаться к `github`/`gitverse`, источник релизов,
пути к дропам, прокси и PATH. В конце `apply` печатает, куда класть релизы и
payload-файлы.

Профиль хранится в `~/.vibe/site-profile.toml` и стоит **ниже** собственных
файлов пользователя (`~/.vibe/config.toml`, `~/.vibe/registry.toml`, проектные
`vibe.toml` и `.vibe/registry.local.toml`), но **выше** встроенных умолчаний.
Если профиль объявляет хотя бы один `[[registry]]`, встроенные реестры
GitHub/GitVerse (`vibespecs`, `vibespecs-gitverse`) перестают действовать.

---

## 2. Хранилище пакетов

### 2.1. Монорепозиторий на SSH Git-сервере

Нужен один пустой bare-репозиторий. API сервера, организации и HTTPS не нужны —
только `git` по SSH. Создайте его средствами сервера или на самом сервере:

```sh
git init --bare /srv/git/vibe/registry.git
```

Раскладка внутри (её ведёт vibe): `<group>/<name>/v<version>/` на ветке `main`
(другая ветка — ключ `ref`), плюс служебный каталог `_index/` (индекс для
поиска, журнал публикаций, записи о зеркалировании). Пустой репозиторий
подходит: первая публикация или первое зеркалирование создаёт ветку.

Доступ — обычной SSH-конфигурацией пользователя (`~/.ssh/config`, ключи).
Пример записи в профиле:

```toml
[[registry]]
name = "corp"
type = "monorepo"      # регистр не важен: MonoRepo, MONOREPO
url  = "git@git.example.corp:vibe/registry.git"   # или ssh://git@git.example.corp/vibe/registry.git
ref  = "main"
```

`type = "monorepo"` никогда не угадывается по адресу — его нужно указать явно.
Для `monorepo` нельзя `auth = "token-env"`.

> На Linux/macOS OpenSSH и vibe отказываются работать с SSH-конфигом, ключом
> или `known_hosts`, если каталог с ними (или домашний каталог) доступен на
> запись группе/всем. Проверка в песочнице упала именно на этом, пока права не
> были исправлены (`chmod 700 ~ ~/.ssh`, `chmod 600 ~/.ssh/config`).

### 2.2. Общая папка (без Git)

Любой каталог, который все видят по одному и тому же пути. vibe считает его
содержимое истиной «как есть» и не занимается блокировками — конфликты
решаются административно.

```toml
[[registry]]
name = "shared"
type = "localdir"
url  = "file:///mnt/vibe/registry"
```

Раскладка та же: `<group>/<name>/v<version>/`, плюс `_index/`, который vibe
переписывает при публикации.

### 2.3. Наполнение из публичного реестра: `vibe registry mirror`

Зеркалирование выполняется на машине с доступом в интернет (или там, где
доступен исходный реестр). Источник — имя настроенного реестра, в том числе
встроенного `vibespecs` (GitHub) или `vibespecs-gitverse`:

```sh
# первый раз: сохранить всё
vibe registry mirror vibespecs --into ssh://git@git.example.corp/vibe/registry.git
```

**Не выполнялось здесь** против GitHub (нет сети). Та же команда выполнена
против локального реестра «по репозиторию на пакет» (тип `git`), в том числе
в пустой bare-репозиторий. Особенности источника без индекса:

- для GitHub без индекса vibe перечисляет репозитории организации через API;
- GitVerse анонимно отвечает 401 — нужен токен на чтение, иначе укажите пакеты
  явно через `--package <group/name>`;
- для источника типа `git` `--package` обязателен;
- у реестра с локальным адресом (`file://…`, абсолютный путь) индекс по
  умолчанию не ищется; `index_url` нужен, только если индекс где-то есть.

Куда можно зеркалировать (`--into`):

| `--into` | Что происходит |
|---|---|
| имя реестра типа `monorepo` или `localdir` (`--into corp`, `--into shared`) | копирование в него (для monorepo — коммит и push) |
| Git-адрес (`ssh://…`, `git@host:path`, `file://…`) или путь к bare-репозиторию | временный клон, один коммит, push (при гонке — один повтор) |
| путь к рабочей копии Git | коммит в неё **без push** (грязная рабочая копия отклоняется) — удобно, если перенос в закрытую сеть идёт вручную |

Последующие запуски инкрементальны: копируется только новое; повтор, когда
нового нет, ничего не коммитит (проверено). Полезные ключи:

```sh
vibe registry mirror upstream --into corp --package org.corp/mirrored --dry-run        # показать план, ничего не писать
vibe registry mirror upstream --into corp --package org.corp/mirrored                  # копирование
vibe registry mirror upstream --into /path/to/wc --package org.corp/mirrored --keep-history  # плюс git bundle истории
```

- `--keep-history` — дополнительно сохранить `git bundle` всех тегов и веток
  каждого исходного репозитория в `_index/mirrors/bundles/<source>/` (до 100 МиБ
  каждый), чтобы потом можно было восстановить точные коммиты и теги.
- `--update-snapshots` — обновить snapshot-версии, изменившиеся у источника.
- `--group <G>` — только пакеты группы; `--package` — только указанные.
- Замороженная версия с изменившимся содержимым — тревога: прогон завершается
  с ошибкой и ничего не пишет.

### 2.4. Публикация внутренних пакетов

Публикация выполняется из каталога проекта (нужен `vibe.toml`; `vibe init`
создаёт его). Целевой реестр — по имени:

```sh
vibe init --path ~/publisher
cd ~/publisher
vibe registry publish /path/to/package --registry corp      # в монорепозиторий (git push, без API)
vibe registry publish /path/to/package --registry shared    # в общую папку
vibe registry publish /path/to/package --registry corp --dry-run
```

Версии неизменяемы: повторная публикация того же содержимого — «уже содержит
идентичное содержимое, ничего не записано» (или «…; only the registry index
was refreshed», если при этом обновлён устаревший `_index/`); другое
содержимое той же версии отклоняется, пока не указан `--replace`.

Реестр можно добавить и командой (вместо правки файлов):

```sh
vibe registry add corp ssh://git@git.example.corp/vibe/registry.git --type monorepo --scope user
vibe registry add shared file:///mnt/vibe/registry --type localdir --scope user
vibe registry list            # все слои; --scope site — только профиль площадки
vibe registry test            # доступность и состояние индекса
```

`--scope`: `project` (`vibe.toml`, по умолчанию), `local`
(`.vibe/registry.local.toml`, не коммитится), `user` (`~/.vibe/registry.toml`).

---

## 3. Релизы vibe

### 3.1. Какие файлы скачать со страницы релиза GitHub

Для версии `V` и платформы `T`:

| Файл | Нужен |
|---|---|
| `DISTRIBUTIONS.json` | всегда (один на релиз) |
| `vibevm-V-T.zip` | для каждой нужной платформы |
| `TOOLS.json` | если используется px |
| `px-0.11.0-T.tar.gz` / `px-0.11.0-T.zip` и `px-0.11.0-T.tool.json` | если используется px (`[proxy] mode = "px"`) |
| `vibevm-V-T.fragment.json`, `vibe-bootstrap-T[.exe]` | не обязательны; можно положить всю страницу релиза |

Платформы `T`: `x86_64-pc-windows-msvc`, `x86_64-unknown-linux-gnu`,
`x86_64-unknown-linux-musl`, `x86_64-apple-darwin`, `aarch64-apple-darwin`.

Точное имя архива px для конкретной машины печатает `vibe self px status`
(на Linux x86_64: `px-0.11.0-x86_64-unknown-linux-gnu.tar.gz`; имена для
других платформ здесь не проверялись — смотрите вывод на целевой машине).

### 3.2. Раскладка дропа релизов

```
/mnt/vibe/releases/
  v1.0.12/
    DISTRIBUTIONS.json
    vibevm-1.0.12-x86_64-unknown-linux-gnu.zip
    vibevm-1.0.12-x86_64-pc-windows-msvc.zip
  v1.0.13/
    ...
```

Допускается и голый `vibevm-<V>-<T>.zip` в корне дропа — тогда он
проверяется только по встроенному в архив манифесту (vibe предупреждает об
этом).

Профиль:

```toml
[artifacts]
vibe = "/mnt/vibe/releases"

[self]
source = "localdir"     # github | gitverse | nexus | localdir; путь по умолчанию — [artifacts] vibe
```

### 3.3. Проверка и обновление

```sh
vibe self source show     # откуда берутся релизы и почему
vibe self releases        # что лежит в дропе, новые сверху (--all — все, а не 10)
vibe self update          # перейти на самую новую версию из дропа
vibe self rollback        # вернуться на предыдущую
vibe artifacts where      # куда и с какими именами класть файлы релиза
```

Пример вывода (проверено):

```
$ vibe self releases
releases at localdir /mnt/vibe/releases ([self] in ~/.vibe/site-profile.toml)
  1.0.13
  1.0.12
```

Разработчик может переопределить источник у себя: `vibe self source set localdir --path <dir>`
(записывает `[self]` в `~/.vibe/config.toml`, это выше профиля);
`vibe self source reset` убирает эту запись и показывает источник, который
действует после этого (например, `[self]` из профиля площадки).

---

## 4. Payload-файлы пакетов (`install -g`)

Приложения (`vibe install -g`) часто скачивают большие файлы по URL с
SHA-256. В закрытой сети эти файлы кладутся в дроп пакетов; vibe ищет их там
**до** обращения к исходному URL и всегда проверяет SHA-256.

```toml
[artifacts]
packages = "/mnt/vibe/payloads"
```

Порядок поиска файла `F` (последний сегмент URL) пакета `G/N@V` с
дайджестом `D`:

1. `<drop>/G/N/V/F`
2. `<drop>/G/N/F`
3. `<drop>/D/F`
4. `<drop>/F`

Файл с неверным SHA-256 отклоняется («does not match its declared sha256») и
не сохраняется. Пример пути: `/mnt/vibe/payloads/org.example/tool/1.2.0/tool-1.2.0-x86_64-unknown-linux-gnu.tar.gz`.

Узнать, что именно и куда класть для конкретного пакета:

```
$ vibe artifacts where org.corp/tool
...
org.corp/tool [src] payload `corp-tool.txt` sha256 651b66b2…8255: missing
    put it at `/mnt/vibe/payloads/org.corp/tool/1.0.0/corp-tool.txt`
```

После того как файл положен, статус меняется на `present`, и
`vibe install -g org.corp/tool` ставит приложение без обращения к сети
(проверено).

Авторы внутренних пакетов могут сразу ссылаться на дроп вместо URL: во входе
`sources` указать `drop = "packages"`, `file = "<имя>"` и `sha256`.

---

## 5. Прокси

**По умолчанию прокси нет** — пока в профиле или в `~/.vibe/config.toml` нет
секции `[proxy]`, vibe ходит напрямую.

### 5.1. Явный прокси

```toml
[proxy]
https    = "http://proxy.example.corp:8080"
http     = "http://proxy.example.corp:8080"
no_proxy = "localhost,127.0.0.1,.example.corp"
```

Ключа `url` в `[proxy]` нет (`unknown field url, expected one of mode, https,
http, no_proxy, pac, upstream, px_args, px_path, px_port, ca_file,
revocation`). `no_proxy` применяется и к `vibe dl --proxy <url>`.

### 5.2. px (прокси с корпоративной аутентификацией)

Если прокси требует Kerberos/NTLM (ответ 407), vibe запускает px — локальный
прокси, который входит с учётными данными сеанса.

```toml
[proxy]
mode = "px"
pac  = "https://pac.example.corp/proxy.pac"   # или абсолютный путь, или file:///…, или "system" (настройки ОС)
# upstream = "proxy.example.corp:8080"        # вместо pac — конкретный вышестоящий прокси
# px_port  = 3129                              # фиксированный порт на loopback (без подбора)
# px_path  = "/opt/px/px"                      # свой исполняемый px
```

#### `pac = "system"` — настройки прокси операционной системы, прочитанные самим vibe

На корпоративном ноутбуке адрес PAC-файла обычно лежит внутри двоичного
значения `DefaultConnectionSettings` (ветка `Connections` в параметрах
обозревателя), а отдельного значения `AutoConfigURL` нет. px, запущенный без
`--pac`, в такой конфигурации не находит ничего, и запросы не проходят.
Поэтому `pac = "system"` означает: **vibe сам читает настройки прокси этой
операционной системы и запускает px с явным `--pac=<url>`** (или с
`--proxy=<host:port>`, если настроен конкретный прокси, а не PAC-файл).
Настройки читаются при каждом запуске px — их могли изменить. Это работает на
любой системе: Windows, macOS, Linux и другие Unix — у каждой свой порядок
поиска, и в отчёте всегда назван источник, который ответил.

Порядок поиска **в Windows** (побеждает первое найденное):

1. Групповая политика: значение `AutoConfigURL` в
   `HKCU\Software\Policies\Microsoft\Windows\CurrentVersion\Internet Settings`,
   затем в `HKLM` по тому же пути.
2. Настройки самого пользователя — через системный вызов
   `WinHttpGetIEProxyConfigForCurrentUser` (он видит и значение
   `DefaultConnectionSettings`). Если вызов не удался или ничего не вернул:
   значение `AutoConfigURL`, а затем разбор `DefaultConnectionSettings` в
   `HKCU\Software\Microsoft\Windows\CurrentVersion\Internet Settings`.
3. Автоопределение (WPAD) — `WinHttpDetectAutoProxyConfigUrl` по DHCP и DNS,
   если автоопределение включено, а адрес PAC не задан.
4. Конкретный прокси-сервер из настроек — он становится `upstream` для px,
   а его список исключений — `--noproxy` (только если `no_proxy` в профиле
   не задан). Собственные `pac`, `upstream` и `no_proxy` всегда важнее
   того, что сказала Windows.
5. Ничего не найдено — px **не** запускается, а в сообщении перечислены все
   прочитанные настройки и что делать: задать `pac = "<адрес PAC>"`,
   абсолютный путь к PAC-файлу или `upstream = "host:port"`.

Порядок поиска **в macOS** — настройки прокси текущего сетевого сервиса,
как их отдаёт `SCDynamicStoreCopyProxies` (те же данные печатает
`scutil --proxy`): `ProxyAutoConfigEnable` + `ProxyAutoConfigURLString` →
адрес PAC; `ProxyAutoDiscoveryEnable` → автоопределение (WPAD, ниже);
`HTTPSEnable` + `HTTPSProxy` + `HTTPSPort` (иначе тройка `HTTP*`) →
`upstream` для px; `ExceptionsList` → `--noproxy`. Затем — переменные
окружения (ниже), затем отказ. Выключенный `…Enable` игнорируется ровно так
же, как его игнорирует сама macOS.

Порядок поиска **в Linux и других Unix** (побеждает первое найденное):

1. GNOME / GTK — `gsettings list-recursively org.gnome.system.proxy`:
   `mode 'auto'` → `autoconfig-url` (пусто → WPAD), `mode 'manual'` → `host`
   и `port` схемы `https` (иначе `http`), `ignore-hosts` → `--noproxy`.
2. KDE — секция `[Proxy Settings]` в `$XDG_CONFIG_HOME/kioslaverc`:
   `ProxyType` 2 → `Proxy Config Script`, 3 → WPAD, 1 → `httpsProxy`
   (иначе `httpProxy`), 4 → отдать решение переменным окружения;
   `NoProxyFor` → `--noproxy`.
3. Переменные окружения — `https_proxy`/`HTTPS_PROXY`, затем
   `http_proxy`/`HTTP_PROXY`, затем `all_proxy`/`ALL_PROXY`; значение
   сводится к `host:port`, поэтому логин и пароль из него отбрасываются и
   никуда не печатаются; `no_proxy` → `--noproxy`. Адрес на loopback
   пропускается — это собственный px самого vibe.
4. Ничего не найдено — px **не** запускается, а в сообщении перечислены все
   просмотренные источники и что делать.

**Автоопределение (WPAD)** — когда настройки просят «определить
автоматически» и адреса не называют: vibe берёт доменные суффиксы поиска
DNS хоста (`search` / `domain` в `/etc/resolv.conf`), строит
`http://wpad.<домен>/wpad.dat`, поднимаясь по меткам, и запрашивает их по
очереди напрямую с коротким таймаутом. Побеждает первый ответ `200`, в теле
которого есть `FindProxyForURL`. Подъём **никогда не доходит до публичного
суффикса** — остаётся не меньше двух меток, то есть `wpad.example.com`
спрашивается, а `wpad.com` нет. Не более шести адресов за раз.

Что получилось, видно в отчётах: `vibe dl --test-proxy` (строка `pac`),
`vibe self doctor --network` и `vibe self apply --show` печатают
`system → https://pac.example.corp/proxy.pac (from user Internet Options)` —
адрес и источник, который ответил: `from GNOME settings`,
`from KDE settings`, `from macOS network settings`,
`from environment https_proxy`, `from WPAD discovery`. Написание `windows`
принимается как синоним `system`.

#### `revocation` — проверка отзыва сертификатов (Windows)

Корпоративный прокси, вскрывающий TLS, подписывает сертификаты своим CA, а
точки распространения CRL и OCSP этого CA обычно недоступны из той же сети.
Windows schannel тогда не может завершить проверку отзыва и обрывает
рукопожатие — та же ошибка, из-за которой `curl` требует `--ssl-no-revoke`.

```toml
[proxy]
revocation = "best-effort"   # "check" (по умолчанию) | "best-effort" | "off"
```

- `check` — поведение по умолчанию и точно такое же, как до появления ключа:
  vibe ничего не добавляет дочернему `git`.
- `best-effort` — отзыв проверяется, но сертификат принимается, если
  состояние отзыва получить не удалось; отозванный сертификат всё равно
  отвергается. Добавляет `-c http.schannelCheckRevoke=best-effort`.
- `off` — проверка отзыва не выполняется вовсе
  (`-c http.schannelCheckRevoke=false`).

Разово: `VIBEVM_PROXY_REVOCATION=best-effort vibe …` (переменная важнее
файла пользователя, он важнее профиля площадки).

Цепочка сертификатов проверяется всегда — этот ключ её не отключает.
Ключ действует только на дочерний `git`, работающий через schannel: git
применяет `http.schannelCheckRevoke` лишь при `http.sslBackend = schannel`,
и vibe этот выбор не меняет. Значение `best-effort` понимает Git for
Windows 2.26 и новее (там оно и по умолчанию); обычный git считает ключ
логическим, поэтому vibe передаёт слово только такому git, а иначе
переходит на `off` с предупреждением. Собственные загрузки vibe используют
rustls и онлайн-проверку отзыва не делают, а px для HTTPS только
туннелирует (`CONNECT`) — ключ им не нужен и до них не доходит.

px ставится из дропа релизов: положите `px-0.11.0-<T>.*` (см. §3.1) и
выполните `vibe self px install`; `vibe self px status` показывает, установлен
ли он. Порядок поиска px: `px_path` → хранилище vibe → `tools/px/` рядом с
vibe → `PATH`.

**Подойдёт любой px (решение владельца 2026-11-09).** vibe нигде не сверяет px
с SHA-256 — ни скачанный из источника релизов или дропа, ни найденный рядом с
`vibe`, ни указанный вручную. Файл закрепления (`xtask/dist/px.toml`) говорит
только, какой px брать **по умолчанию**. `vibe self px install --from
<архив|каталог|исполняемый файл>` берёт `.zip`, `.tar.gz`, распакованный
каталог или сам `px`/`px.exe` — любой версии и для любой платформы, включая
те, для которых готового ассета нет (`x86_64-apple-darwin`); `--force`
заменяет. Всё лежит в одном хранилище
`<install root>/opt/vibevm/tools/px/<версия>/` (`custom`, если версия в имени
не указана), рядом — `PX.toml`: откуда прочитан файл, какой SHA-256 измерен,
когда и что отвечает `px --version`. Это **справка, а не проверка**: ни
`status`, ни `doctor` не ставят на px метку «проверен». Отказ остаётся только
для того, что px вообще не является: архив не распаковывается, выходит за свой
каталог или не содержит программы `px`.

`vibe self apply` сам доустанавливает px, если применяемый профиль выбирает
`[proxy] mode = "px"`: архив рядом с `vibe` (или в `tools/px/` там же)
попадает в хранилище того install root, который выбирает профиль — в том числе
при повторном `vibe self apply`, так что достаточно положить архив и запустить
команду ещё раз. Если px так и нет, в итоге команды стоит `px: NOT INSTALLED` и
предупреждение с именем ассета, каталогом хранилища и
`vibe self px install --from <каталог-или-zip>`; код выхода остаётся 0 —
профиль применён. `vibe self px status` при `mode = "px"` и полном отсутствии
px (хранилище, `px_path`, рядом с vibe, `PATH`) печатает отчёт и возвращает
ненулевой код, как `vibe self doctor` с неустранённой проблемой.

Не выполнялось здесь: реальный запуск px и вход через PAC (нет настоящего
px-архива и нет корпоративного прокси). Выполнено: разбор всех вариантов
`pac`/`upstream`/`px_port` профилем, установка px из локальных файлов и
диагностика отсутствующего px.

### 5.2a. px для других инструментов: `vibe tool px`

Прокси с корпоративной аутентификацией нужен не только vibe: `pip`, `npm`,
`cargo`, `curl` тоже должны ходить через него, и каждый настраивается
по-своему. px — единственное на машине, что умеет здесь входить, поэтому
vibe даёт его взаймы:

```sh
vibe tool px exec -- pip install requests
vibe tool px exec -- npm install
vibe tool px exec -- curl -sSf https://repo.example.corp/health
```

`exec` поднимает **тот же** px, что и обычные операции vibe (те же `pac`
— включая разрешение `system` на Windows, §5.2 — `upstream`, `px_port`,
`px_args`, `px_path`, тот же поиск спутника и тот же срок жизни), затем
запускает команду, найденную на `PATH` (на Windows — с учётом `PATHEXT`,
поэтому находится `npm.cmd`), и останавливает px, когда команда
завершилась. **Код возврата команды становится кодом возврата vibe.**

Дочернему процессу выставляются `HTTPS_PROXY`, `https_proxy`,
`HTTP_PROXY`, `http_proxy` = `http://127.0.0.1:<порт>` и
`NO_PROXY`/`no_proxy` = loopback плюс `no_proxy` из профиля.

```sh
# Инструмент читает переменную со своим именем — назовите её:
vibe tool px exec --proxy-env npm_config_proxy -- npm view express
vibe tool px exec --proxy-env PIP_PROXY -- python -m pip download six --no-deps -d .

# Любая другая переменная — буквально (применяется последней):
vibe tool px exec --env NODE_USE_SYSTEM_CA=1 -- node app.js

# Фиксированный порт на loopback, если политика разрешает только один:
vibe tool px exec --port 3129 -- pip install requests
```

Если прокси в `[proxy]` не настроен вовсе, `exec` отказывается (это и есть
случай, для которого его вызывают) — `--allow-direct` запускает команду
без прокси. При `mode = "explicit"` px не поднимается: команде отдаётся
настроенный прокси как есть.

Для инструмента, который vibe запустить не может (редактор, долгоживущая
оболочка), есть `vibe tool px serve`: px работает на переднем плане до
Ctrl-C и печатает адрес и разрешённый маршрут.

```sh
vibe tool px serve
#   px is listening on http://127.0.0.1:52431
#     upstream  proxy.example.corp:8080
#     no_proxy  localhost,127.0.0.1,::1,.example.corp
```

`vibe tool px status` — тот же отчёт, что `vibe self px status`.

Готовые обёртки семейства **pxworld** избавляют от набора команды руками:
`org.vibevm.pxworld/pxnode` даёт `pxnode`/`pxnpm`/`pxnpx`, а
`org.vibevm.pxworld/pxpip` — `pxpip`; каждая запускает соответствующий
инструмент с `PATH` через `vibe tool px exec`.

### 5.3. Корпоративный корневой сертификат

vibe доверяет системному хранилищу сертификатов ОС. Если корпоративный CA
туда не установлен:

```toml
[proxy]
ca_file = "/mnt/vibe/certs/corp-root-ca.pem"
```

Проверено: без `ca_file` загрузка через перехватывающий TLS прокси даёт
`dl_untrusted_certificate` (код 11), с `ca_file` — `TLS 1.3 · trusted by ca_file`.

### 5.4. Временное отключение

```sh
VIBEVM_PROXY_MODE=none vibe …        # также explicit | px; VIBEVM_PROXY_PAC=<pac> — свой PAC
```

Переменные окружения сильнее файлов пользователя и профиля
(`vibe self apply --show` показывает `proxy mode: none (from VIBEVM_PROXY_MODE)`).
Постоянно — `mode = "none"` в профиле или в `~/.vibe/config.toml`.

### 5.5. Диагностика

```sh
vibe dl --test-proxy                       # проверить настроенный прокси против https://gmail.com
vibe dl --test-proxy https://repo.example.corp/
vibe dl --test-proxy --no-px               # проверить upstream/явный прокси без px
vibe dl https://repo.example.corp/file.zip ./ --sha256 <hex>
vibe self doctor --network                 # по умолчанию проверяет https://gmail.com (как vibe dl --test-proxy)
vibe self doctor --network https://repo.example.corp/
```

`vibe dl --test-proxy` без настроенного прокси отвечает `no proxy is configured`
(код 2). Типичные результаты (проверено на loopback-стенде, кроме таймаута):

| Вывод | Код `dl` | Значение и что делать |
|---|---|---|
| `proxy auth … 407 Proxy Authentication Required` (`dl_proxy_auth`) | 9 (21 для `--test-proxy`) | Прокси требует входа. Включите `mode = "px"` с `pac` или `upstream` |
| `TLS trust … unable to get local issuer certificate … chain ends at CN=…` (`dl_untrusted_certificate`) | 11 (22) | TLS перехватывается корпоративным CA. Установите CA в ОС или задайте `ca_file` |
| `proxy reachable … Connection refused` | 8 (21) | Неверный адрес/порт прокси или прокси не запущен |
| `dns … does not resolve` (`dl_dns`) | 6 | Имя не разрешается; если наружу резолвит только прокси — настройте `[proxy]`, проверьте `no_proxy` |
| таймаут соединения | 7 | Прокси или узел недоступен по сети (не выполнялось здесь: в песочнице без сети соединение отвергается сразу) |
| `px present … px is not installed` | 5 (21) | `vibe self px install` (§5.2) или `VIBEVM_PROXY_MODE=none` |
| `pac … found no proxy in …` | 5 (21) | `pac = "system"`, но прокси не настроен ни в одном источнике этой ОС. В сообщении перечислено, что прочитано; задайте `pac`/`upstream` (§5.2) |
| `CRYPT_E_NO_REVOCATION_CHECK (0x80092012)` / `CRYPT_E_REVOCATION_OFFLINE (0x80092013)` | 13 (22) | Не удаётся проверить отзыв сертификата: CRL/OCSP вскрывающего CA недостижимы. `revocation = "best-effort"` (§5.2) |

Прочие коды `vibe dl`: 3 — запрет `--offline`, 4 — политика источников,
10 — CONNECT отклонён, 12 — имя в сертификате не совпадает, 13 — другая
ошибка TLS, 14/15 — HTTP 4xx/5xx, 16 — несовпадение SHA-256, 17 — файл
назначения существует (`--force`).

---

## 6. Профиль площадки

### 6.1. Полный пример `corp.toml`

Проверен `vibe self apply corp.toml --dry-run` (все секции приняты):

```toml
# corp.toml — профиль площадки vibe для закрытой сети Example Corp.
schema = 1                 # обязательно
name = "example-corp"      # для отчётов

# Реестры пакетов, в порядке приоритета. Объявив их, профиль отключает
# встроенные vibespecs (GitHub) и vibespecs-gitverse.
[[registry]]
name = "corp"
type = "monorepo"          # github | gitverse | git | monorepo | localdir
url = "git@git.example.corp:vibe/registry.git"
ref = "main"
# прочие ключи: naming, auth, token_env, enabled, index_url

[[registry]]
name = "shared"
type = "localdir"
url = "file:///mnt/vibe/registry"

# Запасной адрес для реестра (реплика); ключи: of, url, priority.
[[mirror]]
of = "corp"
url = "ssh://git@git-replica.example.corp/vibe/registry.git"
priority = 10

# [[override]]  — подмена источника одного пакета: pkgref, source_url, ref, reason
# [[remap]]     — перенаправление загрузок (PROP-066): prefix, url | path | deny, sha256,
#                 package, registry, scope, reason, from. Пример:
# [[remap]]
# prefix = "https://downloads.example.org/"
# path = "/mnt/vibe/payloads/downloads"
# reason = "внутреннее зеркало загрузок"

# Никогда не обращаться к GitHub и GitVerse. prefer_hosting — порядок предпочтения.
[source_policy]
exclude_hosting = ["github", "gitverse"]

# Дропы: релизы vibe и payload-файлы пакетов. Пути только абсолютные.
[artifacts]
vibe = "/mnt/vibe/releases"
packages = "/mnt/vibe/payloads"

# Источник релизов и корень установки (<install_root>/opt/bin — шимы,
# <install_root>/opt/vibevm — версии и состояние). Ключи: source, url, api, path, install_root.
[self]
source = "localdir"
install_root = "/opt/work"

# Прокси (§5). Без этой секции прокси нет.
[proxy]
mode = "px"
pac = "https://pac.example.corp/proxy.pac"
# pac = "system"            # настройки прокси этой ОС, читает сам vibe — §5.2
no_proxy = "localhost,127.0.0.1,.example.corp"
ca_file = "/mnt/vibe/certs/corp-root-ca.pem"
# Проверка отзыва сертификатов для дочернего git на schannel (Windows), §5.2:
# "check" (по умолчанию) | "best-effort" | "off".
# revocation = "best-effort"

# true — запретить сеть вовсе (как --offline).
[net]
offline = false

# PATH: "ask" (спросить; по умолчанию «да» на Windows, «нет» на других ОС),
# "always" (писать без вопроса), "never".
[path]
fix = "always"
```

Неизвестные ключи отклоняются с указанием файла, строки и списка допустимых
ключей — опечатка не пройдёт молча.

Вариант путей для Windows: `vibe = '\\fileserver\vibe\releases'`,
`packages = '\\fileserver\vibe\payloads'`, `install_root = "C:/Work"`, в TOML —
в одинарных кавычках. На Linux и macOS `--dry-run` такого профиля проходит:
UNC- и `C:`-пути принимаются с предупреждением, что прочитать их может только
Windows. `pac = "system"` предупреждения больше не вызывает — он читается на
любой системе (§5.2).

### 6.2. Применение и проверка

```sh
vibe self apply corp.toml --dry-run     # проверить, ничего не записывая
vibe self apply corp.toml --diff        # то же + построчный diff файлов
vibe self apply corp.toml               # применить (повтор — без изменений)
vibe self apply corp.toml --fix-path    # добавить каталог шимов в PATH без вопроса
vibe self apply corp.toml --no-fix-path # никогда не трогать PATH
vibe self apply --show                  # действующие настройки: профиль, реестры по слоям, релизы, дропы, прокси
vibe self apply --remove                # удалить то, что поставил apply (свои файлы пользователя не трогаются)
```

Профиль можно указать и URL-ом (`https://…`, `file://…`); по `https` он
загружается через прокси vibe и недоступен под `--offline`.

`apply` записывает только `~/.vibe/site-profile.toml` (копия профиля) и
`~/.vibe/site-profile.receipt.toml`; `--remove` удаляет ровно их. Если `apply`
запущен из скачанного вручную `vibe`, он импортирует этот файл как активную
версию в `<install_root>/opt/vibevm` и создаёт шимы в `<install_root>/opt/bin`.
`install_root` становится постоянным (`VIBEVM_INSTALL_ROOT` в профиле shell на
Unix, в реестре пользователя на Windows) при том же согласии, что и PATH.

В конце вывод `apply` содержит готовые инструкции:

```
put vibe releases into `/mnt/vibe/releases` (localdir), one directory per release, ...
put package payloads into `/mnt/vibe/payloads`; an input `F` (sha256 `D`) of package `G/N@V` is looked for at: ...
next: in each project resolved before this profile, move the locked sources onto the site's registries: `vibe update --sources-only`
```

---

## 7. Новая машина Windows без интернета и с запретом PowerShell-скриптов

**Не выполнялось здесь** (проверка шла на Linux); флаги и пути сверены с
`vibe self apply --help` и описанием `[self] install_root`.

1. Возьмите `vibevm-<версия>-x86_64-pc-windows-msvc.zip` из известного места:
   с общей папки дропа (`\\fileserver\vibe\releases\v<версия>\`) или браузером
   со страницы релиза на GitHub.
2. Снимите блокировку «загружено из интернета» (Mark-of-the-Web): правый клик
   по zip → «Свойства» → флажок «Разблокировать» → «ОК». Делайте это **до**
   распаковки, иначе метку унаследуют все файлы.
3. Распакуйте в разрешённый каталог `C:\Work\opt\bin` — так `vibe.exe`
   окажется там же, где vibe держит шимы при `install_root = "C:/Work"`.
   При AppLocker исполняемые файлы запускаются только из разрешённых каталогов
   (например, `C:\Work`); поэтому `install_root` в профиле должен указывать
   туда же — иначе установленные версии окажутся в запрещённом
   `%USERPROFILE%\.vibe`.
4. В `cmd.exe`, полным путём:

   ```bat
   C:\Work\opt\bin\vibe.exe self apply \\fileserver\vibe\corp.toml --fix-path
   ```

   `--fix-path` прописывает каталог шимов в пользовательский PATH через API
   реестра Windows (без PowerShell и без дочерних процессов).
5. Откройте **новое** окно `cmd` и проверьте:

   ```bat
   vibe self doctor
   ```

**Если PATH не обновился** и кнопка «Переменные среды» в «Свойствах системы»
недоступна: Win+R →

```
rundll32.exe sysdm.cpl,EditEnvironmentVariables
```

и добавьте `C:\Work\opt\bin` в переменную `Path` пользователя вручную.

**Когда сеть и скрипты разрешены**, тот же результат даёт установщик:
`install.ps1 -Site \\fileserver\vibe\corp.toml` (Windows) или
`bash install.sh --site /mnt/vibe/corp.toml` (Linux/macOS; также
`$VIBEVM_SITE_PROFILE`). Профиль проверяется до установки.

---

## 8. Перевод существующих проектов на внутренние источники

Проекты, у которых `vibe.lock` уже указывает на старые реестры, переводятся
без смены версий и содержимого:

```sh
cd <проект>
vibe update --sources-only
```

Для каждого пакета из lock-файла источник выбирается заново по текущей
конфигурации реестров; версия и content hash остаются прежними (проверено:
переход с `localdir` на `monorepo`). Если в новом источнике нет закреплённой
версии, обновление падает — другая версия не подставляется. Сначала
зеркалируйте нужные пакеты (§2.3).

---

## 9. Неполадки

| Симптом | Причина | Решение |
|---|---|---|
| `407 Proxy Authentication Required` (`dl_proxy_auth`) | Прокси требует корпоративного входа | `[proxy] mode = "px"` + `pac`/`upstream`; проверить `vibe dl --test-proxy` |
| `unable to get local issuer certificate … chain ends at CN=…` (`dl_untrusted_certificate`) | Корпоративный CA перехватывает TLS и не установлен в ОС | Установить CA в системное хранилище или `[proxy] ca_file = "<pem>"` |
| `px is not installed: looked for …` | Нет px для `mode = "px"` | Положить `px-0.11.0-<T>.*` в дроп релизов, `vibe self px install` (или `--from <файл>`), проверить `vibe self px status`; временно `VIBEVM_PROXY_MODE=none` |
| `dependency resolution is unsatisfiable: No candidates were found …` | Пакета (или версии) нет ни в одном реестре | Опубликовать или зеркалировать пакет (§2.3–2.4); проверить `vibe registry list`, `vibe search <слово>` |
| `the [artifacts] packages drop holds no <F> (sha256 …); searched …` | В дропе нет payload-файла | `vibe artifacts where <пакет>` — положить файл по пути из `put it at …` |
| `does not match its declared sha256` | В дропе лежит не тот файл | Скачать заново нужную версию; сверить SHA-256 |
| `native context unsupported: original SSH config ownership or permissions are unsafe` | SSH-конфиг, ключ или их каталоги доступны на запись другим | `chmod 700 ~ ~/.ssh`, `chmod 600 ~/.ssh/config` и ключи |
| `vibe search` не находит свежий пакет | Индекс `_index/` не обновлён (пакет залит в репозиторий мимо vibe) | Опубликовать через `vibe registry publish` — неизменная публикация дописывает индекс (`index: refresh`) |
| `MISS shim dir … (NOT on PATH)` в `vibe self doctor` | PATH не обновлён или открыт старый терминал | Открыть новый терминал; `vibe self apply <профиль> --fix-path` или `vibe self doctor --fix`; на Windows — §7 |
| `vibe self update`: `… releases/latest/download/DISTRIBUTIONS.json: excluded HTTP endpoint https://github.com` | Источник релизов остался GitHub, а он запрещён `exclude_hosting` | `[self] source = "localdir"` + `[artifacts] vibe`; проверить `vibe self source show` |
| `vibe self releases` печатает `none` | Дроп пуст или неверная раскладка | `<drop>/v<версия>/` с `DISTRIBUTIONS.json` и `vibevm-<версия>-<T>.zip` (§3.2); `vibe artifacts where` |
