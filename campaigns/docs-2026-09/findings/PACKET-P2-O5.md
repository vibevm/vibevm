# PACKET-P2-O5 — аудитория `agent` и документация вне судейства (A2.12, A2.21)

##subagent-quiet-clause

Ты — воркер кампании документации, исполнитель уровня `opus5`. Boot-лейн
репозитория не читать. Твоя инструкция — этот файл и ровно те файлы,
которые он называет.

## Читать сначала

1. Этот пакет целиком.
2. `campaigns/docs-2026-09/PLAN.md` — атомы **A2.12**, **A2.21**.
3. Находки: `campaigns/docs-2026-09/findings/A0.5-audience-agent.md`
   (12 мест правки; `--view doc` — фильтр, не гейт),
   `A0.19-docs-outside-judging.md` (`[judging] exempt` + ячейка проверки).
4. Норма: `vibevm/vibespecs/modules/vibe-facts/PROP-043-*.xml` —
   `ROW-ATTR-AUDIENCE-VALUES`, `AUDIENCE-VALUES`, `AUDIENCE-DOC-USE` (аудитория
   `agent` допущена коммитом `390c7b1d`); `vibevm/vibespecs/modules/vibe-facts/PROP-047-*.xml`
   `CMD-REPORT` (`--audience user|author|dev|agent`); PROP-057 `observability`
   (`OBS-*`: `[judging] exempt`, doc-пакеты наблюдаемы, но не в долге).
5. Код: `crates/progress-core/` (`Audience`, парсер, `report.rs`, голдены),
   `crates/vibe-facts/`, `facts.toml`, `campaigns/packages-2026-09/tasks/judging-debt.py`
   (как считается долг сегодня), `crates/vibe-specdoc/tests/docs_corpus.rs`
   (карантин страницы `agent/how-agents-read-this-manual.xml` с причиной
   «unknown audience value `agent`» — после A2.12 её запись **удаляется**, и
   счётчики корпуса растут: страница несёт 0 примеров, 8 `rule`, 0 `derived`,
   0 промптов).
6. Правила репозитория: Conventional Commits с телом «почему»; атом —
   коммит; **никаких трейлеров и упоминаний моделей**; `cargo fmt --all`;
   `unwrap`/`expect` в доменной логике запрещены; ошибки цитируют `spec://…`;
   секреты и `infra/` не читать; `git push` не делать.

## Цель — два атома, два коммита

**A2.12** `Audience::Agent`; `ALL` из четырёх; парсер атрибута `audience`
(CSV) принимает `agent`; фильтр отчёта `--audience agent`; тесты и голдены
обновлены; страница `agent/how-agents-read-this-manual.xml` читается
пивотом — запись карантина в `docs_corpus.rs` удалена, счётчики
поправлены (`44 - QUARANTINED.len()` станет 44; `rules` 373 → 381).
Коммит: `feat(progress): admit the agent audience`.

**A2.21** Механизм из A0.19: пакеты вида `doc` (и пакет
`vibevm/vibepacks/org.vibevm.core/vibevm-docs`) наблюдаемы (`vibe facts`
видит их страницы и статусы), но не входят в долг судейства
(`[judging] exempt` в конфигурации или эквивалент, выбранный находкой);
тест на скрипте/конфигурации, который упадёт, если doc-пакет попадёт в
долг. Решить и записать в отчёте X-024: освобождает ли `[judging] exempt` и
от `--exhaustive` — рекомендация центральной сессии: да, страницы — не факты.
Коммит: `chore(facts): keep documentation observed but unjudged`.

## Гейты

`cargo fmt --all --check`, `cargo build --workspace`, `cargo test -p
progress-core -p vibe-facts -p vibe-specdoc`, `cargo clippy` по тронутым
крейтам, `target/debug/vibe.exe facts check --exhaustive` — clean,
`python campaigns/packages-2026-09/tasks/judging-debt.py` — число не растёт.

## Что не делать

В дереве работают воркеры P2-O1 (`crates/vibe-core`, `crates/vibe-wire`,
`crates/vibe-cli/src`, `xtask/src`, `formats/`, `schemas/`) и P1-O5
(тестовые фикстуры lock-файлов) — их файлы не трогать и не стейджить.
Страницы не править. Не открывать других атомов.

## Результат

Два коммита (без push) и `campaigns/docs-2026-09/findings/WORKER-REPORT-P2-O5.md`
по форме отчёта P2-O1.
