# ADV-NATIVE-PACMAN-1

Planned at commit `9d8428d07`. Line numbers below are hints from that commit; find each place by the quoted text or the item name.

**Outcome.** With the pacman adapter package installed, `vibe native list --adapter pacman --json` answers a `vibe-native/1` document whose `adapters[]` row with `id == "pacman"` has `outcome == "done"` and `observed` equal to the names `pacman -Qqe` prints; where no pacman runs, the command exits non-zero with code `adapter-not-detected`.

**Write.** `vibevm/vibepacks/org.vibevm.system/pacman/v1.0.0/vibe.toml`, `vibevm/vibepacks/org.vibevm.system/pacman/v1.0.0/adapter/detect.sh`, `vibevm/vibepacks/org.vibevm.system/pacman/v1.0.0/adapter/list.sh`, `crates/vibe-cli/tests/cli_system_pacman.rs`

**Check.** `cargo test -p vibe-cli --test cli_system_pacman 2>&1 | grep -F "test result: ok. 3 passed"`

**Test.** crates/vibe-cli/tests/cli_system_pacman.rs (new). Its first line is `#![cfg(all(test, target_os = "linux"))]`, then a `//!` module doc, then `mod common;`. It has three #[test] fns: `list_reports_what_pacman_lists_as_explicitly_installed`, `list_refuses_where_no_pacman_runs`, `live_pacman_list_matches_pacman_qqe`. The live test returns at once, and so passes, unless the environment variable VIBE_LIVE_PACMAN is exactly `1`. Before the work the check fails: cargo reports no test target `cli_system_pacman`, so grep finds no line. The check runs on Linux only, like cli_app_extras.rs.

**Read first.**
- crates/vibe-cli/tests/cli_app_extras.rs: fns install, extras, adapters_of and the test the_list_verbs_answer_is_the_file_and_a_missing_one_is_an_empty_list (fixture, install and JSON-row pattern; copy adapters_of, NOT ecosystem_of)
- crates/vibe-cli/tests/common/app_registry.rs: struct AppRegistry (fields `root: tempfile::TempDir`, `user: UserScratch`; fns registry, home, app) and free fns ok, json
- crates/vibe-cli/tests/common/mod.rs: fn copy_tree(src: &Path, dst: &Path), fn workspace_root() -> PathBuf, and the `#![allow(dead_code, unused_imports)]` at its top
- crates/vibe-test-support/src/lib.rs: UserScratch::vibe (the Command every run starts from)
- crates/vibe-core/src/manifest/package/application/ecosystem.rs: struct EcosystemDecl (the shape the platform adapter table copies: id, name, description, verbs as StepDecl)
- crates/vibe-wire/src/generated/application_ecosystem/mod.rs: struct Adapter (id, name, outcome, items, observed, step, log, reason, retry): the row shape of `adapters[]`

**Steps.**
1. Guard, before writing anything: run `grep -rn "platform_adapter" crates/vibe-core/src` and `cargo run -q -p vibe-cli --bin vibe -- native list --help`. Both must show the interface quoted in the first decision (table `platform_adapter`, verb `native list`, flags `--adapter`, `--registry`, `--offline`, `--json`). If either is missing or spelled differently, stop and report a packet defect naming the difference. Do not adapt the package or the test to another spelling, and do not edit crates/vibe-core, crates/vibe-app or crates/vibe-cli/src.
2. Create vibevm/vibepacks/org.vibevm.system/pacman/v1.0.0/vibe.toml with exactly this content:
```
[package]
name = "pacman"
group = "org.vibevm.system"
kind = "app"
version = "1.0.0"
authors = ["Oleg Chirukhin"]
license = "UPL-1.0"
description = "Arch Linux pacman as a vibe platform adapter: the system packages of this machine"
keywords = ["pacman", "arch", "system", "adapter"]

[application]
id = "pacman"

# pacman, declared as a platform adapter: vibe drives these verbs without
# knowing one pacman word. `list` answers into the file vibe names.
[[application.platform_adapter]]
id = "pacman"
name = "pacman packages"
description = "The system packages pacman manages on this Arch Linux machine"

[application.platform_adapter.detect]
id = "pacman-detect"
title = "Check that pacman runs on this machine"
kind = "recipe"
script = "adapter/detect.sh"

[application.platform_adapter.list]
id = "pacman-list"
title = "List the explicitly installed system packages"
kind = "recipe"
script = "adapter/list.sh"

[[installer_options]]
id = "linux-x64"
name = "pacman adapter"
platform = "linux"
platform_name = "Linux"
arch = "x86_64"

[[installer_options.steps]]
id = "adapter"
title = "Copy the pacman adapter scripts"
kind = "copy"
from = "adapter"
to = "adapter"

[[installer_options]]
id = "linux-arm64"
name = "pacman adapter"
platform = "linux"
platform_name = "Linux"
arch = "aarch64"

[[installer_options.steps]]
id = "adapter"
title = "Copy the pacman adapter scripts"
kind = "copy"
from = "adapter"
to = "adapter"
```
3. Create adapter/detect.sh in that package directory with exactly:
```
#!/bin/sh
# The pacman adapter's `detect` verb: succeed only where pacman runs.
set -u
exec pacman -V >/dev/null
```
Create adapter/list.sh with exactly:
```
#!/bin/sh
# The pacman adapter's `list` verb: the explicitly installed packages, one
# name per line, written into the file $VIBE_ECOSYSTEM_LIST names. Packages
# pulled in only as dependencies are left out on purpose: they are pacman's
# business, not something the user added.
set -u
: "${VIBE_ECOSYSTEM_LIST:?the list verb is run by vibe, which names the answer file}"
exec pacman -Qqe >"$VIBE_ECOSYSTEM_LIST"
```
4. Create crates/vibe-cli/tests/cli_system_pacman.rs. Imports: `std::ffi::OsStr`, `std::fs`, `std::os::unix::fs::PermissionsExt`, `std::path::PathBuf`, `std::process::Output`, `common::app_registry::{AppRegistry, json, ok}`, `serde_json::Value`. Add `const FAKE_PACMAN: &str` as a raw string literal that opens with `r#"` directly followed by the shebang (no leading newline) and closes with `"#;` right after the final newline, so its bytes are exactly:
```
#!/bin/sh
set -u
printf '%s\n' "$*" >> "$FAKE_PACMAN_LOG"
case "${1-}" in
    -V) echo "Pacman v7.0.0 (fake)"; exit 0 ;;
    -Qqe) cat "$FAKE_PACMAN_DB"; exit 0 ;;
esac
echo "fake pacman: unsupported arguments: $*" >&2
exit 2
```
(The `\n` inside printf is the two characters backslash and n, which a raw string keeps as they are.)
5. Add these helper fns, each with a one-line `///` doc comment:
- `registry_with_pacman(r: &AppRegistry)`: `common::copy_tree(&common::workspace_root().join("vibevm/vibepacks/org.vibevm.system/pacman/v1.0.0"), &r.registry().join("org.vibevm.system/pacman/v1.0.0"))`.
- `install_adapter(r: &AppRegistry)`: calls registry_with_pacman(r), then `ok(&r.app(&["install", "-g"], "org.vibevm.system/pacman@1.0.0", &["--offline", "--unattended", "--json"]));`.
- `fake_bin(r: &AppRegistry, db: &str) -> PathBuf`: creates `<r.root.path()>/fakebin`, writes FAKE_PACMAN to `fakebin/pacman` and sets mode 0o755 with `fs::Permissions::from_mode`, writes `db` to `<r.root.path()>/pacman.db`, returns the fakebin path.
- `path_with(dir: &PathBuf) -> std::ffi::OsString`: `std::env::join_paths(std::iter::once(dir.clone()).chain(std::env::split_paths(&std::env::var_os("PATH").unwrap_or_default()))).unwrap()`.
- `native(r: &AppRegistry, path: &OsStr, args: &[&str]) -> Output`: `let mut command = r.user.vibe();` then `current_dir(r.root.path())`, env `HOME`=`r.home()`, `PATH`=`path`, `FAKE_PACMAN_LOG`=`<r.root.path()>/pacman.log`, `FAKE_PACMAN_DB`=`<r.root.path()>/pacman.db`; args `"native"`, then `args`, then `"--registry"`, `r.registry()`, `"--offline"`; return `command.output().unwrap()`.
- `adapters_of`: copy fn adapters_of from cli_app_extras.rs verbatim (it reads `document["adapters"]`).
6. Write `list_reports_what_pacman_lists_as_explicitly_installed`:
- `let r = AppRegistry::new();` install_adapter(&r); `let bin = fake_bin(&r, "base\njq\n");`
- `let document = json(&native(&r, &path_with(&bin), &["list", "--adapter", "pacman", "--json"]));`
- `assert_eq!(document["protocol"], "vibe-native/1");`
- `let row = adapters_of(&document, "pacman");` assert `row["outcome"] == "done"` and `row["observed"] == serde_json::json!(["base", "jq"])`.
- Read `<r.root.path()>/pacman.log` and assert some line equals `-Qqe`.

Write `list_refuses_where_no_pacman_runs`:
- install_adapter(&r) with the inherited PATH (no override). Create the empty directory `<r.root.path()>/empty`; that directory alone is PATH (never /usr/bin, so an Arch developer machine refuses too).
- `let out = native(&r, empty.as_os_str(), &["list", "--adapter", "pacman", "--json"]);`
- `assert!(!out.status.success());` and assert `format!("{}{}", String::from_utf8_lossy(&out.stdout), String::from_utf8_lossy(&out.stderr))` contains `adapter-not-detected`, printing the text on failure.
7. Write `live_pacman_list_matches_pacman_qqe`:
- First line: `if std::env::var("VIBE_LIVE_PACMAN").as_deref() != Ok("1") { eprintln!("skipped: set VIBE_LIVE_PACMAN=1 on an Arch machine to run it"); return; }`
- Otherwise install_adapter, run `native(&r, &std::env::var_os("PATH").unwrap_or_default(), &["list", "--adapter", "pacman", "--json"])` through json, and assert the pacman row's `observed` equals the trimmed nonempty stdout lines of `std::process::Command::new("pacman").arg("-Qqe").output().unwrap()` as a JSON array.
The test only reads and never needs root.
8. Run `cargo fmt --all`, then `cargo clippy -p vibe-cli --tests -- -D warnings`: every import above is used by these three tests (`Value` by adapters_of's signature); remove any that is not. `mod common;` already silences dead code inside common. Run `wc -l crates/vibe-cli/tests/cli_system_pacman.rs`: expect about 170 lines, under the 600 limit. Then run the check.

**Decisions already made.**
- Interface this entry consumes. These spellings are binding: the ADV-NATIVE-ADAPTERS entries must deliver exactly them, and the guard step stops the entry if they do not.
- Manifest: `[[application.platform_adapter]]` rows with `id`, `name` and an optional `description`. Verb tables `detect` and `list` are required, `install`, `remove` and `upgrade` are optional; each is a StepDecl (`kind = "recipe"`, package-relative `script`), as in EcosystemDecl.
- The adapter package is an ordinary `kind = "app"` package, installed with `vibe install -g`.
- CLI: `vibe native <list|install|remove|upgrade> [--adapter <id>] [ITEM...] [--allow-privilege] [--registry <dir>] [--offline] [--json]`. `--registry` and `--offline` mean what they mean for `vibe app extras`.
- Verb scripts resolve from the package tree through the resolver, exactly as `application.ecosystem` verbs do today (StepContext.package_root is the resolved package root in crates/vibe-app/src/ecosystem/run.rs). That is why native() passes `--registry` and `--offline`. The installer's copy step only gives the installation a payload; the scripts are not run from it.
- The child PATH is vibe's own PATH (with any prefixes vibe adds), so a `pacman` placed first on the test's PATH is the one the scripts find. The child inherits the rest of vibe's environment, so FAKE_PACMAN_LOG and FAKE_PACMAN_DB reach the fake.
- `list` answers into the file named by `$VIBE_ECOSYSTEM_LIST`: one trimmed nonempty line per item, as for ecosystem adapters today (crates/vibe-app/src/steps/kinds.rs fn environment).
- Before any verb, vibe runs the selected adapter's `detect`. With `--adapter <id>` named and detect failing, the command exits non-zero with code `adapter-not-detected` and runs no other verb script.
- `--json` answers a document with top-level `protocol: "vibe-native/1"`, `ok`, `verb` and `adapters[]`. Each row has the shape of wire `Adapter` in application_ecosystem (id, name, outcome `done|failed|skipped`, items, observed, step, log, reason, retry). A list row carries `outcome` and `observed`. Tests read rows with adapters_of (`document["adapters"]`), never ecosystem_of (`document["ecosystems"]`, which belongs to `vibe-app-extras/1`).
- Package coordinate: org.vibevm.system/pacman 1.0.0, at vibevm/vibepacks/org.vibevm.system/pacman/v1.0.0/. Its files are vibe.toml plus scripts under adapter/. There is no README and no LICENSE file.
- Item unit: the package name only, from `pacman -Qqe`, which lists explicitly installed packages without versions. A listed name is exactly an item that `pacman -S` takes back.
- Detection runs `pacman -V`. It does not test /etc/pacman.conf, so the fake needs no files outside the scratch tree.
- The fake pacman is a raw-string const in the test file, written into a temporary `fakebin/` at test time. It records each call as one line (`$*`) in $FAKE_PACMAN_LOG and serves `-Qqe` from the $FAKE_PACMAN_DB file.
- The test file is Linux-only, like cli_app_extras.rs, and the package declares Linux options only (x86_64 and aarch64, the values of std::env::consts::ARCH there).
- No generated file or tool binary is touched: no wire schema, no #[specmark::spec] marker (this package has no PROP anchor), so no specmap.json or jtd-codegen run is needed.

**Not in this entry.** install, remove and upgrade verbs, a `privileged` list, or a fake sudo (that is ADV-NATIVE-PACMAN-2); any change to crates/vibe-core, crates/vibe-app or crates/vibe-cli/src (the adapter machinery belongs to ADV-NATIVE-ADAPTERS); apt, brew or winget packages; a pacman version column.

**Depends on.** ADV-NATIVE-ADAPTERS-1, ADV-NATIVE-ADAPTERS-2, ADV-NATIVE-ADAPTERS-3, ADV-NATIVE-ADAPTERS-4, ADV-NATIVE-ADAPTERS-5

**Spec edit (apply as a step of this entry).**

none

Entries that write the same file run one after another, never in parallel.
