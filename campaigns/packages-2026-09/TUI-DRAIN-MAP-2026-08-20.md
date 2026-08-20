# WORKER-REPORT-TUI-DRAIN-MAP

## TL;DR

Карта покрывает все **39 фактических атрибутов** `#[allow(dead_code)]` в заданной области. Предлагаемый дренаж: **2 `wire` · 23 `delete` · 1 `justify` · 13 `test-only`**. Два `wire` образуют две независимые строительные задачи в `vibe prefs`: оживить lazy body декларации страницы и перестать рисовать enum-поле вручную, передав его общему `RadioGroup`. Единственный `justify` — контрактный builder `PageDecl::with_scope`; его устаревшую причину про S2 надо заменить точной контрактной причиной. `ComingSoon` в production обслуживает только PNG export, ровно как предписывают PROP-037 `#copy-png` и `#NG-PNG`; лишних и недостающих открытых reserved-точек не найдено. `cargo check -p vibe-cli` завершился с exit 0.

## Метод и границы

- Область: `crates/vibe-cli/src/commands/tree/tui/**`, `crates/vibe-cli/src/commands/prefs/tui/**`, включая `prefs/tui/form/control.rs`.
- Ссылкой считается обращение за пределами строки определения; отдельно различены production и `#[cfg(test)]`.
- `delete` для широкого allow над живым типом/`impl` означает удалить именно широкий allow, затем удалить или сузить оставшиеся неиспользуемые helper-методы.
- `test-only` означает сохранить поверхность только под `#[cfg(test)]` (или заменить её прямым тестовым доступом), а не продолжать подавлять production lint.
- Якоря сверены с `PROP-037-tree-tui.md` и `PROP-041-settings-ui.md`.

## Карта сайтов

### theme — 7 сайтов

| № | Сайт / что подавлено | Причина рядом | Живые ссылки | Контракт | Вердикт-предложение |
|---:|---|---|---|---|---|
| 1 | `tree/tui/theme/mod.rs:41` — модуль `glyphs`, словарь псевдографики | Общая причина: «The submodules are public so the staged Phase-3 API … is reachable …; `#[allow(dead_code)]` covers the items not yet wired through `App`.» | **Жив**: `Theme::new` выбирает `Glyphs::rich/ascii`; `Theme::glyphs` читается всеми renderer-ами. | PROP-037 `#glyph-vocabulary`, `#theme` | `delete` — модуль давно включён; снять широкий allow и отдельно удалить реально неиспользуемые внутренние элементы, которые проявит lint. |
| 2 | `tree/tui/theme/mod.rs:43` — модуль `palette`, semantic roles / `Palette` / `Rgb` | Та же общая причина | **Жив**: `Theme` хранит `Box<dyn Palette>`, `Theme::color` читает роли; стили UI идут только через theme. | `#palette-tokens`, `#theme` | `delete` — production-ветка активна; широкий allow больше не оправдан. |
| 3 | `tree/tui/theme/mod.rs:45` — модуль `palettes`, реестр Rosé Pine/Catppuccin | Та же общая причина | **Жив**: `Theme::new -> palettes::resolve`; `settings.rs` парсит `PaletteName`. | `#palette-tokens`, `#settings` | `delete` — все объявленные palette identities включены в настройки/theme; осушить скрытые внутренние остатки отдельно. |
| 4 | `tree/tui/theme/mod.rs:47` — модуль `tier`, детекция и проекция tier | Та же общая причина | **Жив**: `Theme` хранит `Tier`, `Theme::color` вызывает `project_color`; `settings.rs` вызывает `detect_tier`. | `#rendering-tiers` | `delete` — production-путь активен. |
| 5 | `tree/tui/theme/mod.rs:100` — `Theme::palette_name` | «introspection: read by settings tests + a future settings UI.» | **Только тесты**: `theme/mod.rs:268`, `settings/tests.rs:92,113,131`. | Косвенно `#theme`/`#settings`; getter контрактом не требуется. | `test-only` — перенести getter под `#[cfg(test)]` либо тестировать выбранную палитру через тестовый helper. |
| 6 | `tree/tui/theme/mod.rs:107` — `Theme::tier` | «introspection: read by settings tests + a future settings UI.» | **Только тесты**: `theme/mod.rs:269`, `settings/tests.rs:102`. | Косвенно `#rendering-tiers`; getter не REQ. | `test-only` — сузить доступность до тестов. |
| 7 | `tree/tui/theme/mod.rs:114` — `Theme::is_light` | «introspection: read by a future settings UI / AIUI.» | **Только тесты**: `theme/mod.rs:270,310`. Production вызывает другой метод — `Palette::is_light` внутри `Theme::color`. | Косвенно `#palette-tokens`/`#rendering-tiers`; public getter не требуется. | `test-only` — оставить только тестовую introspection; ссылка на future AIUI противоречит PROP-041 `#non-goals`. |

### ui-виджеты — 12 сайтов

| № | Сайт / что подавлено | Причина рядом | Живые ссылки | Контракт | Вердикт-предложение |
|---:|---|---|---|---|---|
| 8 | `tree/tui/ui/button.rs:50` — тип `Button` | «Phase-3 component foundation; lights up when P6 (quit-confirm) / P7 (ComingSoon) compose it.» | **Жив**: quit confirm `render.rs:340-341`, file destination `copy/file_dest.rs:148-149`, `MsgDialog` `ui/msg_dialog.rs:104`. | `#button`, `#quit-confirm`, `#copy-dest`, `#coming-soon` | `delete` — заявленные пользователи уже работают. |
| 9 | `tree/tui/ui/button.rs:57` — весь `impl Button` | Та же причина | **Жив**: `new/focused/width/render` в указанных production-ветках; `is_focused` — только тесты; `label` — 0 ссылок. | `#button` | `delete` — снять allow с impl; удалить `label`, `is_focused` сузить до `#[cfg(test)]`. |
| 10 | `tree/tui/ui/text_field.rs:42` — тип `TextField` | «Phase-7 component foundation; lights up when the file-path modal (§10.5) composes it.» | **Жив**: prefs form `prefs/tui/form/{control,render}.rs`; file destination `copy/file_dest.rs:142`. | `#text-field`, `#copy-dest`; PROP-041 `#form-per-type` | `delete` — заявленный file-path modal и форма уже используют тип. |
| 11 | `tree/tui/ui/text_field.rs:49` — весь `impl TextField` | Та же причина | **Жив**: `new/focused/with_value/value/type_char/backspace/render`; `is_focused` — только тесты `text_field.rs:207-209`. | `#text-field` | `delete` — снять широкий allow; `is_focused` сделать test-only. |
| 12 | `tree/tui/ui/radio_group.rs:43` — тип `RadioGroup` | «Phase-7 component foundation; lights up when … copy-settings §10.2 modal composes it.» | **Жив**: два поля и два конструктора в `copy/settings.rs:47-59`, навигация `:85-90`, production render. | `#radio-group`, `#copy-flow` | `delete` — copy-settings уже полностью композирует виджет. |
| 13 | `tree/tui/ui/radio_group.rs:51` — весь `impl RadioGroup` | Та же причина | **Жив**: production использует `new/options/selected_index/select_up/select_down/render`; `label` встречается только в тесте `radio_group.rs:195`. | `#radio-group`, `#copy-flow` | `delete` — снять широкий allow; `label` сузить до теста либо удалить тестовый accessor. |
| 14 | `tree/tui/ui/msg_dialog.rs:32` — тип `MsgDialog` | «Phase-3 component foundation; lights up when P6 (quit-confirm) / P7 (ComingSoon) compose it.» | **Жив**: `ComingSoon::new` строит dialog (`coming_soon.rs:42`), production render идёт через `menu/draw.rs:35`. | `#coming-soon`; базовый dialog также соответствует `#quit-confirm`/`#button` композиции | `delete` — production-пользователь существует. |
| 15 | `tree/tui/ui/msg_dialog.rs:39` — весь `impl MsgDialog` | Та же причина | **Жив**: `new/render`; `title` нужен лишь цепочке тестового `ComingSoon::feature`, `body` — только тесту `msg_dialog.rs:167`. | `#coming-soon` | `delete` — убрать широкий allow; `title/body` сделать test-only вместе с тестовыми accessors `ComingSoon`. |
| 16 | `tree/tui/ui/group.rs:52` — `Group::new` | «Unnamed construction + the `name` accessor are reserved for callers that build a frame dynamically; the F2 menu uses `Group::named` today.» | **Только тестовая цепочка**: прямые вызовы `group.rs:133,150`; production-скомпилированный `Default::default` делегирует в `new`, но сам `Default` используется только тестом `:140`. Production UI вызывает `Group::named`. | `#group`; безымянный constructor отдельно не требуется. | `test-only` — `new`/неиспользуемый `Default` оставить тестам либо удалить, тест строить через `named`. |
| 17 | `tree/tui/ui/group.rs:81` — `Group::name` | Явной причины нет; только описание «The group's name, if any.» | **Только тесты**: `group.rs:133-134`. | `#group`; accessor не указан. | `test-only` — сузить accessor до тестов. |
| 18 | `tree/tui/ui/coming_soon.rs:48` — `ComingSoon::feature` | «introspection; the render path reads the title through MsgDialog.» | **Только тесты**: `coming_soon.rs:79`; внутри accessor вызывает `MsgDialog::title`. | `#coming-soon`; accessor не требуется. | `test-only` — тестовая introspection. |
| 19 | `tree/tui/ui/coming_soon.rs:56` — `ComingSoon::body` | «introspection; the render path carries the body through MsgDialog.» | **Только тесты**: `coming_soon.rs:80`. | `#coming-soon`; accessor не требуется. | `test-only` — тестовая introspection. |

### card — 3 сайта

| № | Сайт / что подавлено | Причина рядом | Живые ссылки | Контракт | Вердикт-предложение |
|---:|---|---|---|---|---|
| 20 | `tree/tui/ui/card.rs:73` — `Card::rows` | Doc: «the introspection surface for the card copy provider and tests. Not read by `render` itself.» | **Только тесты**: `modal.rs:237,268,291`; production copy использует `to_markdown`, не `rows`. | `#card`, `#detail-card`; accessor не требуется для production. | `test-only` — сузить до тестов. |
| 21 | `tree/tui/ui/card.rs:80` — `Card::is_empty` | Явной причины нет. | **0 ссылок**; renderer проверяет поле `self.rows.is_empty()` напрямую. | Прямого факта PROP нет. | `delete` — лишний accessor. |
| 22 | `tree/tui/ui/card.rs:91` — `Card::to_markdown` | «first user: copy::card_markdown (§10.1).» | **Жив**: `copy/mod.rs:91` вызывает `card.to_markdown()`. | `#copy-providers`, `#copy-markdown`, `#card` | `delete` — снять устаревший allow, метод production-live. |

### settings — 3 сайта

| № | Сайт / что подавлено | Причина рядом | Живые ссылки | Контракт | Вердикт-предложение |
|---:|---|---|---|---|---|
| 23 | `tree/tui/settings.rs:179` — поле `TreePrefs::palette` | «introspection: carried for a future settings UI / `vibe prefs show`.» | **0 чтений**; только заполнение в `Default` и `snapshot`. Реальный `Theme` строится отдельно через `TreeSettings::theme`. | `#settings` требует persisted palette, но не дублирующее поле snapshot. | `delete` — убрать поле и его записи; active palette уже живёт в `Theme`. |
| 24 | `tree/tui/settings.rs:182` — поле `TreePrefs::tier_override` | Та же причина | **Только тестовые чтения**: `settings/tests.rs:104,119`; production только заполняет поле, а tier уже применяет `TreeSettings::theme`. | `#settings`, `#rendering-tiers`; дублирующее поле не требуется. | `delete` — удалить поле и переписать тесты на `Theme`/resolved prefs. |
| 25 | `tree/tui/settings.rs:256` — `TreeSettings::schema` | «introspection: read by tests + a future `vibe prefs` surface.» | **Только тест**: `settings/tests.rs:35`; prefs production использует общий registration point `build_schema`, а не этот getter. | Косвенно `#settings`; PROP-041 `#aiui-ready`, но сам AIUI — `#non-goals`. | `test-only` — сделать getter тестовым; не оправдывать allow будущим AIUI. |

### state+shape — 4 сайта

| № | Сайт / что подавлено | Причина рядом | Живые ссылки | Контракт | Вердикт-предложение |
|---:|---|---|---|---|---|
| 26 | `tree/tui/shape.rs:36` — `TreeShape::LoadTypeForest` | «selected by the F2 sort menu (Phase 5+); exercised in tests today.» | **Жив через F2 shape branch**: options `menu/sort.rs:73,113`; pipeline `shape.rs:54,69,92`; persistence `settings.rs:126,135`; flatten production + tests. | `#tree-shapes`, `#f2-sort-menu` | `delete` — Phase 5 уже построена. |
| 27 | `tree/tui/shape.rs:41` — `TreeShape::PrunedTree` | Та же причина | **Жив через F2 shape branch**: `menu/sort.rs:74,114`; pipeline `shape.rs:54,70,98`; persistence `settings.rs:127,136`; flatten/state. | `#tree-shapes`, `#f2-sort-menu` | `delete` — снять устаревший allow. |
| 28 | `tree/tui/state.rs:349` — `App::set_shape` | «selected by the F2 sort menu (§7.2, Phase 5+); exercised in tests today.» | **Жив**: `menu/mod.rs:309`, effect `MenuEffect::SetShape`; также state tests. | `#tree-shapes`, `#f2-sort-menu` | `delete` — production controller вызывает метод. |
| 29 | `tree/tui/state.rs:368` — `App::set_static_first` | «selected by the F2 sort menu "Block order" group (§7.2, Phase 7); exercised in tests.» | **Жив**: `menu/mod.rs:310`, effect `MenuEffect::SetStaticFirst`; menu test покрывает выбор. | `#f2-sort-menu`, `#settings` | `delete` — production controller вызывает метод. |

### menu — 1 сайт

| № | Сайт / что подавлено | Причина рядом | Живые ссылки | Контракт | Вердикт-предложение |
|---:|---|---|---|---|---|
| 30 | `tree/tui/menu/mod.rs:106` — `MenuState::coming_soon` | «first user: copy::png_coming_soon (§10.4); wired when copy-settings lands.» | **Жив через PNG branch**: `copy/mod.rs:227`; тесты `menu/mod.rs:432,445`; draw `menu/draw.rs:34-35`. | `#coming-soon`, `#copy-png`, `#NG-PNG` | `delete` — указанный first user уже подключён; комментарий фактически устарел. |

### copy — 1 сайт

| № | Сайт / что подавлено | Причина рядом | Живые ссылки | Контракт | Вердикт-предложение |
|---:|---|---|---|---|---|
| 31 | `tree/tui/copy/settings.rs:104` — `CopySettings::focused_group` | «introspection; exercised in tests.» | **Только тесты**: `copy/settings.rs:196,213,215,221`. Production использует приватный `focused_group_mut` и сравнивает поле `focus` внутри renderer. | `#copy-flow`; public getter не требуется. | `test-only` — сузить getter до тестов. |

### prefs-registry — 5 сайтов

| № | Сайт / что подавлено | Причина рядом | Живые ссылки | Контракт | Вердикт-предложение |
|---:|---|---|---|---|---|
| 32 | `prefs/tui/registry.rs:133` — `PageDecl::with_scope` | «used in tests today; S2's project-scoped pages + the UI use it.» | **Только тесты сегодня**: `registry.rs:433,448`; production builtins намеренно все `Application`, но `PageRegistry::tree` реально фильтрует `PageScope::Project`. | PROP-041 `#declarative-pages` (`page-scope-flag`), `#tree-context` | `justify` — сохранить как контрактный builder для деклараций project pages; заменить устаревшее «S2» на точную причину: required declaration surface, current builtins all user-scoped, branch verified by tests. |
| 33 | `prefs/tui/registry.rs:148` — `PageDecl::with_body` | «S2 fills the form body; S1 ships the placeholder default.» | **0 вызовов**; `PageDecl.body` нигде не читается, `PageBody` имеет только `Placeholder`; `Form::build` строит поля прямо из `decl.keys`. | PROP-041 `#declarative-pages` (`page-lazy-body`) | `wire` — заменить placeholder реальным lazy constructor, вызывать `decl.body` при первом открытии формы и зарегистрировать bodies у builtins (либо явно свести hook к ленивому `Form::build`, но тогда синхронно исправить контракт/модель). |
| 34 | `prefs/tui/registry.rs:186` — `PageRegistry::new` | «used in tests; S2 plugins register into one.» | **Только тест**: `registry.rs:375`; mutation/register API отсутствует, production строит через `PageRegistry::from`. | `#registry-is-introspectable`; отдельный `new` не требуется. | `test-only` — сузить constructor до теста (либо заменить тест на `Default`). |
| 35 | `prefs/tui/registry.rs:205` — `PageRegistry::len` | «introspection: used in tests + the future AIUI surface.» | **Только тесты**: `prefs/tui/settings.rs:131`, `registry.rs:477`. Совпадение `catalogue.rs:350` относится к реестру actions, не к `PageRegistry`. | `#registry-is-introspectable`; AIUI surface — PROP-041 `#non-goals`. | `test-only` — сузить до тестов или использовать `pages().len()`; убрать future-AIUI оправдание. |
| 36 | `prefs/tui/registry.rs:211` — `PageRegistry::is_empty` | «introspection: used in tests + the future AIUI surface.» | **Только тест**: `registry.rs:376`. | `#registry-is-introspectable`; convenience getter не REQ. | `test-only` — сузить или заменить тест на `pages().is_empty()`. |

### form — 3 сайта

| № | Сайт / что подавлено | Причина рядом | Живые ссылки | Контракт | Вердикт-предложение |
|---:|---|---|---|---|---|
| 37 | `prefs/tui/form/control.rs:60` — поле `Selection::label` | «carried so a future modal RadioGroup.render can title itself.» | **0 чтений**; поле только записывается в `Selection::new`. `form/render.rs:200-228` вручную повторяет marks/options `RadioGroup`, не читая label. | PROP-041 `#form-per-type` прямо требует enum → PROP-037 `RadioGroup`; `#built-on-tree-tui` запрещает отдельную библиотеку компонентов. | `wire` — представить selection общим `ui::RadioGroup` (или строить его из `label/options/selected`) и делегировать ему render/navigation вместо ручной копии. |
| 38 | `prefs/tui/form/mod.rs:130` — поле `Form::page_id` | «introspection: carried for the AIUI / a future jump-to-page.» | **0 чтений**; только заполнение в `Form::build`/`for_test`; текущий page id уже хранит `PrefsApp::open_page`. | Косвенно `#declarative-pages`/`#settings-search`; дублирующее поле не требуется, AIUI — `#non-goals`. | `delete` — убрать дубликат и записи. |
| 39 | `prefs/tui/form/mod.rs:133` — поле `Form::title` | «introspection: the border title is sourced from PrefsApp today.» | **0 чтений**; только заполнение; renderer действительно получает заголовок из `PrefsApp`. | Прямого REQ для дубля нет. | `delete` — убрать поле и параметр test constructor. |

## Сводка вердиктов

| Подсистема | Сайтов | `wire` | `delete` | `justify` | `test-only` |
|---|---:|---:|---:|---:|---:|
| theme | 7 | 0 | 4 | 0 | 3 |
| ui-виджеты | 12 | 0 | 8 | 0 | 4 |
| card | 3 | 0 | 2 | 0 | 1 |
| settings | 3 | 0 | 2 | 0 | 1 |
| state+shape | 4 | 0 | 4 | 0 | 0 |
| menu | 1 | 0 | 1 | 0 | 0 |
| copy | 1 | 0 | 0 | 0 | 1 |
| prefs-registry | 5 | 1 | 0 | 1 | 3 |
| form | 3 | 1 | 2 | 0 | 0 |
| **Итого** | **39** | **2** | **23** | **1** | **13** |

### Связные строительные `wire`-задачи

1. **Lazy body страницы настроек** — сайт №33. Сделать `PageDecl.body` настоящим ленивым composition point: реальный body constructor в builtin declarations, вызов только при открытии страницы, тест «tree enumeration не строит body; first open строит один раз». Сейчас комментарии S1/S2 пережили завершение PROP-041, а hook полностью обходится.
2. **Enum form через общий `RadioGroup`** — сайт №37. Убрать параллельную модель/ручной renderer `Selection`, либо завернуть её вокруг `RadioGroup`; сохранить TOML conversion и form lifecycle, но отдать общему виджету label/options/selection/navigation/render. Это одновременно закрывает фактическое расхождение с PROP-041 `#form-per-type` и `#built-on-tree-tui`.

Остальные сайты — не новые фичи, а три механических пакета дренажа: (a) снять stale allow с уже живых production-путей; (b) удалить дубли/нулевые accessors; (c) сузить тестовые introspection helpers до `#[cfg(test)]`.

## ComingSoon: код и сверка со спекой

`rg -n -i 'coming_soon|comingsoon'` дал 57 текстовых попаданий (код, docs и tests). Значимые места складываются в одну production-цепочку:

- Компонент: `ui/coming_soon.rs:32-64` (`ComingSoon` поверх `MsgDialog`), base dialog `ui/msg_dialog.rs`, re-export `ui/mod.rs:68,84`.
- Модель меню: `menu/mod.rs:93` (`MenuKind::ComingSoon`), `:107` (`MenuState::coming_soon`), controller close/no-op `:216`, production draw `menu/draw.rs:34-35`.
- Наблюдаемая model-view форма: `model_view.rs:84-85,126-138` (`coming_soon_menu`). Это не отдельная reserved-фича, а сериализация состояния того же modal.
- Единственная production entry point: `copy/mod.rs:148-163,219-227` — `CopyFormat::Png` вызывает `png_coming_soon`, который открывает `MenuState::coming_soon("PNG export")`.
- Настройка формата: `copy/settings.rs:22-29,58` объявляет PNG и документирует переход к placeholder; тест цепочки — `copy/mod.rs:429-436`.
- Остальные executable-попадания — component/menu tests (`ui/coming_soon.rs:78-89`, `menu/mod.rs:430-450`).

Сверка:

- **PNG** назван точно: PROP-037 `#copy-png` требует открывать `ComingSoon`, а `#NG-PNG` фиксирует его как named non-goal-for-now. Код совпадает.
- **F1 Search Everywhere** больше не reserved: `#f1-search` прямо говорит, что рабочий search engine «supersedes the reserved ComingSoon stub». Отсутствие F1-placeholder корректно.
- **PlantUML/Mermaid** названы `#NG-PLANTUML` как будущие дополнения к списку форматов, но сегодня не выставлены как выбираемые entry points. Поэтому отдельный `ComingSoon` для них пока не пропущен.
- **AIUI / future StructureProvider** названы `#NG-AIUI`, но не являются открытой TUI-командой; placeholder им не требуется.
- **Лишних production reserved-мест нет**: generic `MenuState::coming_soon` имеет ровно одного runtime caller — PNG.
- **Недостающих открытых reserved-мест нет**: каждая реально доступная, но не построенная пользователю ветка в текущем списке — PNG — маршрутизируется в стандартный modal.

## Решения

- Считать атрибут над уже активно используемым типом/модулем stale даже тогда, когда внутри широкого scope остаётся один тестовый accessor: широкий allow получает `delete`, accessor — локальное удаление/`#[cfg(test)]` в том же строительном пакете.
- Не считать «future AIUI» достаточной причиной production `dead_code`: PROP-041 `#aiui-ready` требует чистую архитектуру, а `#non-goals` прямо исключает сам AIUI surface.
- Сохранить `with_scope` как единственную обоснованную контрактную extension surface: PROP-041 требует `scope_flag`, а production tree уже реализует branch. Причину оставить не историческую («S2»), а контрактную.
- Классифицировать `with_body` как `wire`, а не `delete`: декларация lazy body — явный REQ. Текущая модель (`Placeholder`, 0 чтений) не выполняет смысл этого REQ, несмотря на то что `Form::build` сам вызывается лишь после открытия.

## Самопроверка

### Счёт сайтов

```text
raw_literal_hits=40
attribute_sites=39
mapped_attribute_sites=39
```

Запрошенный неякорный поиск литерала видит 40 строк. Одна из них — **не атрибут**, а поясняющий комментарий `tree/tui/theme/mod.rs:40` с текстом `` `#[allow(dead_code)]` covers … ``. Синтаксически привязанный поиск `^\s*#\[allow\(dead_code` даёт 39; таблица содержит ровно 39 строк. Итого: **карта = grep фактических attribute sites = 39**.

### Build gate

```text
$ cargo check -p vibe-cli
    Checking vibe-cli v1.0.0 (...\crates\vibe-cli)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 37.80s
exit code: 0
```

Панель не запускалась. Git-команды не запускались. Исходники не изменялись; единственный созданный артефакт — этот отчёт.

## Deviations

- Вместо `grep` использован `rg`, как предписывает repository harness для поиска. Паттерн и область эквивалентны; для строгого счёта добавлен якорь начала строки, потому что буквальный паттерн совпадает ещё и с одним комментарием.
- Иных отклонений от пакета нет.

## Остатки

- Это замер, поэтому все 39 allow-сайтов намеренно остаются в исходниках до строительных пакетов.
- Снятие четырёх module-wide allow в `theme` и четырёх impl-wide allow у базовых widgets может проявить более мелкие внутренние dead items; строитель должен принять compiler output как второй уровень той же карты, а не вернуть широкий allow.
- Lazy-body задача содержит архитектурную развилку: оживить `PageBody` constructor как требует текущий текст PROP-041 либо доказанно свернуть дублирующий hook в уже ленивый `Form::build` и синхронно поправить контракт. Без одного из этих двух исходов site №33 нельзя считать осушённым.
- После реализации пакетов нужны как минимум `cargo fmt --all`, `cargo check -p vibe-cli`, релевантные TUI/prefs tests и обычные repo gates; в этом measurement-пакете запрошен и выполнен только `cargo check`.
