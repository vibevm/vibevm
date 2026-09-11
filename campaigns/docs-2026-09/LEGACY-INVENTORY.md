# Инвентарь утверждений legacy-документации (P0-C4 / спайк A0.13)

Дерево: `C:\Users\olegc\git\v\vibevm-docs`. Бинарник: `target/debug/vibe.exe` (`vibe --version` = см. WORKER-REPORT-P0-C4.md).

Типы: `cmd` — команда `vibe …`; `path` — путь файла/каталога; `section` — секция/поле манифеста; `flag` — флаг CLI из текста.

| файл | заголовок раздела | утверждение | тип | метка | как проверено | решение |
| --- | --- | --- | --- | --- | --- | --- |
| docs/ALPHA-NOTES.md | Recovery after a breaking update | vibe update --all | cmd | живо | vibe.exe update --help -> exit 0 | перенесён → howto/update-packages#root |
| docs/ALPHA-NOTES.md | Recovery after a breaking update | --all | flag | живо | найден в собранном корпусе `--help` | перенесён → howto/update-packages#root |
| docs/ALPHA-NOTES.md | Recovery after a breaking update | vibe init --path . | cmd | живо | vibe.exe init --help -> exit 0 | перенесён → howto/use-a-private-registry#machine-wide |
| docs/ALPHA-NOTES.md | Recovery after a breaking update | --path | flag | живо | найден в собранном корпусе `--help` | перенесён → howto/update-packages#by-hand |
| docs/ALPHA-NOTES.md | Recovery after a breaking update | vibe install | cmd | живо | vibe.exe install --help -> exit 0 | перенесён → howto/install-a-package#what-happens |
| docs/ALPHA-NOTES.md | Recovery after a breaking update | vibe.lock | path | живо | найден в дереве (имя файла): vibe.lock | перенесён → reference/lock-file#root |
| docs/ALPHA-NOTES.md | Recovery after a breaking update | vibe cache check | cmd | живо | vibe.exe cache check --help -> exit 0 | перенесён → howto/update-packages#recovery |
| docs/ALPHA-NOTES.md | Recovery after a breaking update | vibe cache clean --package org.vibevm.world/wal | cmd | живо | vibe.exe cache clean --help -> exit 0 | перенесён → howto/update-packages#recovery |
| docs/ALPHA-NOTES.md | Recovery after a breaking update | --package | flag | живо | найден в собранном корпусе `--help` | перенесён → howto/update-packages#recovery |
| docs/ALPHA-NOTES.md | Recovery after a breaking update | vibe cache clean --all | cmd | живо | vibe.exe cache clean --help -> exit 0 | перенесён → howto/update-packages#recovery |
| docs/ALPHA-NOTES.md | Recovery after a breaking update | vibe cache clean | cmd | живо | vibe.exe cache clean --help -> exit 0 | перенесён → howto/update-packages#recovery |
| docs/ALPHA-NOTES.md | Recovery after a breaking update | --older-than | flag | живо | найден в собранном корпусе `--help` | перенесён → reference/commands#registries |
| docs/ALPHA-NOTES.md | Known alpha limitations (2026-08-20) | --quiet | flag | живо | найден в собранном корпусе `--help` | перенесён → reference/commands#global |
| docs/ALPHA-NOTES.md | Known alpha limitations (2026-08-20) | --show-origins | flag | устарело | не найден ни в одном собранном `--help` | снят: устарело |
| docs/ALPHA-NOTES.md | Known alpha limitations (2026-08-20) | vibe prefs show-origins [key] | cmd | живо | vibe.exe prefs show-origins --help -> exit 0 | перенесён → reference/settings-and-environment#inside-a-project |
| docs/ALPHA-NOTES.md | Known alpha limitations (2026-08-20) | [key] | section | живо | найдено в crates/vibe-core/src/manifest/artifact/binary_projection.rs:29 | перенесён → reference/commands#root (derived cli-help) |
| docs/ALPHA-NOTES.md | Where to look before updating | vibe <command> --help | cmd | неизвестно | нет подкоманды, флаг не опознан в `vibe --help` | снят: устарело |
| docs/ALPHA-NOTES.md | Where to look before updating | --help | flag | живо | найден в собранном корпусе `--help` | перенесён → howto/read-documentation-locally#by-hand |
| docs/architecture.md | System model | content_hash | section | живо | найдено в crates/vibe-core/src/manifest/document/tests.rs:292 | перенесён → reference/lock-file#package-entries |
| docs/architecture.md | System model | [[registry]] | section | живо | найдено в crates/vibe-core/src/manifest/artifact/tests_validation.rs:403 | перенесён → reference/manifest#sources |
| docs/architecture.md | System model | index_url | section | живо | найдено в crates/vibe-core/src/manifest/document/tests.rs:571 | перенесён → reference/manifest#sources |
| docs/architecture.md | System model | ~/.vibe/cache/ | path | живо | найден на диске (эта машина): C:\Users\olegc\.vibe\cache | перенесён → model/lock-and-store#the-store |
| docs/architecture.md | System model | vibedeps/ | path | неизвестно | генерируемый каталог (материализуется `vibe install`), механическая проверка по дереву неинформативна для непроинсталлированного чекаута | перенесён → model/two-trees#the-rule |
| docs/architecture.md | System model | vibe.lock | path | живо | найден в дереве (имя файла): vibe.lock | перенесён → reference/lock-file#root |
| docs/architecture.md | System model | --offline | flag | живо | найден в собранном корпусе `--help` | перенесён → howto/work-offline#root |
| docs/architecture.md | Main install path | vibe install | cmd | живо | vibe.exe install --help -> exit 0 | перенесён → howto/install-a-package#what-happens |
| docs/architecture.md | Main install path | vibe.toml | path | живо | найден в дереве (имя файла): vibe.toml | перенесён → reference/manifest#root |
| docs/architecture.md | Main install path | vibe update | cmd | живо | vibe.exe update --help -> exit 0 | перенесён → howto/update-packages#root |
| docs/architecture.md | Main install path | vibe uninstall | cmd | живо | vibe.exe uninstall --help -> exit 0 | перенесён → howto/remove-a-package#what-happens |
| docs/architecture.md | Main install path | vibe cache clean | cmd | живо | vibe.exe cache clean --help -> exit 0 | перенесён → howto/update-packages#recovery |
| docs/architecture.md | Storage layout | ~/.vibe/ | path | живо | найден на диске (эта машина): C:\Users\olegc\.vibe | перенесён → reference/settings-and-environment#the-folder |
| docs/architecture.md | Reading order for a contributor | vibe <command> --help | cmd | неизвестно | нет подкоманды, флаг не опознан в `vibe --help` | снят: устарело |
| docs/architecture.md | Reading order for a contributor | --help | flag | живо | найден в собранном корпусе `--help` | перенесён → howto/read-documentation-locally#by-hand |
| docs/authoring-feat.md | Authoring a `feat` package | vibe install feat:<name> | cmd | живо | vibe.exe install --help -> exit 0 | перенесён → howto/install-a-package#what-happens |
| docs/authoring-feat.md | Authoring a `feat` package | vibe build feat:<name> --stack <stack-name> | cmd | живо | vibe.exe build --help -> exit 0 | перенесён → reference/manifest#deliveries |
| docs/authoring-feat.md | Authoring a `feat` package | --stack | flag | живо | найден в собранном корпусе `--help` | перенесён → reference/commands#projects |
| docs/authoring-feat.md | Anatomy of a feat package | [package] | section | живо | найдено в crates/vibe-core/src/manifest/artifact/tests_validation.rs:372 | перенесён → reference/lock-file#package-entries |
| docs/authoring-feat.md | Anatomy of a feat package | vibe.toml | path | живо | найден в дереве (имя файла): vibe.toml | перенесён → reference/manifest#root |
| docs/authoring-feat.md | Anatomy of a feat package | vibedeps/ | path | неизвестно | генерируемый каталог (материализуется `vibe install`), механическая проверка по дереву неинформативна для непроинсталлированного чекаута | перенесён → model/two-trees#the-rule |
| docs/authoring-feat.md | Anatomy of a feat package | vibe install | cmd | живо | vibe.exe install --help -> exit 0 | перенесён → howto/install-a-package#what-happens |
| docs/authoring-feat.md | Acceptance criteria | vibe build | cmd | живо | vibe.exe build --help -> exit 0 | перенесён → reference/manifest#deliveries |
| docs/authoring-feat.md | Manifest: `vibe.toml` | [compatibility] | section | живо | найдено в crates/vibe-core/src/manifest/artifact/tests_validation.rs:429 | перенесён → reference/manifest#requirements |
| docs/authoring-feat.md | Manifest: `vibe.toml` | min_vibe_version | section | живо | найдено в crates/vibe-core/src/manifest/artifact/tests_validation.rs:430 | перенесён → reference/manifest#requirements |
| docs/authoring-feat.md | Manifest: `vibe.toml` | [requires] | section | живо | найдено в crates/vibe-core/src/manifest/document.rs:22 | перенесён → reference/manifest#requirements |
| docs/authoring-feat.md | Manifest: `vibe.toml` | [[requires_any]] | section | живо | найдено в crates/vibe-core/src/manifest/document/validation.rs:53 | перенесён → reference/manifest#requirements |
| docs/authoring-feat.md | Manifest: `vibe.toml` | requires_kinds | section | живо | найдено в crates/vibe-core/src/manifest/package/tests.rs:70 | перенесён → reference/manifest#requirements |
| docs/authoring-feat.md | Manifest: `vibe.toml` | [writes] | section | живо | найдено в crates/vibe-core/src/manifest/document.rs:25 | снят: устарело — таблицы нет в схеме манифеста (crates/vibe-core/src/manifest, проверено 2026-09-12) |
| docs/authoring-feat.md | Manifest: `vibe.toml` | [boot_snippet] | section | живо | найдено в crates/vibe-core/src/manifest/document/tests.rs:81 | перенесён → reference/manifest#deliveries |
| docs/authoring-feat.md | Manifest: `vibe.toml` | [provides] | section | живо | найдено в crates/vibe-core/src/manifest/document/tests.rs:84 | перенесён → reference/manifest#requirements |
| docs/authoring-feat.md | Manifest: `vibe.toml` | [requires.packages] | section | живо | найдено в crates/vibe-core/src/manifest/artifact/tests_validation.rs:398 | перенесён → reference/manifest#requirements |
| docs/authoring-feat.md | Manifest: `vibe.toml` | one_of | section | живо | найдено в crates/vibe-core/src/manifest/artifact/tests.rs:254 | перенесён → reference/manifest#requirements |
| docs/authoring-feat.md | Manifest: `vibe.toml` | vibe check | cmd | живо | vibe.exe check --help -> exit 0 | перенесён → howto/remove-a-package#root |
| docs/authoring-feat.md | Publishing | vibe registry publish ./path/to/your/feat-package | cmd | живо | vibe.exe registry publish --help -> exit 0 | перенесён → howto/publish-a-package#root |
| docs/authoring-feat.md | Publishing | vibe registry publish | cmd | живо | vibe.exe registry publish --help -> exit 0 | перенесён → howto/publish-a-package#root |
| docs/authoring-flow.md | Authoring a `flow` package | vibedeps/ | path | неизвестно | генерируемый каталог (материализуется `vibe install`), механическая проверка по дереву неинформативна для непроинсталлированного чекаута | перенесён → model/two-trees#the-rule |
| docs/authoring-flow.md | Anatomy of a flow package | [package] | section | живо | найдено в crates/vibe-core/src/manifest/artifact/tests_validation.rs:372 | перенесён → reference/lock-file#package-entries |
| docs/authoring-flow.md | Anatomy of a flow package | vibe.toml | path | живо | найден в дереве (имя файла): vibe.toml | перенесён → reference/manifest#root |
| docs/authoring-flow.md | Anatomy of a flow package | vibe install flow:<name> | cmd | живо | vibe.exe install --help -> exit 0 | перенесён → howto/install-a-package#what-happens |
| docs/authoring-flow.md | Anatomy of a flow package | vibe install | cmd | живо | vibe.exe install --help -> exit 0 | перенесён → howto/install-a-package#what-happens |
| docs/authoring-flow.md | Anatomy of a flow package | [writes] | section | живо | найдено в crates/vibe-core/src/manifest/document.rs:25 | снят: устарело — таблицы нет в схеме манифеста (crates/vibe-core/src/manifest, проверено 2026-09-12) |
| docs/authoring-flow.md | The boot snippet | [boot_snippet] | section | живо | найдено в crates/vibe-core/src/manifest/document/tests.rs:81 | перенесён → reference/manifest#deliveries |
| docs/authoring-flow.md | Manifest: `vibe.toml` | [compatibility] | section | живо | найдено в crates/vibe-core/src/manifest/artifact/tests_validation.rs:429 | перенесён → reference/manifest#requirements |
| docs/authoring-flow.md | Manifest: `vibe.toml` | min_vibe_version | section | живо | найдено в crates/vibe-core/src/manifest/artifact/tests_validation.rs:430 | перенесён → reference/manifest#requirements |
| docs/authoring-flow.md | Manifest: `vibe.toml` | [requires.packages] | section | живо | найдено в crates/vibe-core/src/manifest/artifact/tests_validation.rs:398 | перенесён → reference/manifest#requirements |
| docs/authoring-flow.md | Manifest: `vibe.toml` | [provides] | section | живо | найдено в crates/vibe-core/src/manifest/document/tests.rs:84 | перенесён → reference/manifest#requirements |
| docs/authoring-flow.md | Manifest: `vibe.toml` | [requires] | section | живо | найдено в crates/vibe-core/src/manifest/document.rs:22 | перенесён → reference/manifest#requirements |
| docs/authoring-flow.md | Manifest: `vibe.toml` | [[requires_any]] | section | живо | найдено в crates/vibe-core/src/manifest/document/validation.rs:53 | перенесён → reference/manifest#requirements |
| docs/authoring-flow.md | Manifest: `vibe.toml` | [obsoletes] | section | живо | найдено в crates/vibe-core/src/manifest/document/validation.rs:56 | перенесён → reference/manifest#requirements |
| docs/authoring-flow.md | Manifest: `vibe.toml` | [conflicts] | section | живо | найдено в crates/vibe-core/src/manifest/document/validation.rs:59 | перенесён → reference/manifest#requirements |
| docs/authoring-flow.md | Publishing | vibe registry publish ./path/to/your/flow-package | cmd | живо | vibe.exe registry publish --help -> exit 0 | перенесён → howto/publish-a-package#root |
| docs/authoring-flow.md | Publishing | vibe registry publish | cmd | живо | vibe.exe registry publish --help -> exit 0 | перенесён → howto/publish-a-package#root |
| docs/authoring-flow.md | Tips | vibe install flow:<name> --registry ./path/to/your/dir | cmd | живо | vibe.exe install --help -> exit 0 | перенесён → howto/install-a-package#what-happens |
| docs/authoring-flow.md | Tips | --registry | flag | живо | найден в собранном корпусе `--help` | перенесён → howto/publish-a-package#by-hand |
| docs/authoring-stack.md | Authoring a `stack` package | vibe install stack:<name> | cmd | живо | vibe.exe install --help -> exit 0 | перенесён → howto/install-a-package#what-happens |
| docs/authoring-stack.md | Authoring a `stack` package | vibe build feat:<name> --stack rust-cli | cmd | живо | vibe.exe build --help -> exit 0 | перенесён → reference/manifest#deliveries |
| docs/authoring-stack.md | Authoring a `stack` package | --stack | flag | живо | найден в собранном корпусе `--help` | перенесён → reference/commands#projects |
| docs/authoring-stack.md | Anatomy of a stack package | [package] | section | живо | найдено в crates/vibe-core/src/manifest/artifact/tests_validation.rs:372 | перенесён → reference/lock-file#package-entries |
| docs/authoring-stack.md | Anatomy of a stack package | vibe.toml | path | живо | найден в дереве (имя файла): vibe.toml | перенесён → reference/manifest#root |
| docs/authoring-stack.md | Anatomy of a stack package | vibedeps/ | path | неизвестно | генерируемый каталог (материализуется `vibe install`), механическая проверка по дереву неинформативна для непроинсталлированного чекаута | перенесён → model/two-trees#the-rule |
| docs/authoring-stack.md | Anatomy of a stack package | vibe install | cmd | живо | vibe.exe install --help -> exit 0 | перенесён → howto/install-a-package#what-happens |
| docs/authoring-stack.md | Manifest: `vibe.toml` | [compatibility] | section | живо | найдено в crates/vibe-core/src/manifest/artifact/tests_validation.rs:429 | перенесён → reference/manifest#requirements |
| docs/authoring-stack.md | Manifest: `vibe.toml` | min_vibe_version | section | живо | найдено в crates/vibe-core/src/manifest/artifact/tests_validation.rs:430 | перенесён → reference/manifest#requirements |
| docs/authoring-stack.md | Manifest: `vibe.toml` | [writes] | section | живо | найдено в crates/vibe-core/src/manifest/document.rs:25 | снят: устарело — таблицы нет в схеме манифеста (crates/vibe-core/src/manifest, проверено 2026-09-12) |
| docs/authoring-stack.md | Manifest: `vibe.toml` | [boot_snippet] | section | живо | найдено в crates/vibe-core/src/manifest/document/tests.rs:81 | перенесён → reference/manifest#deliveries |
| docs/authoring-stack.md | Manifest: `vibe.toml` | [provides] | section | живо | найдено в crates/vibe-core/src/manifest/document/tests.rs:84 | перенесён → reference/manifest#requirements |
| docs/authoring-stack.md | Manifest: `vibe.toml` | [requires] | section | живо | найдено в crates/vibe-core/src/manifest/document.rs:22 | перенесён → reference/manifest#requirements |
| docs/authoring-stack.md | Manifest: `vibe.toml` | [requires.packages] | section | живо | найдено в crates/vibe-core/src/manifest/artifact/tests_validation.rs:398 | перенесён → reference/manifest#requirements |
| docs/authoring-stack.md | Publishing | vibe registry publish ./path/to/your/stack-package | cmd | живо | vibe.exe registry publish --help -> exit 0 | перенесён → howto/publish-a-package#root |
| docs/authoring-stack.md | Publishing | vibe registry publish | cmd | живо | vibe.exe registry publish --help -> exit 0 | перенесён → howto/publish-a-package#root |
| docs/commands/cache.md | `vibe cache` | vibe cache | cmd | живо | vibe.exe cache --help -> exit 0 | перенесён → howto/update-packages#recovery |
| docs/commands/cache.md | `vibe cache` | ~/.vibe/cache/ | path | живо | найден на диске (эта машина): C:\Users\olegc\.vibe\cache | перенесён → model/lock-and-store#the-store |
| docs/commands/cache.md | Usage | vibe cache [OPTIONS] <COMMAND> | cmd | живо | vibe.exe cache --help -> exit 0 | перенесён → howto/update-packages#recovery |
| docs/commands/cache.md | Subcommands | vibe cache path | cmd | живо | vibe.exe cache path --help -> exit 0 | перенесён → model/lock-and-store#root |
| docs/commands/cache.md | Subcommands | vibe cache list | cmd | живо | vibe.exe cache list --help -> exit 0 | перенесён → howto/read-documentation-locally#root |
| docs/commands/cache.md | Subcommands | vibe cache add <PACKAGES>... | cmd | живо | vibe.exe cache add --help -> exit 0 | перенесён → howto/read-documentation-locally#what-happens |
| docs/commands/cache.md | Subcommands | vibedeps/ | path | неизвестно | генерируемый каталог (материализуется `vibe install`), механическая проверка по дереву неинформативна для непроинсталлированного чекаута | перенесён → model/two-trees#the-rule |
| docs/commands/cache.md | Subcommands | vibe cache clean | cmd | живо | vibe.exe cache clean --help -> exit 0 | перенесён → howto/update-packages#recovery |
| docs/commands/cache.md | Subcommands | --all | flag | живо | найден в собранном корпусе `--help` | перенесён → howto/update-packages#root |
| docs/commands/cache.md | Subcommands | --package | flag | живо | найден в собранном корпусе `--help` | перенесён → howto/update-packages#recovery |
| docs/commands/cache.md | Subcommands | --older-than | flag | живо | найден в собранном корпусе `--help` | перенесён → reference/commands#registries |
| docs/commands/cache.md | Subcommands | vibe cache check | cmd | живо | vibe.exe cache check --help -> exit 0 | перенесён → howto/update-packages#recovery |
| docs/commands/cache.md | Subcommands | vibe cache check --repair | cmd | живо | vibe.exe cache check --help -> exit 0 | перенесён → howto/update-packages#recovery |
| docs/commands/cache.md | Subcommands | vibe update | cmd | живо | vibe.exe update --help -> exit 0 | перенесён → howto/update-packages#root |
| docs/commands/cache.md | Subcommands | --repair | flag | живо | найден в собранном корпусе `--help` | перенесён → reference/commands#registries |
| docs/commands/cache.md | Examples | vibe cache add org.vibevm.world/wal --path ./my-project | cmd | живо | vibe.exe cache add --help -> exit 0 | перенесён → howto/read-documentation-locally#what-happens |
| docs/commands/cache.md | Examples | --path | flag | живо | найден в собранном корпусе `--help` | перенесён → howto/update-packages#by-hand |
| docs/commands/cache.md | Examples | vibe cache check --repair --path ./my-project | cmd | живо | vibe.exe cache check --help -> exit 0 | перенесён → howto/update-packages#recovery |
| docs/commands/cache.md | Examples | vibe cache clean --older-than 90 | cmd | живо | vibe.exe cache clean --help -> exit 0 | перенесён → howto/update-packages#recovery |
| docs/commands/cache.md | Examples | vibe cache clean --package org.vibevm.world/wal@1.0.0 | cmd | живо | vibe.exe cache clean --help -> exit 0 | перенесён → howto/update-packages#recovery |
| docs/commands/cache.md | Examples | vibe cache clean --all | cmd | живо | vibe.exe cache clean --help -> exit 0 | перенесён → howto/update-packages#recovery |
| docs/commands/cache.md | Examples | --json | flag | живо | найден в собранном корпусе `--help` | перенесён → reference/commands#global |
| docs/commands/cache.md | Examples | --quiet | flag | живо | найден в собранном корпусе `--help` | перенесён → reference/commands#global |
| docs/commands/cache.md | Examples | --invoked-by | flag | живо | найден в собранном корпусе `--help` | перенесён → reference/commands#global |
| docs/commands/cache.md | Examples | --unattended | flag | живо | найден в собранном корпусе `--help` | перенесён → reference/commands#global |
| docs/commands/cache.md | Examples | --offline | flag | живо | найден в собранном корпусе `--help` | перенесён → howto/work-offline#root |
| docs/commands/cache.md | Related | vibe install | cmd | живо | vibe.exe install --help -> exit 0 | перенесён → howto/install-a-package#what-happens |
| docs/commands/check.md | `vibe check` | vibe check | cmd | живо | vibe.exe check --help -> exit 0 | перенесён → howto/remove-a-package#root |
| docs/commands/check.md | Usage | vibe check [OPTIONS] | cmd | живо | vibe.exe check --help -> exit 0 | перенесён → howto/remove-a-package#root |
| docs/commands/check.md | Options | --path | flag | живо | найден в собранном корпусе `--help` | перенесён → howto/update-packages#by-hand |
| docs/commands/check.md | Options | --wal-max-age-hours | flag | живо | найден в собранном корпусе `--help` | перенесён → reference/commands#projects |
| docs/commands/check.md | Options | --review-max-age-days | flag | живо | найден в собранном корпусе `--help` | перенесён → reference/commands#projects |
| docs/commands/check.md | Options | --offline | flag | живо | найден в собранном корпусе `--help` | перенесён → howto/work-offline#root |
| docs/commands/check.md | Options | --json | flag | живо | найден в собранном корпусе `--help` | перенесён → reference/commands#global |
| docs/commands/check.md | Options | --quiet | flag | живо | найден в собранном корпусе `--help` | перенесён → reference/commands#global |
| docs/commands/check.md | Options | --invoked-by | flag | живо | найден в собранном корпусе `--help` | перенесён → reference/commands#global |
| docs/commands/check.md | Options | --unattended | flag | живо | найден в собранном корпусе `--help` | перенесён → reference/commands#global |
| docs/commands/check.md | Examples | vibe check --path ./my-project --wal-max-age-hours 48 | cmd | живо | vibe.exe check --help -> exit 0 | перенесён → howto/remove-a-package#root |
| docs/commands/check.md | Examples | vibe check --json | cmd | живо | vibe.exe check --help -> exit 0 | перенесён → howto/remove-a-package#root |
| docs/commands/check.md | Related | vibe show | cmd | живо | vibe.exe show --help -> exit 0 | перенесён → howto/remove-a-package#what-happens |
| docs/commands/check.md | Related | vibe tree | cmd | живо | vibe.exe tree --help -> exit 0 | перенесён → model/boot-lane#root |
| docs/commands/check.md | Related | vibe install | cmd | живо | vibe.exe install --help -> exit 0 | перенесён → howto/install-a-package#what-happens |
| docs/commands/init.md | `vibe init` | vibe init | cmd | живо | vibe.exe init --help -> exit 0 | перенесён → howto/use-a-private-registry#machine-wide |
| docs/commands/init.md | Usage | vibe init [OPTIONS] [POSITIONAL]... | cmd | живо | vibe.exe init --help -> exit 0 | перенесён → howto/use-a-private-registry#machine-wide |
| docs/commands/init.md | Usage | vibe init <project-name> | cmd | живо | vibe.exe init --help -> exit 0 | перенесён → howto/use-a-private-registry#machine-wide |
| docs/commands/init.md | Usage | vibe init <group> <project-name> | cmd | живо | vibe.exe init --help -> exit 0 | перенесён → howto/use-a-private-registry#machine-wide |
| docs/commands/init.md | Usage | vibe init <group>/<package> | cmd | живо | vibe.exe init --help -> exit 0 | перенесён → howto/use-a-private-registry#machine-wide |
| docs/commands/init.md | Usage | vibe init package <group>/<package> [path] | cmd | живо | vibe.exe init --help -> exit 0 | перенесён → howto/set-up-a-workspace#what-happens |
| docs/commands/init.md | Usage | [path] | section | живо | найдено в crates/vibe-core/src/manifest/lockfile/tests.rs:2 | перенесён → reference/commands#root (derived cli-help) |
| docs/commands/init.md | Usage | vibe init group <group> [path] | cmd | живо | vibe.exe init --help -> exit 0 | перенесён → howto/use-a-private-registry#machine-wide |
| docs/commands/init.md | Options | --path | flag | живо | найден в собранном корпусе `--help` | перенесён → howto/update-packages#by-hand |
| docs/commands/init.md | Options | --name | flag | живо | найден в собранном корпусе `--help` | перенесён → reference/commands#projects |
| docs/commands/init.md | Options | --stack | flag | живо | найден в собранном корпусе `--help` | перенесён → reference/commands#projects |
| docs/commands/init.md | Options | vibe.toml | path | живо | найден в дереве (имя файла): vibe.toml | перенесён → reference/manifest#root |
| docs/commands/init.md | Options | --registry-url | flag | живо | найден в собранном корпусе `--help` | перенесён → reference/commands#projects |
| docs/commands/init.md | Options | --registry-ref | flag | живо | найден в собранном корпусе `--help` | перенесён → reference/commands#projects |
| docs/commands/init.md | Options | --no-registry | flag | живо | найден в собранном корпусе `--help` | перенесён → reference/commands#projects |
| docs/commands/init.md | Options | --kind | flag | живо | найден в собранном корпусе `--help` | перенесён → model/packages-and-kinds#the-kinds |
| docs/commands/init.md | Options | --version | flag | живо | найден в собранном корпусе `--help` | перенесён → start/index#step-2 |
| docs/commands/init.md | Options | --author | flag | живо | найден в собранном корпусе `--help` | перенесён → reference/commands#projects |
| docs/commands/init.md | Options | --license | flag | живо | найден в собранном корпусе `--help` | перенесён → reference/commands#projects |
| docs/commands/init.md | Options | --description | flag | живо | найден в собранном корпусе `--help` | перенесён → reference/commands#projects |
| docs/commands/init.md | Options | --format | flag | живо | найден в собранном корпусе `--help` | перенесён → reference/commands#projects |
| docs/commands/init.md | Options | --link | flag | живо | найден в собранном корпусе `--help` | перенесён → reference/commands#projects |
| docs/commands/init.md | Options | --json | flag | живо | найден в собранном корпусе `--help` | перенесён → reference/commands#global |
| docs/commands/init.md | Options | --quiet | flag | живо | найден в собранном корпусе `--help` | перенесён → reference/commands#global |
| docs/commands/init.md | Options | --invoked-by | flag | живо | найден в собранном корпусе `--help` | перенесён → reference/commands#global |
| docs/commands/init.md | Options | --unattended | flag | живо | найден в собранном корпусе `--help` | перенесён → reference/commands#global |
| docs/commands/init.md | Options | --offline | flag | живо | найден в собранном корпусе `--help` | перенесён → howto/work-offline#root |
| docs/commands/init.md | Examples | vibe init hello-vibe | cmd | живо | vibe.exe init --help -> exit 0 | перенесён → start/first-project#what-happens |
| docs/commands/init.md | Examples | vibe init org.example hello-vibe | cmd | живо | vibe.exe init --help -> exit 0 | перенесён → howto/use-a-private-registry#machine-wide |
| docs/commands/init.md | Examples | vibe init package org.example/tools ./hello-vibe | cmd | живо | vibe.exe init --help -> exit 0 | перенесён → howto/set-up-a-workspace#what-happens |
| docs/commands/init.md | Examples | vibe init group org.example ./hello-vibe | cmd | живо | vibe.exe init --help -> exit 0 | перенесён → howto/use-a-private-registry#machine-wide |
| docs/commands/init.md | Related | vibe install | cmd | живо | vibe.exe install --help -> exit 0 | перенесён → howto/install-a-package#what-happens |
| docs/commands/init.md | Related | vibe check | cmd | живо | vibe.exe check --help -> exit 0 | перенесён → howto/remove-a-package#root |
| docs/commands/install.md | `vibe install` | vibe install | cmd | живо | vibe.exe install --help -> exit 0 | перенесён → howto/install-a-package#what-happens |
| docs/commands/install.md | `vibe install` | [requires] | section | живо | найдено в crates/vibe-core/src/manifest/document.rs:22 | перенесён → reference/manifest#requirements |
| docs/commands/install.md | `vibe install` | vibe.toml | path | живо | найден в дереве (имя файла): vibe.toml | перенесён → reference/manifest#root |
| docs/commands/install.md | Usage | vibe install [OPTIONS] [PACKAGES]... | cmd | живо | vibe.exe install --help -> exit 0 | перенесён → howto/install-a-package#what-happens |
| docs/commands/install.md | Important options | --path | flag | живо | найден в собранном корпусе `--help` | перенесён → howto/update-packages#by-hand |
| docs/commands/install.md | Important options | --assume-yes | flag | живо | найден в собранном корпусе `--help` | перенесён → howto/remove-a-package#by-hand |
| docs/commands/install.md | Important options | --exact | flag | живо | найден в собранном корпусе `--help` | перенесён → howto/install-a-package#constraints |
| docs/commands/install.md | Important options | --features | flag | живо | найден в собранном корпусе `--help` | перенесён → reference/commands#projects |
| docs/commands/install.md | Important options | [features] | section | живо | найдено в crates/vibe-core/src/manifest/document/validation.rs:86 | перенесён → reference/manifest#requirements |
| docs/commands/install.md | Important options | --no-default-features | flag | живо | найден в собранном корпусе `--help` | перенесён → reference/commands#projects |
| docs/commands/install.md | Important options | --all-features | flag | живо | найден в собранном корпусе `--help` | перенесён → reference/commands#projects |
| docs/commands/install.md | Important options | --language | flag | живо | найден в собранном корпусе `--help` | перенесён → reference/commands#projects |
| docs/commands/install.md | Important options | --solver | flag | живо | найден в собранном корпусе `--help` | перенесён → reference/commands#projects |
| docs/commands/install.md | Important options | --auth-required | flag | живо | найден в собранном корпусе `--help` | перенесён → howto/use-a-private-registry#edge-cases |
| docs/commands/install.md | Important options | --offline | flag | живо | найден в собранном корпусе `--help` | перенесён → howto/work-offline#root |
| docs/commands/install.md | Important options | --registry | flag | живо | найден в собранном корпусе `--help` | перенесён → howto/publish-a-package#by-hand |
| docs/commands/install.md | Important options | --git | flag | живо | найден в собранном корпусе `--help` | перенесён → reference/commands#projects |
| docs/commands/install.md | Important options | --tag | flag | живо | найден в собранном корпусе `--help` | перенесён → start/install-vibe#edge-cases |
| docs/commands/install.md | Important options | --branch | flag | живо | найден в собранном корпусе `--help` | перенесён → reference/commands#projects |
| docs/commands/install.md | Important options | --rev | flag | живо | найден в собранном корпусе `--help` | перенесён → reference/commands#projects |
| docs/commands/install.md | Important options | --git-auth | flag | живо | найден в собранном корпусе `--help` | перенесён → reference/commands#projects |
| docs/commands/install.md | Important options | --git-token-env | flag | живо | найден в собранном корпусе `--help` | перенесён → reference/commands#projects |
| docs/commands/install.md | Important options | --prefer-embedded | flag | живо | найден в собранном корпусе `--help` | перенесён → reference/commands#projects |
| docs/commands/install.md | Important options | --no-prefer-embedded | flag | живо | найден в собранном корпусе `--help` | перенесён → reference/commands#projects |
| docs/commands/install.md | Important options | --no-default-registry | flag | живо | найден в собранном корпусе `--help` | перенесён → reference/commands#projects |
| docs/commands/install.md | Important options | --prefer-local | flag | живо | найден в собранном корпусе `--help` | перенесён → reference/commands#projects |
| docs/commands/install.md | Important options | --no-prefer-local | flag | живо | найден в собранном корпусе `--help` | перенесён → reference/commands#projects |
| docs/commands/install.md | Important options | --allow-hooks | flag | устарело | не найден ни в одном собранном `--help` | снят: устарело |
| docs/commands/install.md | Important options | vibe install --help | cmd | живо | vibe.exe install --help -> exit 0 | перенесён → howto/install-a-package#what-happens |
| docs/commands/install.md | Important options | --json | flag | живо | найден в собранном корпусе `--help` | перенесён → reference/commands#global |
| docs/commands/install.md | Important options | --quiet | flag | живо | найден в собранном корпусе `--help` | перенесён → reference/commands#global |
| docs/commands/install.md | Important options | --invoked-by | flag | живо | найден в собранном корпусе `--help` | перенесён → reference/commands#global |
| docs/commands/install.md | Important options | --unattended | flag | живо | найден в собранном корпусе `--help` | перенесён → reference/commands#global |
| docs/commands/install.md | Important options | --help | flag | живо | найден в собранном корпусе `--help` | перенесён → howto/read-documentation-locally#by-hand |
| docs/commands/install.md | Examples | vibe install org.vibevm.world/wal | cmd | живо | vibe.exe install --help -> exit 0 | перенесён → howto/install-a-package#what-happens |
| docs/commands/install.md | Examples | vibe install --offline --path ./my-project | cmd | живо | vibe.exe install --help -> exit 0 | перенесён → howto/install-a-package#what-happens |
| docs/commands/install.md | Examples | vibe install tool:example --git https://example.com/example.git --tag v1.0.0 | cmd | живо | vibe.exe install --help -> exit 0 | перенесён → howto/install-a-package#what-happens |
| docs/commands/install.md | Examples | vibedeps/ | path | неизвестно | генерируемый каталог (материализуется `vibe install`), механическая проверка по дереву неинформативна для непроинсталлированного чекаута | перенесён → model/two-trees#the-rule |
| docs/commands/install.md | Examples | ~/.vibe/cache/ | path | живо | найден на диске (эта машина): C:\Users\olegc\.vibe\cache | перенесён → model/lock-and-store#the-store |
| docs/commands/install.md | Examples | vibe.lock | path | живо | найден в дереве (имя файла): vibe.lock | перенесён → reference/lock-file#root |
| docs/commands/install.md | Related | vibe update | cmd | живо | vibe.exe update --help -> exit 0 | перенесён → howto/update-packages#root |
| docs/commands/install.md | Related | vibe uninstall | cmd | живо | vibe.exe uninstall --help -> exit 0 | перенесён → howto/remove-a-package#what-happens |
| docs/commands/install.md | Related | vibe cache | cmd | живо | vibe.exe cache --help -> exit 0 | перенесён → howto/update-packages#recovery |
| docs/commands/list.md | `vibe list` | vibe list | cmd | живо | vibe.exe list --help -> exit 0 | перенесён → reference/machine-formats#root |
| docs/commands/list.md | Usage | vibe list [OPTIONS] | cmd | живо | vibe.exe list --help -> exit 0 | перенесён → reference/machine-formats#root |
| docs/commands/list.md | Options | --path | flag | живо | найден в собранном корпусе `--help` | перенесён → howto/update-packages#by-hand |
| docs/commands/list.md | Options | --kind | flag | живо | найден в собранном корпусе `--help` | перенесён → model/packages-and-kinds#the-kinds |
| docs/commands/list.md | Options | --verbose | flag | живо | найден в собранном корпусе `--help` | перенесён → reference/commands#projects |
| docs/commands/list.md | Options | --offline | flag | живо | найден в собранном корпусе `--help` | перенесён → howto/work-offline#root |
| docs/commands/list.md | Options | --json | flag | живо | найден в собранном корпусе `--help` | перенесён → reference/commands#global |
| docs/commands/list.md | Options | --quiet | flag | живо | найден в собранном корпусе `--help` | перенесён → reference/commands#global |
| docs/commands/list.md | Options | --invoked-by | flag | живо | найден в собранном корпусе `--help` | перенесён → reference/commands#global |
| docs/commands/list.md | Options | --unattended | flag | живо | найден в собранном корпусе `--help` | перенесён → reference/commands#global |
| docs/commands/list.md | Examples | vibe list --kind flow --verbose | cmd | живо | vibe.exe list --help -> exit 0 | перенесён → reference/machine-formats#root |
| docs/commands/list.md | Examples | vibe list --json --path ./my-project | cmd | живо | vibe.exe list --help -> exit 0 | перенесён → reference/machine-formats#root |
| docs/commands/list.md | Related | vibe show | cmd | живо | vibe.exe show --help -> exit 0 | перенесён → howto/remove-a-package#what-happens |
| docs/commands/list.md | Related | vibe tree | cmd | живо | vibe.exe tree --help -> exit 0 | перенесён → model/boot-lane#root |
| docs/commands/list.md | Related | vibe check | cmd | живо | vibe.exe check --help -> exit 0 | перенесён → howto/remove-a-package#root |
| docs/commands/mcp-install.md | `vibe mcp install` — wire vibevm into a coding agent | vibe mcp install | cmd | живо | vibe.exe mcp install --help -> exit 0 | перенесён → authoring/ship-tools-and-mcp-servers#servers |
| docs/commands/mcp-install.md | Two scopes — project vs user | vibe.toml | path | живо | найден в дереве (имя файла): vibe.toml | перенесён → reference/manifest#root |
| docs/commands/mcp-install.md | Two scopes — project vs user | --scope | flag | живо | найден в собранном корпусе `--help` | перенесён → agent/give-your-agent-the-skill#by-hand |
| docs/commands/mcp-install.md | Two scopes — project vs user | --path | flag | живо | найден в собранном корпусе `--help` | перенесён → howto/update-packages#by-hand |
| docs/commands/mcp-install.md | Two scopes — project vs user | vibe mcp upgrade | cmd | живо | vibe.exe mcp upgrade --help -> exit 0 | перенесён → agent/give-your-agent-the-skill#edge-cases |
| docs/commands/mcp-install.md | Two scopes — project vs user | vibe mcp uninstall | cmd | живо | vibe.exe mcp uninstall --help -> exit 0 | перенесён → agent/give-your-agent-the-skill#edge-cases |
| docs/commands/mcp-install.md | Two install kinds — MCP and SKILL.md | --what | flag | живо | найден в собранном корпусе `--help` | перенесён → reference/commands#agents |
| docs/commands/mcp-install.md | Supported agents | config.toml | path | живо | найден в дереве (имя файла): config.toml | перенесён → howto/work-offline#switching-it-on |
| docs/commands/mcp-install.md | Usage | vibe mcp install [--path <dir>] | cmd | живо | vibe.exe mcp install --help -> exit 0 | перенесён → authoring/ship-tools-and-mcp-servers#servers |
| docs/commands/mcp-install.md | Usage | --agent | flag | живо | найден в собранном корпусе `--help` | перенесён → agent/give-your-agent-the-skill#by-hand |
| docs/commands/mcp-install.md | Usage | --auto | flag | живо | найден в собранном корпусе `--help` | перенесён → agent/give-your-agent-the-skill#what-happens |
| docs/commands/mcp-install.md | Usage | --dry-run | flag | живо | найден в собранном корпусе `--help` | перенесён → howto/publish-a-package#root |
| docs/commands/mcp-install.md | Usage | --yes | flag | живо | найден в собранном корпусе `--help` | перенесён → agent/give-your-agent-the-skill#what-happens |
| docs/commands/mcp-install.md | Usage | --force | flag | живо | найден в собранном корпусе `--help` | перенесён → model/two-trees#what-regenerates |
| docs/commands/mcp-install.md | Flags | --project-vs-user | flag | устарело | не найден ни в одном собранном `--help` | снят: устарело |
| docs/commands/mcp-install.md | Flags | --mcp-and-skillmd | flag | устарело | не найден ни в одном собранном `--help` | снят: устарело |
| docs/commands/mcp-install.md | Flags | vibe install | cmd | живо | vibe.exe install --help -> exit 0 | перенесён → howto/install-a-package#what-happens |
| docs/commands/mcp-install.md | Flags | --assume-yes | flag | живо | найден в собранном корпусе `--help` | перенесён → howto/remove-a-package#by-hand |
| docs/commands/mcp-install.md | Flags | --unattended | flag | живо | найден в собранном корпусе `--help` | перенесён → reference/commands#global |
| docs/commands/mcp-install.md | Flags | --json | flag | живо | найден в собранном корпусе `--help` | перенесён → reference/commands#global |
| docs/commands/mcp-install.md | Flags | --quiet | flag | живо | найден в собранном корпусе `--help` | перенесён → reference/commands#global |
| docs/commands/mcp-install.md | Flags | invoked_by | section | устарело | поле не найдено ни в manifest source, ни в примерах vibe.toml | перенесён → reference/machine-formats#the-envelope |
| docs/commands/mcp-install.md | Flags | --invoked-by | flag | живо | найден в собранном корпусе `--help` | перенесён → reference/commands#global |
| docs/commands/mcp-install.md | Bootstrap (first install, no project yet) | vibe mcp install --auto --scope user | cmd | живо | vibe.exe mcp install --help -> exit 0 | перенесён → authoring/ship-tools-and-mcp-servers#servers |
| docs/commands/mcp-install.md | Bootstrap (first install, no project yet) | vibe mcp install --agent opencode --scope user --what both | cmd | живо | vibe.exe mcp install --help -> exit 0 | перенесён → authoring/ship-tools-and-mcp-servers#servers |
| docs/commands/mcp-install.md | Project-level (already inside a vibevm project) | vibe mcp install --auto | cmd | живо | vibe.exe mcp install --help -> exit 0 | перенесён → authoring/ship-tools-and-mcp-servers#servers |
| docs/commands/mcp-install.md | Project-level (already inside a vibevm project) | vibe mcp install --agent opencode --scope project --what skill | cmd | живо | vibe.exe mcp install --help -> exit 0 | перенесён → authoring/ship-tools-and-mcp-servers#servers |
| docs/commands/mcp-install.md | Both — project pinned + user fallback | vibe mcp install --scope both --auto | cmd | живо | vibe.exe mcp install --help -> exit 0 | перенесён → authoring/ship-tools-and-mcp-servers#servers |
| docs/commands/mcp-install.md | Provisioning a fresh user account (no project yet) | vibe --unattended mcp install \ | cmd | живо | глобальный флаг --unattended найден в `vibe --help` | перенесён → reference/commands#global |
| docs/commands/mcp-install.md | Pre-flight diff before applying | vibe mcp install --auto --dry-run | cmd | живо | vibe.exe mcp install --help -> exit 0 | перенесён → authoring/ship-tools-and-mcp-servers#servers |
| docs/commands/mcp-install.md | Human-readable | vibevm/SKILL.md | path | устарело | путь не существует: vibevm/SKILL.md | перенесён → agent/give-your-agent-the-skill#what-happens |
| docs/commands/mcp-install.md | Output (JSON) | mcp_servers | section | живо | найдено в crates/vibe-core/src/manifest/document/tests.rs:660 | перенесён → reference/machine-formats#root (derived jtd-schema) |
| docs/commands/mcp-install.md | Codex (TOML, `mcp_servers`) | [mcp_servers.vibevm] | section | устарело | не найдено ни в manifest source, ни в примерах vibe.toml | снят: устарело |
| docs/commands/mcp-install.md | SKILL.md (Claude Code, OpenCode, Codex) | vibe init | cmd | живо | vibe.exe init --help -> exit 0 | перенесён → howto/use-a-private-registry#machine-wide |
| docs/commands/mcp-install.md | SKILL.md (Claude Code, OpenCode, Codex) | vibe <subcmd> --help | cmd | неизвестно | нет подкоманды, флаг не опознан в `vibe --help` | снят: устарело |
| docs/commands/mcp-install.md | SKILL.md (Claude Code, OpenCode, Codex) | --help | flag | живо | найден в собранном корпусе `--help` | перенесён → howto/read-documentation-locally#by-hand |
| docs/commands/mcp-install.md | SKILL.md (Claude Code, OpenCode, Codex) | crates/vibe-cli/src/commands/skill_template.md | path | устарело | путь не существует: crates/vibe-cli/src/commands/skill_template.md | снят: устарело |
| docs/commands/mcp-install.md | Related | vibe mcp status | cmd | живо | vibe.exe mcp status --help -> exit 0 | перенесён → agent/give-your-agent-the-skill#root |
| docs/commands/mcp-install.md | Related | vibe mcp serve | cmd | живо | vibe.exe mcp serve --help -> exit 0 | перенесён → agent/ask-your-agent#two-transports |
| docs/commands/mcp-install.md | Related | vibe show config | cmd | живо | vibe.exe show config --help -> exit 0 | перенесён → reference/settings-and-environment#edge-cases |
| docs/commands/mcp-serve.md | `vibe mcp serve` — Model Context Protocol server | vibe mcp serve | cmd | живо | vibe.exe mcp serve --help -> exit 0 | перенесён → agent/ask-your-agent#two-transports |
| docs/commands/mcp-serve.md | `vibe mcp serve` — Model Context Protocol server | vibe mcp install | cmd | живо | vibe.exe mcp install --help -> exit 0 | перенесён → authoring/ship-tools-and-mcp-servers#servers |
| docs/commands/mcp-serve.md | `vibe mcp serve` — Model Context Protocol server | vibe install | cmd | живо | vibe.exe install --help -> exit 0 | перенесён → howto/install-a-package#what-happens |
| docs/commands/mcp-serve.md | Usage | vibe mcp serve [--path <dir>] | cmd | живо | vibe.exe mcp serve --help -> exit 0 | перенесён → agent/ask-your-agent#two-transports |
| docs/commands/mcp-serve.md | Usage | --path | flag | живо | найден в собранном корпусе `--help` | перенесён → howto/update-packages#by-hand |
| docs/commands/mcp-serve.md | Flags | vibe.toml | path | живо | найден в дереве (имя файла): vibe.toml | перенесён → reference/manifest#root |
| docs/commands/mcp-serve.md | Tools exposed | query_package | section | устарело | поле не найдено ни в manifest source, ни в примерах vibe.toml | снят: устарело |
| docs/commands/mcp-serve.md | Tools exposed | files_written | section | живо | найдено в crates/vibe-core/src/manifest/lockfile/tests.rs:38 | перенесён → reference/machine-formats#root (derived jtd-schema) |
| docs/commands/mcp-serve.md | Tools exposed | read_subskill | section | живо | найдено в crates/vibe-core/src/manifest/lockfile.rs:473 | снят: внутренний интерфейс MCP-сервера; список инструментов вернётся генератором (X-031) |
| docs/commands/mcp-serve.md | Tools exposed | materialise_subskill | section | устарело | поле не найдено ни в manifest source, ни в примерах vibe.toml | снят: устарело |
| docs/commands/mcp-serve.md | Edge cases | vibe.lock | path | живо | найден в дереве (имя файла): vibe.lock | перенесён → reference/lock-file#root |
| docs/commands/mcp-serve.md | Edge cases | vibe update | cmd | живо | vibe.exe update --help -> exit 0 | перенесён → howto/update-packages#root |
| docs/commands/mcp-serve.md | Related | vibe mcp status | cmd | живо | vibe.exe mcp status --help -> exit 0 | перенесён → agent/give-your-agent-the-skill#root |
| docs/commands/mcp-serve.md | Related | vibe show subskills | cmd | живо | vibe.exe show subskills --help -> exit 0 | перенесён → howto/remove-a-package#what-happens |
| docs/commands/mcp-status.md | `vibe mcp status` — preview agent integration state | vibe mcp status | cmd | живо | vibe.exe mcp status --help -> exit 0 | перенесён → agent/give-your-agent-the-skill#root |
| docs/commands/mcp-status.md | `vibe mcp status` — preview agent integration state | vibe mcp install | cmd | живо | vibe.exe mcp install --help -> exit 0 | перенесён → authoring/ship-tools-and-mcp-servers#servers |
| docs/commands/mcp-status.md | `vibe mcp status` — preview agent integration state | vibe mcp upgrade | cmd | живо | vibe.exe mcp upgrade --help -> exit 0 | перенесён → agent/give-your-agent-the-skill#edge-cases |
| docs/commands/mcp-status.md | Usage | vibe mcp status [--path <dir>] | cmd | живо | vibe.exe mcp status --help -> exit 0 | перенесён → agent/give-your-agent-the-skill#root |
| docs/commands/mcp-status.md | Usage | --path | flag | живо | найден в собранном корпусе `--help` | перенесён → howto/update-packages#by-hand |
| docs/commands/mcp-status.md | Flags | vibe.toml | path | живо | найден в дереве (имя файла): vibe.toml | перенесён → reference/manifest#root |
| docs/commands/mcp-status.md | Flags | --json | flag | живо | найден в собранном корпусе `--help` | перенесён → reference/commands#global |
| docs/commands/mcp-status.md | Flags | --quiet | flag | живо | найден в собранном корпусе `--help` | перенесён → reference/commands#global |
| docs/commands/mcp-status.md | Flags | invoked_by | section | устарело | поле не найдено ни в manifest source, ни в примерах vibe.toml | перенесён → reference/machine-formats#the-envelope |
| docs/commands/mcp-status.md | Flags | --invoked-by | flag | живо | найден в собранном корпусе `--help` | перенесён → reference/commands#global |
| docs/commands/mcp-status.md | Human-readable | config.toml | path | живо | найден в дереве (имя файла): config.toml | перенесён → howto/work-offline#switching-it-on |
| docs/commands/mcp-status.md | Output (JSON) | vibevm/SKILL.md | path | устарело | путь не существует: vibevm/SKILL.md | перенесён → agent/give-your-agent-the-skill#what-happens |
| docs/commands/mcp-status.md | Output (JSON) | vibe mcp install --dry-run | cmd | живо | vibe.exe mcp install --help -> exit 0 | перенесён → authoring/ship-tools-and-mcp-servers#servers |
| docs/commands/mcp-status.md | Output (JSON) | skill_results | section | устарело | поле не найдено ни в manifest source, ни в примерах vibe.toml | снят: устарело |
| docs/commands/mcp-status.md | Output (JSON) | --dry-run | flag | живо | найден в собранном корпусе `--help` | перенесён → howto/publish-a-package#root |
| docs/commands/mcp-status.md | CI usage — drift gate | vibe --json mcp status \ | cmd | живо | глобальный флаг --json найден в `vibe --help` | перенесён → reference/commands#global |
| docs/commands/mcp-status.md | Related | vibe mcp uninstall | cmd | живо | vibe.exe mcp uninstall --help -> exit 0 | перенесён → agent/give-your-agent-the-skill#edge-cases |
| docs/commands/mcp-status.md | Related | vibe mcp serve | cmd | живо | vibe.exe mcp serve --help -> exit 0 | перенесён → agent/ask-your-agent#two-transports |
| docs/commands/mcp-uninstall.md | `vibe mcp uninstall` — remove vibevm from coding agents | vibe mcp uninstall | cmd | живо | vibe.exe mcp uninstall --help -> exit 0 | перенесён → agent/give-your-agent-the-skill#edge-cases |
| docs/commands/mcp-uninstall.md | `vibe mcp uninstall` — remove vibevm from coding agents | vibe mcp install | cmd | живо | vibe.exe mcp install --help -> exit 0 | перенесён → authoring/ship-tools-and-mcp-servers#servers |
| docs/commands/mcp-uninstall.md | `vibe mcp uninstall` — remove vibevm from coding agents | --config-only | flag | живо | найден в собранном корпусе `--help` | перенесён → reference/commands#agents |
| docs/commands/mcp-uninstall.md | `vibe mcp uninstall` — remove vibevm from coding agents | --skill-only | flag | живо | найден в собранном корпусе `--help` | перенесён → reference/commands#agents |
| docs/commands/mcp-uninstall.md | Usage | vibe mcp uninstall [--path <dir>] | cmd | живо | vibe.exe mcp uninstall --help -> exit 0 | перенесён → agent/give-your-agent-the-skill#edge-cases |
| docs/commands/mcp-uninstall.md | Usage | --path | flag | живо | найден в собранном корпусе `--help` | перенесён → howto/update-packages#by-hand |
| docs/commands/mcp-uninstall.md | Usage | --scope | flag | живо | найден в собранном корпусе `--help` | перенесён → agent/give-your-agent-the-skill#by-hand |
| docs/commands/mcp-uninstall.md | Usage | --agent | flag | живо | найден в собранном корпусе `--help` | перенесён → agent/give-your-agent-the-skill#by-hand |
| docs/commands/mcp-uninstall.md | Usage | --dry-run | flag | живо | найден в собранном корпусе `--help` | перенесён → howto/publish-a-package#root |
| docs/commands/mcp-uninstall.md | Usage | --yes | flag | живо | найден в собранном корпусе `--help` | перенесён → agent/give-your-agent-the-skill#what-happens |
| docs/commands/mcp-uninstall.md | Flags | --assume-yes | flag | живо | найден в собранном корпусе `--help` | перенесён → howto/remove-a-package#by-hand |
| docs/commands/mcp-uninstall.md | Flags | --unattended | flag | живо | найден в собранном корпусе `--help` | перенесён → reference/commands#global |
| docs/commands/mcp-uninstall.md | Flags | --json | flag | живо | найден в собранном корпусе `--help` | перенесён → reference/commands#global |
| docs/commands/mcp-uninstall.md | Flags | --quiet | flag | живо | найден в собранном корпусе `--help` | перенесён → reference/commands#global |
| docs/commands/mcp-uninstall.md | Removal contract | mcp_servers | section | живо | найдено в crates/vibe-core/src/manifest/document/tests.rs:660 | перенесён → reference/machine-formats#root (derived jtd-schema) |
| docs/commands/mcp-uninstall.md | Removal contract | vibevm/SKILL.md | path | устарело | путь не существует: vibevm/SKILL.md | перенесён → agent/give-your-agent-the-skill#what-happens |
| docs/commands/mcp-uninstall.md | Removal contract | [mcp_servers] | section | живо | найдено в crates/vibe-core/src/manifest/document/tests.rs:660 | перенесён → reference/machine-formats#root (derived jtd-schema) |
| docs/commands/mcp-uninstall.md | Removal contract | vibe.toml | path | живо | найден в дереве (имя файла): vibe.toml | перенесён → reference/manifest#root |
| docs/commands/mcp-uninstall.md | Removal contract | vibe.lock | path | живо | найден в дереве (имя файла): vibe.lock | перенесён → reference/lock-file#root |
| docs/commands/mcp-uninstall.md | Examples | vibe mcp uninstall --auto-equivalent: --agent all --scope both --yes | cmd | живо | vibe.exe mcp uninstall --help -> exit 0 | перенесён → agent/give-your-agent-the-skill#edge-cases |
| docs/commands/mcp-uninstall.md | Examples | --auto-equivalent | flag | устарело | не найден ни в одном собранном `--help` | снят: устарело |
| docs/commands/mcp-uninstall.md | Examples | vibe mcp uninstall --scope project | cmd | живо | vibe.exe mcp uninstall --help -> exit 0 | перенесён → agent/give-your-agent-the-skill#edge-cases |
| docs/commands/mcp-uninstall.md | Examples | vibe mcp uninstall --skill-only | cmd | живо | vibe.exe mcp uninstall --help -> exit 0 | перенесён → agent/give-your-agent-the-skill#edge-cases |
| docs/commands/mcp-uninstall.md | Examples | vibe mcp uninstall --agent opencode --scope both | cmd | живо | vibe.exe mcp uninstall --help -> exit 0 | перенесён → agent/give-your-agent-the-skill#edge-cases |
| docs/commands/mcp-uninstall.md | Examples | vibe mcp uninstall --dry-run | cmd | живо | vibe.exe mcp uninstall --help -> exit 0 | перенесён → agent/give-your-agent-the-skill#edge-cases |
| docs/commands/mcp-uninstall.md | Related | vibe mcp upgrade | cmd | живо | vibe.exe mcp upgrade --help -> exit 0 | перенесён → agent/give-your-agent-the-skill#edge-cases |
| docs/commands/mcp-uninstall.md | Related | vibe mcp status | cmd | живо | vibe.exe mcp status --help -> exit 0 | перенесён → agent/give-your-agent-the-skill#root |
| docs/commands/mcp-upgrade.md | `vibe mcp upgrade` — refresh stale vibevm integrations | vibe mcp upgrade | cmd | живо | vibe.exe mcp upgrade --help -> exit 0 | перенесён → agent/give-your-agent-the-skill#edge-cases |
| docs/commands/mcp-upgrade.md | `vibe mcp upgrade` — refresh stale vibevm integrations | vibe mcp install | cmd | живо | vibe.exe mcp install --help -> exit 0 | перенесён → authoring/ship-tools-and-mcp-servers#servers |
| docs/commands/mcp-upgrade.md | `vibe mcp upgrade` — refresh stale vibevm integrations | crates/vibe-cli | path | живо | путь существует: crates/vibe-cli | перенесён → architecture/how-vibe-is-built#the-crates |
| docs/commands/mcp-upgrade.md | `vibe mcp upgrade` — refresh stale vibevm integrations | --path | flag | живо | найден в собранном корпусе `--help` | перенесён → howto/update-packages#by-hand |
| docs/commands/mcp-upgrade.md | Usage | vibe mcp upgrade [--path <dir>] | cmd | живо | vibe.exe mcp upgrade --help -> exit 0 | перенесён → agent/give-your-agent-the-skill#edge-cases |
| docs/commands/mcp-upgrade.md | Usage | --scope | flag | живо | найден в собранном корпусе `--help` | перенесён → agent/give-your-agent-the-skill#by-hand |
| docs/commands/mcp-upgrade.md | Usage | --agent | flag | живо | найден в собранном корпусе `--help` | перенесён → agent/give-your-agent-the-skill#by-hand |
| docs/commands/mcp-upgrade.md | Usage | --config-only | flag | живо | найден в собранном корпусе `--help` | перенесён → reference/commands#agents |
| docs/commands/mcp-upgrade.md | Usage | --skill-only | flag | живо | найден в собранном корпусе `--help` | перенесён → reference/commands#agents |
| docs/commands/mcp-upgrade.md | Usage | --dry-run | flag | живо | найден в собранном корпусе `--help` | перенесён → howto/publish-a-package#root |
| docs/commands/mcp-upgrade.md | Usage | --yes | flag | живо | найден в собранном корпусе `--help` | перенесён → agent/give-your-agent-the-skill#what-happens |
| docs/commands/mcp-upgrade.md | Flags | vibe.toml | path | живо | найден в дереве (имя файла): vibe.toml | перенесён → reference/manifest#root |
| docs/commands/mcp-upgrade.md | Flags | --assume-yes | flag | живо | найден в собранном корпусе `--help` | перенесён → howto/remove-a-package#by-hand |
| docs/commands/mcp-upgrade.md | Flags | --unattended | flag | живо | найден в собранном корпусе `--help` | перенесён → reference/commands#global |
| docs/commands/mcp-upgrade.md | Flags | --json | flag | живо | найден в собранном корпусе `--help` | перенесён → reference/commands#global |
| docs/commands/mcp-upgrade.md | Flags | --quiet | flag | живо | найден в собранном корпусе `--help` | перенесён → reference/commands#global |
| docs/commands/mcp-upgrade.md | Examples | vibe mcp upgrade --dry-run | cmd | живо | vibe.exe mcp upgrade --help -> exit 0 | перенесён → agent/give-your-agent-the-skill#edge-cases |
| docs/commands/mcp-upgrade.md | Examples | vibe mcp upgrade --scope user | cmd | живо | vibe.exe mcp upgrade --help -> exit 0 | перенесён → agent/give-your-agent-the-skill#edge-cases |
| docs/commands/mcp-upgrade.md | Examples | vibe mcp upgrade --skill-only | cmd | живо | vibe.exe mcp upgrade --help -> exit 0 | перенесён → agent/give-your-agent-the-skill#edge-cases |
| docs/commands/mcp-upgrade.md | Examples | vibe mcp upgrade --agent opencode --skill-only | cmd | живо | vibe.exe mcp upgrade --help -> exit 0 | перенесён → agent/give-your-agent-the-skill#edge-cases |
| docs/commands/mcp-upgrade.md | Examples | vibe --json mcp upgrade --dry-run \| jq -e ' | cmd | живо | глобальный флаг --json найден в `vibe --help` | перенесён → reference/commands#global |
| docs/commands/mcp-upgrade.md | Human-readable | config.toml | path | живо | найден в дереве (имя файла): config.toml | перенесён → howto/work-offline#switching-it-on |
| docs/commands/mcp-upgrade.md | Human-readable | vibevm/SKILL.md | path | устарело | путь не существует: vibevm/SKILL.md | перенесён → agent/give-your-agent-the-skill#what-happens |
| docs/commands/mcp-upgrade.md | Related | vibe mcp uninstall | cmd | живо | vibe.exe mcp uninstall --help -> exit 0 | перенесён → agent/give-your-agent-the-skill#edge-cases |
| docs/commands/mcp-upgrade.md | Related | vibe mcp status | cmd | живо | vibe.exe mcp status --help -> exit 0 | перенесён → agent/give-your-agent-the-skill#root |
| docs/commands/mcp.md | `vibe mcp` | vibe mcp | cmd | живо | vibe.exe mcp --help -> exit 0 | перенесён → authoring/ship-tools-and-mcp-servers#servers |
| docs/commands/mcp.md | Usage | vibe mcp [OPTIONS] <COMMAND> | cmd | живо | vibe.exe mcp --help -> exit 0 | перенесён → authoring/ship-tools-and-mcp-servers#servers |
| docs/commands/mcp.md | Subcommands | vibe mcp serve | cmd | живо | vibe.exe mcp serve --help -> exit 0 | перенесён → agent/ask-your-agent#two-transports |
| docs/commands/mcp.md | Subcommands | vibe mcp install | cmd | живо | vibe.exe mcp install --help -> exit 0 | перенесён → authoring/ship-tools-and-mcp-servers#servers |
| docs/commands/mcp.md | Subcommands | vibe mcp status | cmd | живо | vibe.exe mcp status --help -> exit 0 | перенесён → agent/give-your-agent-the-skill#root |
| docs/commands/mcp.md | Subcommands | vibe mcp upgrade | cmd | живо | vibe.exe mcp upgrade --help -> exit 0 | перенесён → agent/give-your-agent-the-skill#edge-cases |
| docs/commands/mcp.md | Subcommands | vibe mcp uninstall | cmd | живо | vibe.exe mcp uninstall --help -> exit 0 | перенесён → agent/give-your-agent-the-skill#edge-cases |
| docs/commands/mcp.md | Subcommands | vibe mcp <command> --help | cmd | живо | vibe.exe mcp --help -> exit 0 | перенесён → authoring/ship-tools-and-mcp-servers#servers |
| docs/commands/mcp.md | Subcommands | --json | flag | живо | найден в собранном корпусе `--help` | перенесён → reference/commands#global |
| docs/commands/mcp.md | Subcommands | --quiet | flag | живо | найден в собранном корпусе `--help` | перенесён → reference/commands#global |
| docs/commands/mcp.md | Subcommands | --invoked-by | flag | живо | найден в собранном корпусе `--help` | перенесён → reference/commands#global |
| docs/commands/mcp.md | Subcommands | --unattended | flag | живо | найден в собранном корпусе `--help` | перенесён → reference/commands#global |
| docs/commands/mcp.md | Subcommands | --offline | flag | живо | найден в собранном корпусе `--help` | перенесён → howto/work-offline#root |
| docs/commands/mcp.md | Subcommands | --help | flag | живо | найден в собранном корпусе `--help` | перенесён → howto/read-documentation-locally#by-hand |
| docs/commands/mcp.md | Examples | vibe mcp install --help | cmd | живо | vibe.exe mcp install --help -> exit 0 | перенесён → authoring/ship-tools-and-mcp-servers#servers |
| docs/commands/mcp.md | Examples | vibe mcp upgrade --help | cmd | живо | vibe.exe mcp upgrade --help -> exit 0 | перенесён → agent/give-your-agent-the-skill#edge-cases |
| docs/commands/mcp.md | Examples | vibe mcp serve --path ./my-project | cmd | живо | vibe.exe mcp serve --help -> exit 0 | перенесён → agent/ask-your-agent#two-transports |
| docs/commands/mcp.md | Examples | --path | flag | живо | найден в собранном корпусе `--help` | перенесён → howto/update-packages#by-hand |
| docs/commands/registry-add.md | `vibe registry add` — register a new `[[registry]]` in `vibe.toml` | vibe registry add | cmd | живо | vibe.exe registry add --help -> exit 0 | перенесён → howto/use-a-private-registry#what-happens |
| docs/commands/registry-add.md | `vibe registry add` — register a new `[[registry]]` in `vibe.toml` | [[registry]] | section | живо | найдено в crates/vibe-core/src/manifest/artifact/tests_validation.rs:403 | перенесён → reference/manifest#sources |
| docs/commands/registry-add.md | `vibe registry add` — register a new `[[registry]]` in `vibe.toml` | vibe.toml | path | живо | найден в дереве (имя файла): vibe.toml | перенесён → reference/manifest#root |
| docs/commands/registry-add.md | `vibe registry add` — register a new `[[registry]]` in `vibe.toml` | --position | flag | живо | найден в собранном корпусе `--help` | перенесён → howto/use-a-private-registry#what-happens |
| docs/commands/registry-add.md | `vibe registry add` — register a new `[[registry]]` in `vibe.toml` | vibe install | cmd | живо | vibe.exe install --help -> exit 0 | перенесён → howto/install-a-package#what-happens |
| docs/commands/registry-add.md | Usage | vibe registry add <NAME> <URL> | cmd | живо | vibe.exe registry add --help -> exit 0 | перенесён → howto/use-a-private-registry#what-happens |
| docs/commands/registry-add.md | Usage | --ref | flag | живо | найден в собранном корпусе `--help` | перенесён → reference/commands#registries |
| docs/commands/registry-add.md | Usage | --naming | flag | живо | найден в собранном корпусе `--help` | перенесён → reference/commands#registries |
| docs/commands/registry-add.md | Usage | --auth | flag | живо | найден в собранном корпусе `--help` | перенесён → howto/use-a-private-registry#what-happens |
| docs/commands/registry-add.md | Usage | --token-env | flag | живо | найден в собранном корпусе `--help` | перенесён → reference/commands#registries |
| docs/commands/registry-add.md | Usage | --path | flag | живо | найден в собранном корпусе `--help` | перенесён → howto/update-packages#by-hand |
| docs/commands/registry-add.md | Usage | --json | flag | живо | найден в собранном корпусе `--help` | перенесён → reference/commands#global |
| docs/commands/registry-add.md | Usage | --quiet | flag | живо | найден в собранном корпусе `--help` | перенесён → reference/commands#global |
| docs/commands/registry-add.md | Arguments | vibe registry publish | cmd | живо | vibe.exe registry publish --help -> exit 0 | перенесён → howto/publish-a-package#root |
| docs/commands/registry-add.md | Arguments | [[mirror]] | section | живо | найдено в crates/vibe-core/src/manifest/artifact/tests_validation.rs:406 | перенесён → reference/manifest#sources |
| docs/commands/registry-add.md | Arguments | [[override]] | section | живо | найдено в crates/vibe-core/src/manifest/document/tests.rs:50 | перенесён → reference/manifest#sources |
| docs/commands/registry-add.md | Arguments | --registry | flag | живо | найден в собранном корпусе `--help` | перенесён → howto/publish-a-package#by-hand |
| docs/commands/registry-add.md | Output shape — human | vibe registry add: `private` registered (2 total registries). | cmd | живо | vibe.exe registry --help -> exit 0 | перенесён → howto/use-a-private-registry#root |
| docs/commands/registry-add.md | Examples | vibe registry add private "git@gitverse.ru:somecorp" | cmd | живо | vibe.exe registry add --help -> exit 0 | перенесён → howto/use-a-private-registry#what-happens |
| docs/commands/registry-add.md | Examples | vibe registry add fork "https://github.com/me/forks" --position primary --naming "kind/name" --ref develop | cmd | живо | vibe.exe registry add --help -> exit 0 | перенесён → howto/use-a-private-registry#what-happens |
| docs/commands/registry-add.md | Examples | vibe registry add scratch "file:///abs/path/to/local-org" --quiet | cmd | живо | vibe.exe registry add --help -> exit 0 | перенесён → howto/use-a-private-registry#what-happens |
| docs/commands/registry-add.md | Examples | vibe registry add public "https://github.com/vibespecs" --json \| jq .registry.adapter | cmd | живо | vibe.exe registry add --help -> exit 0 | перенесён → howto/use-a-private-registry#what-happens |
| docs/commands/registry-add.md | Examples | vibe registry add internal "https://gitlab.company.com/vibespecs" \ | cmd | живо | vibe.exe registry add --help -> exit 0 | перенесён → howto/use-a-private-registry#what-happens |
| docs/commands/registry-add.md | Examples | vibe registry add internal "https://gitlab.company.com/vibespecs" --auth token-env | cmd | живо | vibe.exe registry add --help -> exit 0 | перенесён → howto/use-a-private-registry#what-happens |
| docs/commands/registry-add.md | Examples | vibe registry add internal-ssh "git@gitlab.company.com:vibespecs" --auth ssh | cmd | живо | vibe.exe registry add --help -> exit 0 | перенесён → howto/use-a-private-registry#what-happens |
| docs/commands/registry-add.md | Errors | extract_org_segment | section | устарело | поле не найдено ни в manifest source, ни в примерах vibe.toml | снят: устарело |
| docs/commands/registry-add.md | Errors | extract_host_segment | section | устарело | поле не найдено ни в manifest source, ни в примерах vibe.toml | снят: устарело |
| docs/commands/registry-add.md | Errors | vibe init | cmd | живо | vibe.exe init --help -> exit 0 | перенесён → howto/use-a-private-registry#machine-wide |
| docs/commands/registry-add.md | Related | vibe registry list | cmd | живо | vibe.exe registry list --help -> exit 0 | перенесён → howto/use-a-private-registry#root |
| docs/commands/registry-add.md | Related | vibe registry sync | cmd | живо | vibe.exe registry sync --help -> exit 0 | перенесён → reference/machine-formats#the-documents |
| docs/commands/registry-list.md | `vibe registry list` — show configured registries, mirrors, overrides | vibe registry list | cmd | живо | vibe.exe registry list --help -> exit 0 | перенесён → howto/use-a-private-registry#root |
| docs/commands/registry-list.md | `vibe registry list` — show configured registries, mirrors, overrides | [[registry]] | section | живо | найдено в crates/vibe-core/src/manifest/artifact/tests_validation.rs:403 | перенесён → reference/manifest#sources |
| docs/commands/registry-list.md | `vibe registry list` — show configured registries, mirrors, overrides | [[mirror]] | section | живо | найдено в crates/vibe-core/src/manifest/artifact/tests_validation.rs:406 | перенесён → reference/manifest#sources |
| docs/commands/registry-list.md | `vibe registry list` — show configured registries, mirrors, overrides | [[override]] | section | живо | найдено в crates/vibe-core/src/manifest/document/tests.rs:50 | перенесён → reference/manifest#sources |
| docs/commands/registry-list.md | `vibe registry list` — show configured registries, mirrors, overrides | vibe.toml | path | живо | найден в дереве (имя файла): vibe.toml | перенесён → reference/manifest#root |
| docs/commands/registry-list.md | Usage | vibe registry list [--path <dir>] | cmd | живо | vibe.exe registry list --help -> exit 0 | перенесён → howto/use-a-private-registry#root |
| docs/commands/registry-list.md | Usage | --path | flag | живо | найден в собранном корпусе `--help` | перенесён → howto/update-packages#by-hand |
| docs/commands/registry-list.md | Usage | --json | flag | живо | найден в собранном корпусе `--help` | перенесён → reference/commands#global |
| docs/commands/registry-list.md | Usage | --quiet | flag | живо | найден в собранном корпусе `--help` | перенесён → reference/commands#global |
| docs/commands/registry-list.md | Flags | vibe registry list: <N> registries, <M> mirrors, <K> overrides. | cmd | живо | vibe.exe registry --help -> exit 0 | перенесён → howto/use-a-private-registry#root |
| docs/commands/registry-list.md | Output shape — human | vibe registry list: 2 registries, 3 mirrors, 1 override. | cmd | живо | vibe.exe registry --help -> exit 0 | перенесён → howto/use-a-private-registry#root |
| docs/commands/registry-list.md | Output shape — human | vibe registry publish | cmd | живо | vibe.exe registry publish --help -> exit 0 | перенесён → howto/publish-a-package#root |
| docs/commands/registry-list.md | Output shape — human | vibe install | cmd | живо | vibe.exe install --help -> exit 0 | перенесён → howto/install-a-package#what-happens |
| docs/commands/registry-list.md | Output shape — human | --registry | flag | живо | найден в собранном корпусе `--help` | перенесён → howto/publish-a-package#by-hand |
| docs/commands/registry-list.md | Examples | vibe registry list                                # human-readable | cmd | живо | vibe.exe registry list --help -> exit 0 | перенесён → howto/use-a-private-registry#root |
| docs/commands/registry-list.md | Examples | vibe registry list --json \| jq '.registries[].name'    # registry names | cmd | живо | vibe.exe registry list --help -> exit 0 | перенесён → howto/use-a-private-registry#root |
| docs/commands/registry-list.md | Examples | vibe registry list --json \| jq '.registries[] \| select(.adapter == null)'   # registries without publish support | cmd | живо | vibe.exe registry list --help -> exit 0 | перенесён → howto/use-a-private-registry#root |
| docs/commands/registry-list.md | Examples | vibe registry list --quiet                        # one-line summary | cmd | живо | vibe.exe registry list --help -> exit 0 | перенесён → howto/use-a-private-registry#root |
| docs/commands/registry-list.md | Related | vibe registry sync | cmd | живо | vibe.exe registry sync --help -> exit 0 | перенесён → reference/machine-formats#the-documents |
| docs/commands/registry-publish.md | `vibe registry publish` — publish a package directory | vibe registry publish | cmd | живо | vibe.exe registry publish --help -> exit 0 | перенесён → howto/publish-a-package#root |
| docs/commands/registry-publish.md | `vibe registry publish` — publish a package directory | [package] | section | живо | найдено в crates/vibe-core/src/manifest/artifact/tests_validation.rs:372 | перенесён → reference/lock-file#package-entries |
| docs/commands/registry-publish.md | `vibe registry publish` — publish a package directory | vibe.toml | path | живо | найден в дереве (имя файла): vibe.toml | перенесён → reference/manifest#root |
| docs/commands/registry-publish.md | Usage | vibe registry publish <source> [--registry <name>] [--path <project>] | cmd | живо | vibe.exe registry publish --help -> exit 0 | перенесён → howto/publish-a-package#root |
| docs/commands/registry-publish.md | Usage | --registry | flag | живо | найден в собранном корпусе `--help` | перенесён → howto/publish-a-package#by-hand |
| docs/commands/registry-publish.md | Usage | --path | flag | живо | найден в собранном корпусе `--help` | перенесён → howto/update-packages#by-hand |
| docs/commands/registry-publish.md | Usage | --dry-run | flag | живо | найден в собранном корпусе `--help` | перенесён → howto/publish-a-package#root |
| docs/commands/registry-publish.md | Usage | --json | flag | живо | найден в собранном корпусе `--help` | перенесён → reference/commands#global |
| docs/commands/registry-publish.md | Usage | --quiet | flag | живо | найден в собранном корпусе `--help` | перенесён → reference/commands#global |
| docs/commands/registry-publish.md | Flags | [[registry]] | section | живо | найдено в crates/vibe-core/src/manifest/artifact/tests_validation.rs:403 | перенесён → reference/manifest#sources |
| docs/commands/registry-publish.md | Authentication | ~/.vibe/ | path | живо | найден на диске (эта машина): C:\Users\olegc\.vibe | перенесён → reference/settings-and-environment#the-folder |
| docs/commands/registry-publish.md | Authentication | ~/.vibe/git.publish.token | path | неизвестно | секрет-подобный путь (R-12) — существование на диске не проверялось намеренно | перенесён → reference/settings-and-environment#the-folder |
| docs/commands/registry-publish.md | Pipeline | [requires] | section | живо | найдено в crates/vibe-core/src/manifest/document.rs:22 | перенесён → reference/manifest#requirements |
| docs/commands/registry-publish.md | Pipeline | [conflicts] | section | живо | найдено в crates/vibe-core/src/manifest/document/validation.rs:59 | перенесён → reference/manifest#requirements |
| docs/commands/registry-publish.md | Pipeline | [dependencies] | section | живо | найдено в crates/vibe-core/src/manifest/package/capabilities.rs:312 | снят: устарело — таблицы нет в схеме манифеста (crates/vibe-core/src/manifest, проверено 2026-09-12) |
| docs/commands/registry-publish.md | Pipeline | auto_init | section | устарело | поле не найдено ни в manifest source, ни в примерах vibe.toml | снят: устарело |
| docs/commands/registry-publish.md | Pipeline | clone_url | section | устарело | поле не найдено ни в manifest source, ни в примерах vibe.toml | снят: устарело |
| docs/commands/registry-publish.md | Examples | vibe registry publish ./vibevm/vibepacks/org.vibevm.world/wal/v1.0.0 --dry-run | cmd | живо | vibe.exe registry publish --help -> exit 0 | перенесён → howto/publish-a-package#root |
| docs/commands/registry-publish.md | Examples | vibevm/vibepacks/org.vibevm.world/wal/v1.0.0 | path | живо | путь существует: vibevm/vibepacks/org.vibevm.world/wal/v1.0.0 | перенесён → howto/publish-a-package#by-hand (слот `vibevm/vibepacks/<группа>/<имя>/v<версия>`) |
| docs/commands/registry-publish.md | Examples | vibe registry publish ./vibevm/vibepacks/org.vibevm.world/wal/v1.0.0 | cmd | живо | vibe.exe registry publish --help -> exit 0 | перенесён → howto/publish-a-package#root |
| docs/commands/registry-publish.md | Examples | vibe registry publish ./vibevm/vibepacks/org.vibevm.world/wal/v1.0.0 --registry corporate | cmd | живо | vibe.exe registry publish --help -> exit 0 | перенесён → howto/publish-a-package#root |
| docs/commands/registry-publish.md | Examples | vibevm/vibepacks/ | path | живо | путь существует: vibevm/vibepacks/ | перенесён → howto/publish-a-package#root |
| docs/commands/registry-publish.md | Examples | vibe registry publish "$pkg_dir" --json | cmd | живо | vibe.exe registry publish --help -> exit 0 | перенесён → howto/publish-a-package#root |
| docs/commands/registry-publish.md | Related | vibe install | cmd | живо | vibe.exe install --help -> exit 0 | перенесён → howto/install-a-package#what-happens |
| docs/commands/registry-redirect-sync.md | `vibe registry redirect-sync` — mirror target tags into a stub | vibe registry redirect-sync | cmd | живо | vibe.exe registry redirect-sync --help -> exit 0 | перенесён → howto/use-a-private-registry#root |
| docs/commands/registry-redirect-sync.md | `vibe registry redirect-sync` — mirror target tags into a stub | vibe-redirect.toml | path | устарело | ни один файл с именем vibe-redirect.toml не найден в дереве | снят: устарело |
| docs/commands/registry-redirect-sync.md | `vibe registry redirect-sync` — mirror target tags into a stub | pinned_ref | section | живо | найдено в crates/vibe-core/src/manifest/redirect.rs:66 | перенесён → howto/use-a-private-registry#redirects |
| docs/commands/registry-redirect-sync.md | Usage | vibe registry redirect-sync <pkgref> | cmd | живо | vibe.exe registry redirect-sync --help -> exit 0 | перенесён → howto/use-a-private-registry#root |
| docs/commands/registry-redirect-sync.md | Usage | --registry | flag | живо | найден в собранном корпусе `--help` | перенесён → howto/publish-a-package#by-hand |
| docs/commands/registry-redirect-sync.md | Usage | --path | flag | живо | найден в собранном корпусе `--help` | перенесён → howto/update-packages#by-hand |
| docs/commands/registry-redirect-sync.md | Usage | --dry-run | flag | живо | найден в собранном корпусе `--help` | перенесён → howto/publish-a-package#root |
| docs/commands/registry-redirect-sync.md | Usage | --json | flag | живо | найден в собранном корпусе `--help` | перенесён → reference/commands#global |
| docs/commands/registry-redirect-sync.md | Usage | --quiet | flag | живо | найден в собранном корпусе `--help` | перенесён → reference/commands#global |
| docs/commands/registry-redirect-sync.md | Flags | [[registry]] | section | живо | найдено в crates/vibe-core/src/manifest/artifact/tests_validation.rs:403 | перенесён → reference/manifest#sources |
| docs/commands/registry-redirect-sync.md | Flags | vibe.toml | path | живо | найден в дереве (имя файла): vibe.toml | перенесён → reference/manifest#root |
| docs/commands/registry-redirect-sync.md | Authentication | vibe registry publish | cmd | живо | vibe.exe registry publish --help -> exit 0 | перенесён → howto/publish-a-package#root |
| docs/commands/registry-redirect-sync.md | Authentication | repo_exists | section | устарело | поле не найдено ни в manifest source, ни в примерах vibe.toml | снят: устарело |
| docs/commands/registry-redirect-sync.md | Authentication | [redirect] | section | живо | найдено в crates/vibe-core/src/manifest/redirect.rs:29 | перенесён → howto/use-a-private-registry#redirects |
| docs/commands/registry-redirect-sync.md | Pipeline | vibe registry redirect <pkgref> --to <url> | cmd | живо | vibe.exe registry redirect --help -> exit 0 | перенесён → howto/use-a-private-registry#root |
| docs/commands/registry-redirect-sync.md | Pipeline | --to | flag | живо | найден в собранном корпусе `--help` | перенесён → authoring/specs-agents-can-cite#the-dialect |
| docs/commands/registry-redirect-sync.md | Pipeline | ref_policy | section | живо | найдено в crates/vibe-core/src/manifest/redirect.rs:40 | перенесён → howto/use-a-private-registry#redirects |
| docs/commands/registry-redirect-sync.md | Pipeline | --tags | flag | устарело | не найден ни в одном собранном `--help` | снят: устарело |
| docs/commands/registry-redirect-sync.md | Pipeline | vibe install | cmd | живо | vibe.exe install --help -> exit 0 | перенесён → howto/install-a-package#what-happens |
| docs/commands/registry-redirect-sync.md | Examples | vibe registry redirect-sync flow:internal-helper | cmd | живо | vibe.exe registry redirect-sync --help -> exit 0 | перенесён → howto/use-a-private-registry#root |
| docs/commands/registry-redirect-sync.md | Examples | vibe registry redirect-sync flow:internal-helper --dry-run | cmd | живо | vibe.exe registry redirect-sync --help -> exit 0 | перенесён → howto/use-a-private-registry#root |
| docs/commands/registry-redirect-sync.md | Error surface | vibe registry redirect | cmd | живо | vibe.exe registry redirect --help -> exit 0 | перенесён → howto/use-a-private-registry#root |
| docs/commands/registry-redirect-update.md | `vibe registry redirect-update` — rewrite an existing registry stub's marker | vibe registry redirect-update | cmd | живо | vibe.exe registry redirect-update --help -> exit 0 | перенесён → howto/use-a-private-registry#root |
| docs/commands/registry-redirect-update.md | `vibe registry redirect-update` — rewrite an existing registry stub's marker | vibe-redirect.toml | path | устарело | ни один файл с именем vibe-redirect.toml не найден в дереве | снят: устарело |
| docs/commands/registry-redirect-update.md | Usage | vibe registry redirect-update <pkgref> | cmd | живо | vibe.exe registry redirect-update --help -> exit 0 | перенесён → howto/use-a-private-registry#root |
| docs/commands/registry-redirect-update.md | Usage | --to | flag | живо | найден в собранном корпусе `--help` | перенесён → authoring/specs-agents-can-cite#the-dialect |
| docs/commands/registry-redirect-update.md | Usage | --ref-policy | flag | живо | найден в собранном корпусе `--help` | перенесён → reference/commands#registries |
| docs/commands/registry-redirect-update.md | Usage | --pinned-ref | flag | живо | найден в собранном корпусе `--help` | перенесён → reference/commands#registries |
| docs/commands/registry-redirect-update.md | Usage | --target-auth | flag | живо | найден в собранном корпусе `--help` | перенесён → reference/commands#registries |
| docs/commands/registry-redirect-update.md | Usage | --target-token-env | flag | живо | найден в собранном корпусе `--help` | перенесён → reference/commands#registries |
| docs/commands/registry-redirect-update.md | Usage | --description | flag | живо | найден в собранном корпусе `--help` | перенесён → reference/commands#registries |
| docs/commands/registry-redirect-update.md | Usage | --clear-description | flag | живо | найден в собранном корпусе `--help` | перенесён → reference/commands#registries |
| docs/commands/registry-redirect-update.md | Usage | --registry | flag | живо | найден в собранном корпусе `--help` | перенесён → howto/publish-a-package#by-hand |
| docs/commands/registry-redirect-update.md | Usage | --trust-redirect | flag | живо | найден в собранном корпусе `--help` | перенесён → reference/commands#registries |
| docs/commands/registry-redirect-update.md | Usage | --resync | flag | живо | найден в собранном корпусе `--help` | перенесён → reference/commands#registries |
| docs/commands/registry-redirect-update.md | Usage | --path | flag | живо | найден в собранном корпусе `--help` | перенесён → howto/update-packages#by-hand |
| docs/commands/registry-redirect-update.md | Usage | --dry-run | flag | живо | найден в собранном корпусе `--help` | перенесён → howto/publish-a-package#root |
| docs/commands/registry-redirect-update.md | Usage | --json | flag | живо | найден в собранном корпусе `--help` | перенесён → reference/commands#global |
| docs/commands/registry-redirect-update.md | Usage | --quiet | flag | живо | найден в собранном корпусе `--help` | перенесён → reference/commands#global |
| docs/commands/registry-redirect-update.md | Flags | [redirect] | section | живо | найдено в crates/vibe-core/src/manifest/redirect.rs:29 | перенесён → howto/use-a-private-registry#redirects |
| docs/commands/registry-redirect-update.md | Flags | pinned_ref | section | живо | найдено в crates/vibe-core/src/manifest/redirect.rs:66 | перенесён → howto/use-a-private-registry#redirects |
| docs/commands/registry-redirect-update.md | Flags | [[registry]] | section | живо | найдено в crates/vibe-core/src/manifest/artifact/tests_validation.rs:403 | перенесён → reference/manifest#sources |
| docs/commands/registry-redirect-update.md | Flags | token_env | section | живо | найдено в crates/vibe-core/src/manifest/document/tests.rs:569 | перенесён → reference/manifest#sources |
| docs/commands/registry-redirect-update.md | Flags | vibe.toml | path | живо | найден в дереве (имя файла): vibe.toml | перенесён → reference/manifest#root |
| docs/commands/registry-redirect-update.md | Flags | target_url | section | живо | найдено в crates/vibe-core/src/manifest/redirect.rs:8 | перенесён → howto/use-a-private-registry#redirects |
| docs/commands/registry-redirect-update.md | Flags | ref_policy | section | живо | найдено в crates/vibe-core/src/manifest/redirect.rs:40 | перенесён → howto/use-a-private-registry#redirects |
| docs/commands/registry-redirect-update.md | Flags | vibe registry redirect-sync <pkgref> | cmd | живо | vibe.exe registry redirect-sync --help -> exit 0 | перенесён → howto/use-a-private-registry#root |
| docs/commands/registry-redirect-update.md | Authentication | vibe registry redirect | cmd | живо | vibe.exe registry redirect --help -> exit 0 | перенесён → howto/use-a-private-registry#root |
| docs/commands/registry-redirect-update.md | Authentication | ~/.vibe/ | path | живо | найден на диске (эта машина): C:\Users\olegc\.vibe | перенесён → reference/settings-and-environment#the-folder |
| docs/commands/registry-redirect-update.md | Authentication | ~/.vibe/git.publish.token | path | неизвестно | секрет-подобный путь (R-12) — существование на диске не проверялось намеренно | перенесён → reference/settings-and-environment#the-folder |
| docs/commands/registry-redirect-update.md | Pipeline | vibe registry redirect <pkgref> --to ... | cmd | живо | vibe.exe registry redirect --help -> exit 0 | перенесён → howto/use-a-private-registry#root |
| docs/commands/registry-redirect-update.md | Pipeline | repo_exists | section | устарело | поле не найдено ни в manifest source, ни в примерах vibe.toml | снят: устарело |
| docs/commands/registry-redirect-update.md | Pipeline | vibe registry redirect-sync | cmd | живо | vibe.exe registry redirect-sync --help -> exit 0 | перенесён → howto/use-a-private-registry#root |
| docs/commands/registry-redirect-update.md | JSON output (`--json`) | vibe registry redirect-update … --dry-run --json | cmd | живо | vibe.exe registry redirect-update --help -> exit 0 | перенесён → howto/use-a-private-registry#root |
| docs/commands/registry-redirect-update.md | JSON output (`--json`) | trust_required | section | устарело | поле не найдено ни в manifest source, ни в примерах vibe.toml | снят: устарело |
| docs/commands/registry-redirect-update.md | Examples | vibe registry redirect-update flow:internal-helper \ | cmd | живо | vibe.exe registry redirect-update --help -> exit 0 | перенесён → howto/use-a-private-registry#root |
| docs/commands/registry-redirect-update.md | Examples | vibe registry redirect-update flow:internal-helper --clear-description | cmd | живо | vibe.exe registry redirect-update --help -> exit 0 | перенесён → howto/use-a-private-registry#root |
| docs/commands/registry-redirect-update.md | Examples | vibe registry redirect-update flow:legacy \ | cmd | живо | vibe.exe registry redirect-update --help -> exit 0 | перенесён → howto/use-a-private-registry#root |
| docs/commands/registry-redirect-update.md | Examples | vibe registry redirect-update flow:internal-secret \ | cmd | живо | vibe.exe registry redirect-update --help -> exit 0 | перенесён → howto/use-a-private-registry#root |
| docs/commands/registry-redirect-update.md | Error surface | vibe registry redirect <pkgref> --to <url> | cmd | живо | vibe.exe registry redirect --help -> exit 0 | перенесён → howto/use-a-private-registry#root |
| docs/commands/registry-redirect-update.md | Error surface | vibe registry publish | cmd | живо | vibe.exe registry publish --help -> exit 0 | перенесён → howto/publish-a-package#root |
| docs/commands/registry-redirect.md | `vibe registry redirect` — create a registry stub | vibe registry redirect | cmd | живо | vibe.exe registry redirect --help -> exit 0 | перенесён → howto/use-a-private-registry#root |
| docs/commands/registry-redirect.md | `vibe registry redirect` — create a registry stub | vibe install <pkgref> | cmd | живо | vibe.exe install --help -> exit 0 | перенесён → howto/install-a-package#what-happens |
| docs/commands/registry-redirect.md | `vibe registry redirect` — create a registry stub | vibe-redirect.toml | path | устарело | ни один файл с именем vibe-redirect.toml не найден в дереве | снят: устарело |
| docs/commands/registry-redirect.md | Usage | vibe registry redirect <pkgref> --to <url> | cmd | живо | vibe.exe registry redirect --help -> exit 0 | перенесён → howto/use-a-private-registry#root |
| docs/commands/registry-redirect.md | Usage | --to | flag | живо | найден в собранном корпусе `--help` | перенесён → authoring/specs-agents-can-cite#the-dialect |
| docs/commands/registry-redirect.md | Usage | --registry | flag | живо | найден в собранном корпусе `--help` | перенесён → howto/publish-a-package#by-hand |
| docs/commands/registry-redirect.md | Usage | --ref-policy | flag | живо | найден в собранном корпусе `--help` | перенесён → reference/commands#registries |
| docs/commands/registry-redirect.md | Usage | --pinned-ref | flag | живо | найден в собранном корпусе `--help` | перенесён → reference/commands#registries |
| docs/commands/registry-redirect.md | Usage | --target-auth | flag | живо | найден в собранном корпусе `--help` | перенесён → reference/commands#registries |
| docs/commands/registry-redirect.md | Usage | --target-token-env | flag | живо | найден в собранном корпусе `--help` | перенесён → reference/commands#registries |
| docs/commands/registry-redirect.md | Usage | --description | flag | живо | найден в собранном корпусе `--help` | перенесён → reference/commands#registries |
| docs/commands/registry-redirect.md | Usage | --sync | flag | живо | найден в собранном корпусе `--help` | перенесён → reference/commands#registries |
| docs/commands/registry-redirect.md | Usage | --path | flag | живо | найден в собранном корпусе `--help` | перенесён → howto/update-packages#by-hand |
| docs/commands/registry-redirect.md | Usage | --dry-run | flag | живо | найден в собранном корпусе `--help` | перенесён → howto/publish-a-package#root |
| docs/commands/registry-redirect.md | Usage | --json | flag | живо | найден в собранном корпусе `--help` | перенесён → reference/commands#global |
| docs/commands/registry-redirect.md | Usage | --quiet | flag | живо | найден в собранном корпусе `--help` | перенесён → reference/commands#global |
| docs/commands/registry-redirect.md | Flags | [[registry]] | section | живо | найдено в crates/vibe-core/src/manifest/artifact/tests_validation.rs:403 | перенесён → reference/manifest#sources |
| docs/commands/registry-redirect.md | Flags | vibe.toml | path | живо | найден в дереве (имя файла): vibe.toml | перенесён → reference/manifest#root |
| docs/commands/registry-redirect.md | Flags | [redirect] | section | живо | найдено в crates/vibe-core/src/manifest/redirect.rs:29 | перенесён → howto/use-a-private-registry#redirects |
| docs/commands/registry-redirect.md | Flags | vibe show <pkgref> | cmd | живо | vibe.exe show --help -> exit 0 | перенесён → howto/remove-a-package#what-happens |
| docs/commands/registry-redirect.md | Flags | vibe registry redirect-sync <pkgref> | cmd | живо | vibe.exe registry redirect-sync --help -> exit 0 | перенесён → howto/use-a-private-registry#root |
| docs/commands/registry-redirect.md | Authentication | vibe registry publish | cmd | живо | vibe.exe registry publish --help -> exit 0 | перенесён → howto/publish-a-package#root |
| docs/commands/registry-redirect.md | Authentication | ~/.vibe/ | path | живо | найден на диске (эта машина): C:\Users\olegc\.vibe | перенесён → reference/settings-and-environment#the-folder |
| docs/commands/registry-redirect.md | Authentication | ~/.vibe/git.publish.token | path | неизвестно | секрет-подобный путь (R-12) — существование на диске не проверялось намеренно | перенесён → reference/settings-and-environment#the-folder |
| docs/commands/registry-redirect.md | Pipeline | vibe registry redirect-update | cmd | живо | vibe.exe registry redirect-update --help -> exit 0 | перенесён → howto/use-a-private-registry#root |
| docs/commands/registry-redirect.md | Pipeline | repo_exists | section | устарело | поле не найдено ни в manifest source, ни в примерах vibe.toml | снят: устарело |
| docs/commands/registry-redirect.md | Examples | vibe registry redirect flow:internal-helper \ | cmd | живо | vibe.exe registry redirect --help -> exit 0 | перенесён → howto/use-a-private-registry#root |
| docs/commands/registry-redirect.md | Examples | vibe registry redirect flow:legacy-pinned \ | cmd | живо | vibe.exe registry redirect --help -> exit 0 | перенесён → howto/use-a-private-registry#root |
| docs/commands/registry-redirect.md | Examples | vibe registry redirect flow:internal-secret \ | cmd | живо | vibe.exe registry redirect --help -> exit 0 | перенесён → howto/use-a-private-registry#root |
| docs/commands/registry-redirect.md | Related | vibe registry redirect-sync | cmd | живо | vibe.exe registry redirect-sync --help -> exit 0 | перенесён → howto/use-a-private-registry#root |
| docs/commands/registry-remove.md | `vibe registry remove` — drop a `[[registry]]` or `[[mirror]]` from `vibe.toml` | vibe registry remove | cmd | живо | vibe.exe registry remove --help -> exit 0 | перенесён → howto/use-a-private-registry#root |
| docs/commands/registry-remove.md | `vibe registry remove` — drop a `[[registry]]` or `[[mirror]]` from `vibe.toml` | [[registry]] | section | живо | найдено в crates/vibe-core/src/manifest/artifact/tests_validation.rs:403 | перенесён → reference/manifest#sources |
| docs/commands/registry-remove.md | `vibe registry remove` — drop a `[[registry]]` or `[[mirror]]` from `vibe.toml` | [[mirror]] | section | живо | найдено в crates/vibe-core/src/manifest/artifact/tests_validation.rs:406 | перенесён → reference/manifest#sources |
| docs/commands/registry-remove.md | `vibe registry remove` — drop a `[[registry]]` or `[[mirror]]` from `vibe.toml` | vibe.toml | path | живо | найден в дереве (имя файла): vibe.toml | перенесён → reference/manifest#root |
| docs/commands/registry-remove.md | `vibe registry remove` — drop a `[[registry]]` or `[[mirror]]` from `vibe.toml` | vibe registry remove registry <NAME>            # remove a [[registry]] | cmd | живо | vibe.exe registry remove registry --help -> exit 0 | перенесён → howto/use-a-private-registry#root |
| docs/commands/registry-remove.md | `vibe registry remove` — drop a `[[registry]]` or `[[mirror]]` from `vibe.toml` | vibe registry remove mirror   <OF> <URL>        # remove a [[mirror]] | cmd | живо | vibe.exe registry remove mirror --help -> exit 0 | перенесён → howto/use-a-private-registry#root |
| docs/commands/registry-remove.md | `vibe registry remove` — drop a `[[registry]]` or `[[mirror]]` from `vibe.toml` | vibe registry sync | cmd | живо | vibe.exe registry sync --help -> exit 0 | перенесён → reference/machine-formats#the-documents |
| docs/commands/registry-remove.md | Usage | vibe registry remove registry <NAME> | cmd | живо | vibe.exe registry remove registry --help -> exit 0 | перенесён → howto/use-a-private-registry#root |
| docs/commands/registry-remove.md | Usage | --path | flag | живо | найден в собранном корпусе `--help` | перенесён → howto/update-packages#by-hand |
| docs/commands/registry-remove.md | Usage | --json | flag | живо | найден в собранном корпусе `--help` | перенесён → reference/commands#global |
| docs/commands/registry-remove.md | Usage | --quiet | flag | живо | найден в собранном корпусе `--help` | перенесён → reference/commands#global |
| docs/commands/registry-remove.md | Usage | vibe registry remove mirror <OF> <URL> | cmd | живо | vibe.exe registry remove mirror --help -> exit 0 | перенесён → howto/use-a-private-registry#root |
| docs/commands/registry-remove.md | Output shape — human | vibe registry remove: 1 registry remain. | cmd | живо | vibe.exe registry --help -> exit 0 | перенесён → howto/use-a-private-registry#root |
| docs/commands/registry-remove.md | Output shape — human | vibe registry remove: 0 mirrors remain. | cmd | живо | vibe.exe registry --help -> exit 0 | перенесён → howto/use-a-private-registry#root |
| docs/commands/registry-remove.md | JSON shape | total_registries | section | устарело | поле не найдено ни в manifest source, ни в примерах vibe.toml | снят: устарело |
| docs/commands/registry-remove.md | JSON shape | total_mirrors | section | устарело | поле не найдено ни в manifest source, ни в примерах vibe.toml | снят: устарело |
| docs/commands/registry-remove.md | Errors | vibe registry add | cmd | живо | vibe.exe registry add --help -> exit 0 | перенесён → howto/use-a-private-registry#what-happens |
| docs/commands/registry-remove.md | Errors | vibe registry remove mirror | cmd | живо | vibe.exe registry remove mirror --help -> exit 0 | перенесён → howto/use-a-private-registry#root |
| docs/commands/registry-remove.md | Errors | vibe registry list | cmd | живо | vibe.exe registry list --help -> exit 0 | перенесён → howto/use-a-private-registry#root |
| docs/commands/registry-remove.md | Errors | vibe registry set-mirror | cmd | живо | vibe.exe registry set-mirror --help -> exit 0 | перенесён → howto/use-a-private-registry#edge-cases |
| docs/commands/registry-remove.md | Errors | vibe init | cmd | живо | vibe.exe init --help -> exit 0 | перенесён → howto/use-a-private-registry#machine-wide |
| docs/commands/registry-remove.md | Related | [[override]] | section | живо | найдено в crates/vibe-core/src/manifest/document/tests.rs:50 | перенесён → reference/manifest#sources |
| docs/commands/registry-set-mirror.md | `vibe registry set-mirror` — add a `[[mirror]]` block to `vibe.toml` | vibe registry set-mirror | cmd | живо | vibe.exe registry set-mirror --help -> exit 0 | перенесён → howto/use-a-private-registry#edge-cases |
| docs/commands/registry-set-mirror.md | `vibe registry set-mirror` — add a `[[mirror]]` block to `vibe.toml` | [[mirror]] | section | живо | найдено в crates/vibe-core/src/manifest/artifact/tests_validation.rs:406 | перенесён → reference/manifest#sources |
| docs/commands/registry-set-mirror.md | `vibe registry set-mirror` — add a `[[mirror]]` block to `vibe.toml` | vibe.toml | path | живо | найден в дереве (имя файла): vibe.toml | перенесён → reference/manifest#root |
| docs/commands/registry-set-mirror.md | Usage | vibe registry set-mirror <OF> <URL> | cmd | живо | vibe.exe registry set-mirror --help -> exit 0 | перенесён → howto/use-a-private-registry#edge-cases |
| docs/commands/registry-set-mirror.md | Usage | --priority | flag | живо | найден в собранном корпусе `--help` | перенесён → reference/commands#registries |
| docs/commands/registry-set-mirror.md | Usage | --path | flag | живо | найден в собранном корпусе `--help` | перенесён → howto/update-packages#by-hand |
| docs/commands/registry-set-mirror.md | Usage | --json | flag | живо | найден в собранном корпусе `--help` | перенесён → reference/commands#global |
| docs/commands/registry-set-mirror.md | Usage | --quiet | flag | живо | найден в собранном корпусе `--help` | перенесён → reference/commands#global |
| docs/commands/registry-set-mirror.md | Arguments | [[registry]] | section | живо | найдено в crates/vibe-core/src/manifest/artifact/tests_validation.rs:403 | перенесён → reference/manifest#sources |
| docs/commands/registry-set-mirror.md | Arguments | vibe registry vendor | cmd | живо | vibe.exe registry vendor --help -> exit 0 | перенесён → howto/work-offline#air-gapped |
| docs/commands/registry-set-mirror.md | Output shape — human | vibe registry set-mirror: 2 total mirrors configured. | cmd | живо | vibe.exe registry --help -> exit 0 | перенесён → howto/use-a-private-registry#root |
| docs/commands/registry-set-mirror.md | JSON shape | vibe registry add | cmd | живо | vibe.exe registry add --help -> exit 0 | перенесён → howto/use-a-private-registry#what-happens |
| docs/commands/registry-set-mirror.md | JSON shape | attached_to | section | устарело | поле не найдено ни в manifest source, ни в примерах vibe.toml | снят: устарело |
| docs/commands/registry-set-mirror.md | Examples | vibe registry set-mirror vibespecs "https://github-mirror.example/vibespecs" --priority 10 | cmd | живо | vibe.exe registry set-mirror --help -> exit 0 | перенесён → howto/use-a-private-registry#edge-cases |
| docs/commands/registry-set-mirror.md | Examples | vibe registry set-mirror "*" "https://offline.example/cache" --priority 50 | cmd | живо | vibe.exe registry set-mirror --help -> exit 0 | перенесён → howto/use-a-private-registry#edge-cases |
| docs/commands/registry-set-mirror.md | Examples | vibe registry set-mirror vibespecs "git@backup-host:vibespecs" --priority 100 --quiet | cmd | живо | vibe.exe registry set-mirror --help -> exit 0 | перенесён → howto/use-a-private-registry#edge-cases |
| docs/commands/registry-set-mirror.md | Examples | vibe registry set-mirror "*" "file:///abs/path/to/local-org" --json \| jq .attached_to | cmd | живо | vibe.exe registry set-mirror --help -> exit 0 | перенесён → howto/use-a-private-registry#edge-cases |
| docs/commands/registry-set-mirror.md | Errors | vibe init | cmd | живо | vibe.exe init --help -> exit 0 | перенесён → howto/use-a-private-registry#machine-wide |
| docs/commands/registry-set-mirror.md | Related | vibe registry list | cmd | живо | vibe.exe registry list --help -> exit 0 | перенесён → howto/use-a-private-registry#root |
| docs/commands/registry-set-mirror.md | Related | [[override]] | section | живо | найдено в crates/vibe-core/src/manifest/document/tests.rs:50 | перенесён → reference/manifest#sources |
| docs/commands/registry-sync.md | `vibe registry sync` — refresh per-package registry clones | vibe registry sync | cmd | живо | vibe.exe registry sync --help -> exit 0 | перенесён → reference/machine-formats#the-documents |
| docs/commands/registry-sync.md | `vibe registry sync` — refresh per-package registry clones | vibe install | cmd | живо | vibe.exe install --help -> exit 0 | перенесён → howto/install-a-package#what-happens |
| docs/commands/registry-sync.md | `vibe registry sync` — refresh per-package registry clones | vibe.lock | path | живо | найден в дереве (имя файла): vibe.lock | перенесён → reference/lock-file#root |
| docs/commands/registry-sync.md | Usage | vibe registry sync [--path <dir>] | cmd | живо | vibe.exe registry sync --help -> exit 0 | перенесён → reference/machine-formats#the-documents |
| docs/commands/registry-sync.md | Usage | --path | flag | живо | найден в собранном корпусе `--help` | перенесён → howto/update-packages#by-hand |
| docs/commands/registry-sync.md | Usage | --json | flag | живо | найден в собранном корпусе `--help` | перенесён → reference/commands#global |
| docs/commands/registry-sync.md | Usage | --quiet | flag | живо | найден в собранном корпусе `--help` | перенесён → reference/commands#global |
| docs/commands/registry-sync.md | Flags | vibe.toml | path | живо | найден в дереве (имя файла): vibe.toml | перенесён → reference/manifest#root |
| docs/commands/registry-sync.md | Flags | vibe registry sync: <N> refreshed, <K> skipped. | cmd | живо | vibe.exe registry --help -> exit 0 | перенесён → howto/use-a-private-registry#root |
| docs/commands/registry-sync.md | What gets refreshed | [[registry]] | section | живо | найдено в crates/vibe-core/src/manifest/artifact/tests_validation.rs:403 | перенесён → reference/manifest#sources |
| docs/commands/registry-sync.md | What gets refreshed | --prune | flag | устарело | не найден ни в одном собранном `--help` | снят: устарело |
| docs/commands/registry-sync.md | What gets refreshed | --hard | flag | устарело | не найден ни в одном собранном `--help` | снят: устарело |
| docs/commands/registry-sync.md | What gets refreshed | [[override]] | section | живо | найдено в crates/vibe-core/src/manifest/document/tests.rs:50 | перенесён → reference/manifest#sources |
| docs/commands/registry-sync.md | What gets refreshed | source_url | section | живо | найдено в crates/vibe-core/src/manifest/document/tests.rs:52 | перенесён → reference/lock-file#package-entries |
| docs/commands/registry-sync.md | What gets refreshed | source_ref | section | живо | найдено в crates/vibe-core/src/manifest/lockfile/tests.rs:33 | перенесён → reference/lock-file#package-entries |
| docs/commands/registry-sync.md | What gets refreshed | --registry | flag | живо | найден в собранном корпусе `--help` | перенесён → howto/publish-a-package#by-hand |
| docs/commands/registry-sync.md | Examples | vibe registry sync                      # refresh everything in the lockfile | cmd | живо | vibe.exe registry sync --help -> exit 0 | перенесён → reference/machine-formats#the-documents |
| docs/commands/registry-sync.md | Examples | vibe registry sync --json \| jq '.refreshed \| length'   # how many were refreshed | cmd | живо | vibe.exe registry sync --help -> exit 0 | перенесён → reference/machine-formats#the-documents |
| docs/commands/registry-sync.md | Examples | vibe registry sync --quiet              # one-line summary | cmd | живо | vibe.exe registry sync --help -> exit 0 | перенесён → reference/machine-formats#the-documents |
| docs/commands/registry-sync.md | Examples | vibe registry sync                      # pull new tags | cmd | живо | vibe.exe registry sync --help -> exit 0 | перенесён → reference/machine-formats#the-documents |
| docs/commands/registry-sync.md | Examples | vibe install flow:wal@^0.2 --assume-yes # now sees v0.2.x if upstream tagged it | cmd | живо | vibe.exe install --help -> exit 0 | перенесён → howto/install-a-package#what-happens |
| docs/commands/registry-sync.md | Examples | --assume-yes | flag | живо | найден в собранном корпусе `--help` | перенесён → howto/remove-a-package#by-hand |
| docs/commands/registry-test.md | `vibe registry test` — probe each registry's reachability and auth | vibe registry test | cmd | живо | vibe.exe registry test --help -> exit 0 | перенесён → howto/use-a-private-registry#root |
| docs/commands/registry-test.md | `vibe registry test` — probe each registry's reachability and auth | [[registry]] | section | живо | найдено в crates/vibe-core/src/manifest/artifact/tests_validation.rs:403 | перенесён → reference/manifest#sources |
| docs/commands/registry-test.md | `vibe registry test` — probe each registry's reachability and auth | vibe.toml | path | живо | найден в дереве (имя файла): vibe.toml | перенесён → reference/manifest#root |
| docs/commands/registry-test.md | `vibe registry test` — probe each registry's reachability and auth | vibe install | cmd | живо | vibe.exe install --help -> exit 0 | перенесён → howto/install-a-package#what-happens |
| docs/commands/registry-test.md | Usage | vibe registry test [--path <dir>] | cmd | живо | vibe.exe registry test --help -> exit 0 | перенесён → howto/use-a-private-registry#root |
| docs/commands/registry-test.md | Usage | --path | flag | живо | найден в собранном корпусе `--help` | перенесён → howto/update-packages#by-hand |
| docs/commands/registry-test.md | Usage | --json | flag | живо | найден в собранном корпусе `--help` | перенесён → reference/commands#global |
| docs/commands/registry-test.md | Usage | --quiet | flag | живо | найден в собранном корпусе `--help` | перенесён → reference/commands#global |
| docs/commands/registry-test.md | Flags | vibe registry test: <ok>/<total> reachable. | cmd | живо | vibe.exe registry --help -> exit 0 | перенесён → howto/use-a-private-registry#root |
| docs/commands/registry-test.md | Output shape — human | vibe registry test: 1/4 reachable. | cmd | живо | vibe.exe registry --help -> exit 0 | перенесён → howto/use-a-private-registry#root |
| docs/commands/registry-test.md | Examples | vibe registry test --quiet | cmd | живо | vibe.exe registry test --help -> exit 0 | перенесён → howto/use-a-private-registry#root |
| docs/commands/registry-test.md | Examples | vibe registry test --json \| jq '.registries[] \| select(.status == "missing-token")' | cmd | живо | vibe.exe registry test --help -> exit 0 | перенесён → howto/use-a-private-registry#root |
| docs/commands/registry-test.md | Examples | vibe registry test --json \| jq '.summary.reachable == .summary.total' | cmd | живо | vibe.exe registry test --help -> exit 0 | перенесён → howto/use-a-private-registry#root |
| docs/commands/registry-test.md | Related | vibe registry list | cmd | живо | vibe.exe registry list --help -> exit 0 | перенесён → howto/use-a-private-registry#root |
| docs/commands/registry-vendor.md | `vibe registry vendor` — generate a local mirror directory | vibe registry vendor | cmd | живо | vibe.exe registry vendor --help -> exit 0 | перенесён → howto/work-offline#air-gapped |
| docs/commands/registry-vendor.md | `vibe registry vendor` — generate a local mirror directory | [[registry]] | section | живо | найдено в crates/vibe-core/src/manifest/artifact/tests_validation.rs:403 | перенесён → reference/manifest#sources |
| docs/commands/registry-vendor.md | `vibe registry vendor` — generate a local mirror directory | [[mirror]] | section | живо | найдено в crates/vibe-core/src/manifest/artifact/tests_validation.rs:406 | перенесён → reference/manifest#sources |
| docs/commands/registry-vendor.md | `vibe registry vendor` — generate a local mirror directory | vibe.lock | path | живо | найден в дереве (имя файла): vibe.lock | перенесён → reference/lock-file#root |
| docs/commands/registry-vendor.md | Usage | vibe registry vendor [--out <dir>] [--force] | cmd | живо | vibe.exe registry vendor --help -> exit 0 | перенесён → howto/work-offline#air-gapped |
| docs/commands/registry-vendor.md | Usage | --out | flag | живо | найден в собранном корпусе `--help` | перенесён → reference/commands#registries |
| docs/commands/registry-vendor.md | Usage | --force | flag | живо | найден в собранном корпусе `--help` | перенесён → model/two-trees#what-regenerates |
| docs/commands/registry-vendor.md | Usage | --path | flag | живо | найден в собранном корпусе `--help` | перенесён → howto/update-packages#by-hand |
| docs/commands/registry-vendor.md | Usage | --json | flag | живо | найден в собранном корпусе `--help` | перенесён → reference/commands#global |
| docs/commands/registry-vendor.md | Usage | --quiet | flag | живо | найден в собранном корпусе `--help` | перенесён → reference/commands#global |
| docs/commands/registry-vendor.md | Flags | vibe.toml | path | живо | найден в дереве (имя файла): vibe.toml | перенесён → reference/manifest#root |
| docs/commands/registry-vendor.md | Flags | out_dir | section | устарело | поле не найдено ни в manifest source, ни в примерах vibe.toml | снят: устарело |
| docs/commands/registry-vendor.md | Flags | suggested_mirror_url | section | устарело | поле не найдено ни в manifest source, ни в примерах vibe.toml | снят: устарело |
| docs/commands/registry-vendor.md | Flags | vibe registry vendor: <N> vendored, <K> skipped. | cmd | живо | vibe.exe registry --help -> exit 0 | перенесён → howto/use-a-private-registry#root |
| docs/commands/registry-vendor.md | What gets vendored | refresh_package | section | устарело | поле не найдено ни в manifest source, ни в примерах vibe.toml | снят: устарело |
| docs/commands/registry-vendor.md | What gets vendored | [[override]] | section | живо | найдено в crates/vibe-core/src/manifest/document/tests.rs:50 | перенесён → reference/manifest#sources |
| docs/commands/registry-vendor.md | What gets vendored | --registry | flag | живо | найден в собранном корпусе `--help` | перенесён → howto/publish-a-package#by-hand |
| docs/commands/registry-vendor.md | What gets vendored | vibe install | cmd | живо | vibe.exe install --help -> exit 0 | перенесён → howto/install-a-package#what-happens |
| docs/commands/registry-vendor.md | What gets vendored | content_hash | section | живо | найдено в crates/vibe-core/src/manifest/document/tests.rs:292 | перенесён → reference/lock-file#package-entries |
| docs/commands/registry-vendor.md | Wiring the result into `vibe.toml` | --offline | flag | живо | найден в собранном корпусе `--help` | перенесён → howto/work-offline#root |
| docs/commands/registry-vendor.md | Examples | vibe registry vendor --out /opt/vibevm-mirror --force | cmd | живо | vibe.exe registry vendor --help -> exit 0 | перенесён → howto/work-offline#air-gapped |
| docs/commands/registry-vendor.md | Examples | vibe registry vendor --json \| jq -r '.suggested_mirror_url' | cmd | живо | vibe.exe registry vendor --help -> exit 0 | перенесён → howto/work-offline#air-gapped |
| docs/commands/registry-vendor.md | Related | vibe registry sync | cmd | живо | vibe.exe registry sync --help -> exit 0 | перенесён → reference/machine-formats#the-documents |
| docs/commands/registry-vendor.md | Related | source_ref | section | живо | найдено в crates/vibe-core/src/manifest/lockfile/tests.rs:33 | перенесён → reference/lock-file#package-entries |
| docs/commands/registry-vendor.md | Related | vibe registry set-mirror | cmd | живо | vibe.exe registry set-mirror --help -> exit 0 | перенесён → howto/use-a-private-registry#edge-cases |
| docs/commands/registry.md | `vibe registry` | vibe registry | cmd | живо | vibe.exe registry --help -> exit 0 | перенесён → howto/use-a-private-registry#root |
| docs/commands/registry.md | Usage | vibe registry [OPTIONS] <COMMAND> | cmd | живо | vibe.exe registry --help -> exit 0 | перенесён → howto/use-a-private-registry#root |
| docs/commands/registry.md | Subcommands | [[registry]] | section | живо | найдено в crates/vibe-core/src/manifest/artifact/tests_validation.rs:403 | перенесён → reference/manifest#sources |
| docs/commands/registry.md | Subcommands | [[mirror]] | section | живо | найдено в crates/vibe-core/src/manifest/artifact/tests_validation.rs:406 | перенесён → reference/manifest#sources |
| docs/commands/registry.md | Subcommands | [[override]] | section | живо | найдено в crates/vibe-core/src/manifest/document/tests.rs:50 | перенесён → reference/manifest#sources |
| docs/commands/registry.md | Subcommands | vibe.toml | path | живо | найден в дереве (имя файла): vibe.toml | перенесён → reference/manifest#root |
| docs/commands/registry.md | Subcommands | --json | flag | живо | найден в собранном корпусе `--help` | перенесён → reference/commands#global |
| docs/commands/registry.md | Subcommands | --quiet | flag | живо | найден в собранном корпусе `--help` | перенесён → reference/commands#global |
| docs/commands/registry.md | Subcommands | --invoked-by | flag | живо | найден в собранном корпусе `--help` | перенесён → reference/commands#global |
| docs/commands/registry.md | Subcommands | --unattended | flag | живо | найден в собранном корпусе `--help` | перенесён → reference/commands#global |
| docs/commands/registry.md | Subcommands | --offline | flag | живо | найден в собранном корпусе `--help` | перенесён → howto/work-offline#root |
| docs/commands/registry.md | Examples | vibe registry list | cmd | живо | vibe.exe registry list --help -> exit 0 | перенесён → howto/use-a-private-registry#root |
| docs/commands/registry.md | Examples | vibe registry test | cmd | живо | vibe.exe registry test --help -> exit 0 | перенесён → howto/use-a-private-registry#root |
| docs/commands/registry.md | Examples | vibe registry sync | cmd | живо | vibe.exe registry sync --help -> exit 0 | перенесён → reference/machine-formats#the-documents |
| docs/commands/registry.md | Examples | vibe registry vendor --help | cmd | живо | vibe.exe registry vendor --help -> exit 0 | перенесён → howto/work-offline#air-gapped |
| docs/commands/registry.md | Examples | --help | flag | живо | найден в собранном корпусе `--help` | перенесён → howto/read-documentation-locally#by-hand |
| docs/commands/registry.md | Examples | vibe registry <command> --help | cmd | живо | vibe.exe registry --help -> exit 0 | перенесён → howto/use-a-private-registry#root |
| docs/commands/registry.md | Examples | index_url | section | живо | найдено в crates/vibe-core/src/manifest/document/tests.rs:571 | перенесён → reference/manifest#sources |
| docs/commands/registry.md | Related | vibe search | cmd | живо | vibe.exe search --help -> exit 0 | перенесён → model/packages-and-kinds#the-kinds |
| docs/commands/registry.md | Related | vibe install | cmd | живо | vibe.exe install --help -> exit 0 | перенесён → howto/install-a-package#what-happens |
| docs/commands/registry.md | Related | vibe cache | cmd | живо | vibe.exe cache --help -> exit 0 | перенесён → howto/update-packages#recovery |
| docs/commands/reinstall.md | `vibe reinstall` — recompute the materialised dependencies and boot artifacts | vibe reinstall | cmd | живо | vibe.exe reinstall --help -> exit 0 | перенесён → howto/update-packages#recovery |
| docs/commands/reinstall.md | `vibe reinstall` — recompute the materialised dependencies and boot artifacts | vibe update | cmd | живо | vibe.exe update --help -> exit 0 | перенесён → howto/update-packages#root |
| docs/commands/reinstall.md | `vibe reinstall` — recompute the materialised dependencies and boot artifacts | vibedeps/ | path | неизвестно | генерируемый каталог (материализуется `vibe install`), механическая проверка по дереву неинформативна для непроинсталлированного чекаута | перенесён → model/two-trees#the-rule |
| docs/commands/reinstall.md | `vibe reinstall` — recompute the materialised dependencies and boot artifacts | vibe.lock | path | живо | найден в дереве (имя файла): vibe.lock | перенесён → reference/lock-file#root |
| docs/commands/reinstall.md | Usage | vibe reinstall [<path>] [--force] [--assume-yes] | cmd | живо | vibe.exe reinstall --help -> exit 0 | перенесён → howto/update-packages#recovery |
| docs/commands/reinstall.md | Usage | --force | flag | живо | найден в собранном корпусе `--help` | перенесён → model/two-trees#what-regenerates |
| docs/commands/reinstall.md | Usage | --assume-yes | flag | живо | найден в собранном корпусе `--help` | перенесён → howto/remove-a-package#by-hand |
| docs/commands/reinstall.md | Usage | --json | flag | живо | найден в собранном корпусе `--help` | перенесён → reference/commands#global |
| docs/commands/reinstall.md | Usage | --quiet | flag | живо | найден в собранном корпусе `--help` | перенесён → reference/commands#global |
| docs/commands/reinstall.md | Flags | --yes | flag | живо | найден в собранном корпусе `--help` | перенесён → agent/give-your-agent-the-skill#what-happens |
| docs/commands/reinstall.md | Flags | --unattended | flag | живо | найден в собранном корпусе `--help` | перенесён → reference/commands#global |
| docs/commands/reinstall.md | Re-fetch (`--force`) | [[registry]] | section | живо | найдено в crates/vibe-core/src/manifest/artifact/tests_validation.rs:403 | перенесён → reference/manifest#sources |
| docs/commands/reinstall.md | Re-fetch (`--force`) | [[mirror]] | section | живо | найдено в crates/vibe-core/src/manifest/artifact/tests_validation.rs:406 | перенесён → reference/manifest#sources |
| docs/commands/reinstall.md | Re-fetch (`--force`) | [[override]] | section | живо | найдено в crates/vibe-core/src/manifest/document/tests.rs:50 | перенесён → reference/manifest#sources |
| docs/commands/reinstall.md | Re-fetch (`--force`) | content_hash | section | живо | найдено в crates/vibe-core/src/manifest/document/tests.rs:292 | перенесён → reference/lock-file#package-entries |
| docs/commands/reinstall.md | Errors | vibe init | cmd | живо | vibe.exe init --help -> exit 0 | перенесён → howto/use-a-private-registry#machine-wide |
| docs/commands/reinstall.md | Errors | vibe.toml | path | живо | найден в дереве (имя файла): vibe.toml | перенесён → reference/manifest#root |
| docs/commands/reinstall.md | JSON output (`--json`) | nodes_regenerated | section | устарело | поле не найдено ни в manifest source, ни в примерах vibe.toml | снят: устарело |
| docs/commands/reinstall.md | Examples | vibe reinstall --assume-yes | cmd | живо | vibe.exe reinstall --help -> exit 0 | перенесён → howto/update-packages#recovery |
| docs/commands/reinstall.md | Examples | vibe reinstall --force --assume-yes | cmd | живо | vibe.exe reinstall --help -> exit 0 | перенесён → howto/update-packages#recovery |
| docs/commands/reinstall.md | Examples | vibe reinstall packages/flow-wal | cmd | живо | vibe.exe reinstall --help -> exit 0 | перенесён → howto/update-packages#recovery |
| docs/commands/reinstall.md | Examples | vibe --json reinstall --assume-yes \| jq '.nodes_regenerated' | cmd | живо | глобальный флаг --json найден в `vibe --help` | перенесён → reference/commands#global |
| docs/commands/reinstall.md | `reinstall` vs `install` vs `update` | vibe install | cmd | живо | vibe.exe install --help -> exit 0 | перенесён → howto/install-a-package#what-happens |
| docs/commands/reinstall.md | `reinstall` vs `install` vs `update` | [requires] | section | живо | найдено в crates/vibe-core/src/manifest/document.rs:22 | перенесён → reference/manifest#requirements |
| docs/commands/search.md | `vibe search` | vibe search | cmd | живо | vibe.exe search --help -> exit 0 | перенесён → model/packages-and-kinds#the-kinds |
| docs/commands/search.md | Usage | vibe search [OPTIONS] <QUERY...> | cmd | живо | vibe.exe search --help -> exit 0 | перенесён → model/packages-and-kinds#the-kinds |
| docs/commands/search.md | Usage | vibe search [OPTIONS] --purl <PURL> | cmd | живо | vibe.exe search --help -> exit 0 | перенесён → model/packages-and-kinds#the-kinds |
| docs/commands/search.md | Usage | --purl | flag | живо | найден в собранном корпусе `--help` | перенесён → reference/commands#registries |
| docs/commands/search.md | Options | --kind | flag | живо | найден в собранном корпусе `--help` | перенесён → model/packages-and-kinds#the-kinds |
| docs/commands/search.md | Options | --registry | flag | живо | найден в собранном корпусе `--help` | перенесён → howto/publish-a-package#by-hand |
| docs/commands/search.md | Options | --limit | flag | живо | найден в собранном корпусе `--help` | перенесён → reference/commands#global |
| docs/commands/search.md | Options | --full-scan | flag | живо | найден в собранном корпусе `--help` | перенесён → reference/commands#registries |
| docs/commands/search.md | Options | ~/.vibe/search-cache/ | path | неизвестно | рантайм-путь, не найден на этой машине: ~/.vibe/search-cache/ | перенесён → reference/settings-and-environment#the-folder |
| docs/commands/search.md | Options | --no-cache | flag | живо | найден в собранном корпусе `--help` | перенесён → reference/commands#registries |
| docs/commands/search.md | Options | --cache-ttl | flag | живо | найден в собранном корпусе `--help` | перенесён → reference/commands#registries |
| docs/commands/search.md | Options | vibe.toml | path | живо | найден в дереве (имя файла): vibe.toml | перенесён → reference/manifest#root |
| docs/commands/search.md | Options | --path | flag | живо | найден в собранном корпусе `--help` | перенесён → howto/update-packages#by-hand |
| docs/commands/search.md | Options | --offline | flag | живо | найден в собранном корпусе `--help` | перенесён → howto/work-offline#root |
| docs/commands/search.md | Options | --json | flag | живо | найден в собранном корпусе `--help` | перенесён → reference/commands#global |
| docs/commands/search.md | Options | --quiet | flag | живо | найден в собранном корпусе `--help` | перенесён → reference/commands#global |
| docs/commands/search.md | Options | --invoked-by | flag | живо | найден в собранном корпусе `--help` | перенесён → reference/commands#global |
| docs/commands/search.md | Options | --unattended | flag | живо | найден в собранном корпусе `--help` | перенесён → reference/commands#global |
| docs/commands/search.md | Examples | vibe search write ahead log | cmd | живо | vibe.exe search --help -> exit 0 | перенесён → model/packages-and-kinds#the-kinds |
| docs/commands/search.md | Examples | vibe search wal --kind flow --registry vibespecs | cmd | живо | vibe.exe search --help -> exit 0 | перенесён → model/packages-and-kinds#the-kinds |
| docs/commands/search.md | Examples | vibe search --purl pkg:cargo/serde | cmd | живо | vibe.exe search --help -> exit 0 | перенесён → model/packages-and-kinds#the-kinds |
| docs/commands/search.md | Examples | vibe search wal --no-cache --limit 50 | cmd | живо | vibe.exe search --help -> exit 0 | перенесён → model/packages-and-kinds#the-kinds |
| docs/commands/search.md | Examples | [[registry]] | section | живо | найдено в crates/vibe-core/src/manifest/artifact/tests_validation.rs:403 | перенесён → reference/manifest#sources |
| docs/commands/search.md | Related | vibe registry | cmd | живо | vibe.exe registry --help -> exit 0 | перенесён → howto/use-a-private-registry#root |
| docs/commands/search.md | Related | vibe install | cmd | живо | vibe.exe install --help -> exit 0 | перенесён → howto/install-a-package#what-happens |
| docs/commands/show.md | `vibe show` | vibe show | cmd | живо | vibe.exe show --help -> exit 0 | перенесён → howto/remove-a-package#what-happens |
| docs/commands/show.md | Usage | vibe show [OPTIONS] <COMMAND> | cmd | живо | vibe.exe show --help -> exit 0 | перенесён → howto/remove-a-package#what-happens |
| docs/commands/show.md | Subcommands | vibe show effective | cmd | живо | vibe.exe show effective --help -> exit 0 | перенесён → howto/remove-a-package#what-happens |
| docs/commands/show.md | Subcommands | vibe show config | cmd | живо | vibe.exe show config --help -> exit 0 | перенесён → reference/settings-and-environment#edge-cases |
| docs/commands/show.md | Subcommands | vibe show features | cmd | живо | vibe.exe show features --help -> exit 0 | перенесён → howto/remove-a-package#what-happens |
| docs/commands/show.md | Subcommands | vibe show subskills | cmd | живо | vibe.exe show subskills --help -> exit 0 | перенесён → howto/remove-a-package#what-happens |
| docs/commands/show.md | Subcommands | vibe show purls | cmd | живо | vibe.exe show purls --help -> exit 0 | перенесён → howto/remove-a-package#what-happens |
| docs/commands/show.md | Subcommands | vibe show <command> --help | cmd | живо | vibe.exe show --help -> exit 0 | перенесён → howto/remove-a-package#what-happens |
| docs/commands/show.md | Subcommands | --json | flag | живо | найден в собранном корпусе `--help` | перенесён → reference/commands#global |
| docs/commands/show.md | Subcommands | --quiet | flag | живо | найден в собранном корпусе `--help` | перенесён → reference/commands#global |
| docs/commands/show.md | Subcommands | --invoked-by | flag | живо | найден в собранном корпусе `--help` | перенесён → reference/commands#global |
| docs/commands/show.md | Subcommands | --unattended | flag | живо | найден в собранном корпусе `--help` | перенесён → reference/commands#global |
| docs/commands/show.md | Subcommands | --offline | flag | живо | найден в собранном корпусе `--help` | перенесён → howto/work-offline#root |
| docs/commands/show.md | Subcommands | --help | flag | живо | найден в собранном корпусе `--help` | перенесён → howto/read-documentation-locally#by-hand |
| docs/commands/show.md | Examples | vibe show config --json | cmd | живо | vibe.exe show config --help -> exit 0 | перенесён → reference/settings-and-environment#edge-cases |
| docs/commands/show.md | Related | vibe list | cmd | живо | vibe.exe list --help -> exit 0 | перенесён → reference/machine-formats#root |
| docs/commands/show.md | Related | vibe tree | cmd | живо | vibe.exe tree --help -> exit 0 | перенесён → model/boot-lane#root |
| docs/commands/show.md | Related | vibe check | cmd | живо | vibe.exe check --help -> exit 0 | перенесён → howto/remove-a-package#root |
| docs/commands/tree.md | `vibe tree` | vibe tree | cmd | живо | vibe.exe tree --help -> exit 0 | перенесён → model/boot-lane#root |
| docs/commands/tree.md | Usage | vibe tree [OPTIONS] | cmd | живо | vibe.exe tree --help -> exit 0 | перенесён → model/boot-lane#root |
| docs/commands/tree.md | Options | --path | flag | живо | найден в собранном корпусе `--help` | перенесён → howto/update-packages#by-hand |
| docs/commands/tree.md | Options | --plain | flag | живо | найден в собранном корпусе `--help` | перенесён → howto/remove-a-package#by-hand |
| docs/commands/tree.md | Options | --console | flag | живо | найден в собранном корпусе `--help` | перенесён → reference/commands#projects |
| docs/commands/tree.md | Options | --terminal | flag | живо | найден в собранном корпусе `--help` | перенесён → reference/commands#projects |
| docs/commands/tree.md | Options | --json | flag | живо | найден в собранном корпусе `--help` | перенесён → reference/commands#global |
| docs/commands/tree.md | Options | --quiet | flag | живо | найден в собранном корпусе `--help` | перенесён → reference/commands#global |
| docs/commands/tree.md | Options | --invoked-by | flag | живо | найден в собранном корпусе `--help` | перенесён → reference/commands#global |
| docs/commands/tree.md | Options | --unattended | flag | живо | найден в собранном корпусе `--help` | перенесён → reference/commands#global |
| docs/commands/tree.md | Options | --offline | flag | живо | найден в собранном корпусе `--help` | перенесён → howto/work-offline#root |
| docs/commands/tree.md | Examples | vibe tree --plain | cmd | живо | vibe.exe tree --help -> exit 0 | перенесён → model/boot-lane#root |
| docs/commands/tree.md | Examples | vibe tree --json --path ./my-project | cmd | живо | vibe.exe tree --help -> exit 0 | перенесён → model/boot-lane#root |
| docs/commands/tree.md | Examples | vibe tree --console | cmd | живо | vibe.exe tree --help -> exit 0 | перенесён → model/boot-lane#root |
| docs/commands/tree.md | Related | vibe list | cmd | живо | vibe.exe list --help -> exit 0 | перенесён → reference/machine-formats#root |
| docs/commands/tree.md | Related | vibe show | cmd | живо | vibe.exe show --help -> exit 0 | перенесён → howto/remove-a-package#what-happens |
| docs/commands/tree.md | Related | vibe check | cmd | живо | vibe.exe check --help -> exit 0 | перенесён → howto/remove-a-package#root |
| docs/commands/uninstall.md | `vibe uninstall` | vibe uninstall | cmd | живо | vibe.exe uninstall --help -> exit 0 | перенесён → howto/remove-a-package#what-happens |
| docs/commands/uninstall.md | Usage | vibe uninstall [OPTIONS] <PACKAGE> | cmd | живо | vibe.exe uninstall --help -> exit 0 | перенесён → howto/remove-a-package#what-happens |
| docs/commands/uninstall.md | Options | --path | flag | живо | найден в собранном корпусе `--help` | перенесён → howto/update-packages#by-hand |
| docs/commands/uninstall.md | Options | --assume-yes | flag | живо | найден в собранном корпусе `--help` | перенесён → howto/remove-a-package#by-hand |
| docs/commands/uninstall.md | Options | --offline | flag | живо | найден в собранном корпусе `--help` | перенесён → howto/work-offline#root |
| docs/commands/uninstall.md | Options | --json | flag | живо | найден в собранном корпусе `--help` | перенесён → reference/commands#global |
| docs/commands/uninstall.md | Options | --quiet | flag | живо | найден в собранном корпусе `--help` | перенесён → reference/commands#global |
| docs/commands/uninstall.md | Options | --invoked-by | flag | живо | найден в собранном корпусе `--help` | перенесён → reference/commands#global |
| docs/commands/uninstall.md | Options | --unattended | flag | живо | найден в собранном корпусе `--help` | перенесён → reference/commands#global |
| docs/commands/uninstall.md | Example | vibe uninstall flow:wal --path ./my-project | cmd | живо | vibe.exe uninstall --help -> exit 0 | перенесён → howto/remove-a-package#what-happens |
| docs/commands/uninstall.md | Example | vibe cache clean | cmd | живо | vibe.exe cache clean --help -> exit 0 | перенесён → howto/update-packages#recovery |
| docs/commands/uninstall.md | Related | vibe install | cmd | живо | vibe.exe install --help -> exit 0 | перенесён → howto/install-a-package#what-happens |
| docs/commands/uninstall.md | Related | vibe list | cmd | живо | vibe.exe list --help -> exit 0 | перенесён → reference/machine-formats#root |
| docs/commands/uninstall.md | Related | vibe cache | cmd | живо | vibe.exe cache --help -> exit 0 | перенесён → howto/update-packages#recovery |
| docs/commands/update.md | `vibe update` | vibe update | cmd | живо | vibe.exe update --help -> exit 0 | перенесён → howto/update-packages#root |
| docs/commands/update.md | Usage | vibe update [OPTIONS] [PACKAGES]... | cmd | живо | vibe.exe update --help -> exit 0 | перенесён → howto/update-packages#root |
| docs/commands/update.md | Usage | vibe update --all [OPTIONS] | cmd | живо | vibe.exe update --help -> exit 0 | перенесён → howto/update-packages#root |
| docs/commands/update.md | Usage | --all | flag | живо | найден в собранном корпусе `--help` | перенесён → howto/update-packages#root |
| docs/commands/update.md | Options | vibe.lock | path | живо | найден в дереве (имя файла): vibe.lock | перенесён → reference/lock-file#root |
| docs/commands/update.md | Options | --path | flag | живо | найден в собранном корпусе `--help` | перенесён → howto/update-packages#by-hand |
| docs/commands/update.md | Options | --assume-yes | flag | живо | найден в собранном корпусе `--help` | перенесён → howto/remove-a-package#by-hand |
| docs/commands/update.md | Options | vibe.toml | path | живо | найден в дереве (имя файла): vibe.toml | перенесён → reference/manifest#root |
| docs/commands/update.md | Options | --exact | flag | живо | найден в собранном корпусе `--help` | перенесён → howto/install-a-package#constraints |
| docs/commands/update.md | Options | --auth-required | flag | живо | найден в собранном корпусе `--help` | перенесён → howto/use-a-private-registry#edge-cases |
| docs/commands/update.md | Options | --offline | flag | живо | найден в собранном корпусе `--help` | перенесён → howto/work-offline#root |
| docs/commands/update.md | Options | --json | flag | живо | найден в собранном корпусе `--help` | перенесён → reference/commands#global |
| docs/commands/update.md | Options | --quiet | flag | живо | найден в собранном корпусе `--help` | перенесён → reference/commands#global |
| docs/commands/update.md | Options | --invoked-by | flag | живо | найден в собранном корпусе `--help` | перенесён → reference/commands#global |
| docs/commands/update.md | Options | --unattended | flag | живо | найден в собранном корпусе `--help` | перенесён → reference/commands#global |
| docs/commands/update.md | Examples | vibe update flow:wal | cmd | живо | vibe.exe update --help -> exit 0 | перенесён → howto/update-packages#root |
| docs/commands/update.md | Examples | vibe update --all --assume-yes | cmd | живо | vibe.exe update --help -> exit 0 | перенесён → howto/update-packages#root |
| docs/commands/update.md | Examples | vibe update --all --offline --path ./my-project | cmd | живо | vibe.exe update --help -> exit 0 | перенесён → howto/update-packages#root |
| docs/commands/update.md | Related | vibe install | cmd | живо | vibe.exe install --help -> exit 0 | перенесён → howto/install-a-package#what-happens |
| docs/commands/update.md | Related | vibe cache | cmd | живо | vibe.exe cache --help -> exit 0 | перенесён → howto/update-packages#recovery |
| docs/commands/update.md | Related | vibe check | cmd | живо | vibe.exe check --help -> exit 0 | перенесён → howto/remove-a-package#root |
| docs/commands/version.md | `vibe version` — print version information | vibe version | cmd | живо | vibe.exe version --help -> exit 0 | перенесён → reference/lock-file#meta |
| docs/commands/version.md | `vibe version` — print version information | vibe --version | cmd | живо | глобальный флаг --version найден в `vibe --help` | перенесён → reference/commands#global |
| docs/commands/version.md | `vibe version` — print version information | --version | flag | живо | найден в собранном корпусе `--help` | перенесён → start/index#step-2 |
| docs/commands/version.md | Usage | vibe -V | cmd | живо | глобальный флаг -V найден в `vibe --help` | перенесён → reference/commands#global |
| docs/commands/version.md | Output | --json | flag | живо | найден в собранном корпусе `--help` | перенесён → reference/commands#global |
| docs/commands/version.md | Output | --quiet | flag | живо | найден в собранном корпусе `--help` | перенесён → reference/commands#global |
| docs/commands/version.md | Related | vibe init | cmd | живо | vibe.exe init --help -> exit 0 | перенесён → howto/use-a-private-registry#machine-wide |
| docs/commands/version.md | Related | [meta] | section | живо | найдено в crates/vibe-core/src/manifest/lockfile/tests.rs:19 | перенесён → reference/lock-file#meta |
| docs/commands/version.md | Related | vibe.toml | path | живо | найден в дереве (имя файла): vibe.toml | перенесён → reference/manifest#root |
| docs/commands/version.md | Related | vibe.lock | path | живо | найден в дереве (имя файла): vibe.lock | перенесён → reference/lock-file#root |
| docs/commands/workspace-publish.md | `vibe workspace publish` — publish a workspace's members as separate repositories | vibe workspace publish | cmd | живо | vibe.exe workspace publish --help -> exit 0 | перенесён → howto/publish-a-package#workspaces |
| docs/commands/workspace-publish.md | `vibe workspace publish` — publish a workspace's members as separate repositories | vibe registry publish | cmd | живо | vibe.exe registry publish --help -> exit 0 | перенесён → howto/publish-a-package#root |
| docs/commands/workspace-publish.md | Usage | vibe workspace publish [--member <rel-path>] | cmd | живо | vibe.exe workspace publish --help -> exit 0 | перенесён → howto/publish-a-package#workspaces |
| docs/commands/workspace-publish.md | Usage | --member | flag | живо | найден в собранном корпусе `--help` | перенесён → howto/publish-a-package#workspaces |
| docs/commands/workspace-publish.md | Usage | --dry-run | flag | живо | найден в собранном корпусе `--help` | перенесён → howto/publish-a-package#root |
| docs/commands/workspace-publish.md | Usage | --path | flag | живо | найден в собранном корпусе `--help` | перенесён → howto/update-packages#by-hand |
| docs/commands/workspace-publish.md | Usage | --json | flag | живо | найден в собранном корпусе `--help` | перенесён → reference/commands#global |
| docs/commands/workspace-publish.md | Usage | --quiet | flag | живо | найден в собранном корпусе `--help` | перенесён → reference/commands#global |
| docs/commands/workspace-publish.md | Which nodes are published | [package] | section | живо | найдено в crates/vibe-core/src/manifest/artifact/tests_validation.rs:372 | перенесён → reference/lock-file#package-entries |
| docs/commands/workspace-publish.md | Order — dependency-first | rel_path | section | устарело | поле не найдено ни в manifest source, ни в примерах vibe.toml | снят: устарело |
| docs/commands/workspace-publish.md | What a published copy carries | [origin] | section | живо | найдено в crates/vibe-core/src/manifest/document/tests.rs:178 | перенесён → howto/publish-a-package#workspaces |
| docs/commands/workspace-publish.md | What a published copy carries | generated_by | section | живо | найдено в crates/vibe-core/src/manifest/document/tests.rs:182 | перенесён → reference/lock-file#meta |
| docs/commands/workspace-publish.md | What a published copy carries | generated_at | section | живо | найдено в crates/vibe-core/src/manifest/document/tests.rs:183 | перенесён → reference/lock-file#meta |
| docs/commands/workspace-publish.md | What a published copy carries | vibe.toml | path | живо | найден в дереве (имя файла): vibe.toml | перенесён → reference/manifest#root |
| docs/commands/workspace-publish.md | Authentication | ~/.vibe/ | path | живо | найден на диске (эта машина): C:\Users\olegc\.vibe | перенесён → reference/settings-and-environment#the-folder |
| docs/commands/workspace-publish.md | Authentication | [[registry]] | section | живо | найдено в crates/vibe-core/src/manifest/artifact/tests_validation.rs:403 | перенесён → reference/manifest#sources |
| docs/commands/workspace-publish.md | Examples | vibe workspace publish --dry-run | cmd | живо | vibe.exe workspace publish --help -> exit 0 | перенесён → howto/publish-a-package#workspaces |
| docs/commands/workspace-publish.md | Examples | vibe workspace publish --member packages/flow-wal | cmd | живо | vibe.exe workspace publish --help -> exit 0 | перенесён → howto/publish-a-package#workspaces |
| docs/commands/workspace-publish.md | Deferred | has_issues | section | устарело | поле не найдено ни в manifest source, ни в примерах vibe.toml | снят: устарело |
| docs/commands/workspace-publish.md | Deferred | published_repos | section | устарело | поле не найдено ни в manifest source, ни в примерах vibe.toml | снят: устарело |
| docs/commands/workspace-publish.md | Deferred | --archive | flag | устарело | не найден ни в одном собранном `--help` | снят: устарело |
| docs/faq/README.md | Adding an entry | vibe.toml | path | живо | найден в дереве (имя файла): vibe.toml | перенесён → reference/manifest#root |
| docs/faq/version-conflicts.md | Resolving version conflicts | [[override]] | section | живо | найдено в crates/vibe-core/src/manifest/document/tests.rs:50 | перенесён → reference/manifest#sources |
| docs/faq/version-conflicts.md | TL;DR | vibe update | cmd | живо | vibe.exe update --help -> exit 0 | перенесён → howto/update-packages#root |
| docs/faq/version-conflicts.md | TL;DR | [patch] | section | живо | найдено в crates/vibe-core/src/manifest/document.rs:141 | снят: устарело — таблицы нет в схеме манифеста (crates/vibe-core/src/manifest, проверено 2026-09-12) |
| docs/faq/version-conflicts.md | TL;DR | vibe.toml | path | живо | найден в дереве (имя файла): vibe.toml | перенесён → reference/manifest#root |
| docs/faq/version-conflicts.md | TL;DR | [workspace.versions] | section | живо | найдено в crates/vibe-core/src/manifest/document/tests.rs:125 | снят: устарело — таблицы нет в схеме манифеста (crates/vibe-core/src/manifest, проверено 2026-09-12) |
| docs/faq/version-conflicts.md | Is it actually a conflict? | --solver | flag | живо | найден в собранном корпусе `--help` | перенесён → reference/commands#projects |
| docs/faq/version-conflicts.md | The fix ladder | [requires.packages] | section | живо | найдено в crates/vibe-core/src/manifest/artifact/tests_validation.rs:398 | перенесён → reference/manifest#requirements |
| docs/faq/version-conflicts.md | 2. Update or downgrade the intermediate dependency | vibe install | cmd | живо | vibe.exe install --help -> exit 0 | перенесён → howto/install-a-package#what-happens |
| docs/faq/version-conflicts.md | 3. `[[override]]` — force a version | source_url | section | живо | найдено в crates/vibe-core/src/manifest/document/tests.rs:52 | перенесён → reference/lock-file#package-entries |
| docs/faq/version-conflicts.md | 3. `[[override]]` — force a version | --overrides | flag | устарело | не найден ни в одном собранном `--help` | снят: устарело |
| docs/faq/version-conflicts.md | 3. `[[override]]` — force a version | [[registry]] | section | живо | найдено в crates/vibe-core/src/manifest/artifact/tests_validation.rs:403 | перенесён → reference/manifest#sources |
| docs/faq/version-conflicts.md | 3. `[[override]]` — force a version | vibe list --overrides | cmd | живо | vibe.exe list --help -> exit 0 | перенесён → reference/machine-formats#root |
| docs/faq/version-conflicts.md | 3. `[[override]]` — force a version | content_hash | section | живо | найдено в crates/vibe-core/src/manifest/document/tests.rs:292 | перенесён → reference/lock-file#package-entries |
| docs/faq/version-conflicts.md | 3. `[[override]]` — force a version | vibe.lock | path | живо | найден в дереве (имя файла): vibe.lock | перенесён → reference/lock-file#root |
| docs/faq/version-conflicts.md | 5. `version.var` / `[workspace.versions]` — centralise your own versions | [workspace] | section | живо | найдено в crates/vibe-core/src/manifest/artifact/tests_validation.rs:15 | перенесён → howto/set-up-a-workspace#root |
| docs/faq/version-conflicts.md | 5. `version.var` / `[workspace.versions]` — centralise your own versions | vibevm/auth | path | устарело | путь не существует: vibevm/auth | перенесён → howto/set-up-a-workspace#what-happens |
| docs/git-source-dependencies.md | Git-source dependencies — whole-repo-as-package | [[registry]] | section | живо | найдено в crates/vibe-core/src/manifest/artifact/tests_validation.rs:403 | перенесён → reference/manifest#sources |
| docs/git-source-dependencies.md | Git-source dependencies — whole-repo-as-package | [package] | section | живо | найдено в crates/vibe-core/src/manifest/artifact/tests_validation.rs:372 | перенесён → reference/lock-file#package-entries |
| docs/git-source-dependencies.md | Git-source dependencies — whole-repo-as-package | [dependencies] | section | живо | найдено в crates/vibe-core/src/manifest/package/capabilities.rs:312 | снят: устарело — таблицы нет в схеме манифеста (crates/vibe-core/src/manifest, проверено 2026-09-12) |
| docs/git-source-dependencies.md | Git-source dependencies — whole-repo-as-package | vibe.toml | path | живо | найден в дереве (имя файла): vibe.toml | перенесён → reference/manifest#root |
| docs/git-source-dependencies.md | When to use | [[override]] | section | живо | найдено в crates/vibe-core/src/manifest/document/tests.rs:50 | перенесён → reference/manifest#sources |
| docs/git-source-dependencies.md | Wire form | [requires.packages] | section | живо | найдено в crates/vibe-core/src/manifest/artifact/tests_validation.rs:398 | перенесён → reference/manifest#requirements |
| docs/git-source-dependencies.md | Wire form | vibevm/rust-cli | path | устарело | путь не существует: vibevm/rust-cli | снят: устарело |
| docs/git-source-dependencies.md | Wire form | token_env | section | живо | найдено в crates/vibe-core/src/manifest/document/tests.rs:569 | перенесён → reference/manifest#sources |
| docs/git-source-dependencies.md | Inline-table fields | vibe install | cmd | живо | vibe.exe install --help -> exit 0 | перенесён → howto/install-a-package#what-happens |
| docs/git-source-dependencies.md | Inline-table fields | vibe update | cmd | живо | vibe.exe update --help -> exit 0 | перенесён → howto/update-packages#root |
| docs/git-source-dependencies.md | Adding a git-source via CLI | vibe install flow:wal@^0.3 | cmd | живо | vibe.exe install --help -> exit 0 | перенесён → howto/install-a-package#what-happens |
| docs/git-source-dependencies.md | Adding a git-source via CLI | vibe install flow:internal-helper \ | cmd | живо | vibe.exe install --help -> exit 0 | перенесён → howto/install-a-package#what-happens |
| docs/git-source-dependencies.md | Adding a git-source via CLI | --git | flag | живо | найден в собранном корпусе `--help` | перенесён → reference/commands#projects |
| docs/git-source-dependencies.md | Adding a git-source via CLI | --tag | flag | живо | найден в собранном корпусе `--help` | перенесён → start/install-vibe#edge-cases |
| docs/git-source-dependencies.md | Adding a git-source via CLI | vibe install flow:experimental \ | cmd | живо | vibe.exe install --help -> exit 0 | перенесён → howto/install-a-package#what-happens |
| docs/git-source-dependencies.md | Adding a git-source via CLI | --branch | flag | живо | найден в собранном корпусе `--help` | перенесён → reference/commands#projects |
| docs/git-source-dependencies.md | Adding a git-source via CLI | vibe install flow:fork \ | cmd | живо | vibe.exe install --help -> exit 0 | перенесён → howto/install-a-package#what-happens |
| docs/git-source-dependencies.md | Adding a git-source via CLI | --rev | flag | живо | найден в собранном корпусе `--help` | перенесён → reference/commands#projects |
| docs/git-source-dependencies.md | Adding a git-source via CLI | vibe install flow:secret \ | cmd | живо | vibe.exe install --help -> exit 0 | перенесён → howto/install-a-package#what-happens |
| docs/git-source-dependencies.md | Adding a git-source via CLI | --git-auth | flag | живо | найден в собранном корпусе `--help` | перенесён → reference/commands#projects |
| docs/git-source-dependencies.md | Adding a git-source via CLI | --git-token-env | flag | живо | найден в собранном корпусе `--help` | перенесён → reference/commands#projects |
| docs/git-source-dependencies.md | Adding a git-source via CLI | --exact | flag | живо | найден в собранном корпусе `--help` | перенесён → howto/install-a-package#constraints |
| docs/git-source-dependencies.md | Adding a git-source via CLI | --registry | flag | живо | найден в собранном корпусе `--help` | перенесён → howto/publish-a-package#by-hand |
| docs/git-source-dependencies.md | Resolution order | [patch] | section | живо | найдено в crates/vibe-core/src/manifest/document.rs:141 | снят: устарело — таблицы нет в схеме манифеста (crates/vibe-core/src/manifest, проверено 2026-09-12) |
| docs/git-source-dependencies.md | Identity | vibevm/internal | path | устарело | путь не существует: vibevm/internal | перенесён → start/what-a-project-contains#who-writes-what |
| docs/git-source-dependencies.md | Mutability and `vibe update` | resolved_commit | section | живо | найдено в crates/vibe-core/src/manifest/lockfile/tests.rs:34 | перенесён → reference/lock-file#package-entries |
| docs/git-source-dependencies.md | Lockfile | source_kind | section | живо | найдено в crates/vibe-core/src/manifest/lockfile/tests.rs:36 | перенесён → reference/lock-file#package-entries |
| docs/git-source-dependencies.md | Lockfile | source_url | section | живо | найдено в crates/vibe-core/src/manifest/document/tests.rs:52 | перенесён → reference/lock-file#package-entries |
| docs/git-source-dependencies.md | Lockfile | vibe.lock | path | живо | найден в дереве (имя файла): vibe.lock | перенесён → reference/lock-file#root |
| docs/git-source-dependencies.md | Lockfile | [[package]] | section | живо | найдено в crates/vibe-core/src/manifest/lockfile/tests.rs:26 | перенесён → reference/lock-file#package-entries |
| docs/git-source-dependencies.md | Lockfile | source_ref | section | живо | найдено в crates/vibe-core/src/manifest/lockfile/tests.rs:33 | перенесён → reference/lock-file#package-entries |
| docs/git-source-dependencies.md | Lockfile | content_hash | section | живо | найдено в crates/vibe-core/src/manifest/document/tests.rs:292 | перенесён → reference/lock-file#package-entries |
| docs/git-source-dependencies.md | Transitive dependencies | [requires] | section | живо | найдено в crates/vibe-core/src/manifest/document.rs:22 | перенесён → reference/manifest#requirements |
| docs/git-source-dependencies.md | Comparison with `[[override]]` | vibe list --overrides | cmd | живо | vibe.exe list --help -> exit 0 | перенесён → reference/machine-formats#root |
| docs/git-source-dependencies.md | Comparison with `[[override]]` | --overrides | flag | устарело | не найден ни в одном собранном `--help` | снят: устарело |
| docs/git-source-dependencies.md | Comparison with `[[override]]` | vibe uninstall <pkgref> | cmd | живо | vibe.exe uninstall --help -> exit 0 | перенесён → howto/remove-a-package#what-happens |
| docs/git-source-dependencies.md | Out of scope | [[mirror]] | section | живо | найдено в crates/vibe-core/src/manifest/artifact/tests_validation.rs:406 | перенесён → reference/manifest#sources |
| docs/git-source-dependencies.md | Out of scope | vibe registry test | cmd | живо | vibe.exe registry test --help -> exit 0 | перенесён → howto/use-a-private-registry#root |
| docs/glossary.md | apply (install pipeline stage) | vibe install | cmd | живо | vibe.exe install --help -> exit 0 | перенесён → howto/install-a-package#what-happens |
| docs/glossary.md | boot snippet | [boot_snippet] | section | живо | найдено в crates/vibe-core/src/manifest/document/tests.rs:81 | перенесён → reference/manifest#deliveries |
| docs/glossary.md | boot artifacts | [[entry]] | section | живо | найдено в crates/vibe-core/src/manifest/artifact/binary_projection.rs:6 | перенесён → model/boot-lane#the-order |
| docs/glossary.md | link type (boot inclusion type) | [requires.packages] | section | живо | найдено в crates/vibe-core/src/manifest/artifact/tests_validation.rs:398 | перенесён → reference/manifest#requirements |
| docs/glossary.md | link type (boot inclusion type) | vibe.toml | path | живо | найден в дереве (имя файла): vibe.toml | перенесён → reference/manifest#root |
| docs/glossary.md | canonical URL (registry) | [[registry]] | section | живо | найдено в crates/vibe-core/src/manifest/artifact/tests_validation.rs:403 | перенесён → reference/manifest#sources |
| docs/glossary.md | capability | [provides] | section | живо | найдено в crates/vibe-core/src/manifest/document/tests.rs:84 | перенесён → reference/manifest#requirements |
| docs/glossary.md | capability | [requires] | section | живо | найдено в crates/vibe-core/src/manifest/document.rs:22 | перенесён → reference/manifest#requirements |
| docs/glossary.md | capability | crates/vibe-core/src/capability_ref.rs | path | живо | путь существует: crates/vibe-core/src/capability_ref.rs | снят: пути исходников не входят в руководство; архитектура ссылается на карту трассируемости (architecture/traceability#root) |
| docs/glossary.md | content_hash | [[package]] | section | живо | найдено в crates/vibe-core/src/manifest/lockfile/tests.rs:26 | перенесён → reference/lock-file#package-entries |
| docs/glossary.md | content_hash | crates/vibe-registry/src/lib.rs | path | живо | путь существует: crates/vibe-registry/src/lib.rs | снят: пути исходников не входят в руководство; архитектура ссылается на карту трассируемости (architecture/traceability#root) |
| docs/glossary.md | content_hash | vibe.lock | path | живо | найден в дереве (имя файла): vibe.lock | перенесён → reference/lock-file#root |
| docs/glossary.md | content drift | content_hash | section | живо | найдено в crates/vibe-core/src/manifest/document/tests.rs:292 | перенесён → reference/lock-file#package-entries |
| docs/glossary.md | group | [a-z0-9_-] | section | устарело | не найдено ни в manifest source, ни в примерах vibe.toml | снят: устарело |
| docs/glossary.md | group | [package] | section | живо | найдено в crates/vibe-core/src/manifest/artifact/tests_validation.rs:372 | перенесён → reference/lock-file#package-entries |
| docs/glossary.md | kind | --kind | flag | живо | найден в собранном корпусе `--help` | перенесён → model/packages-and-kinds#the-kinds |
| docs/glossary.md | manifest | [project] | section | живо | найдено в crates/vibe-core/src/manifest/artifact/applicability.rs:16 | перенесён → reference/manifest#one-file |
| docs/glossary.md | manifest | [workspace] | section | живо | найдено в crates/vibe-core/src/manifest/artifact/tests_validation.rs:15 | перенесён → howto/set-up-a-workspace#root |
| docs/glossary.md | manifest | crates/vibe-core/src/manifest/ | path | живо | путь существует: crates/vibe-core/src/manifest/ | перенесён → reference/manifest#root |
| docs/glossary.md | manifest | crates/vibe-core/src/manifest | path | живо | путь существует: crates/vibe-core/src/manifest | перенесён → reference/manifest#root |
| docs/glossary.md | mirror | [[mirror]] | section | живо | найдено в crates/vibe-core/src/manifest/artifact/tests_validation.rs:406 | перенесён → reference/manifest#sources |
| docs/glossary.md | materialised dependency | vibedeps/ | path | неизвестно | генерируемый каталог (материализуется `vibe install`), механическая проверка по дереву неинформативна для непроинсталлированного чекаута | перенесён → model/two-trees#the-rule |
| docs/glossary.md | override | vibe install <pkgref> | cmd | живо | vibe.exe install --help -> exit 0 | перенесён → howto/install-a-package#what-happens |
| docs/glossary.md | override | [[override]] | section | живо | найдено в crates/vibe-core/src/manifest/document/tests.rs:50 | перенесён → reference/manifest#sources |
| docs/glossary.md | override | source_url | section | живо | найдено в crates/vibe-core/src/manifest/document/tests.rs:52 | перенесён → reference/lock-file#package-entries |
| docs/glossary.md | pkgref (package reference) | crates/vibe-core/src/package_ref.rs | path | живо | путь существует: crates/vibe-core/src/package_ref.rs | снят: пути исходников не входят в руководство; архитектура ссылается на карту трассируемости (architecture/traceability#root) |
| docs/glossary.md | plan (install pipeline stage) | crates/vibe-install/src/lib.rs | path | живо | путь существует: crates/vibe-install/src/lib.rs | снят: пути исходников не входят в руководство; архитектура ссылается на карту трассируемости (architecture/traceability#root) |
| docs/glossary.md | resolve (install pipeline stage) | crates/vibe-resolver/src/lib.rs | path | живо | путь существует: crates/vibe-resolver/src/lib.rs | снят: пути исходников не входят в руководство; архитектура ссылается на карту трассируемости (architecture/traceability#root) |
| docs/glossary.md | root dependency | vibe uninstall <root> | cmd | живо | vibe.exe uninstall --help -> exit 0 | перенесён → howto/remove-a-package#what-happens |
| docs/glossary.md | root dependency | vibe uninstall <transitive> | cmd | живо | vibe.exe uninstall --help -> exit 0 | перенесён → howto/remove-a-package#what-happens |
| docs/glossary.md | root dependency | [meta] | section | живо | найдено в crates/vibe-core/src/manifest/lockfile/tests.rs:19 | перенесён → reference/lock-file#meta |
| docs/glossary.md | `source_ref` | source_ref | section | живо | найдено в crates/vibe-core/src/manifest/lockfile/tests.rs:33 | перенесён → reference/lock-file#package-entries |
| docs/glossary.md | `[package]` (manifest table) | crates/vibe-core/src/manifest/package.rs | path | живо | путь существует: crates/vibe-core/src/manifest/package.rs | снят: пути исходников не входят в руководство; архитектура ссылается на карту трассируемости (architecture/traceability#root) |
| docs/glossary.md | vibe.toml | [active] | section | живо | найдено в crates/vibe-core/src/manifest/artifact/tests_validation.rs:401 | перенесён → reference/manifest#root (derived manifest-field, фаза 2 A2.13) |
| docs/glossary.md | vibe.toml | [llm] | section | живо | найдено в crates/vibe-core/src/manifest/document/tests.rs:36 | перенесён → reference/manifest#root (derived manifest-field, фаза 2 A2.13) |
| docs/glossary.md | vibe.toml | [origin] | section | живо | найдено в crates/vibe-core/src/manifest/document/tests.rs:178 | перенесён → reference/manifest#root (derived manifest-field, фаза 2 A2.13) |
| docs/glossary.md | vibe.toml | crates/vibe-core/src/manifest/project.rs | path | живо | путь существует: crates/vibe-core/src/manifest/project.rs | снят: пути исходников не входят в руководство; архитектура ссылается на карту трассируемости (architecture/traceability#root) |
| docs/glossary.md | Anti-vocabulary | vibe vendor | cmd | устарело | vibe.exe vendor --help -> exit 2; резолвится только до '(корень)' | снят: устарело |
| docs/guides/agent-mcp-quickstart-opencode.md | Quickstart: opencode + vibevm hello-world | vibe init | cmd | живо | vibe.exe init --help -> exit 0 | перенесён → howto/use-a-private-registry#machine-wide |
| docs/guides/agent-mcp-quickstart-opencode.md | 0. Prerequisites | vibe install flow:wal | cmd | живо | vibe.exe install --help -> exit 0 | перенесён → howto/install-a-package#what-happens |
| docs/guides/agent-mcp-quickstart-opencode.md | 1. Make `vibe` discoverable to opencode | vibe mcp serve | cmd | живо | vibe.exe mcp serve --help -> exit 0 | перенесён → agent/ask-your-agent#two-transports |
| docs/guides/agent-mcp-quickstart-opencode.md | 1. Make `vibe` discoverable to opencode | crates/vibe-cli | path | живо | путь существует: crates/vibe-cli | перенесён → architecture/how-vibe-is-built#the-crates |
| docs/guides/agent-mcp-quickstart-opencode.md | 1. Make `vibe` discoverable to opencode | --path | flag | живо | найден в собранном корпусе `--help` | перенесён → howto/update-packages#by-hand |
| docs/guides/agent-mcp-quickstart-opencode.md | 1. Make `vibe` discoverable to opencode | --locked | flag | устарело | не найден ни в одном собранном `--help` | снят: устарело |
| docs/guides/agent-mcp-quickstart-opencode.md | 1. Make `vibe` discoverable to opencode | vibe --version        # vibe 0.1.0-dev | cmd | живо | глобальный флаг --version найден в `vibe --help` | перенесён → reference/commands#global |
| docs/guides/agent-mcp-quickstart-opencode.md | 1. Make `vibe` discoverable to opencode | --version | flag | живо | найден в собранном корпусе `--help` | перенесён → start/index#step-2 |
| docs/guides/agent-mcp-quickstart-opencode.md | 1. Make `vibe` discoverable to opencode | vibe --version | cmd | живо | глобальный флаг --version найден в `vibe --help` | перенесён → reference/commands#global |
| docs/guides/agent-mcp-quickstart-opencode.md | 2. Bootstrap vibevm globally (one-time, no project needed) | vibe.toml | path | живо | найден в дереве (имя файла): vibe.toml | перенесён → reference/manifest#root |
| docs/guides/agent-mcp-quickstart-opencode.md | 2. Bootstrap vibevm globally (one-time, no project needed) | vibe mcp install --auto --scope user --invoked-by manual-bootstrap | cmd | живо | vibe.exe mcp install --help -> exit 0 | перенесён → authoring/ship-tools-and-mcp-servers#servers |
| docs/guides/agent-mcp-quickstart-opencode.md | 2. Bootstrap vibevm globally (one-time, no project needed) | --auto | flag | живо | найден в собранном корпусе `--help` | перенесён → agent/give-your-agent-the-skill#what-happens |
| docs/guides/agent-mcp-quickstart-opencode.md | 2. Bootstrap vibevm globally (one-time, no project needed) | --scope | flag | живо | найден в собранном корпусе `--help` | перенесён → agent/give-your-agent-the-skill#by-hand |
| docs/guides/agent-mcp-quickstart-opencode.md | 2. Bootstrap vibevm globally (one-time, no project needed) | --invoked-by | flag | живо | найден в собранном корпусе `--help` | перенесён → reference/commands#global |
| docs/guides/agent-mcp-quickstart-opencode.md | 2. Bootstrap vibevm globally (one-time, no project needed) | vibevm/SKILL.md | path | устарело | путь не существует: vibevm/SKILL.md | перенесён → agent/give-your-agent-the-skill#what-happens |
| docs/guides/agent-mcp-quickstart-opencode.md | Prompt A — minimum (just probe MCP wiring) | query_package | section | устарело | поле не найдено ни в manifest source, ни в примерах vibe.toml | снят: устарело |
| docs/guides/agent-mcp-quickstart-opencode.md | Prompt B — bootstrap a hello-world project (full demo) | --help | flag | живо | найден в собранном корпусе `--help` | перенесён → howto/read-documentation-locally#by-hand |
| docs/guides/agent-mcp-quickstart-opencode.md | Prompt B — bootstrap a hello-world project (full demo) | vibe.lock | path | живо | найден в дереве (имя файла): vibe.lock | перенесён → reference/lock-file#root |
| docs/guides/agent-mcp-quickstart-opencode.md | Prompt B — bootstrap a hello-world project (full demo) | vibe list | cmd | живо | vibe.exe list --help -> exit 0 | перенесён → reference/machine-formats#root |
| docs/guides/agent-mcp-quickstart-opencode.md | Prompt B — bootstrap a hello-world project (full demo) | vibe show config | cmd | живо | vibe.exe show config --help -> exit 0 | перенесён → reference/settings-and-environment#edge-cases |
| docs/guides/agent-mcp-quickstart-opencode.md | Prompt B — bootstrap a hello-world project (full demo) | vibe outdated | cmd | живо | vibe.exe outdated --help -> exit 0 | перенесён → model/versions#root |
| docs/guides/agent-mcp-quickstart-opencode.md | Prompt B — bootstrap a hello-world project (full demo) | vibe … | cmd | неизвестно | нет подкоманды, флаг не опознан в `vibe --help` | снят: устарело |
| docs/guides/agent-mcp-quickstart-opencode.md | Prompt B — bootstrap a hello-world project (full demo) | vibe install <pkgref> | cmd | живо | vibe.exe install --help -> exit 0 | перенесён → howto/install-a-package#what-happens |
| docs/guides/agent-mcp-quickstart-opencode.md | Prompt C — operator wants project-scope skill committed too | vibe mcp install --scope project --what skill --agent opencode --invoked-by opencode | cmd | живо | vibe.exe mcp install --help -> exit 0 | перенесён → authoring/ship-tools-and-mcp-servers#servers |
| docs/guides/agent-mcp-quickstart-opencode.md | Prompt C — operator wants project-scope skill committed too | --what | flag | живо | найден в собранном корпусе `--help` | перенесён → reference/commands#agents |
| docs/guides/agent-mcp-quickstart-opencode.md | Prompt C — operator wants project-scope skill committed too | --agent | flag | живо | найден в собранном корпусе `--help` | перенесён → agent/give-your-agent-the-skill#by-hand |
| docs/guides/agent-mcp-quickstart-opencode.md | Acceptance checklist | vibe mcp install --auto --scope user | cmd | живо | vibe.exe mcp install --help -> exit 0 | перенесён → authoring/ship-tools-and-mcp-servers#servers |
| docs/guides/agent-mcp-quickstart-opencode.md | Acceptance checklist | read_subskill | section | живо | найдено в crates/vibe-core/src/manifest/lockfile.rs:473 | снят: внутренний интерфейс MCP-сервера; список инструментов вернётся генератором (X-031) |
| docs/guides/agent-mcp-quickstart-opencode.md | Acceptance checklist | materialise_subskill | section | устарело | поле не найдено ни в manifest source, ни в примерах vibe.toml | снят: устарело |
| docs/guides/agent-mcp-quickstart-opencode.md | Acceptance checklist | vibe install | cmd | живо | vibe.exe install --help -> exit 0 | перенесён → howto/install-a-package#what-happens |
| docs/guides/agent-mcp-quickstart-opencode.md | Acceptance checklist | vibedeps/ | path | неизвестно | генерируемый каталог (материализуется `vibe install`), механическая проверка по дереву неинформативна для непроинсталлированного чекаута | перенесён → model/two-trees#the-rule |
| docs/guides/agent-mcp-quickstart-opencode.md | Acceptance checklist | vibe --json mcp status | cmd | живо | глобальный флаг --json найден в `vibe --help` | перенесён → reference/commands#global |
| docs/guides/agent-mcp-quickstart-opencode.md | Acceptance checklist | --json | flag | живо | найден в собранном корпусе `--help` | перенесён → reference/commands#global |
| docs/guides/agent-mcp-quickstart-opencode.md | Lifecycle commands | vibe mcp install | cmd | живо | vibe.exe mcp install --help -> exit 0 | перенесён → authoring/ship-tools-and-mcp-servers#servers |
| docs/guides/agent-mcp-quickstart-opencode.md | Lifecycle commands | vibe mcp upgrade | cmd | живо | vibe.exe mcp upgrade --help -> exit 0 | перенесён → agent/give-your-agent-the-skill#edge-cases |
| docs/guides/agent-mcp-quickstart-opencode.md | Lifecycle commands | vibe mcp uninstall | cmd | живо | vibe.exe mcp uninstall --help -> exit 0 | перенесён → agent/give-your-agent-the-skill#edge-cases |
| docs/guides/agent-mcp-quickstart-opencode.md | Lifecycle commands | vibe mcp status | cmd | живо | vibe.exe mcp status --help -> exit 0 | перенесён → agent/give-your-agent-the-skill#root |
| docs/guides/agent-mcp-quickstart-opencode.md | Lifecycle commands | vibe mcp upgrade --invoked-by manual --yes     # refresh user-level installs | cmd | живо | vibe.exe mcp upgrade --help -> exit 0 | перенесён → agent/give-your-agent-the-skill#edge-cases |
| docs/guides/agent-mcp-quickstart-opencode.md | Lifecycle commands | --yes | flag | живо | найден в собранном корпусе `--help` | перенесён → agent/give-your-agent-the-skill#what-happens |
| docs/guides/agent-mcp-quickstart-opencode.md | Troubleshooting | vibe not found | cmd | устарело | vibe.exe not --help -> exit 2; резолвится только до '(корень)' | снят: устарело |
| docs/guides/agent-mcp-quickstart-opencode.md | Troubleshooting | vibe install flow:org.vibevm.world/wal | cmd | живо | vibe.exe install --help -> exit 0 | перенесён → howto/install-a-package#what-happens |
| docs/guides/agent-mcp-quickstart-opencode.md | Troubleshooting | vibe mcp uninstall --auto-equivalent: --agent all --scope user --yes | cmd | живо | vibe.exe mcp uninstall --help -> exit 0 | перенесён → agent/give-your-agent-the-skill#edge-cases |
| docs/guides/agent-mcp-quickstart-opencode.md | Troubleshooting | --auto-equivalent | flag | устарело | не найден ни в одном собранном `--help` | снят: устарело |
| docs/guides/agent-mcp-quickstart-opencode.md | Maintenance | crates/vibe-cli/src/commands/skill_template.md | path | устарело | путь не существует: crates/vibe-cli/src/commands/skill_template.md | снят: устарело |
| docs/loading-model.md | The loading model — how a vibevm project boots | vibe install | cmd | живо | vibe.exe install --help -> exit 0 | перенесён → howto/install-a-package#what-happens |
| docs/loading-model.md | Materialised `vibedeps/` — vibevm's | vibedeps/ | path | неизвестно | генерируемый каталог (материализуется `vibe install`), механическая проверка по дереву неинформативна для непроинсталлированного чекаута | перенесён → model/two-trees#the-rule |
| docs/loading-model.md | Materialised `vibedeps/` — vibevm's | vibe.toml | path | живо | найден в дереве (имя файла): vibe.toml | перенесён → reference/manifest#root |
| docs/loading-model.md | Materialised `vibedeps/` — vibevm's | [writes] | section | живо | найдено в crates/vibe-core/src/manifest/document.rs:25 | снят: устарело — таблицы нет в схеме манифеста (crates/vibe-core/src/manifest, проверено 2026-09-12) |
| docs/loading-model.md | Generated artifacts: `INLINE.md` and `INDEX.md` | [[entry]] | section | живо | найдено в crates/vibe-core/src/manifest/artifact/binary_projection.rs:6 | перенесён → model/boot-lane#the-order |
| docs/loading-model.md | Generated artifacts: `INLINE.md` and `INDEX.md` | vibedeps/stack-rust/2.1.0/boot/rust.md | path | неизвестно | генерируемый каталог (материализуется `vibe install`), механическая проверка по дереву неинформативна для непроинсталлированного чекаута | снят: устарело — пример пути прежней раскладки |
| docs/loading-model.md | Link types — `inline`, `static`, `dynamic` | [requires.packages] | section | живо | найдено в crates/vibe-core/src/manifest/artifact/tests_validation.rs:398 | перенесён → reference/manifest#requirements |
| docs/loading-model.md | Link types — `inline`, `static`, `dynamic` | [activation] | section | живо | найдено в crates/vibe-core/src/manifest/subskill.rs:35 | перенесён → reference/manifest#root (derived manifest-field, фаза 2 A2.13) |
| docs/loading-model.md | Link types — `inline`, `static`, `dynamic` | [boot_snippet] | section | живо | найдено в crates/vibe-core/src/manifest/document/tests.rs:81 | перенесён → reference/manifest#deliveries |
| docs/loading-model.md | The managed `<vibevm>` block | vibe init | cmd | живо | vibe.exe init --help -> exit 0 | перенесён → howto/use-a-private-registry#machine-wide |
| docs/loading-model.md | Regenerating the model | vibe reinstall | cmd | живо | vibe.exe reinstall --help -> exit 0 | перенесён → howto/update-packages#recovery |
| docs/loading-model.md | Regenerating the model | vibe reinstall --force | cmd | живо | vibe.exe reinstall --help -> exit 0 | перенесён → howto/update-packages#recovery |
| docs/loading-model.md | Regenerating the model | vibe.lock | path | живо | найден в дереве (имя файла): vibe.lock | перенесён → reference/lock-file#root |
| docs/loading-model.md | Regenerating the model | --force | flag | живо | найден в собранном корпусе `--help` | перенесён → model/two-trees#what-regenerates |
| docs/lockfile-format.md | `vibe.lock` — schema reference | vibe.lock | path | живо | найден в дереве (имя файла): vibe.lock | перенесён → reference/lock-file#root |
| docs/lockfile-format.md | `vibe.lock` — schema reference | vibe list | cmd | живо | vibe.exe list --help -> exit 0 | перенесён → reference/machine-formats#root |
| docs/lockfile-format.md | `vibe.lock` — schema reference | vibe uninstall | cmd | живо | vibe.exe uninstall --help -> exit 0 | перенесён → howto/remove-a-package#what-happens |
| docs/lockfile-format.md | `vibe.lock` — schema reference | vibe reinstall | cmd | живо | vibe.exe reinstall --help -> exit 0 | перенесён → howto/update-packages#recovery |
| docs/lockfile-format.md | `vibe.lock` — schema reference | vibe registry sync | cmd | живо | vibe.exe registry sync --help -> exit 0 | перенесён → reference/machine-formats#the-documents |
| docs/lockfile-format.md | `vibe.lock` — schema reference | vibedeps/ | path | неизвестно | генерируемый каталог (материализуется `vibe install`), механическая проверка по дереву неинформативна для непроинсталлированного чекаута | перенесён → model/two-trees#the-rule |
| docs/lockfile-format.md | `vibe.lock` — schema reference | crates/vibe-core/src/manifest/lockfile.rs | path | живо | путь существует: crates/vibe-core/src/manifest/lockfile.rs | снят: пути исходников не входят в руководство; архитектура ссылается на карту трассируемости (architecture/traceability#root) |
| docs/lockfile-format.md | Top-level shape | [meta] | section | живо | найдено в crates/vibe-core/src/manifest/lockfile/tests.rs:19 | перенесён → reference/lock-file#meta |
| docs/lockfile-format.md | Top-level shape | generated_by | section | живо | найдено в crates/vibe-core/src/manifest/document/tests.rs:182 | перенесён → reference/lock-file#meta |
| docs/lockfile-format.md | Top-level shape | generated_at | section | живо | найдено в crates/vibe-core/src/manifest/document/tests.rs:183 | перенесён → reference/lock-file#meta |
| docs/lockfile-format.md | Top-level shape | schema_version | section | живо | найдено в crates/vibe-core/src/manifest/lockfile/tests.rs:22 | перенесён → reference/lock-file#meta |
| docs/lockfile-format.md | Top-level shape | root_dependencies | section | живо | найдено в crates/vibe-core/src/manifest/lockfile/tests.rs:24 | перенесён → reference/lock-file#meta |
| docs/lockfile-format.md | Top-level shape | vibevm/rust-cli | path | устарело | путь не существует: vibevm/rust-cli | снят: устарело |
| docs/lockfile-format.md | Top-level shape | [[package]] | section | живо | найдено в crates/vibe-core/src/manifest/lockfile/tests.rs:26 | перенесён → reference/lock-file#package-entries |
| docs/lockfile-format.md | Top-level shape | vibe install | cmd | живо | vibe.exe install --help -> exit 0 | перенесён → howto/install-a-package#what-happens |
| docs/lockfile-format.md | Top-level shape | vibe update | cmd | живо | vibe.exe update --help -> exit 0 | перенесён → howto/update-packages#root |
| docs/lockfile-format.md | Top-level shape | deny_unknown_fields | section | живо | найдено в crates/vibe-core/src/manifest/artifact/wire.rs:31 | перенесён → reference/lock-file#edge-cases |
| docs/lockfile-format.md | `[meta]` fields | vibe <version> | cmd | неизвестно | нет подкоманды, флаг не опознан в `vibe --help` | снят: устарело |
| docs/lockfile-format.md | `[meta]` fields | register_installed | section | устарело | поле не найдено ни в manifest source, ни в примерах vibe.toml | снят: устарело |
| docs/lockfile-format.md | `[meta]` fields | unregister_installed | section | устарело | поле не найдено ни в manifest source, ни в примерах vibe.toml | снят: устарело |
| docs/lockfile-format.md | `[meta]` fields | vibe install <pkgref> | cmd | живо | vibe.exe install --help -> exit 0 | перенесён → howto/install-a-package#what-happens |
| docs/lockfile-format.md | `[[package]]` entries | [[registry]] | section | живо | найдено в crates/vibe-core/src/manifest/artifact/tests_validation.rs:403 | перенесён → reference/manifest#sources |
| docs/lockfile-format.md | `[[package]]` entries | vibe.toml | path | живо | найден в дереве (имя файла): vibe.toml | перенесён → reference/manifest#root |
| docs/lockfile-format.md | `[[package]]` entries | --registry | flag | живо | найден в собранном корпусе `--help` | перенесён → howto/publish-a-package#by-hand |
| docs/lockfile-format.md | `[[package]]` entries | source_kind | section | живо | найдено в crates/vibe-core/src/manifest/lockfile/tests.rs:36 | перенесён → reference/lock-file#package-entries |
| docs/lockfile-format.md | `[[package]]` entries | [requires.packages] | section | живо | найдено в crates/vibe-core/src/manifest/artifact/tests_validation.rs:398 | перенесён → reference/manifest#requirements |
| docs/lockfile-format.md | `[[package]]` entries | [[override]] | section | живо | найдено в crates/vibe-core/src/manifest/document/tests.rs:50 | перенесён → reference/manifest#sources |
| docs/lockfile-format.md | `[[package]]` entries | source_url | section | живо | найдено в crates/vibe-core/src/manifest/document/tests.rs:52 | перенесён → reference/lock-file#package-entries |
| docs/lockfile-format.md | `[[package]]` entries | source_ref | section | живо | найдено в crates/vibe-core/src/manifest/lockfile/tests.rs:33 | перенесён → reference/lock-file#package-entries |
| docs/lockfile-format.md | `[[package]]` entries | vibe check | cmd | живо | vibe.exe check --help -> exit 0 | перенесён → howto/remove-a-package#root |
| docs/lockfile-format.md | `[[package]]` entries | resolved_commit | section | живо | найдено в crates/vibe-core/src/manifest/lockfile/tests.rs:34 | перенесён → reference/lock-file#package-entries |
| docs/lockfile-format.md | `[[package]]` entries | content_hash | section | живо | найдено в crates/vibe-core/src/manifest/document/tests.rs:292 | перенесён → reference/lock-file#package-entries |
| docs/lockfile-format.md | `[[package]]` entries | boot_snippet | section | живо | найдено в crates/vibe-core/src/manifest/document/tests.rs:81 | перенесён → reference/manifest#deliveries |
| docs/lockfile-format.md | `[[package]]` entries | files_written | section | живо | найдено в crates/vibe-core/src/manifest/lockfile/tests.rs:38 | перенесён → reference/lock-file#package-entries |
| docs/lockfile-format.md | `[[package]]` entries | vibe list --overrides | cmd | живо | vibe.exe list --help -> exit 0 | перенесён → reference/machine-formats#root |
| docs/lockfile-format.md | `[[package]]` entries | --overrides | flag | устарело | не найден ни в одном собранном `--help` | снят: устарело |
| docs/lockfile-format.md | `[[package]]` entries | --trust-mirror | flag | устарело | не найден ни в одном собранном `--help` | снят: устарело |
| docs/lockfile-format.md | Identity model | crates/vibe-install/src/lib.rs | path | живо | путь существует: crates/vibe-install/src/lib.rs | снят: пути исходников не входят в руководство; архитектура ссылается на карту трассируемости (architecture/traceability#root) |
| docs/lockfile-format.md | Identity model | flake.lock | path | устарело | ни один файл с именем flake.lock не найден в дереве | снят: устарело |
| docs/lockfile-format.md | Tooling examples | --output-format | flag | устарело | не найден ни в одном собранном `--help` | снят: устарело |
| docs/lockfile-format.md | Tooling examples | vibe list --json | cmd | живо | vibe.exe list --help -> exit 0 | перенесён → reference/machine-formats#root |
| docs/lockfile-format.md | Tooling examples | --json | flag | живо | найден в собранном корпусе `--help` | перенесён → reference/commands#global |
| docs/lockfile-format.md | Worked example | vibe uninstall flow:atomic-commits | cmd | живо | vibe.exe uninstall --help -> exit 0 | перенесён → howto/remove-a-package#what-happens |
| docs/lockfile-format.md | Worked example | vibe uninstall flow:wal | cmd | живо | vibe.exe uninstall --help -> exit 0 | перенесён → howto/remove-a-package#what-happens |
| docs/lockfile-format.md | Worked example | vibe update --prune | cmd | живо | vibe.exe update --help -> exit 0 | перенесён → howto/update-packages#root |
| docs/lockfile-format.md | Worked example | --prune | flag | устарело | не найден ни в одном собранном `--help` | снят: устарело |
| docs/lockfile-format.md | Worked example | vibedeps/flow-wal/0.1.0/ | path | неизвестно | генерируемый каталог (материализуется `vibe install`), механическая проверка по дереву неинформативна для непроинсталлированного чекаута | перенесён → howto/publish-a-package#root |
| docs/lockfile-format.md | Worked example | vibedeps/flow-atomic-commits/0.1.0/ | path | неизвестно | генерируемый каталог (материализуется `vibe install`), механическая проверка по дереву неинформативна для непроинсталлированного чекаута | перенесён → howto/publish-a-package#root |
| docs/README.md | vibevm documentation | vibe <command> --help | cmd | неизвестно | нет подкоманды, флаг не опознан в `vibe --help` | снят: устарело |
| docs/README.md | vibevm documentation | --help | flag | живо | найден в собранном корпусе `--help` | перенесён → howto/read-documentation-locally#by-hand |
| docs/README.md | Start here | vibedeps/ | path | неизвестно | генерируемый каталог (материализуется `vibe install`), механическая проверка по дереву неинформативна для непроинсталлированного чекаута | перенесён → model/two-trees#the-rule |
| docs/README.md | Core commands | vibe init | cmd | живо | vibe.exe init --help -> exit 0 | перенесён → howto/use-a-private-registry#machine-wide |
| docs/README.md | Core commands | vibe install | cmd | живо | vibe.exe install --help -> exit 0 | перенесён → howto/install-a-package#what-happens |
| docs/README.md | Core commands | vibe update | cmd | живо | vibe.exe update --help -> exit 0 | перенесён → howto/update-packages#root |
| docs/README.md | Core commands | vibe uninstall | cmd | живо | vibe.exe uninstall --help -> exit 0 | перенесён → howto/remove-a-package#what-happens |
| docs/README.md | Core commands | vibe list | cmd | живо | vibe.exe list --help -> exit 0 | перенесён → reference/machine-formats#root |
| docs/README.md | Core commands | vibe search | cmd | живо | vibe.exe search --help -> exit 0 | перенесён → model/packages-and-kinds#the-kinds |
| docs/README.md | Core commands | vibe check | cmd | живо | vibe.exe check --help -> exit 0 | перенесён → howto/remove-a-package#root |
| docs/README.md | Core commands | vibe cache | cmd | живо | vibe.exe cache --help -> exit 0 | перенесён → howto/update-packages#recovery |
| docs/README.md | Core commands | vibe registry | cmd | живо | vibe.exe registry --help -> exit 0 | перенесён → howto/use-a-private-registry#root |
| docs/README.md | Core commands | vibe show | cmd | живо | vibe.exe show --help -> exit 0 | перенесён → howto/remove-a-package#what-happens |
| docs/README.md | Core commands | vibe tree | cmd | живо | vibe.exe tree --help -> exit 0 | перенесён → model/boot-lane#root |
| docs/README.md | Core commands | vibe mcp | cmd | живо | vibe.exe mcp --help -> exit 0 | перенесён → authoring/ship-tools-and-mcp-servers#servers |
| docs/README.md | Machine-readable inventory | SITE-MANIFEST.toml | path | живо | найден в дереве (имя файла): SITE-MANIFEST.toml | снят: инвентарь старого сайта уходит вместе с docs-legacy (PROP-057 §legacy) |
| docs/registry-auth.md | Authenticating against private registries | [[registry]] | section | живо | найдено в crates/vibe-core/src/manifest/artifact/tests_validation.rs:403 | перенесён → reference/manifest#sources |
| docs/registry-auth.md | TL;DR | --unattended | flag | живо | найден в собранном корпусе `--help` | перенесён → reference/commands#global |
| docs/registry-auth.md | TL;DR | --token-env | flag | живо | найден в собранном корпусе `--help` | перенесён → reference/commands#registries |
| docs/registry-auth.md | `auth = "none"` (default) | vibe install | cmd | живо | vibe.exe install --help -> exit 0 | перенесён → howto/install-a-package#what-happens |
| docs/registry-auth.md | `auth = "token-env"` | token_env | section | живо | найдено в crates/vibe-core/src/manifest/document/tests.rs:569 | перенесён → reference/manifest#sources |
| docs/registry-auth.md | Bash / zsh | vibe install flow:internal-helper --assume-yes | cmd | живо | vibe.exe install --help -> exit 0 | перенесён → howto/install-a-package#what-happens |
| docs/registry-auth.md | Bash / zsh | --assume-yes | flag | живо | найден в собранном корпусе `--help` | перенесён → howto/remove-a-package#by-hand |
| docs/registry-auth.md | Token discipline | vibe.toml | path | живо | найден в дереве (имя файла): vibe.toml | перенесён → reference/manifest#root |
| docs/registry-auth.md | Token discipline | vibe.lock | path | живо | найден в дереве (имя файла): vibe.lock | перенесён → reference/lock-file#root |
| docs/registry-auth.md | Setting up via the CLI | vibe registry add | cmd | живо | vibe.exe registry add --help -> exit 0 | перенесён → howto/use-a-private-registry#what-happens |
| docs/registry-auth.md | Setting up via the CLI | --auth | flag | живо | найден в собранном корпусе `--help` | перенесён → howto/use-a-private-registry#what-happens |
| docs/registry-auth.md | Setting up via the CLI | vibe registry add internal "https://gitlab.company.com/vibespecs" --auth token-env | cmd | живо | vibe.exe registry add --help -> exit 0 | перенесён → howto/use-a-private-registry#what-happens |
| docs/registry-auth.md | Setting up via the CLI | vibe registry add internal "https://gitlab.company.com/vibespecs" \ | cmd | живо | vibe.exe registry add --help -> exit 0 | перенесён → howto/use-a-private-registry#what-happens |
| docs/registry-auth.md | Setting up via the CLI | vibe registry add internal-ssh "git@gitlab.company.com:vibespecs" --auth ssh | cmd | живо | vibe.exe registry add --help -> exit 0 | перенесён → howto/use-a-private-registry#what-happens |
| docs/registry-auth.md | Setting up via the CLI | vibe registry add corporate "https://corp.example.com/vibespecs" --auth credential-helper | cmd | живо | vibe.exe registry add --help -> exit 0 | перенесён → howto/use-a-private-registry#what-happens |
| docs/registry-auth.md | Strict-auth posture (`--auth-required`) | --auth-required | flag | живо | найден в собранном корпусе `--help` | перенесён → howto/use-a-private-registry#edge-cases |
| docs/registry-auth.md | Strict-auth posture (`--auth-required`) | vibe install --auth-required | cmd | живо | vibe.exe install --help -> exit 0 | перенесён → howto/install-a-package#what-happens |
| docs/registry-auth.md | Strict-auth posture (`--auth-required`) | vibe install --unattended --auth-required flow:internal-helper | cmd | живо | vibe.exe install --help -> exit 0 | перенесён → howto/install-a-package#what-happens |
| docs/registry-auth.md | Token never on disk — verifying | ~/.vibe/registries/ | path | живо | найден на диске (эта машина): C:\Users\olegc\.vibe\registries | перенесён → reference/settings-and-environment#the-folder |
| docs/registry-auth.md | Diagnosing reachability before an install | vibe registry test | cmd | живо | vibe.exe registry test --help -> exit 0 | перенесён → howto/use-a-private-registry#root |
| docs/registry-auth.md | Diagnosing reachability before an install | vibe registry test --json \| jq '.registries[] \| select(.status != "reachable")' | cmd | живо | vibe.exe registry test --help -> exit 0 | перенесён → howto/use-a-private-registry#root |
| docs/registry-auth.md | Diagnosing reachability before an install | --json | flag | живо | найден в собранном корпусе `--help` | перенесён → reference/commands#global |
| docs/registry-auth.md | Machine-readable resolution failures | vibe install --json | cmd | живо | vibe.exe install --help -> exit 0 | перенесён → howto/install-a-package#what-happens |
| docs/registry-auth.md | Machine-readable resolution failures | error_kind | section | устарело | поле не найдено ни в manifest source, ни в примерах vibe.toml | снят: устарело |
| docs/registry-auth.md | "GUI popup keeps appearing in CI" | vibe mcp install | cmd | живо | vibe.exe mcp install --help -> exit 0 | перенесён → authoring/ship-tools-and-mcp-servers#servers |
| docs/registry-auth.md | "MissingToken even though I set the env-var" | vibe registry list --json \| jq '.registries[].token_env' | cmd | живо | vibe.exe registry list --help -> exit 0 | перенесён → howto/use-a-private-registry#root |
| docs/registry-auth.md | "401 against my private registry, token IS set" | read_repository | section | устарело | поле не найдено ни в manifest source, ни в примерах vibe.toml | снят: устарело |
| docs/registry-auth.md | "401 against my private registry, token IS set" | read_registry | section | устарело | поле не найдено ни в manifest source, ни в примерах vibe.toml | снят: устарело |
| docs/registry-auth.md | "401 against my public registry, but it's only a missing repo" | vibe registry list --json \| jq '.registries[] \| {name, url, auth}' | cmd | живо | vibe.exe registry list --help -> exit 0 | перенесён → howto/use-a-private-registry#root |
| docs/registry-auth.md | "I want to override `auth = "none"` for one package without changing the registry" | [[override]] | section | живо | найдено в crates/vibe-core/src/manifest/document/tests.rs:50 | перенесён → reference/manifest#sources |
| docs/registry-auth.md | "I want to override `auth = "none"` for one package without changing the registry" | source_url | section | живо | найдено в crates/vibe-core/src/manifest/document/tests.rs:52 | перенесён → reference/lock-file#package-entries |
| docs/registry-auth.md | See also | vibe registry add --auth --token-env | cmd | живо | vibe.exe registry add --help -> exit 0 | перенесён → howto/use-a-private-registry#what-happens |
| docs/registry-redirect.md | Registry redirect — delegated package via stub repo | vibe install <pkgref> | cmd | живо | vibe.exe install --help -> exit 0 | перенесён → howto/install-a-package#what-happens |
| docs/registry-redirect.md | Registry redirect — delegated package via stub repo | vibe-redirect.toml | path | устарело | ни один файл с именем vibe-redirect.toml не найден в дереве | снят: устарело |
| docs/registry-redirect.md | When to use | [[registry]] | section | живо | найдено в crates/vibe-core/src/manifest/artifact/tests_validation.rs:403 | перенесён → reference/manifest#sources |
| docs/registry-redirect.md | When to use | [requires.packages] | section | живо | найдено в crates/vibe-core/src/manifest/artifact/tests_validation.rs:398 | перенесён → reference/manifest#requirements |
| docs/registry-redirect.md | When to use | vibe.toml | path | живо | найден в дереве (имя файла): vibe.toml | перенесён → reference/manifest#root |
| docs/registry-redirect.md | Marker file | [redirect] | section | живо | найдено в crates/vibe-core/src/manifest/redirect.rs:29 | перенесён → howto/use-a-private-registry#redirects |
| docs/registry-redirect.md | Marker file | target_url | section | живо | найдено в crates/vibe-core/src/manifest/redirect.rs:8 | перенесён → howto/use-a-private-registry#redirects |
| docs/registry-redirect.md | Marker file | ref_policy | section | живо | найдено в crates/vibe-core/src/manifest/redirect.rs:40 | перенесён → howto/use-a-private-registry#redirects |
| docs/registry-redirect.md | Marker file | pinned_ref | section | живо | найдено в crates/vibe-core/src/manifest/redirect.rs:66 | перенесён → howto/use-a-private-registry#redirects |
| docs/registry-redirect.md | Marker file | token_env | section | живо | найдено в crates/vibe-core/src/manifest/document/tests.rs:569 | перенесён → reference/manifest#sources |
| docs/registry-redirect.md | Resolver behaviour | [package] | section | живо | найдено в crates/vibe-core/src/manifest/artifact/tests_validation.rs:372 | перенесён → reference/lock-file#package-entries |
| docs/registry-redirect.md | Resolver behaviour | target_ref | section | живо | найдено в crates/vibe-core/src/manifest/deploy/applicability.rs:231 | перенесён → howto/use-a-private-registry#redirects |
| docs/registry-redirect.md | Tag visibility | vibe registry publish | cmd | живо | vibe.exe registry publish --help -> exit 0 | перенесён → howto/publish-a-package#root |
| docs/registry-redirect.md | Identity | content_hash | section | живо | найдено в crates/vibe-core/src/manifest/document/tests.rs:292 | перенесён → reference/lock-file#package-entries |
| docs/registry-redirect.md | Lockfile | source_url | section | живо | найдено в crates/vibe-core/src/manifest/document/tests.rs:52 | перенесён → reference/lock-file#package-entries |
| docs/registry-redirect.md | Lockfile | via_redirect | section | живо | найдено в crates/vibe-core/src/manifest/lockfile.rs:410 | перенесён → howto/use-a-private-registry#redirects |
| docs/registry-redirect.md | Lockfile | vibe.lock | path | живо | найден в дереве (имя файла): vibe.lock | перенесён → reference/lock-file#root |
| docs/registry-redirect.md | Lockfile | [[package]] | section | живо | найдено в crates/vibe-core/src/manifest/lockfile/tests.rs:26 | перенесён → reference/lock-file#package-entries |
| docs/registry-redirect.md | Lockfile | source_kind | section | живо | найдено в crates/vibe-core/src/manifest/lockfile/tests.rs:36 | перенесён → reference/lock-file#package-entries |
| docs/registry-redirect.md | Lockfile | source_ref | section | живо | найдено в crates/vibe-core/src/manifest/lockfile/tests.rs:33 | перенесён → reference/lock-file#package-entries |
| docs/registry-redirect.md | Lockfile | resolved_commit | section | живо | найдено в crates/vibe-core/src/manifest/lockfile/tests.rs:34 | перенесён → reference/lock-file#package-entries |
| docs/registry-redirect.md | Lockfile | vibe show <pkgref> | cmd | живо | vibe.exe show --help -> exit 0 | перенесён → howto/remove-a-package#what-happens |
| docs/registry-redirect.md | Lockfile | vibe list --json | cmd | живо | vibe.exe list --help -> exit 0 | перенесён → reference/machine-formats#root |
| docs/registry-redirect.md | Lockfile | --json | flag | живо | найден в собранном корпусе `--help` | перенесён → reference/commands#global |
| docs/registry-redirect.md | Trust model | vibe install | cmd | живо | vibe.exe install --help -> exit 0 | перенесён → howto/install-a-package#what-happens |
| docs/registry-redirect.md | Trust model | vibe update | cmd | живо | vibe.exe update --help -> exit 0 | перенесён → howto/update-packages#root |
| docs/registry-redirect.md | Trust model | --trust-redirect | flag | живо | найден в собранном корпусе `--help` | перенесён → reference/commands#registries |
| docs/registry-redirect.md | Trust model | --trust-mirror | flag | устарело | не найден ни в одном собранном `--help` | снят: устарело |
| docs/registry-redirect.md | Auth — two independent layers | inject_token | section | устарело | поле не найдено ни в manifest source, ни в примерах vibe.toml | снят: устарело |
| docs/registry-redirect.md | Auth — two independent layers | set_remote_url | section | устарело | поле не найдено ни в manifest source, ни в примерах vibe.toml | снят: устарело |
| docs/registry-redirect.md | `vibe registry redirect` (recommended) | vibe registry redirect | cmd | живо | vibe.exe registry redirect --help -> exit 0 | перенесён → howto/use-a-private-registry#root |
| docs/registry-redirect.md | `vibe registry redirect` (recommended) | ~/.vibe/ | path | живо | найден на диске (эта машина): C:\Users\olegc\.vibe | перенесён → reference/settings-and-environment#the-folder |
| docs/registry-redirect.md | `vibe registry redirect` (recommended) | vibe registry redirect flow:internal-helper \ | cmd | живо | vibe.exe registry redirect --help -> exit 0 | перенесён → howto/use-a-private-registry#root |
| docs/registry-redirect.md | `vibe registry redirect` (recommended) | --to | flag | живо | найден в собранном корпусе `--help` | перенесён → authoring/specs-agents-can-cite#the-dialect |
| docs/registry-redirect.md | `vibe registry redirect` (recommended) | --description | flag | живо | найден в собранном корпусе `--help` | перенесён → reference/commands#projects |
| docs/registry-redirect.md | `vibe registry redirect` (recommended) | vibe registry redirect flow:legacy-pinned \ | cmd | живо | vibe.exe registry redirect --help -> exit 0 | перенесён → howto/use-a-private-registry#root |
| docs/registry-redirect.md | `vibe registry redirect` (recommended) | --ref-policy | flag | живо | найден в собранном корпусе `--help` | перенесён → reference/commands#registries |
| docs/registry-redirect.md | `vibe registry redirect` (recommended) | --pinned-ref | flag | живо | найден в собранном корпусе `--help` | перенесён → reference/commands#registries |
| docs/registry-redirect.md | `vibe registry redirect` (recommended) | vibe registry redirect flow:internal-secret \ | cmd | живо | vibe.exe registry redirect --help -> exit 0 | перенесён → howto/use-a-private-registry#root |
| docs/registry-redirect.md | `vibe registry redirect` (recommended) | --target-auth | flag | живо | найден в собранном корпусе `--help` | перенесён → reference/commands#registries |
| docs/registry-redirect.md | `vibe registry redirect` (recommended) | --target-token-env | flag | живо | найден в собранном корпусе `--help` | перенесён → reference/commands#registries |
| docs/registry-redirect.md | `vibe registry redirect` (recommended) | --sync | flag | живо | найден в собранном корпусе `--help` | перенесён → reference/commands#registries |
| docs/registry-redirect.md | `vibe registry redirect` (recommended) | vibe registry redirect-sync | cmd | живо | vibe.exe registry redirect-sync --help -> exit 0 | перенесён → howto/use-a-private-registry#root |
| docs/registry-redirect.md | Rewriting an existing stub's marker — `vibe registry redirect-update` | vibe registry redirect-update | cmd | живо | vibe.exe registry redirect-update --help -> exit 0 | перенесён → howto/use-a-private-registry#root |
| docs/registry-redirect.md | Rewriting an existing stub's marker — `vibe registry redirect-update` | vibe registry redirect-update flow:internal-helper \ | cmd | живо | vibe.exe registry redirect-update --help -> exit 0 | перенесён → howto/use-a-private-registry#root |
| docs/registry-redirect.md | Rewriting an existing stub's marker — `vibe registry redirect-update` | --resync | flag | живо | найден в собранном корпусе `--help` | перенесён → reference/commands#registries |
| docs/registry-redirect.md | Surfacing target tags into the stub — `vibe registry redirect-sync` | vibe registry redirect-sync flow:internal-helper | cmd | живо | vibe.exe registry redirect-sync --help -> exit 0 | перенесён → howto/use-a-private-registry#root |
| docs/registry-redirect.md | Surfacing target tags into the stub — `vibe registry redirect-sync` | --tags | flag | устарело | не найден ни в одном собранном `--help` | снят: устарело |
| docs/registry-redirect.md | Manual procedure (fallback) | vibe install flow:internal-helper@^0.1 | cmd | живо | vibe.exe install --help -> exit 0 | перенесён → howto/install-a-package#what-happens |
| docs/registry-redirect.md | Comparison with related mechanisms | [[mirror]] | section | живо | найдено в crates/vibe-core/src/manifest/artifact/tests_validation.rs:406 | перенесён → reference/manifest#sources |
| docs/registry-redirect.md | Comparison with related mechanisms | [[override]] | section | живо | найдено в crates/vibe-core/src/manifest/document/tests.rs:50 | перенесён → reference/manifest#sources |
| docs/registry-redirect.md | Out of scope for v0 | [redirect.deprecated] | section | устарело | не найдено ни в manifest source, ни в примерах vibe.toml | снят: устарело |
| docs/registry-redirect.md | Out of scope for v0 | new_pkgref | section | устарело | поле не найдено ни в manifest source, ни в примерах vibe.toml | снят: устарело |
| docs/troubleshooting.md | Troubleshooting | vibe_registry | section | живо | найдено в crates/vibe-core/src/manifest/project.rs:208 | перенесён → diagnostics/errors#edge-cases |
| docs/troubleshooting.md | Troubleshooting | vibe_publish | section | устарело | поле не найдено ни в manifest source, ни в примерах vibe.toml | перенесён → diagnostics/errors#edge-cases |
| docs/troubleshooting.md | `package `…` is already installed at version `…` — use `vibe update` instead` | vibe install <pkgref> | cmd | живо | vibe.exe install --help -> exit 0 | перенесён → howto/install-a-package#what-happens |
| docs/troubleshooting.md | `package `…` is already installed at version `…` — use `vibe update` instead` | content_hash | section | живо | найдено в crates/vibe-core/src/manifest/document/tests.rs:292 | перенесён → reference/lock-file#package-entries |
| docs/troubleshooting.md | `package `…` is already installed at version `…` — use `vibe update` instead` | vibe install | cmd | живо | vibe.exe install --help -> exit 0 | перенесён → howto/install-a-package#what-happens |
| docs/troubleshooting.md | `package `…` is already installed at version `…` — use `vibe update` instead` | vibe update | cmd | живо | vibe.exe update --help -> exit 0 | перенесён → howto/update-packages#root |
| docs/troubleshooting.md | `package `…` is already installed at version `…` — use `vibe update` instead` | vibe uninstall <pkgref> && vibe install <pkgref>@<version> | cmd | живо | vibe.exe uninstall --help -> exit 0 | перенесён → howto/remove-a-package#what-happens |
| docs/troubleshooting.md | `package `…` is already installed at version `…` — use `vibe update` instead` | vibe uninstall | cmd | живо | vibe.exe uninstall --help -> exit 0 | перенесён → howto/remove-a-package#what-happens |
| docs/troubleshooting.md | `package `…` is already installed at version `…` — use `vibe update` instead` | vibe list | cmd | живо | vibe.exe list --help -> exit 0 | перенесён → reference/machine-formats#root |
| docs/troubleshooting.md | `content drift on `…@…`: lockfile pins `sha256:…` but the source served `sha256:…`…` | [[mirror]] | section | живо | найдено в crates/vibe-core/src/manifest/artifact/tests_validation.rs:406 | перенесён → reference/manifest#sources |
| docs/troubleshooting.md | `content drift on `…@…`: lockfile pins `sha256:…` but the source served `sha256:…`…` | [[override]] | section | живо | найдено в crates/vibe-core/src/manifest/document/tests.rs:50 | перенесён → reference/manifest#sources |
| docs/troubleshooting.md | `content drift on `…@…`: lockfile pins `sha256:…` but the source served `sha256:…`…` | vibe uninstall <pkgref> && vibe install <pkgref> | cmd | живо | vibe.exe uninstall --help -> exit 0 | перенесён → howto/remove-a-package#what-happens |
| docs/troubleshooting.md | `content drift on `…@…`: lockfile pins `sha256:…` but the source served `sha256:…`…` | vibe.toml | path | живо | найден в дереве (имя файла): vibe.toml | перенесён → reference/manifest#root |
| docs/troubleshooting.md | `content drift on `…@…`: lockfile pins `sha256:…` but the source served `sha256:…`…` | --trust-mirror | flag | устарело | не найден ни в одном собранном `--help` | снят: устарело |
| docs/troubleshooting.md | `content drift on `…@…`: lockfile pins `sha256:…` but the source served `sha256:…`…` | vibe.lock | path | живо | найден в дереве (имя файла): vibe.lock | перенесён → reference/lock-file#root |
| docs/troubleshooting.md | `malformed <vibevm> block in `…`` | vibedeps/ | path | неизвестно | генерируемый каталог (материализуется `vibe install`), механическая проверка по дереву неинформативна для непроинсталлированного чекаута | перенесён → model/two-trees#the-rule |
| docs/troubleshooting.md | `malformed <vibevm> block in `…`` | vibe check | cmd | живо | vibe.exe check --help -> exit 0 | перенесён → howto/remove-a-package#root |
| docs/troubleshooting.md | `package `…` is not installed` | vibe uninstall <pkgref> | cmd | живо | vibe.exe uninstall --help -> exit 0 | перенесён → howto/remove-a-package#what-happens |
| docs/troubleshooting.md | `the materialised vibedeps/ tree is incomplete — … slot(s) missing` | vibe reinstall | cmd | живо | vibe.exe reinstall --help -> exit 0 | перенесён → howto/update-packages#recovery |
| docs/troubleshooting.md | `the materialised vibedeps/ tree is incomplete — … slot(s) missing` | --force | flag | живо | найден в собранном корпусе `--help` | перенесён → model/two-trees#what-regenerates |
| docs/troubleshooting.md | `the materialised vibedeps/ tree is incomplete — … slot(s) missing` | vibe reinstall --force | cmd | живо | vibe.exe reinstall --help -> exit 0 | перенесён → howto/update-packages#recovery |
| docs/troubleshooting.md | `package declares a [boot_snippet].source `…` that does not exist in the package` | [boot_snippet] | section | живо | найдено в crates/vibe-core/src/manifest/document/tests.rs:81 | перенесён → reference/manifest#deliveries |
| docs/troubleshooting.md | `package declares a [boot_snippet].source `…` that does not exist in the package` | [writes] | section | живо | найдено в crates/vibe-core/src/manifest/document.rs:25 | снят: устарело — таблицы нет в схеме манифеста (crates/vibe-core/src/manifest, проверено 2026-09-12) |
| docs/troubleshooting.md | `user declined the install plan` | --assume-yes | flag | живо | найден в собранном корпусе `--help` | перенесён → howto/remove-a-package#by-hand |
| docs/troubleshooting.md | `user declined the install plan` | --json | flag | живо | найден в собранном корпусе `--help` | перенесён → reference/commands#global |
| docs/troubleshooting.md | `package `kind:name` is not in the registry` | vibe install <kind>:<name> | cmd | живо | vibe.exe install --help -> exit 0 | перенесён → howto/install-a-package#what-happens |
| docs/troubleshooting.md | `package `kind:name` is not in the registry` | [[registry]] | section | живо | найдено в crates/vibe-core/src/manifest/artifact/tests_validation.rs:403 | перенесён → reference/manifest#sources |
| docs/troubleshooting.md | `no version of `kind:name` matches `…`` | vibe install flow:wal | cmd | живо | vibe.exe install --help -> exit 0 | перенесён → howto/install-a-package#what-happens |
| docs/troubleshooting.md | `no version of `kind:name` matches `…`` | --tags | flag | устарело | не найден ни в одном собранном `--help` | снят: устарело |
| docs/troubleshooting.md | `no registry configured. Pass `--registry <path>` or add a `[[registry]]` entry to `vibe.toml`.` | --registry | flag | живо | найден в собранном корпусе `--help` | перенесён → howto/publish-a-package#by-hand |
| docs/troubleshooting.md | `no registry configured. Pass `--registry <path>` or add a `[[registry]]` entry to `vibe.toml`.` | vibe init --no-registry | cmd | живо | vibe.exe init --help -> exit 0 | перенесён → howto/use-a-private-registry#machine-wide |
| docs/troubleshooting.md | `no registry configured. Pass `--registry <path>` or add a `[[registry]]` entry to `vibe.toml`.` | --no-registry | flag | живо | найден в собранном корпусе `--help` | перенесён → reference/commands#projects |
| docs/troubleshooting.md | `no registry configured. Pass `--registry <path>` or add a `[[registry]]` entry to `vibe.toml`.` | vibe init | cmd | живо | vibe.exe init --help -> exit 0 | перенесён → howto/use-a-private-registry#machine-wide |
| docs/troubleshooting.md | `registry meta file at `…` is malformed: …` | meta.toml | path | устарело | ни один файл с именем meta.toml не найден в дереве | снят: устарело |
| docs/troubleshooting.md | `registry meta file at `…` is malformed: …` | ~/.vibe/registries/ | path | живо | найден на диске (эта машина): C:\Users\olegc\.vibe\registries | перенесён → reference/settings-and-environment#the-folder |
| docs/troubleshooting.md | `the `git` executable is not available on PATH; install git …` | --version | flag | живо | найден в собранном корпусе `--help` | перенесён → start/index#step-2 |
| docs/troubleshooting.md | `file `…` not found in `…` at ref `…`` | --remote | flag | устарело | не найден ни в одном собранном `--help` | снят: устарело |
| docs/troubleshooting.md | `file `…` not found in `…` at ref `…`` | --format | flag | живо | найден в собранном корпусе `--help` | перенесён → reference/commands#projects |
| docs/troubleshooting.md | Publish errors | vibe registry publish | cmd | живо | vibe.exe registry publish --help -> exit 0 | перенесён → howto/publish-a-package#root |
| docs/troubleshooting.md | `publish refused: token lacks `repo:create` permission in organization `…` on `…`.` | ~/.vibe/git.publish.token | path | неизвестно | секрет-подобный путь (R-12) — существование на диске не проверялось намеренно | перенесён → reference/settings-and-environment#the-folder |
| docs/troubleshooting.md | `publish refused: tag `…` already exists on `…`. Pick a new version` | [package] | section | живо | найдено в crates/vibe-core/src/manifest/artifact/tests_validation.rs:372 | перенесён → reference/lock-file#package-entries |
| docs/troubleshooting.md | `package `…` declares `[conflicts]` against `…`, which is also being installed in this graph` | [conflicts] | section | живо | найдено в crates/vibe-core/src/manifest/document/validation.rs:59 | перенесён → reference/manifest#requirements |
| docs/troubleshooting.md | `capability `…` required by `…` is not provided by any package in the resolved graph.` | [requires] | section | живо | найдено в crates/vibe-core/src/manifest/document.rs:22 | перенесён → reference/manifest#requirements |
| docs/troubleshooting.md | `capability `…` required by `…` is not provided by any package in the resolved graph.` | [provides] | section | живо | найдено в crates/vibe-core/src/manifest/document/tests.rs:84 | перенесён → reference/manifest#requirements |
| docs/troubleshooting.md | `capability `…` required by `…` is not provided by any package in the resolved graph.` | [[requires_any]] | section | живо | найдено в crates/vibe-core/src/manifest/document/validation.rs:53 | перенесён → reference/manifest#requirements |
| docs/troubleshooting.md | `all alternatives in `[[requires_any]]` declared by `…` failed to resolve` | one_of | section | живо | найдено в crates/vibe-core/src/manifest/artifact/tests.rs:254 | перенесён → reference/manifest#requirements |
| docs/troubleshooting.md | `no `vibe.toml` in `…`; run `vibe init` first` | vibe init --path <project> | cmd | живо | vibe.exe init --help -> exit 0 | перенесён → howto/use-a-private-registry#machine-wide |
| docs/troubleshooting.md | `no `vibe.toml` in `…`; run `vibe init` first` | --path | flag | живо | найден в собранном корпусе `--help` | перенесён → howto/update-packages#by-hand |
| docs/troubleshooting.md | `no TTY available for confirmation; re-run with `--assume-yes` to apply this plan non-interactively` | --yes | flag | живо | найден в собранном корпусе `--help` | перенесён → agent/give-your-agent-the-skill#what-happens |
| docs/troubleshooting.md | When the message isn't here | vibe --version | cmd | живо | глобальный флаг --version найден в `vibe --help` | перенесён → reference/commands#global |
| docs/troubleshooting.md | When the message isn't here | vibevm/vibevm/issues | path | устарело | путь не существует: vibevm/vibevm/issues | снят: устарело |
| docs/version-syntax.md | Version syntax in vibevm | vibe install <pkgref> | cmd | живо | vibe.exe install --help -> exit 0 | перенесён → howto/install-a-package#what-happens |
| docs/version-syntax.md | Version syntax in vibevm | [requires.packages] | section | живо | найдено в crates/vibe-core/src/manifest/artifact/tests_validation.rs:398 | перенесён → reference/manifest#requirements |
| docs/version-syntax.md | Version syntax in vibevm | [provides] | section | живо | найдено в crates/vibe-core/src/manifest/document/tests.rs:84 | перенесён → reference/manifest#requirements |
| docs/version-syntax.md | Version syntax in vibevm | [[override]] | section | живо | найдено в crates/vibe-core/src/manifest/document/tests.rs:50 | перенесён → reference/manifest#sources |
| docs/version-syntax.md | Version syntax in vibevm | vibe.toml | path | живо | найден в дереве (имя файла): vibe.toml | перенесён → reference/manifest#root |
| docs/version-syntax.md | TL;DR | vibe install flow:wal | cmd | живо | vibe.exe install --help -> exit 0 | перенесён → howto/install-a-package#what-happens |
| docs/version-syntax.md | TL;DR | vibe.lock | path | живо | найден в дереве (имя файла): vibe.lock | перенесён → reference/lock-file#root |
| docs/version-syntax.md | TL;DR | --exact | flag | живо | найден в собранном корпусе `--help` | перенесён → howto/install-a-package#constraints |
| docs/version-syntax.md | Two-file model | Cargo.toml | path | живо | найден в дереве (имя файла): Cargo.toml | перенесён → authoring/ship-tools-and-mcp-servers#binaries |
| docs/version-syntax.md | Two-file model | Cargo.lock | path | живо | найден в дереве (имя файла): Cargo.lock | снят: аналогия, не утверждение о продукте |
| docs/version-syntax.md | What lands in `vibe.toml` after `vibe install` | vibe install | cmd | живо | vibe.exe install --help -> exit 0 | перенесён → howto/install-a-package#what-happens |
| docs/version-syntax.md | What lands in `vibe.toml` after `vibe install` | [requires] | section | живо | найдено в crates/vibe-core/src/manifest/document.rs:22 | перенесён → reference/manifest#requirements |
| docs/version-syntax.md | Rule 1 — No version on CLI → caret of resolved version | vibe install flow:wal --assume-yes | cmd | живо | vibe.exe install --help -> exit 0 | перенесён → howto/install-a-package#what-happens |
| docs/version-syntax.md | Rule 1 — No version on CLI → caret of resolved version | --assume-yes | flag | живо | найден в собранном корпусе `--help` | перенесён → howto/remove-a-package#by-hand |
| docs/version-syntax.md | Rule 2 — Explicit constraint on CLI → preserved verbatim | vibe install flow:wal@^0.1 --assume-yes | cmd | живо | vibe.exe install --help -> exit 0 | перенесён → howto/install-a-package#what-happens |
| docs/version-syntax.md | Rule 3 — `--exact` overrides everything | vibe install flow:wal --exact --assume-yes | cmd | живо | vibe.exe install --help -> exit 0 | перенесён → howto/install-a-package#what-happens |
| docs/version-syntax.md | Rule 3 — `--exact` overrides everything | vibe install flow:wal@^0.1 --exact --assume-yes | cmd | живо | vibe.exe install --help -> exit 0 | перенесён → howto/install-a-package#what-happens |
| docs/version-syntax.md | Rule 3 — `--exact` overrides everything | vibe update | cmd | живо | vibe.exe update --help -> exit 0 | перенесён → howto/update-packages#root |
| docs/version-syntax.md | Rule 3 — `--exact` overrides everything | --save-exact | flag | живо | найден в собранном корпусе `--help` | перенесён → reference/commands#projects |
| docs/version-syntax.md | What `vibe update` does | vibe install flow:wal@^0.2 | cmd | живо | vibe.exe install --help -> exit 0 | перенесён → howto/install-a-package#what-happens |
| docs/version-syntax.md | What's in `vibe.lock` | [[package]] | section | живо | найдено в crates/vibe-core/src/manifest/lockfile/tests.rs:26 | перенесён → reference/lock-file#package-entries |
| docs/version-syntax.md | What's in `vibe.lock` | source_url | section | живо | найдено в crates/vibe-core/src/manifest/document/tests.rs:52 | перенесён → reference/lock-file#package-entries |
| docs/version-syntax.md | What's in `vibe.lock` | source_ref | section | живо | найдено в crates/vibe-core/src/manifest/lockfile/tests.rs:33 | перенесён → reference/lock-file#package-entries |
| docs/version-syntax.md | What's in `vibe.lock` | resolved_commit | section | живо | найдено в crates/vibe-core/src/manifest/lockfile/tests.rs:34 | перенесён → reference/lock-file#package-entries |
| docs/version-syntax.md | What's in `vibe.lock` | content_hash | section | живо | найдено в crates/vibe-core/src/manifest/document/tests.rs:292 | перенесён → reference/lock-file#package-entries |
| docs/version-syntax.md | What's in `vibe.lock` | [meta] | section | живо | найдено в crates/vibe-core/src/manifest/lockfile/tests.rs:19 | перенесён → reference/lock-file#meta |
| docs/version-syntax.md | Comparison table — vibevm vs other ecosystems | --locked | flag | устарело | не найден ни в одном собранном `--help` | снят: устарело |
| docs/version-syntax.md | Comparison table — vibevm vs other ecosystems | --lock | flag | устарело | не найден ни в одном собранном `--help` | снят: устарело |
| docs/version-syntax.md | Comparison table — vibevm vs other ecosystems | yarn.lock | path | устарело | ни один файл с именем yarn.lock не найден в дереве | снят: устарело |
| docs/version-syntax.md | Comparison table — vibevm vs other ecosystems | poetry.lock | path | устарело | ни один файл с именем poetry.lock не найден в дереве | снят: устарело |
| docs/version-syntax.md | Comparison table — vibevm vs other ecosystems | Gemfile.lock | path | устарело | ни один файл с именем Gemfile.lock не найден в дереве | снят: устарело |

## Сводка

- Файлов просканировано: 48
- Файлов, давших хотя бы одно утверждение: 48
- Утверждений всего (после dedup по файлу): 1184
- живо: 1073 (90.6%)
- устарело: 84 (7.1%)
- неизвестно: 27 (2.3%)

Доля `неизвестно` ниже 20%: 2.3%.

## Карта файлов docs/** (первый заголовок и содержание)

| файл | первый заголовок | о чём (авто) |
| --- | --- | --- |
| docs/ALPHA-NOTES.md | vibevm 1.0.0 alpha notes | The owner's semver mandate is literal: **«1.0.0 будет ломаться»**. |
| docs/architecture.md | vibevm — architecture | This is the contributor-facing map of the 1.0.0 tree. The source code is the authority for current implementation detail; [`VIBEVM-SPEC.md`](../VIBEVM-SPEC.md) and the PROP documents under [`spec/`... |
| docs/authoring-feat.md | Authoring a `feat` package | A **feat** is a *functional feature* — a self-contained capability of an application, expressed entirely as specification. Where flows describe how the team works (process), feats describe what the... |
| docs/authoring-flow.md | Authoring a `flow` package | A **flow** is a discipline / process module — a set of conventions, protocols, and reminders an AI agent reads at session start so it follows the team's working agreements. Flows are the "how we wo... |
| docs/authoring-stack.md | Authoring a `stack` package | A **stack** is a *language / framework target* — the runtime surface a feat compiles against. Stacks describe how feats become real software: which language, which framework, which build system, wh... |
| docs/commands/cache.md | `vibe cache` | Operate on the machine-global package store at `~/.vibe/cache/`. The store works outside a project, grows only through explicit fetches, and is never evicted automatically. |
| docs/commands/check.md | `vibe check` | Run the spec-consistency linter against a project tree. |
| docs/commands/init.md | `vibe init` | Scaffold a project, a package, or a package group. |
| docs/commands/install.md | `vibe install` | Resolve and install one or more packages into a project. With package arguments, the command also adds or updates the corresponding entries in `vibe.toml`; without arguments, it installs `[requires... |
| docs/commands/list.md | `vibe list` | List packages recorded in the project's lockfile. |
| docs/commands/mcp-install.md | `vibe mcp install` — wire vibevm into a coding agent | Detects supported coding agents on this machine + the current project, writes the per-agent MCP server configuration, and (optionally) installs the `vibevm` SKILL.md instructing the agent how to us... |
| docs/commands/mcp-serve.md | `vibe mcp serve` — Model Context Protocol server | Runs the JSON-RPC 2.0 server for the [Model Context Protocol](https://modelcontextprotocol.io) over stdio. Coding agents (Claude Code, Claude Desktop, Cursor, OpenCode, Codex) pick it up via their ... |
| docs/commands/mcp-status.md | `vibe mcp status` — preview agent integration state | Read-only counterpart of [`vibe mcp install`](mcp-install.md) + [`vibe mcp upgrade`](mcp-upgrade.md). Walks every supported agent (Claude Code, Claude Desktop, Cursor, OpenCode, Codex) across both ... |
| docs/commands/mcp-uninstall.md | `vibe mcp uninstall` — remove vibevm from coding agents | Mirror of [`vibe mcp install`](mcp-install.md): same scope axis (project / user / both), same agent filter, same `--config-only` / `--skill-only` toggle. Drops the `vibevm` key from each agent's MC... |
| docs/commands/mcp-upgrade.md | `vibe mcp upgrade` — refresh stale vibevm integrations | Scans known per-agent MCP-config files and SKILL.md paths, compares the on-disk shape to what the current `vibe` binary would write, and rewrites only the diverged ones. **Does not create new insta... |
| docs/commands/mcp.md | `vibe mcp` | Serve project state over the Model Context Protocol and manage vibevm integration with supported coding agents. |
| docs/commands/registry-add.md | `vibe registry add` — register a new `[[registry]]` in `vibe.toml` | Mutates `vibe.toml` to add a new `[[registry]]` block. The new entry is appended by default; pass `--position primary` to make it the first registry (the default for publish + the first stop on res... |
| docs/commands/registry-list.md | `vibe registry list` — show configured registries, mirrors, overrides | Read-only inspector for the project's resolution configuration. Prints every `[[registry]]`, `[[mirror]]`, and `[[override]]` block from `vibe.toml`, the host adapter each registry would dispatch t... |
| docs/commands/registry-publish.md | `vibe registry publish` — publish a package directory | Maintainer-side command. Takes a directory containing a `vibe.toml` with a `[package]` table and publishes it as a tagged release in the configured registry's organization. Creates the per-package ... |
| docs/commands/registry-redirect-sync.md | `vibe registry redirect-sync` — mirror target tags into a stub | Maintainer-side command. Reads an existing registry stub's `vibe-redirect.toml`, enumerates target-side tags, and pushes the missing ones into the stub. Per PROP-002 §2.4.2 `pass-through-tag` polic... |
| docs/commands/registry-redirect-update.md | `vibe registry redirect-update` — rewrite an existing registry stub's marker | Maintainer-side command. Mutates the `vibe-redirect.toml` of an existing stub repo in-place: each flag is optional, so the command does a true partial update — fields not passed retain their curren... |
| docs/commands/registry-redirect.md | `vibe registry redirect` — create a registry stub | Maintainer-side command. Creates a registry stub repo carrying `vibe-redirect.toml` instead of package content; consumers reach the package transparently through the stub via `vibe install <pkgref>... |
| docs/commands/registry-remove.md | `vibe registry remove` — drop a `[[registry]]` or `[[mirror]]` from `vibe.toml` | Mutates `vibe.toml` to remove either a registry block (by name) or a mirror block (by exact `(of, url)` match). The two targets are spelled as subsubcommands so the CLI surface is unambiguous and s... |
| docs/commands/registry-set-mirror.md | `vibe registry set-mirror` — add a `[[mirror]]` block to `vibe.toml` | Mutates `vibe.toml` to add a new `[[mirror]]` block. A mirror is a transparent alternative URL for a registry: when the primary URL fails (or by priority order), the resolver falls through to the m... |
| docs/commands/registry-sync.md | `vibe registry sync` — refresh per-package registry clones | Walks the project's `vibe.lock` and refreshes the on-disk clone of every package referenced by it — the per-package model's equivalent of `git fetch` against a single monorepo. Useful before a `vib... |
| docs/commands/registry-test.md | `vibe registry test` — probe each registry's reachability and auth | Read-only diagnostic. For every `[[registry]]` configured in the project's `vibe.toml`, the command performs a single `git ls-remote` (with token injection if `auth = "token-env"`) and reports the ... |
| docs/commands/registry-vendor.md | `vibe registry vendor` — generate a local mirror directory | Walks the project's `vibe.lock` and produces a directory containing one bare git repo per `[[registry]]`-served lockfile entry. The directory is a drop-in source for `[[mirror]] url = "file:///<abs... |
| docs/commands/registry.md | `vibe registry` | Inspect, configure, test, synchronize, publish, redirect, and export package registries. |
| docs/commands/reinstall.md | `vibe reinstall` — recompute the materialised dependencies and boot artifacts | Recomputes a workspace's materialised state without re-resolving. `vibe reinstall` is the regeneration command of the loading model: it rebuilds the `vibedeps/` tree and the per-node boot artifacts... |
| docs/commands/search.md | `vibe search` | Search configured registry indexes by free text, or perform an exact Package URL lookup. |
| docs/commands/show.md | `vibe show` | Inspect computed project state. |
| docs/commands/tree.md | `vibe tree` | Analyze the resolved spec and dependency tree. The command shows boot-load type, transitive and conditional edges, the static/dynamic boot lanes, and in-place `@spec` markers without mutating the p... |
| docs/commands/uninstall.md | `vibe uninstall` | Remove one installed package from a project. |
| docs/commands/update.md | `vibe update` | Re-fetch and apply changes for selected installed packages, or for the whole lockfile. |
| docs/commands/version.md | `vibe version` — print version information | Prints the binary's version. Both `vibe version` and `vibe --version` work — the former is a subcommand, the latter is a global flag. Output is identical. |
| docs/commands/workspace-publish.md | `vibe workspace publish` — publish a workspace's members as separate repositories | Maintainer-side command. Discovers the workspace enclosing the current directory and publishes every self-publishing member as its own repository in the registry organization — the per-package [`vi... |
| docs/faq/README.md | FAQ | Answers to real questions developers ask about vibevm, written up as standalone pages so they can be searched, linked, and folded into the wider documentation over time. Each entry states the quest... |
| docs/faq/version-conflicts.md | Resolving version conflicts | **Q: A dependency deep in my tree pins a version that clashes with what I (or another dependency) want, and the install fails with an unsatisfiable-graph error. How do I fix it? Can I force a speci... |
| docs/git-source-dependencies.md | Git-source dependencies — whole-repo-as-package | vibevm normally resolves dependencies through a `[[registry]]` org — `org.vibevm.world/wal` becomes `<org>/org.vibevm.world_wal` per the registry's `naming` convention. M1.15 adds a second shape: d... |
| docs/glossary.md | Glossary | Definitive vocabulary for the vibevm project. Spec-text, PROPs, code, docs, and commit messages all draw from this list — when a term appears with a specific meaning here, that's the meaning everyw... |
| docs/guides/agent-mcp-quickstart-opencode.md | Quickstart: opencode + vibevm hello-world | End-to-end walkthrough that takes a fresh machine with `opencode` already installed, sets up vibevm globally (one-time bootstrap), and demonstrates that opencode can **create a vibevm-managed hello... |
| docs/loading-model.md | The loading model — how a vibevm project boots | This page explains what `vibe install` produces and how an AI agent loads a vibevm project at session start. The authoritative contracts are [PROP-009](../spec/modules/vibe-workspace/PROP-009-loadi... |
| docs/lockfile-format.md | `vibe.lock` — schema reference | Authoritative reference for the `vibe.lock` file at the root of every vibevm project. The lockfile is the source of truth for what is installed; `vibe list` reads it, `vibe uninstall` reads it to f... |
| docs/README.md | vibevm documentation | Operator documentation for vibevm 1.0.0. The live binary remains the authority for CLI syntax: use `vibe <command> --help` alongside these pages. |
| docs/registry-auth.md | Authenticating against private registries | vibevm supports four authentication regimes per `[[registry]]` block. The default (`auth = "none"`) covers public registries — every git host vibevm has shipped against by default, including the ca... |
| docs/registry-redirect.md | Registry redirect — delegated package via stub repo | A registry org's package slot may carry a `vibe-redirect.toml` marker file pointing at an external git repo where the package's actual content lives, instead of carrying the package content directl... |
| docs/troubleshooting.md | Troubleshooting | First-aid for the errors `vibe` surfaces. Each entry: what you see, what it means, what to do. |
| docs/version-syntax.md | Version syntax in vibevm | Everywhere a vibevm command takes a package reference (`vibe install <pkgref>`, `[requires.packages]` in `vibe.toml`, `[provides].capabilities` in a package's `vibe.toml`, `[[override]] pkgref = ..... |
