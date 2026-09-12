# PACKET-P5-O2 — контейнеры сайта: рендерер, nginx выдачи, аналитика, локальная проба (A5.7, A5.9, A5.6 шаги 1–2)

##subagent-quiet-clause

## Слово владельца 2026-09-12: Docker на этой машине не используется

Владелец сказал: Docker не работает и использовать его не нужно. Поэтому
всё ниже, что говорит «собрать образ», «`docker compose up`», «проба в
контейнере», «`docker version`», «`docker compose config`» — **не
выполняется**: ни клиент, ни демон не запускать, ничего не собирать и не
поднимать. Что остаётся:

- **файлы** — `docker/Dockerfile`, `docker/compose.yaml`, `docker/nginx.conf`,
  `docker/site.toml` пишутся как описано (они нужны серверу, где
  контейнеры есть), плюс скрипт стадии сборки образа, который генерирует
  `csp.conf` из `csp.txt`, — написан и проверен **как скрипт** над
  `dist/csp.txt` без образа;
- **проверки — статические**: node-тест web-пакета над текстом
  `nginx.conf` и `compose.yaml` (каждое требование A5.7 — правило в
  файле: `/.vibe-site/` → 404, `q-manifest.json` → 404, immutable-кеш на
  `/assets/`, `/build/`, `/fonts/` и не на `/_astro/`, `charset=utf-8` на
  `.md`/`.xml`/`.txt`/`.json`, относительные редиректы `/doc` → `/doc/`
  и `/en/` → `/`, `include csp.conf`, ни одного `Location: http://`,
  плейсхолдеры имён и порта); генератор `csp.conf` — тест над реальным
  `csp.txt` сборки;
- **живая проба** — только то, что не требует контейнеров: `node
  tools/build.mjs static` над деревьями билдера и `curl` к `vibe doc
  serve` там, где проверяется поведение страниц; тег аналитики A5.9 —
  тест над собранной страницей с website-id из окружения и без него;
- в отчёте прямо: «контейнеры не собирались и не запускались по слову
  владельца; первая проба образов — на сервере вместе с владельцем (A5.6
  п. 3+)». В `DEV-GUIDE.md` раздел «собрать сайт в Docker» пишется как
  инструкция для сервера с пометкой, что на дев-машине без Docker шаги
  статические.

Ты — воркер кампании документации, исполнитель уровня `opus5`. Boot-лейн
репозитория не читать. Твоя инструкция — этот файл и ровно те файлы,
которые он называет. Предусловия: в ветке коммиты P4-O3 (статический
адаптер, `csp.txt`, `resolve.json`), P4-O5 (лендинг, корневые файлы,
`site/src/config.ts`), P5-O1 (`vibe doc build-site`) — проверь `git log
--oneline -140` и прочитай отчёты `WORKER-REPORT-P4-O3.md`,
`WORKER-REPORT-P4-O5.md` (решения 2, 5, 6; X-040), `WORKER-REPORT-P5-O1.md`
(конфигурация, выход, состояние), `WORKER-REPORT-P4-O1.md` §А-2 (порядок
сборок).

## Читать сначала

1. Этот пакет целиком; отчёты выше.
2. `campaigns/docs-2026-09/PLAN.md` — атомы **A5.6** (только шаги 1 и 2;
   шаги 3–7 — центральная сессия с владельцем), **A5.7**, **A5.9**; правила
   **R-09**, **R-24** (ничего на сервере: ни SSH, ни compose соседей, ни
   хостового nginx, ни портов, ни сертификатов — этот пакет целиком
   локальный, в Docker на дев-машине), **R-25** (никаких адресов, IP и
   имён с сервера в артефактах); `DEFERRALS.md` X-035 (CSP из `csp.txt`),
   X-040 (пути ассетов `/assets/`, `/build/` вместо `/_astro/`), X-020
   (`q-manifest.json` — 404).
3. Решения: `vibevm/vibespecs/design/documentation-vision.xml` D-23
   (модель деплоя: два контейнера — выдача и рендерер), D-24 (тег Umami:
   тот же website-id и host-url, значения из конфигурации A5.1; в
   локальном и embedded-режимах тега нет), D-28 (один домен, один сайт).
4. Норма: PROP-057 `site` (SITE-HOSTING-FORM, SITE-TWO-CONTAINERS,
   SITE-CONTAINER-NGINX, SITE-DEPLOY-MODEL, SITE-WHO-COMMITS-WHAT,
   SITE-ANALYTICS, SITE-DNS-TLS-EXIST), `stack` (STACK-NODE-SERVER-ONLY,
   STACK-BUILD-HYGIENE), `seo` (SEO-CHARSET-AND-REDIRECTS).
5. Образец, **только чтение** (R-28): `C:\Users\olegc\git\v\vibevm-org\nginx.conf`
   (77 строк — наследуется: `charset`, `charset_types`, `absolute_redirect
   off`, `port_in_redirect off`, `expires -1` для HTML, immutable-кеш
   ассетов и шрифтов, редирект `/en/`, заголовки безопасности,
   `error_page 404`, `try_files`), `Dockerfile` и деплой-скрипт лендинга
   там же (форма стадий и как контейнер выдаёт статику — только форма,
   без адресов).
6. Код: `vibevm/vibepacks/org.vibevm.doc/web/v0.1.0/` (`tools/build.mjs`,
   `tools/root-files.mjs`, `site/src/config.ts`, `site/src/landing/head.ts`,
   `site/src/seo/**`, `site.example.toml`), `crates/vibe-cli` (`vibe doc
   build-site`), `DEV-GUIDE.md` §2.6 и §8 (получают раздел «собрать сайт в
   Docker» тем же коммитом — R-11).
7. Правила репозитория: Conventional Commits с телом «почему»; атом —
   коммит; **коммитить только явной формой `git commit -m … -- <свои пути>`**;
   **никаких трейлеров и упоминаний моделей**; `git push` не делать; чужие
   незакоммиченные файлы не стейджить; `cargo` на хосте не запускать —
   сборка `vibe` идёт внутри Docker; Docker Desktop на машине есть
   (проверь `docker version`); сеть — образы с Docker Hub, crates.io и
   настроенный npm-реестр внутри сборки образа; секреты и `infra/` не
   читать; ничего не публиковать.

## Цель — три атома, три коммита

**A5.7 nginx выдачи** — `vibevm/vibepacks/org.vibevm.doc/web/v0.1.0/docker/nginx.conf`
на весь домен: наследует `vibevm-org/nginx.conf` по списку п. 5, с
поправками: immutable-кеш на `/assets/`, `/build/` и `/fonts/` (X-040), а
не `/_astro/`; `q-manifest.json` — 404 (STACK-BUILD-HYGIENE); `/.vibe-site/` и всё под ним — 404 (состояние и деревья билдера лежат внутри выхода намеренно — отчёт P5-O1, «Раскладка выхода»); CSP из
`dist/csp.txt` сборки (заголовок собирается стадией сборки образа, не
рукой: `include /etc/nginx/csp.conf`, который генерирует `Dockerfile` из
`csp.txt`; X-044: хэшей 47 на фикстуре и ~150 на руководстве, поэтому заголовок вешается только на HTML-ответы — через `map` по типу ответа, не `add_header` на весь сервер, — а его размер в байтах на странице руководства идёт в отчёт владельцу); `/doc/` и `/doc` — относительные редиректы к каталогу со
слэшем; `.md`, `.xml`, `.txt`, `.json` — `charset=utf-8`; `/en/` и `/en` →
`/` (301, относительный `Location`). Тест по выходу сборки в контейнере
(`docker run` образа выдачи над `dist/`): заголовки на `.txt`, `.xml`,
`.md` содержат `charset=utf-8`; `/en/` даёт относительный `Location`;
`/doc` → `/doc/`; `q-manifest.json` → 404; `/.vibe-site/state.json` → 404; CSP присутствует и совпадает с
`csp.txt` на HTML и отсутствует на `/assets/**`. Коммит:
`feat(web): ship the serving nginx config for the whole domain`.

**A5.9 Тег аналитики** — статический адаптер рендерит тег Umami
(`<script defer src="/u/s.js" data-website-id="…" data-host-url="…">`) на
лендинге и на страницах документации из конфигурации сайта
(`site.toml` `[site.analytics]` → окружение сборки → `site/src/config.ts`,
как у P4-O5); при пустом website-id тега нет; в embedded-сборке и в
локальном читателе тега нет никогда (тест: `grep` по `dist-embedded/` —
пусто; тест P4-O3/A4.15 для сервера — если уже есть). Пути `/u/s.js` и
`/u/e` сайт не обслуживает и в nginx выдачи не описывает (их отдаёт домен).
Коммит: `feat(web): reuse the landing's first-party analytics tag`.

Предусловие: P4-O8 (`feat(web): build one library per source documentation`) в ветке — без него сборка сайта принимает одну исходную документацию, а билдер отдаёт 48 (отчёт P5-O1, А-1); `host_url` и `default_theme` из `site.toml` подключай к оболочке в A5.9 теми же именами, что читает `site/src/config.ts` (X-058).

**A5.6 шаги 1–2, локальная проба** — `docker/Dockerfile` рендерера:
стадия Node (образ с точной версией 24.18.0, `corepack enable`, `pnpm
install --frozen-lockfile`, `pnpm build:static`), стадия Rust (образ с
Rust, `cargo build -p vibe-cli --release` из текущего чекаута — без
Node, PROP-057 STACK-NODE-SERVER-ONLY), стадия рендера (`vibe doc
build-site --config /site.toml --out /out` по конфигурации A5.1, выход
Qwik-сборки рядом), стадия выдачи — стоковый nginx с `docker/nginx.conf`
и `/out`; `docker/compose.yaml` из двух сервисов — выдача и рендерер (имена
сервисов и порт — **плейсхолдеры** `site` и `8080`; настоящие имена и порт
подставит владелец по своему runbook — R-25); `docker/site.toml` —
конфигурация по умолчанию (F-35: `https://vibevm.org`, реестр
`github.com/vibespecs`, хост `github.com/vibevm/vibevm` `main`) без
секретов и website-id (пустой). Локальная проба на дев-машине: `docker
compose -f docker/compose.yaml up --build`, затем `curl -sI
http://127.0.0.1:8080/` и `/doc/` → `200`; `/en/` → относительный
`Location: /`; ни одного `Location: http://`; `/llms.txt` и `/doc/llms.txt`
с `charset=utf-8`; ключ-файл IndexNow (тестовое значение из окружения) →
`200`; тест паритета A4.17 зелёный на этом же выходе; `docker compose
down`. Коммит: `feat(web): build and serve the whole site from two containers`.
`DEV-GUIDE.md` — раздел «собрать сайт в Docker» тем же коммитом.

## Гейты (вывод дословно в отчёт)

`docker version`; `docker compose config` без ошибок; сборка образов до
конца (время сборки каждой стадии — в отчёт, A0.28); все `curl` выше с
заголовками дословно; `node tools/parity.mjs <эталон> <выход>` зелёный;
`node tools/lint-links.mjs` зелёный над выходом; `pnpm floor` web-пакета
зелёный (конфиги в `docker/` вне пола — скажи, если пол их заметил);
размер образа выдачи и рендерера — в отчёт; `docker compose down` и
удаление тестовых образов в конце.

## Что не делать

Ничего на сервере и ничего по SSH (R-24) — даже проверить «а как там
сейчас»; хостовый nginx, порты, сертификаты, compose соседей, VPN — не
упоминать и не трогать; реальные имена сервисов, порты и адреса сервера в
файлы не писать (R-25): только плейсхолдеры с комментарием «подставляет
владелец»; `vibevm-org` не править (R-28); PROP-файлы и вижен не менять;
website-id и ключ IndexNow — из окружения, в репозитории пусто; провайдер
Node в плоскости build (F-19) — не начинать: сборка идёт в Docker.

## Результат

Три коммита (без push) и `campaigns/docs-2026-09/findings/WORKER-REPORT-P5-O2.md`
по форме отчёта P4-O5: что унаследовано из nginx лендинга и что изменено
с причиной, вывод локальной пробы дословно, времена стадий сборки,
размеры образов, что не сделано и почему. Никаких путей вне репозитория,
IP, имён серверов и секретов в отчёте (R-25).
