# PACKET-P2-O4 — крейт `vibe-doc`: раннер примеров и `derived` (A2.0, A2.9, A2.10)

##subagent-quiet-clause

Ты — воркер кампании документации, исполнитель уровня `opus5`. Boot-лейн
репозитория не читать. Твоя инструкция — этот файл и ровно те файлы,
которые он называет. Предусловие: коммит пакета P2-O2 (A2.8, словарь
документации в пивоте) уже в ветке — проверь `git log --oneline -8` и
прочитай `campaigns/docs-2026-09/findings/WORKER-REPORT-P2-O2.md`.

## Читать сначала

1. Этот пакет целиком; отчёт P2-O2.
2. `campaigns/docs-2026-09/PLAN.md` — §2.8 «Чеклист нового крейта»
   **целиком** (все семь пунктов), атомы **A2.0**, **A2.9**, **A2.10**.
3. `campaigns/docs-2026-09/findings/A0.12-example-runner.md` (макет: три
   переменные изоляции, cwd — песочница, окружение не меняет поведение,
   классы нормализации, форма `example.toml`, валидатор JTD) и
   `A0.12-runner-mock.py` (эталон поведения; переписывается на Rust, не
   вызывается).
4. `campaigns/docs-2026-09/EXAMPLES-TODO.md` — правила песочницы и таблица
   фикстур третьей редакции: первый корпус раннера; локальный реестр как
   `file://` на копию нужных пакетов внутри фикстуры (X-025);
   `findings/WORKER-REPORT-PP-C2b.md` — как строились фикстуры и
   нормализовались выводы; `findings/PP-C2-expects/fixtures.md`.
5. Норма: PROP-057 `pipeline` (PIPE-EXAMPLE-RUNNER, PIPE-DERIVED),
   `invariants` (INV-EXAMPLES-RUN), `observability` (OBS-EXAMPLES-GOLDEN —
   `spec/work`), PROP-045 §7 (ROW-DOCVOCAB-EXAMPLE*, ROW-DOCVOCAB-DERIVED*);
   `campaigns/docs-2026-09/DEFERRALS.md` X-025, X-026, X-030 — решения,
   которые раннер и генератор должны принять: реестр внутри фикстуры с
   подстановкой `${REGISTRY}`, статус «снимается на релизе» для примера
   (`captured="release"` или эквивалент — назови в отчёте), грамматика `ref`
   у `manifest-field`.
6. Корпус: 44 страницы `vibevm/vibepacks/org.vibevm.core/vibevm-docs/v0.1.0/vibevm/vibespecs/**`
   — 39 примеров с `expect`, 15 без (их снимает твой раннер в режиме
   `--accept`), 72 `derived` (`cli-help`, `jtd-schema`, `manifest-field`).
7. Правила репозитория: Conventional Commits с телом «почему»; атом —
   коммит; **никаких трейлеров и упоминаний моделей**; `cargo fmt --all`;
   `unwrap`/`expect` в доменной логике запрещены; ошибки цитируют
   `spec://…`; первая строка `lib.rs` — `specmark::scope!(…PROP-057#…)`;
   `git push` не делать; настоящий дом `~/.vibe` раннер не трогает никогда
   (трипвайр как в `tools/user-home-tripwire.sh`).

## Цель — три атома, три коммита

**A2.0 + A2.9** Новый крейт `crates/vibe-doc` по чеклисту §2.8; модуль
`examples`: обход страниц пакета, сбор `Block::Example`, фикстуры из
`examples/<fixture>/` пакета (`example.toml` по форме A0.12 §5:
дерево, cwd, нормализация, подстановки `${REGISTRY}`, карта `--json` →
JTD), исполнение собранным бинарником в песочнице под `VIBE_SETTINGS`,
нормализация, точное сравнение (без шаблонов), отчёт с диффом; `--accept`
переписывает `expect` в исходнике только по флагу; примеры со статусом
«снимается на релизе» и «ждёт» пропускаются с причиной. Перенеси фикстуры
`EXAMPLES-TODO.md` в `examples/` пакета руководства (`none`, `empty`,
`hello-vibe-*`, `project`, `flow-slot`, `package-spec`, `workspace*`,
`hello-cargo`, `hello-deploy`, `hello-vibe-scrape`, `host`) с локальным
реестром-копией только нужных пакетов (`org.vibevm.world/wal` и его
замыкание); фикстуры с `cargo` строят крошечный крейт на месте. CLI: `vibe
doc check --examples [--accept] --path <пакет>`. Прогон по руководству: 39
снятых совпадают (иначе — разбор в отчёте, не правка `expect`), 15 —
сняты `--accept` и вставлены. Панель: шаг `run_step "documented examples
run and match"` в `tools/self-check.sh` после сборки бинарника. Коммиты:
`feat(doc): open the vibe-doc crate` (A2.0, если скелет отдельно) и
`feat(doc): run documented examples so the docs cannot lie` (A2.9).

**A2.10** Генераторы `derived`: `cli-help` — `vibe <команда> --help`
собранным бинарником; `jtd-schema` — таблица полей из схемы по `FormatId`
(`formats/REGISTRY.toml` → путь); `manifest-field` — по X-030: `ref` без
координаты — поле текущего пакета, `<координата>` — весь манифест,
`<координата>#<поле>` — одно поле. Детерминизм: двойной прогон — одни байты;
`vibe doc check --derived` сообщает расхождение с последним построением;
текст никогда не пишется в страницы. Коммит:
`feat(doc): derive command, schema and manifest references at build time`.

## Гейты

`cargo fmt --all --check`, `cargo build --workspace`, `cargo test -p
vibe-doc` и тронутых крейтов, `cargo clippy … -D warnings`, `cargo xtask
conform check` (периметр нового крейта — §2.8 п. 6), `cargo xtask specmap`
(ноль suspects), `bash tools/self-check.sh` до конца — новый шаг зелёный.

## Что не делать

Прозу страниц не менять (только `expect` через `--accept` и ничего больше);
зону кампании (кроме отчёта и `EXAMPLES-TODO.md` — колонка «Состояние» на
«снято <хэш>») не трогать; PROP-файлы не менять; `git push` не делать.

## Результат

Коммиты (без push) и `campaigns/docs-2026-09/findings/WORKER-REPORT-P2-O4.md`
по форме отчёта P2-O1, плюс список примеров, которые не совпали, с
разбором «страница или продукт».
