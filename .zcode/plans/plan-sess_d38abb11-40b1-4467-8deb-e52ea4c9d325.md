# План: вынос vibeterm, vibeframe, vibe-launcher в vibevm-term

Две фазы: **1** — перенос packages (код + спеки + launcher + ideas-icons) в `vibevm-term`; **2** — Rust-правки в `vibevm` (убрать apps/launcher machinery из install, resolver через PATH) + полноценный version-manager в vibevm-term.

Дисциплина: Rule 1 (no AI attribution), Rule 3 (atomic Conventional Commits), Edit/Write only (not PowerShell), heredoc-commits.

## Принципы
- **Гибридная модель:** vibevm (опенсорсное ядро: package manager, vibe CLI, vibe tree, version manager) + vibevm-term (закрытые GUI-продукты: vibeterm, vibeframe, term-common, vibe-launcher). Интеграция бесшовная.
- App-код в `app/` внутри версии пакета.
- `common` = spec + общий JS-код (`@org.vibevm/term-common`, file:dep).
- **AI-Native Rust-стек подключается как static-transitive dep** во ВСЕ packages vibe-term (как fractality это делает: `requires.packages "stack:org.vibevm.ai-native/rust-ai-native" = "^0.7.0"` + `redbook = "^0.2.0"`, vendored в `vibedeps/`, boot-сниппеты ai-native в `spec/boot/INDEX.md`). Внутри vibe-term — практики AI-Native Rust: cells, `scope!`/specmark-equivalent traceability, strict tsconfig (для JS), branded types, Result errors, vitest.
- Apps/launchers пакуются сами через **собственный version-manager** (портированный с vibe vvm), кладут бинарь в `~/opt/bin`.
- App-имена в PATH: `vibeterm`/`vibeframe`.
- `vibe tree` fallback: если vibeframe не в PATH → in-place рендер.
- **vibe-launcher переносится целиком** в vibevm-term.
- `ideas-icons` переносится селективно.
- **PROP-019 контракты ПЕРЕНОСЯТСЯ в vibe-term** (репо self-contained): `common/v0.1.0/spec/modules/term-common/PROP-vvm.md` — полный normative контракт version-manager'а (instance layout, current pointer, dedup-skip, relocate, verb set). Cross-link к `spec://vibevm/common/PROP-019#root` — provenance (Rust twin), как PROP-039 ↔ vibeterm-core.

# ФАЗА 1 — Перенос packages в vibevm-term

## Целевая структура `C:/Users/olegc/git/v/vibevm-term/`

```
vibevm-term/
├── README.md, AGENTS/CLAUDE/GEMINI.md, .gitignore, vibe.toml
└── org.vibevm.term/
    ├── common/v0.1.0/          # shared spec + JS + version-manager library
    │   ├── vibe.toml           # requires: redbook ^0.2.0, rust-ai-native ^0.7.0 (static-transitive)
    │   ├── vibe.lock
    │   ├── package.json (@org.vibevm/term-common, ESM)
    │   ├── vibedeps/           # vendored static deps (populated by `vibe install`)
    │   │   ├── flow-redbook/0.2.0/
    │   │   ├── stack-rust-ai-native-lang/0.7.0/
    │   │   └── ...
    │   ├── src/{args.mjs, keymap.mjs, packaging.mjs}
    │   ├── vvm/                # НОВОЕ (Фаза 2): ported vibe version-manager
    │   │   ├── store.mjs, placer.mjs, builder.mjs, model.mjs
    │   │   ├── relocate.mjs, env.mjs, doctor.mjs, cli.mjs
    │   ├── test/{args,keymap}.test.mjs
    │   └── spec/
    │       ├── boot/{INDEX.md (incl ai-native static snippets), 00-core.md, 10-tool-term-common.md}
    │       ├── WAL.md
    │       └── modules/term-common/
    │           ├── PROP-control-protocol.md, PROP-discovery.md
    │           ├── PROP-icon-osc.md, PROP-pty-lifecycle.md
    │           ├── PROP-env-markers.md, PROP-electron-packaging.md
    │           ├── PROP-vvm.md          # НОВОЕ (Фаза 2): ported PROP-019 contract
    │           └── PROP-path-install.md # НОВОЕ (Фаза 2)
    ├── launcher/v0.1.0/        # vibe-launcher Rust workspace
    │   ├── vibe.toml           # requires: redbook, rust-ai-native (static-transitive)
    │   ├── Cargo.toml ([workspace]), crates/vibe-launcher/ (из vibevm целиком)
    │   │   └── assets/icons/{vibetree,vibeterm,vibeframe}.{ico,png,svg}
    │   ├── vibedeps/
    │   ├── bin/self.mjs(Ф2), scripts/install.mjs(Ф2)
    │   └── spec/{boot/, modules/vibe-launcher/{PROP-043-gui-launchers, PROP-043-self-install(Ф2)}.md, WAL.md}
    ├── vibeterm/v0.1.0/
    │   ├── vibe.toml           # requires: common =0.1.0, redbook, rust-ai-native (static-transitive)
    │   ├── AGENTS/CLAUDE/GEMINI.md
    │   ├── app/                # из apps/vibeterm/
    │   ├── vibedeps/
    │   ├── bin/self.mjs(Ф2), scripts/install.mjs(Ф2)
    │   └── spec/
    │       ├── boot/, WAL.md
    │       ├── modules/vibeterm/{PROP-044,PROP-046,PROP-047,architecture,design-system,PROP-self-install(Ф2)}.md
    │       ├── terraforms/, research/, manual-tests/ (перенесены)
    │       └── ideas-icons/{vibeterm/, enlarged-attempts/vibeterm-enlarged.{png,svg}}
    └── vibeframe/v0.1.0/
        ├── vibe.toml           # requires: common, redbook, rust-ai-native (static-transitive)
        ├── app/ (из apps/vibeframe/)
        ├── vibedeps/
        ├── bin/self.mjs(Ф2), scripts/install.mjs(Ф2)
        └── spec/{boot/, modules/vibeframe/{PROP-045, PROP-self-install(Ф2)}.md, WAL.md}
```

**Не переносим:** `research/vibeterm/projectx-function-map.md` (third-party), `apps/*/dist/`, `package-lock.json` (regen), `ideas-icons/{default,palette,README,vibetree-*,charcoal/plum/espresso/mint/amber variants}` (общее + host-side).

## URI scheme
- `spec://vibevm/modules/vibeterm/PROP-044#regions` → `spec://vibeterm/PROP-044#regions`
- `spec://vibevm/modules/vibeframe/PROP-045#role` → `spec://vibeframe/PROP-045#role`
- `spec://vibevm/modules/vibe-launcher/PROP-043#root` → `spec://vibe-launcher/PROP-043#root`
- `spec://vibevm/common/PROP-019#root` → **`spec://term-common/PROP-vvm#root`** (ported contract, provenance к Rust twin в vibevm)
- common-контракты: `spec://term-common/PROP-icon-osc#root`
- Cross-refs НА host (другие PROP'ы vibe-cli/vibe-tree если появятся) — остаются `spec://vibevm/...`, помечаем `(host repo, cross-repo contract)`.

## Шаги Фазы 1 (атомарные коммиты в vibevm-term)

**A.0** Скелет репо: `git init`, `.gitignore`, `README.md`, `AGENTS/CLAUDE/GEMINI.md`, `vibe.toml`. `chore(term): repo skeleton`.

**B. common-пакет:**
- B.1 Скелет с `requires.packages` (redbook + rust-ai-native static-transitive), `vibe install` для populate `vibedeps/`. `feat(term-common): package skeleton with ai-native static deps`.
- B.2 `src/{args,keymap}.mjs` + tests (AI-Native TS discipline: strict tsconfig для shared code, branded types где уместно). `feat(term-common): shared arg/keymap helpers`.
- B.3 Generalized `src/packaging.mjs`. `feat(term-common): generalized electron-packaging driver`.
- B.4 Spec контракты `spec/modules/term-common/` (6 PROP'ов; PROP-vvm и PROP-path-install — Фаза 2). `feat(term-common): normative shared contracts`.

**C. launcher-пакет:**
- C.1 Скелет с ai-native static deps. `feat(launcher): package skeleton`.
- C.2 Перенос `crates/vibe-launcher/` целиком + `assets/icons/*`, адаптировать `build.rs`. `feat(launcher): move vibe-launcher crate from vibevm`.
- C.3 Verify `cargo build -p vibe-launcher` зелёный. `test(launcher): build verifies after the move`.
- C.4 Перенос `PROP-043-gui-launchers.md` с rewrite URI; убрать host-side REQs про `#self-install`. `feat(launcher): normative spec PROP-043`.
- C.5 `spec/WAL.md`. `feat(launcher): initial WAL checkpoint`.

**D. vibeterm-пакет:** D.1 Скелет с ai-native static deps + common=0.1.0. D.2 Перенос app-кода → `app/`. `feat(vibeterm): move Electron app into app/`. D.3 Refactor term-common imports + AI-Native TS discipline (strict tsconfig.engine.json переезжает, branded types, Result errors). `refactor(vibeterm): consume term-common + ai-native TS discipline`. D.4 Перенос спек `PROP-044/046/047` + lore с rewrite URI. D.5 Перенос terraforms/research/MT. D.6 Перенос `ideas-icons/vibeterm/` + enlarged. D.7 WAL.

**E. vibeframe-пакет (аналогично D):** E.1–E.6.

**F. Verify Ф1:** F.1 `vibe install` в каждом package populate `vibedeps/` корректно. F.2 `npm install && npm test` в apps. F.3 `cargo build -p vibe-launcher` зелёный. F.4 Grep: nothing не ссылается на несуществующий `spec://vibevm/modules/vibeterm|vibeframe|vibe-launcher/...`. F.5 AI-Native floor: `tsc --noEmit` + `vitest` зелёные, specmap-traceability присутствует. F.6 TL;DR.

# ФАЗА 2 — Rust-правки в vibevm (2a) + version-manager в vibevm-term (2b)

## Фаза 2a — изменения в vibevm (атомарные коммиты)

**G. install.rs:** G.1 Убрать вызовы `packager.package(...)` + `walk_vibeterm_dist` + параметр `packager`. G.2 Убрать параметр `launchers` + `launchers.refresh(...)`. G.3 Адаптировать тесты. `refactor(vvm): stop packaging/building apps+launchers`.

**H.** Удалить `vvm/launchers.rs` + `mod launchers;`. `refactor(vvm): remove the launcher-installer machinery`.
**I.** Удалить `vvm/vibeterm_packager.rs` + `mod vibeterm_packager;`. `refactor(vvm): remove the vibeterm-packager seam`.
**J. vvm/doctor.rs:** Убрать `vibeterm_packaged` advisory + `packaged_vibeterm_exe()` + node/npm-packaging advisory. `refactor(vvm): drop the packaged-vibeterm and packaging-tools advisories`.

**K. term.rs:** K.1 `resolve_app` новые tiers: Tier 1 env → Tier 2 `<instance>/<app>/` (back-compat) → Tier 3 PATH lookup → (удалить Tier 4 dev walk-up). K.2 Расширить `VibetermShape`. K.3 NotFound для vibeframe при caller `vibe tree` → `LaunchDecision::InPlace` → `open_in_vibeterm` рендерит через `host::run_upgraded`. K.4 Убрать silent fallback.

**L. vibe-launcher crate:** L.1 Удалить `crates/vibe-launcher/`. L.2 Убрать из workspace members + `Cargo.lock`. L.3 Удалить `assets/icons/{vibetree,vibeterm,vibeframe}.*`.

**M. tools/:** M.1 `self-check.sh` убрать vibeterm gate. M.2 `first-run.sh` убрать apps build.

**N. Спеки:** N.1 `PROP-019 §2.7` переписать (apps/launchers external products), удалить `#self-install` — note «moved to spec://term-common/PROP-vvm». N.2 `PROP-042 §4/§5/§5.1` — terminal-side REQs → cross-repo contract. N.3 `conform.toml` — comments остаются.

**O.** `specmap.json` — удалить blocks `PROP-043/044/045/046/047`, regenerate.

**P. Удалить перенесённое из vibevm:** P.1 `apps/vibeterm/`, `apps/vibeframe/`. P.2 `spec/modules/{vibeterm,vibeframe,vibe-launcher}/`. P.3 `research/vibeterm/` (кроме projectx-function-map.md). P.4 terraforms (TERMINAL-AIUI, VIBE-LAUNCHERS). P.5 manual-tests (MT-04, MT-05). P.6 ideas-icons/vibeterm + enlarged (vibeterm/vibeframe).

**Q. Verify Ф2a:** Q.1 `self-check.sh` зелёный. Q.2 `cargo build` workspace зелёный. Q.3 `cargo test -p vibe-cli` зелёный. Q.4 `vibe tree` без apps → in-place. Q.5 `vibe tree` с vibeframe в PATH → открывает. Q.6 `vibe self install` не package'ит apps/launchers. Q.7 `vibe self doctor` без advisory.

## Фаза 2b — version-manager в vibevm-term (полноценный port с PROP-019 контрактами)

Портируем `crates/vibe-cli/src/commands/vvm/` (Rust) в TypeScript (`common/v0.1.0/vvm/`) + портируем `spec/common/PROP-019-version-manager.md` целиком в `common/v0.1.0/spec/modules/term-common/PROP-vvm.md`. AI-Native TS discipline для всего нового кода.

`<product> self <verb>` CLI: `install|update|use|ls|current|which|doctor|remove|gc|env|relocate`. State в `~/opt/<product>/` (аналог `~/opt/vibevm/`).

### R. common: normative contract port (PROP-019 → PROP-vvm)
- **R.0** `common/v0.1.0/spec/modules/term-common/PROP-vvm.md` — port `PROP-019-version-manager.md` целиком: instance layout, `current` pointer, dedup-skip, selector model, `relocate` (source_path rewrite), verb set, install lock, self-install placement. Cross-link `spec://vibevm/common/PROP-019#root` как provenance (Rust twin, shared identity-grammar). `feat(term-common): PROP-vvm ported from PROP-019`.
- **R.1** `PROP-path-install.md` — `~/opt/<product>` placement, rename-aside, shortcuts. `feat(term-common): PROP-path-install normative contract`.

### S. common: version-manager library (port из vibe vvm, AI-Native TS)
- **S.1** `common/v0.1.0/vvm/model.mjs` — port `model.rs` (`VersionId`, `Kind`, `Profile`, `InstallRecord`, `Selector::parse`, `State`) с branded types. `feat(term-common): vvm model (ported)`.
- **S.2** `common/v0.1.0/vvm/store.mjs` — port `store.rs` (`VersionStore`, `instance_dir`, `current` pointer, `state.toml`) с `Result` errors. `feat(term-common): vvm store (ported)`.
- **S.3** `common/v0.1.0/vvm/placer.mjs` — port `placer.rs` (diff-copy по manifest, atomic instance creation). `feat(term-common): vvm placer (ported)`.
- **S.4** `common/v0.1.0/vvm/builder.mjs` — Builder abstraction: `ElectronAppBuilder` (→ `packaging.mjs`), `LauncherBuilder` (→ `cargo build -p vibe-launcher`). `feat(term-common): vvm builder abstraction`.
- **S.5** `common/v0.1.0/vvm/install.mjs` — port `install.rs::perform_install` (lock, build, place с dedup-skip, record provenance incl. `source_path`, flip `current`, refresh shortcut). `feat(term-common): vvm perform_install (ported)`.
- **S.6** `common/v0.1.0/vvm/relocate.mjs` — port `relocate.rs` (repoint `source_path`, удалить instances из старого tree; active — repoint не delete). `feat(term-common): vvm relocate (ported)`.
- **S.7** `common/v0.1.0/vvm/env.mjs` — port `env.rs` (detect `VIBEVM_INSTALL_ROOT`/`~/opt`, shims, shell). `feat(term-common): vvm env (ported)`.
- **S.8** `common/v0.1.0/vvm/doctor.mjs` — port `doctor.rs` (probe instance, shim dir on PATH, active version). `feat(term-common): vvm doctor (ported)`.
- **S.9** `common/v0.1.0/vvm/cli.mjs` — CLI dispatcher `parse(argv) → { verb, args }`. `feat(term-common): vvm CLI dispatcher`.
- **S.10** Port тесты из vibe vvm (`store.rs`, `placer.rs`, `install.rs`, `relocate/tests.rs`) в `common/v0.1.0/vvm/*.test.mjs` — conformance к PROP-vvm. `test(term-common): vvm port conformance`.

### T. vibeterm self CLI
- **T.1** `vibeterm/v0.1.0/bin/self.mjs` — entry: `import { vvmMain } from '@org.vibevm/term-common/vvm/cli'; vvmMain({ product: 'vibeterm', builder: 'electron-app' })`. Все verbs делегируют в common vvm. `feat(vibeterm): self CLI (install/update/use/relocate/...)`.
- **T.2** `scripts/install.mjs` wrapper. `feat(vibeterm): install wrapper`.
- **T.3** `PROP-self-install.md` — продукт-специфичный контракт (packaged exe через `@electron/packager`, GUI-subsystem). `feat(vibeterm): PROP-self-install`.
- **T.4** Verify: `node bin/self.mjs install latest` → instance, `current`, shim в `~/opt/bin/vibeterm`. `self use`/`relocate`/`ls` работают. `test(vibeterm): version-manager works`.

### U. vibeframe self CLI (аналогично T):** U.1–U.4.

### V. launcher self CLI (Rust-builder)
Launcher'ы — Rust-бинари. Version-manager переиспользует common vvm, `Builder = LauncherBuilder` (`cargo build -p vibe-launcher --release`). State в `~/opt/launcher/`. Три exe (`vibetree`/`vibeterm`/`vibeframe`) — один instance (один крейт), шимы/shortcuts для каждого.
- **V.1** `launcher/v0.1.0/bin/self.mjs` с `product='launcher'`, `builder='cargo-vibe-launcher'`. `feat(launcher): self CLI with cargo-builder`.
- **V.2** `scripts/install.mjs`. `feat(launcher): install wrapper`.
- **V.3** `PROP-043-self-install.md` — extracted-product `#self-install`: `~/opt/launcher/`, place 3 exe, shortcuts. `feat(launcher): PROP-043 self-install`.
- **V.4** Verify: `launcher self install` → 3 exe в `~/opt/bin`, shortcuts. `test(launcher): version-manager works`.

### W. Verify Ф2b
- **W.1** После всех `*-self install` → `~/opt/bin` содержит `vibe` + `vibeterm`/`vibeframe`/`vibetree` (последние — launcher bin'ы). Все в PATH.
- **W.2** `vibe tree` находит vibeframe в PATH → открывает.
- **W.3** `vibeterm self use <selector>` переключает; `vibeterm self current`/`ls` работают.
- **W.4** `vibeterm self relocate <new-path>` после перемещения репо — переписывает source provenance, `ls` не показывает stale.
- **W.5** Port-тесты conformance между Rust vibe-vvm и TS term-vvm зелёные (те же входы → те же outputs на уровне модели).
- **W.6** AI-Native floor зелёный (`tsc`, `vitest`, specmap-traceability) во всех packages vibe-term.

## Риски и guardrails
- **AI-Native static deps:** после B.1/C.1/D.1/E.1 запустить `vibe install` для populate `vibedeps/`. Если resolve fails (local registry не находит ai-native) — fallback: copy vendored из `fractality/v0.1.0/vibedeps/`. Зафиксировать в WAL.
- **spec:// rewrite + contract port:** PROP-vvm — полный port PROP-019 (не paraphrase), anchored под `spec://term-common/PROP-vvm#<section>`; cross-link к Rust twin как provenance. Конformance goldens (S.10) — тот же набор тестов что Rust side (по модели).
- **Фаза 2b Rust→TS port fidelity:** сохранять все инварианты (dedup-skip по manifest, source_path provenance, atomic instance flip). Cells с single registration points, no sibling coupling.
- **Фаза 2a G+H+I+L atomicity:** install.rs + launchers.rs + vibeterm_packager.rs + удаление vibe-launcher согласованы.
- **file:dep к term-common:** если npm блокирует, vendored fallback.
- **node-pty/Electron binary:** требует network; offline → apps не стартуют, packages не нарушены.
- **Фаза 2a K.3 in-place fallback:** behaviour-чувствительный; тестировать Windows + Git Bash.
- **Фаза 2b rename-aside:** locked-running бинарь rename в `.old-<n>` перед overwrite; sweep при следующем update.
- **Фаза 2b cross-platform:** Windows shortcuts (PowerShell `WScript.Shell`); Linux `.desktop` / macOS `.app` tracked separately.
- **Дисциплина:** Edit/Write only; heredoc-commits; NO AI attribution.

## Что НЕ в scope
- Публикация vibe-term пакетов в registry `vibespecs` (пока только локальный `file:///.../vibevm-term`).
- Linux `.desktop` / macOS `.app` install-machinery (Windows first).
- Conformance-golden инфра (CI) между Rust vibe-vvm и TS term-vvm — тесты port'ятся и запускаются в обоих репо, но единый golden-fixture с cross-floor drift detection — отдельный заход.