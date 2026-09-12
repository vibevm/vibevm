# PACKET-P6-C2 — импорт регламента сопровождения как design-документа (A6.4, часть «MAINTENANCE.md рядом с виженом»)

##subagent-quiet-clause

Ты — воркер кампании документации, дешёвая модель для механического
импорта со скриптом. Boot-лейн репозитория не читать; твоя инструкция —
этот файл и файлы, которые он называет. Секреты, `~/.vibe/*.token`,
`infra/` не читать; сеть не нужна.

## Задача

`campaigns/docs-2026-09/MAINTENANCE.md` (регламент сопровождения
документации, 340 строк, русский) импортируется в
`vibevm/vibespecs/design/documentation-maintenance.xml` **тем же способом,
каким P1-O1 импортировал вижен** в `vibevm/vibespecs/design/documentation-vision.xml`:
прочитай `findings/WORKER-REPORT-P1-O1.md` целиком (шаги, решения,
скрипт `mark-facts.py` в §12 приложения) и `documentation-vision.xml`
(образец формы: `<title id="root">`, `<status … comment="…"/>`,
`companion-line`, секции с `title`, `<facts ordered="false">` с
детерминированными идентификаторами фактов, `<fence>` дословно).

Правила импорта:

- каждый абзац, строка таблицы и пункт списка — факт `status="spec/done"`
  (design-документ — «почему и как мы решили», не норма); идентификаторы
  детерминированы схемой из P1-O1 (`<секция>-<n>`); таблицы — как их
  разметил P1-O1 в вижене (посмотри, как там сделано, и повтори);
- заголовок документа: «Сопровождение документации — регламент, черновик
  кампании»; `status` `stage="spec" state="done"` с комментарием: «design
  rationale behind PROP-058 — the maintenance regulation drafted 2026-09-10
  and rehearsed in phase 6; imported 2026-09-12; non-normative, PROP-058
  wins where they disagree»; `companion-line`: **Explains:**
  [PROP-058](../common/PROP-058-documentation-maintenance.xml) — если файла
  PROP-058 ещё нет в дереве в момент твоей работы, ссылку всё равно
  пиши (центральная сессия пишет PROP-058 параллельно);
- провенанс и упоминания scratch-путей не переносятся; ничего не
  переписывать и не сокращать — только разметка;
- `MAINTENANCE.md` в зоне остаётся как есть (его статус-комментарий
  дополняет центральная сессия).

## Гейты (вывод дословно в отчёт)

`target/debug/vibe.exe facts check --exhaustive` — clean (число файлов
выросло на один); `target/debug/vibe.exe check --path .` — без новых
ошибок; `cargo xtask specmap` — 0 suspects (файл `specmap.json` вернуть к
HEAD); разбор XML стандартным парсером — без ошибок.

## Коммит и отчёт

Один коммит явной формой `git commit -m "docs(design): import the documentation maintenance regulation" -m "<почему>" -- vibevm/vibespecs/design/documentation-maintenance.xml`
(новый файл сначала `git add -- <файл>`); никаких трейлеров и упоминаний
моделей; `git push` не делать; чужие незакоммиченные файлы не стейджить
(в дереве работают центральная сессия и воркеры). Отчёт
`campaigns/docs-2026-09/findings/WORKER-REPORT-P6-C2.md` (не коммитить):
число фактов, схема идентификаторов, вывод гейтов, что не перенеслось.
Ни одного пути вне репозитория.
