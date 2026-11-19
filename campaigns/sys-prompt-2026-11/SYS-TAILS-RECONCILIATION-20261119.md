# SYS-TAILS — актуализация оставшегося scope

Жанр: campaign reconciliation / архитектурная подготовка.
Дата: 2026-11-19. Owner request: «кстати, мы забыли SYS-TAILS, ее тоже надо делать (и посмотреть, насколько она вообще актуальна)».
Статус: initial source/code reconciliation; не полный acceptance и не разрешение запускать live provider.

## Исходный узел

Existing `SYS-TAILS` находится внутри `SYS-PROMPT`, зависит от `SYS-QUALIFY` и уже помечен active. Не создавать заменяющую кампанию или терять старый ID. Его historical evidence перечисляет interactive session, Windows live row C8 и host-mismatch warning; последняя часть устарела относительно actual code и более позднего campaign log.

## Уже выполнено — не делать повторно без конкретной invalidation

- C1/C2/C4 shape/divergent-copy/read-carrier behavior, обычный `vibe check` finding, extension-world fixture: campaign LOG содержит поставки campaign tail. Проверить применимость evidence к затронутым owners при окончательной сверке, а не заново реализовывать эти механизмы.
- Межплатформенная determinism Linux/Windows/macOS закрыта поздней записью `2026-11-10 — determinism measured on macOS and Windows`, с одинаковыми artifact hashes на admitted source; старое утверждение «две платформы не измерены» — история, не current outstanding work.
- Host-mismatch warning реализован. Наличие независимо проверено в `crates/vibe-workspace/src/system_prompt/check/warnings.rs::host_mismatch_warning`, `prepare.rs`, `prepare/tests.rs` и `crates/vibe-cli/tests/cli_prompt_matrix.rs::a_launch_notes_the_host_mismatch_on_stderr_and_still_starts_the_harness`. LOG заканчивается записью о завершении этой части SP-FMT-101. Не планировать реализацию по старому обозначению SP-FMT-105 или объявлять эти тесты заново запущенными: сейчас проверены sources, не новый runtime receipt.

## Оставшееся — проверить актуальность и довести finite outcomes

1. **Current instruction/router delivery.** Historical log сообщает18 tracked instruction files со старым redirect из-за отсутствовавшего doc.web. Проверить реальные текущие generated/owned paths и source status; исчезнувший blocker не считать вечным, но не запускать mutating install в dirty roots без bound scope. Совместить с current USER-PROJECT behavior, не создавать второй boot owner.
2. **Interactive harness delivery.** Разделить уже доказанный slot/prefix delivery, фактический terminal UX и ещё не разрешённые live runs. Current version/source/context и точные утверждения должны быть определены до запуска. Print-mode не доказывает visual interactive behavior.
3. **Windows C8.** Проверить точный scope исторической строки против уже имеющихся native evidence и owner exception, разрешающего не выполнять физические Windows manual/Terminal проверки. Не переносить exception автоматически на любые authenticated live-provider calls и не просить снова ручной Ctrl+C, который уже сделан nonblocking. При неопределённости дать конечную requirement/disposition запись, не новый prerequisite ladder.
4. **Other harness claims.** Старое «шесть harnesses — research» нельзя целиком принять как implementation debt или как current capability. Hermes/Pi — отдельная новая owner task; её current primary-source research и actual native integration идут в собственном scope. Не дублировать под SYS-TAILS их adapter implementation или USER-PROJECT tests.
5. **Final evidence and docs.** Сопоставить existing PLAN/LOG/RESULTS, current owning contracts и actual source plan. Обновить current status, сохранив append-only historical failures и границы evidence. Living physical acceptance guide дополняется только actual manual claims.

## Принятый промежуточный результат — canonical TS-demo

2026-11-19 architect независимо прочитал `CANONICAL-TS-RESULT.json`, сверил SHA256 всех пяти canonical outputs в `/data/git/vibevm/research/ts-demo`, разобрал INDEX и проверил наличие каждого input. Actual ordinary install exit0; четыре package rows совпадают с проверенным candidate по versions/sources/hashes, authored entrypoint bytes сохранены. Scoped marker routers и optional personal boot присутствуют. Предыдущий source-free reinstall candidate сохранил bytes/mtime routers, lock и INDEX.

TS-demo outcome принят в этой области: обновлены три routers и lock через штатного owner; INDEX не изменился, dependency inputs существуют благодаря actual materialization, не raw copy. Schema6→7 соответствует текущему writer, не будущему latest-only proposal. Machine-local `/data` source epoch объявлен честно; native TypeScript tool build этим не доказан. Остальные три canonical root и host-derived refresh остаются открытыми; это не acceptance всего SYS-TAILS.

## Принятый промежуточный результат — canonical Rust-demo

2026-11-19 architect независимо сверил все пять hashes из `CANONICAL-RUST-RESULT.json` с actual destination `/data/git/vibevm/research/rust-demo`, проверил каждый INDEX input и точное равенство package rows reviewed isolated candidate. Actual ordinary install: четыре mandatory build contributions успешны; четыре optional MCP registrations отказали без явного scope и не считаются выполненными. Source-free candidate reinstall без mounted source registry сохранил bytes/mtime пяти outputs.

Scoped canonical Rust outcome принят вместе с reviewed package fixes: literal `cargo-clippy` prerequisite, project-root binding и explicit build consent внутри уже согласованной contribution. POSIX и native PowerShell script cases имеют отдельные receipts; это не утверждение о native Windows Rust build. Допустимая поставка — четыре изменённых Rust root files, три canonical package files и report, без bulk dependencies, неизменённого INDEX и чужих dirty changes.

Итого приняты два demo roots из четырёх. Два nested package/project roots и host-derived delegation-rules refresh остаются открытыми. Это не закрывает Hermes/Pi или весь SYS-PROMPT.

## Preservation gap — canonical delegation refresh

2026-11-19 independently read `DELEGATION-PRESERVATION-GAP.json`: nested pinned refetch task btwl7ehiy had already materialized31 slots before a full before-state slot snapshot/record-untracked inventory was captured. Only root inputs were preserved. Current31 ownership records do not establish prior state. The actual unrecorded-slot migration may replace a whole directory, so absence of lost unknown user files cannot be proved from available evidence. This is a process defect, not a successful preservation assertion.

Canonical delegation acceptance is held pending finite effect/recoverability review. No further force/slot deletion/cleanup or canonical fractality writes until the relevant old state is preserved and effects reviewed. Existing active Cargo is not stopped merely to manufacture a checkpoint; retain logs, old tracked blobs, root backups and candidate evidence. Recovery claims must distinguish actual observed changes, recoverable tracked/source bytes and unknown unrecorded data. Independent isolated carrier work can continue. Do not retroactively declare before-state preservation or silently broaden cleanup.

## Subsequent owner-directed selected-root recovery

The owner answered the architect's disposition question: «я решил этот вопрос вручную вместе с vibevm-builder». Do not repeat that question or interpret the answer as a blanket no-data-loss assertion. Builder subsequently reported an explicit owner instruction limited to delegation-rules nested vibedeps: preserve the current trackable state in commit d7e21620bd33b6cbef2f83062588c117fb452e87, retain a full external backup including ignored build outputs, clear only the selected derived contents and regenerate through the pinned owner, comparing immediate package-directory names/count before and after. The reported verified backup contains5307files/662214849bytes; clean regeneration is in progress, not accepted yet. This later instruction supersedes the selected-root cleanup hold only within that exact scope, not the earlier unknown-before-state limitation or restrictions on other roots. The architect did not execute deletion and reported no active own consumers; that statement is not proof of global process absence. Verify terminal regeneration receipts and actual count/identity deltas before acceptance.

## Порядок работы и acceptance

Начать с короткой актуализации existing SYS-TAILS и ready shared seams; Hermes/Pi research можно готовить параллельно без пересекающихся product writes. Не вводить вымышленную обязательную зависимость «все SYS-TAILS закрыть до Hermes/Pi», если actual contract этого не требует. Реальные общие prerequisites и integration owner фиксируются перед implementation dispatch.

Завершение: every original tail имеет disposition delivered/evidence-valid, bounded actual repair, explicitly unsupported/out-of-scope или owner acceptance exception. Outstanding live/manual facts не превращаются в PASS. Authoritative multi-user-planning обновляется до renderer; дерево отражает действительные remaining tasks, а не stale log prose.

Никакие provider calls, installs, credentials/config changes, external publication или worktree creation этим документом не выполнены. Все новые task/worktrees размещаются под `/data` по owner directive.
