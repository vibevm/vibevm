# VIBE-NEEDS: архитектурное расширение кампании

Требования владельца от 2026-11-07 встроены в те же L0–L5. Исторические222задачи и113старых приёмок не перенумеровываются; это дополнительный обязательный owner scope.

**Приоритет:** среди одинаково готовых по архитектуре задач VIBE-NEEDS выполняются раньше остальных; зависимости не перескакиваются. Внутри потребностей №1 имеет P1.

| Узел | Слой | Результат | Зависимости |
|---|---|---|---|
| VN-SOURCE-FACTS | L0 | Единые факты выбранного и фактически полученного источника | существующие owners |
| VN-SOURCE-POLICY | L0 | Политика registry pin, hosting preference и exclusion | существующие owners |
| VN-EFFECT-CONTRACT | L0 | Границы plan/cancel и честные результаты эффектов | существующие owners |
| VN-ACTIVATION-INTENT | L0 | Desired enable/disable отдельно от observed agent projection | существующие owners |
| VN-ENV-POLICY | L0 | Политика durable environment и наблюдение profile | существующие owners |
| VN-SCAFFOLD-INPUT | L0 | Нормализованный выбор типа/цели/метода scaffold | существующие owners |
| VN-SOURCE-OBSERVATION | L1 | Сохранение фактов resolution/acquisition и причин кандидатов | VN-SOURCE-FACTS |
| VN-CONTACT-ADMISSION | L1 | Exclusion до probe/fetch/redirect/remap/fallback | VN-SOURCE-POLICY, VN-SOURCE-FACTS |
| VN-PREVIEW-INPUT | L1 | Read-only получение метаданных для полного плана | VN-EFFECT-CONTRACT, VN-CONTACT-ADMISSION |
| VN-DISK-COUNTER | L1 | Счётчик успешных logical writes в существующих transfer events | VN-EFFECT-CONTRACT |
| VN-SOURCE-CHOICE | L2 | Один resolver: pins и same-version hosting preference | VN-SOURCE-POLICY, VN-SOURCE-OBSERVATION, VN-CONTACT-ADMISSION |
| VN-PROJECT-PLAN | L2 | Полный план closure без product/cache/profile записей | VN-PREVIEW-INPUT, VN-SOURCE-CHOICE |
| VN-APP-PROVENANCE | L3 | Долговечный источник app через install/update/rollback/import | VN-SOURCE-OBSERVATION |
| VN-ACTIVATION-RECONCILE | L3 | Повторяемое disable/re-enable с сохранением ownership | VN-ACTIVATION-INTENT |
| VN-CANCEL-APPLY | L3 | Cooperative stop, guarded rollback и retained recovery | VN-EFFECT-CONTRACT |
| VN-ENV-ACTIVATION | L3 | Управляемая активация без неожиданного profile edit | VN-ENV-POLICY |
| VN-SCAFFOLD | L4 | Валидные scaffold выбранного способа установки | VN-SCAFFOLD-INPUT |
| VN-SURFACES | L5 | CLI/JSON/events из общих owners, не GUI-эвристики | VN-PROJECT-PLAN, VN-APP-PROVENANCE, VN-ACTIVATION-RECONCILE, VN-CANCEL-APPLY, VN-ENV-ACTIVATION, VN-SCAFFOLD, VN-DISK-COUNTER |
| VN-COMPAT-CHECK | Q | Проверить существующие scaffolds и B-224 без повторной реализации | существующие owners |

## Решения о границах

- Общие source facts/policy строятся на действующих resolver/registry/Locator. Не создаём второй solver или source world.
- Источник, выбранный заранее, и фактический путь получения различаются; commit/hosting/candidate reason могут быть unknown/not-probed.
- Preview не получает разрешения записывать cache только потому, что нынешний planner делает acquisition. Это отдельная effect capability, пока не реализована.
- Pins адресуют effective registry catalog текущей операции (project/user merge); неизвестный alias и несовместимые pins явно отказывают. Не добавляем скрытые registries из dependency metadata.
- Предпочтение hosting не меняет выбор версии само по себе; без настройки сохраняется действующая политика.
- Exclusion применяется до managed network contact, включая redirects/mirrors/remap/retry; непрозрачные transport paths должны быть квалифицированы или отказаны, не объявлены безопасными.
- Отмена local owned writes использует existing recovery. После remote commit нужен честный completed/unknown/recovery outcome; обещать восстановление старого remote мира нельзя.
- Profile observation не доказывает login effectiveness; unknown лучше выдуманного boolean.
- Уже существующие minimal MCP/remap/app scaffolds и B224 проверяются, а не переписываются.

## Первый срез

Общие source-evidence types и передача уже наблюдаемых фактов без потерь. Затем source policy/contact admission и общий resolver/preview. Durable app provenance использует ту же модель; публичные поверхности поступают по завершённым вертикалям. Ни один пункт VIBE-NEEDS пока не принят целиком.

Аудиты: `../research/vibe-needs-{sources,operations,entrypoints}-audit.md/json`. Статусы для app-владельца ведутся также в исходном VIBE-NEEDS файле.

### Обязательная граница VN-SOURCE-OBSERVATION: реальное project install event

Проверено в текущем main: `commands/install/observer.rs` передаёт в `report::present_resolution` только `ResolvedDep[]`; локальный `PlanEntry` выводит `package/version` в `packages`. Между тем `schemas/install_plan.jtd.json` описывает `plans` и обещает вывод перед подтверждением. Это отдельное расхождение живого producer и канонического контракта. Следующий source-observation consumer должен согласовать observer/carrier, фактически полученные source facts и generated schema, сохранив действительную последовательность событий. Нельзя объявлять project install event реализованным на основании app plan/list/describe или переносить в него сведения, восстановленные из текущих настроек. Этот пункт остаётся обязательным; текущий VN-SOURCE-FACTS его не закрывает.

### Принят общий источник фактов — 2df122323

VN-SOURCE-FACTS принят: generated evidence, durable app lifecycle/old unknown, фактические пути acquisition, корректный lock source_kind. Потребность2 частично обеспечена;1–3 целиком не приняты. Следующий consumer — реальное project install:plan через уже подготовленные данные и существующий observer, с исправлением расхождения packages/plans. Его фундамент готов; source policy/read-only preview/candidate reasons остаются отдельными обязательствами.

### Real project event accepted; exact Need2 scope corrected

Project event acceptedc600cb539 through the existing prepared plan/observer; current generated schema now matches packages. Full Need1 remains open. Exact original Need2 audit confirms inventory/list/describe/global-plan plus old unknown were all delivered by2df122323; standalone hosting classification was an extra condition incorrectly imposed by root, so Need2 is accepted in the documented Linux/source scope. Hosting policy/classification and project preview remain Need1/3 work. VN-APP-PROVENANCE depends on accepted SourceFacts, not on future skipped-candidate discovery; the initial dependency was unnecessarily broad. Historical222/113 unchanged.

### Hosting observation accepted — 1f0b53aa0

Known hosting labels now accompany project/global source reports through one pure core observer and shared report helper. Persisted facts unchanged; selected origin and differing recorded route remain distinct. This closes only the hosting-if-known reporting facet, not preference/exclusion or a no-contact guarantee. Contact audit identifies early index/raw HTTP probes, redirects/retries, warm Git origins, canonical marker fetches and submodules as actual next admission boundaries.
