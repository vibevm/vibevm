# PACKET-P5-O1 — реестровый билдер: конфигурация, лента индекса, рендер уровня 0, полки, адреса (A5.1–A5.5)

##subagent-quiet-clause

Ты — воркер кампании документации, исполнитель уровня `opus5`. Boot-лейн
репозитория не читать. Твоя инструкция — этот файл и ровно те файлы,
которые он называет. Предусловия: в ветке коммиты P2-O8 (`c3983882` —
`vibe doc build`, адресная карта), P2-O10 (снимки поверхности, очередь),
P4-O3 (статический адаптер и SEO: что сборка сайта ждёт от выхода
`vibe doc build`, `resolve.json`, `csp.txt`) и P4-O4 (оболочка, `media`
в манифесте) — проверь `git log --oneline -120` и прочитай отчёты
`WORKER-REPORT-P2-O8.md` §A2.18, `WORKER-REPORT-P2-O7.md` §A2.14 (что
несёт манифест, официальность как вычисление), `WORKER-REPORT-P4-O3.md`,
`WORKER-REPORT-P4-O4.md` (хвост `media`), `WORKER-REPORT-P2-O10.md`
(`vibe doc surface`, файл состояния).

## Читать сначала

1. Этот пакет целиком; отчёты выше.
2. `campaigns/docs-2026-09/PLAN.md` — атомы **A5.1**, **A5.2**, **A5.3**,
   **A5.4**, **A5.5**; гейт фазы 5; правила **R-08**, **R-09**, **R-23**
   (официальность не хранится флагом — вычисляется по рёбрам при каждом
   рендере), **R-24** (ничего на сервере; этот пакет целиком локальный);
   развилки **F-35** (адреса: `https://vibevm.org/doc/`, хост
   `github.com/vibevm/vibevm` ветка `main`, реестр `github.com/vibespecs`
   — OPEN у владельца, бери как значения по умолчанию конфигурации, не
   как константы кода), **F-39**, **F-40**, **F-48**; `DEFERRALS.md`
   X-001 (`cache add` и «bytes untouched»), X-017.
3. Находки: `findings/A0.8-store-and-sources.md` (API store, lock,
   реестра), `findings/A0.17-intree-to-store.md`, `findings/A0.20-host-from-source.md`
   (корень хоста — `[project]`; сайт читает выкладку), `findings/A0.21-index-card-fields.md`
   (обратные запросы на стороне сайта; `[translations]` не хранится; язык
   — `[i18n].canonical`), `findings/A0.28-render-cost.md` (dev-профиль,
   инкрементальность, дебаунс), `findings/A0.19-docs-outside-judging.md`.
4. Решения: `vibevm/vibespecs/design/documentation-vision.xml` D-04, D-07,
   D-16 (опрос хоста: раз в час, дебаунс 5 мин — рекомендация, OPEN у
   владельца, значения по умолчанию конфигурации), D-18, D-19 (полки по
   рёбрам, звёздочка — схождение), D-20, D-24 (аналитика — значения из
   конфигурации, не из оболочки), D-27 (никакой истории публикаций),
   D-28.
5. Норма: PROP-057 `site` целиком (SITE-TWO-SOURCES, SITE-SOURCE-REGISTRY,
   SITE-SOURCE-HOST, SITE-HOST-CHECKOUT, SITE-RENDER-KEY,
   SITE-RENDER-IDEMPOTENT, SITE-HOST-POLL, SITE-HOSTING-FORM,
   SITE-TWO-CONTAINERS, SITE-DEPLOY-MODEL, SITE-WHO-COMMITS-WHAT,
   SITE-CANONICAL-LATEST, SITE-VERSION-SHOWS-CURRENT), `relation`
   (REL-* — официальность, `[documentation]`, компаньоны), `localization`
   (LOC-* — переводы и их официальность), `card` (CARD-SITE-COPIES),
   `pipeline` (PIPE-*), `seo` (SEO-CANONICAL-HREFLANG, SEO-SITEMAP),
   `observability` (OBS-VERSIONS-ARE-CONTRACTS); PROP-005 (клиент индекса:
   `repomd.json`, `primary.jsonl`, `by-name`), PROP-010 (store), PROP-002
   §2.2 (`[[registry]]` — та же форма для источников сайта).
6. Код: `crates/vibe-doc/` (`build.rs`, манифест, `llms`, карточка),
   `crates/vibe-registry/src/index_client/` (probe, `locate.rs`, чтение
   `primary.jsonl`), `crates/vibe-registry` (store: `lookup`, `list_all`,
   `resolve_and_fetch`, `cache add`), `crates/vibe-workspace` (host checkout
   как источник: `LocalRegistry`), `crates/vibe-cli/src/cli/doc.rs` и
   `commands/doc/**` (подкоманды — отдельными модулями, в `doc.rs` только
   диспетчер, как у P2-O10 и P4-O4), `formats/REGISTRY.toml` и `schemas/`
   (файл состояния билдера — JTD-формат в реестре), `schemas/doc_manifest.jtd.json`.
7. Правила репозитория: Conventional Commits с телом «почему»; атом —
   коммит; **коммитить только явной формой `git commit -m … -- <свои пути>`**;
   **никаких трейлеров и упоминаний моделей**; `cargo fmt --all`;
   `unwrap`/`expect` в доменной логике запрещены; ошибки цитируют
   `spec://…`; wire только через схему → `cargo xtask codegen`; гейты в
   приватном `CARGO_TARGET_DIR=<scratch>\target-p5o1`, каталог удалить в
   конце; секреты и `infra/` не читать; `git push` не делать; чужие
   незакоммиченные файлы не стейджить; сеть — только к реестру
   `github.com/vibespecs` и его индексу (чтение) и к настроенному
   npm-реестру, если понадобится `pnpm install`; **ничего не публиковать
   и не деплоить**.

## Цель — пять атомов, пять коммитов

**A5.1 Конфигурация сайта** — `site.toml` (пример в
`vibevm/vibepacks/org.vibevm.doc/web/v0.1.0/site.example.toml`, схема JTD
в реестре форматов): два источника той же формы, что `[[registry]]` —
`[[source.registry]]` (`url`, `index_url`, `naming`) и `[source.host]`
(`git`, `ref = "main"`, `debounce_minutes = 60`); блок `[site]`:
`base_path = "/doc"`, `origin`, `default_theme`, `[site.analytics]` с
`website_id` и `host_url` (пустые по умолчанию — тег не рендерится).
Поверхность — `vibe doc build-site --config site.toml --out <каталог>`
над той же библиотекой; именно её запустит контейнер-рендерер (A5.6).
Коммит: `feat(doc): configure the site from two sources and one table`.

**A5.2 Лента индекса** — опрос `repomd.json` и `primary.jsonl` через
клиент индекса: множество «координата · версия · content hash» как оно в
индексе **сейчас** (переопубликованная версия — просто новый хэш по тому
же адресу; никакой истории — D-27); сравнение с файлом состояния билдера
(`<out>/.vibe-site/state.json`, JTD-формат в реестре: что отрендерено, с
каким хэшем, когда); очередь пересборки — только изменившиеся пары. Хост
— опрос ветки `main` источника хоста (A0.20): при новом коммите
пересборка с дебаунсом из конфигурации; текущий рендер хранится, и
предыдущий — до успешного завершения нового (атомарная подмена каталога).
Коммит: `feat(doc): read the index feed and queue what changed`.

**A5.3 Рендер уровня 0** — для каждой пары из очереди: прогрев в store
билдера (`cache add` с закрытием по `[[documents]]` — REL-WARMUP-CLOSURE)
→ `vibe doc build` уровня 0 и уровня 1: карточка с картинками или
плейсхолдерами, манифест, README, boot-сниппет, спеки с якорями, скиллы,
бинарники, MCP, зависимые, «объяснено в», «переведено на» (LEVEL-ZERO) →
страницы версии по адресам SITE-MOUNT; ошибка рендера — страница «render
failed» с причиной по тому же адресу, остальные пары не блокируются.
Коммит: `feat(doc): render every published version from its own bytes`.

Контракт с web-пакетом (из отчёта `WORKER-REPORT-P4-O3.md`, обязателен):
статическая сборка — `node tools/build.mjs static` с переменной
`VIBE_DOC_OUT` — список каталогов выхода `vibe doc build` через
разделитель путей платформы (`;` на Windows, `:` на POSIX); `vibe doc
build` пишет **одну** проекцию за прогон, поэтому на издание — три
каталога (`html`, `md`, `xml`) с одной координатой, и шаг копирования
читает все деревья координаты по очереди; переменную ставить из Rust
(`Command::env`), не через оболочку — под MSYS список режется по `:`
(отчёт P4-O3, А-4); `VITE_DOC_MEDIA` (карта координата → превью 1200×630)
и origin приходят тем же каналом. Гейт числа страниц: 2 × (страницы +
страница пакета) на издание и язык + `/doc/` + каталоги языков + 4
маршрута лендинга — адрес `latest` материализован (решение 1 отчёта).
Дерево пакета, которого нет в библиотеке страниц, копируется под своей
координатой с печатью строки — это не ошибка. Линтер
`tools/lint-links.mjs` и `tools/parity.mjs` гоняются над **этой**
сборкой (X-043), их вывод — в отчёт.

**A5.4 Полки, языки и связи** — официальность документации и переводов по
D-04/D-18/D-19 из манифестов (`[documentation]` предмета, `[[documents]]`,
`[translates]`, `[i18n].canonical`), community — по рёбрам из индекса,
зависимые — из индекса, «объяснено в» — из карт doc-пакетов, отставание
переводов — из проверки A2.15 (структурное); всё пересчитывается при
каждой пересборке, никакого хранимого флага (R-23, F-39). Коммит:
`feat(doc): compute shelves, languages and relations from the edges`.

**A5.5 `latest`, canonical, `hreflang`, sitemap** — `latest` как алиас
на новейшую версию по семверу в пределах языка, `rel=canonical` на
`latest` в языке, `hreflang` по языкам, `/doc/sitemap.xml` индексный по
пакетам и языкам — пересчёт при каждой пересборке; выход совместим с
тем, что P4-O3 ждёт от `vibe doc build` (проверь его линтером ссылок над
собранным сайтом). Коммит:
`feat(doc): recompute latest, canonical, hreflang and the sitemap on every build`.

## Гейты (вывод дословно в отчёт)

`cargo fmt --all --check`; `cargo build -p vibe-doc -p vibe-cli`; тесты
`vibe-doc` (лента, состояние, очередь — на фикстурном индексе, без сети),
`cargo clippy`; `cargo xtask check-codegen`; `cargo xtask specmap` (0
suspects); `vibe.exe facts check --exhaustive` clean; **живой прогон**:
`vibe doc build-site --config <site.toml на github.com/vibespecs и
локальный чекаут хоста> --out <scratch>` собирает три реальных пакета из
`vibespecs` и хост по `main` без ручных шагов; для одного пакета видны
официальная и community-полки со звёздочками; повторный прогон без
изменений индекса — **ноль перерендеров** (число в отчёт); `pnpm
build:static` web-пакета над этим выходом и линтер ссылок P4-O3 —
зелёные; `df -h` до и после, приватный каталог сборки удалён.

## Что не делать

Ничего на сервере, никакого деплоя, никакой публикации (R-24); Docker и
nginx — пакет P5-O2; официальность флагом не хранить (R-23); историю
публикаций не хранить (D-27); web-пакет не править (только собирать);
PROP-файлы и вижен не менять; секреты — нет: токены сайту не нужны, все
источники читаются анонимно.

## Диск и каталог сборки

Приватный `CARGO_TARGET_DIR` весит ~30 ГБ; диск машины уже упирался в
ноль во время фазы. В конце работы **удали** свой приватный каталог
сборки и проверь `df -h` до и после; при нехватке места гейты падают
ошибками линковки (`link.exe … 1108/1140/1201`), не имеющими отношения к
правкам — повтори после освобождения места, а не ищи причину в коде.

## Результат

Пять коммитов (без push) и `campaigns/docs-2026-09/findings/WORKER-REPORT-P5-O1.md`
по форме отчёта P2-O8: форма `site.toml` и файла состояния, вывод живого
прогона (сколько пар, сколько отрендерено, ноль перерендеров повторно),
полки одного пакета с обоснованием звёздочек, вывод гейтов дословно, что
не сделано и почему. Никаких путей вне репозитория, IP и секретов в
отчёте (R-25).
