# ADV-NATIVE-PACMAN-2

Planned at commit `9d8428d07`. Line numbers below are hints from that commit; find each place by the quoted text or the item name.

**Outcome.** With the pacman adapter installed, `vibe native install|remove|upgrade --adapter pacman --allow-privilege --json` runs `pacman -S --needed --noconfirm -- ITEMS`, `pacman -Rs --noconfirm -- ITEMS` and `pacman -Syu --noconfirm` through the escalation step; without `--allow-privilege`, install exits non-zero with code `privilege-required` and pacman is never asked to change anything.

**Write.** `vibevm/vibepacks/org.vibevm.system/pacman/v1.0.0/vibe.toml`, `vibevm/vibepacks/org.vibevm.system/pacman/v1.0.0/adapter/install.sh`, `vibevm/vibepacks/org.vibevm.system/pacman/v1.0.0/adapter/remove.sh`, `vibevm/vibepacks/org.vibevm.system/pacman/v1.0.0/adapter/upgrade.sh`, `crates/vibe-cli/tests/cli_system_pacman.rs`

**Check.** `cargo test -p vibe-cli --test cli_system_pacman 2>&1 | grep -F "test result: ok. 7 passed"`

**Test.** crates/vibe-cli/tests/cli_system_pacman.rs gains four #[test] fns: `install_hands_the_items_to_pacman_sync`, `remove_hands_the_items_to_pacman_remove`, `upgrade_runs_a_full_system_upgrade`, `install_without_privilege_consent_refuses_and_changes_nothing`. The file then has 7 tests: 3 from ADV-NATIVE-PACMAN-1 and these 4. Before the work the run reports 3 passed, so the grep for 7 fails.

**Read first.**
- crates/vibe-cli/tests/cli_system_pacman.rs: const FAKE_PACMAN and fns install_adapter, fake_bin, path_with, native, adapters_of (from ADV-NATIVE-PACMAN-1)
- vibevm/vibepacks/org.vibevm.system/pacman/v1.0.0/vibe.toml
- crates/vibe-cli/tests/cli_app_extras.rs: fn items_reach_the_recipe_as_exactly_those_argv_elements_and_nothing_is_evaluated (items reach a recipe as argv)
- crates/vibe-app/src/installer/source_build.rs: fns script_command and recipe_command (the recipe child is `/bin/sh <script>` plus the items as argv)

**Steps.**
1. Guard, before writing anything: run `cargo run -q -p vibe-cli --bin vibe -- native install --help` and `grep -rn "privileged" crates/vibe-core/src/manifest`. Both must show the interface in the first decision (`--allow-privilege`, field `privileged`). If either is missing or spelled differently, stop and report a packet defect naming the difference; do not adapt to another spelling and do not edit crates/vibe-core, crates/vibe-app or crates/vibe-cli/src.
2. In vibe.toml, directly after the line
```
description = "The system packages pacman manages on this Arch Linux machine"
```
insert the line
```
privileged = ["install", "remove", "upgrade"]
```
Directly after the `[application.platform_adapter.list]` table (after its line `script = "adapter/list.sh"`), insert exactly:
```

[application.platform_adapter.install]
id = "pacman-install"
title = "Install system packages with pacman"
kind = "recipe"
script = "adapter/install.sh"

[application.platform_adapter.remove]
id = "pacman-remove"
title = "Remove system packages with pacman"
kind = "recipe"
script = "adapter/remove.sh"

[application.platform_adapter.upgrade]
id = "pacman-upgrade"
title = "Upgrade the whole system with pacman"
kind = "recipe"
script = "adapter/upgrade.sh"
```
3. Create adapter/install.sh with exactly:
```
#!/bin/sh
# The pacman adapter's `install` verb: the items arrive as "$@", opaque
# argv elements, never a command string. --needed keeps a package that is
# already current untouched.
set -u
[ "$#" -gt 0 ] || exit 0
exec pacman -S --needed --noconfirm -- "$@"
```
Create adapter/remove.sh with exactly:
```
#!/bin/sh
# The pacman adapter's `remove` verb: remove the items and the dependencies
# that nothing else needs any more.
set -u
[ "$#" -gt 0 ] || exit 0
exec pacman -Rs --noconfirm -- "$@"
```
Create adapter/upgrade.sh with exactly:
```
#!/bin/sh
# The pacman adapter's `upgrade` verb: a full system upgrade. It takes no
# items, because Arch does not support partial upgrades.
set -u
exec pacman -Syu --noconfirm
```
4. In crates/vibe-cli/tests/cli_system_pacman.rs replace the whole FAKE_PACMAN const so that the raw string literal (opening `r#"` directly followed by the shebang, closing `"#;` right after the final newline) holds exactly:
```
#!/bin/sh
set -u
printf '%s\n' "$*" >> "$FAKE_PACMAN_LOG"
case "${1-}" in
    -V) echo "Pacman v7.0.0 (fake)"; exit 0 ;;
    -Qqe) cat "$FAKE_PACMAN_DB"; exit 0 ;;
    -S|-Rs)
        verb="$1"; shift
        while [ "$#" -gt 0 ] && [ "$1" != "--" ]; do shift; done
        [ "$#" -gt 0 ] && shift
        for item in "$@"; do
            if [ "$verb" = "-S" ]; then
                grep -qx -- "$item" "$FAKE_PACMAN_DB" || printf '%s\n' "$item" >> "$FAKE_PACMAN_DB"
            else
                grep -vx -- "$item" "$FAKE_PACMAN_DB" > "$FAKE_PACMAN_DB.next" || true
                mv "$FAKE_PACMAN_DB.next" "$FAKE_PACMAN_DB"
            fi
        done
        exit 0 ;;
    -Syu) exit 0 ;;
esac
echo "fake pacman: unsupported arguments: $*" >&2
exit 2
```
Add `const FAKE_SUDO: &str` the same way (raw string, no leading newline) holding exactly:
```
#!/bin/sh
set -u
while [ "$#" -gt 0 ]; do
    case "$1" in
        --) shift; break ;;
        -*) shift ;;
        *) break ;;
    esac
done
exec "$@"
```
Make fake_bin also write FAKE_SUDO to `fakebin/sudo` with mode 0o755.
5. Add the helper `pacman_calls(r: &AppRegistry) -> Vec<String>` with a one-line `///` doc: the lines of `<r.root.path()>/pacman.log`, or an empty Vec when the file is missing (`fs::read_to_string(..).map(|t| t.lines().map(str::to_owned).collect()).unwrap_or_default()`).

Write these tests. Each makes `AppRegistry::new()`, calls install_adapter, calls fake_bin, and passes `path_with(&bin)` as PATH.
- `install_hands_the_items_to_pacman_sync`: db `base\n`. `let document = json(&native(&r, &path, &["install", "--adapter", "pacman", "jq", "ripgrep", "--allow-privilege", "--json"]));` Assert `adapters_of(&document, "pacman")["outcome"] == "done"`. Assert pacman_calls contains the line `-S --needed --noconfirm -- jq ripgrep`. Then a list run (`["list", "--adapter", "pacman", "--json"]`) reports `observed == serde_json::json!(["base", "jq", "ripgrep"])`.
- `remove_hands_the_items_to_pacman_remove`: db `base\njq\n`. Run `["remove", "--adapter", "pacman", "jq", "--allow-privilege", "--json"]` through json; assert outcome `done`, the line `-Rs --noconfirm -- jq`, and a later list reporting `observed == serde_json::json!(["base"])`.
- `upgrade_runs_a_full_system_upgrade`: db `base\n`. Run `["upgrade", "--adapter", "pacman", "--allow-privilege", "--json"]` through json; assert outcome `done` and the line `-Syu --noconfirm`.
6. Write `install_without_privilege_consent_refuses_and_changes_nothing`: db `base\n`, then `let out = native(&r, &path, &["install", "--adapter", "pacman", "jq", "--json"]);` (no `--allow-privilege`).
- Assert `!out.status.success()` and that stdout followed by stderr contains `privilege-required`.
- Assert no pacman_calls line starts with `-S`, and `fs::read_to_string(<r.root.path()>/pacman.db)` is still `base\n`.
Do not assert that sudo was called: CI containers often run as root, where vibe skips escalation.
7. Run `cargo fmt --all`, then `cargo clippy -p vibe-cli --tests -- -D warnings` (no new imports are needed beyond those of ADV-NATIVE-PACMAN-1; remove any left unused). Run `wc -l crates/vibe-cli/tests/cli_system_pacman.rs`: expect about 330, under 600. Then run the check.

**Decisions already made.**
- Interface this entry consumes. These spellings are binding: the ADV-NATIVE-ADAPTERS entries must deliver exactly them, and the guard step stops the entry if they do not.
- The adapter table field `privileged` is a list of verb names that need root.
- The consent flag is `--allow-privilege`. It is required even when vibe already runs as root.
- Without consent, a privileged verb exits non-zero with code `privilege-required` before its script starts.
- With consent and an effective uid other than 0, vibe looks up `sudo` on the PATH the child gets (never an absolute /usr/bin/sudo) and runs the argv `[<that sudo>, "-n", "--", "/bin/sh", <script>, ITEMS...]`, the recipe child of fn recipe_command behind `sudo -n --`, with the child environment set on the sudo process. With uid 0 it runs `/bin/sh <script> ITEMS...` directly.
- install and remove receive the items as "$@"; upgrade receives none.
- pacman command lines, fixed:
- install: `pacman -S --needed --noconfirm -- ITEMS`
- remove: `pacman -Rs --noconfirm -- ITEMS` (also removes dependencies that nothing else needs any more)
- upgrade: `pacman -Syu --noconfirm`, always a full upgrade with no items
The scripts never call sudo themselves; escalation is vibe's step. They read no VIBE_* variable, so real sudo resetting the environment does not affect them.
- The fake sudo drops any leading options and an optional `--`, then execs the rest as the same user. So the tests pass as root (no sudo used) and as an ordinary user (fake sudo used), and they hold whatever options vibe puts before `--`.
- Each script is held in a raw string literal so the bytes written are exactly the fenced block.
- Runs strictly after ADV-NATIVE-PACMAN-1: it edits the same vibe.toml and the same test file.
- No generated file, wire schema or #[specmark::spec] marker is touched.

**Not in this entry.** `pacman -Sy` refresh as its own verb, AUR helpers, `--root`/image-build modes, sudo alternatives (doas, pkexec), any change to crates/vibe-core, crates/vibe-app or crates/vibe-cli/src, Library screens.

**Depends on.** ADV-NATIVE-PACMAN-1, ADV-NATIVE-ADAPTERS-1, ADV-NATIVE-ADAPTERS-2, ADV-NATIVE-ADAPTERS-3, ADV-NATIVE-ADAPTERS-4, ADV-NATIVE-ADAPTERS-5

**Spec edit (apply as a step of this entry).**

none

Entries that write the same file run one after another, never in parallel.
