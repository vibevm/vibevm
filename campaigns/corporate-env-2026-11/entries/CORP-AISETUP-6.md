# CORP-AISETUP-6

Planned at commit `9d8428d07`. Line numbers are hints from that commit; find each place by the quoted text or the item name.

**Outcome.** Attended native installation without --configure-ai asks once whether to configure an AI agent (default skip)

**Write.** `crates/vibe-bootstrap/src/optional_setup.rs`, `crates/vibe-bootstrap/src/kit.rs`, `campaigns/corporate-env-2026-11/FORMAT-CHANGES.md`

**Check.** `bash tools/cargo-test-ran.sh -p vibe-bootstrap --lib optional_setup::tests::first_install_question_accepts_only_explicit_yes`

**Test.** crates/vibe-bootstrap/src/optional_setup.rs tests::first_install_question_accepts_only_explicit_yes. `accepted` is true for 'y', 'Y', ' yes \n' and 'YES', and false for '', '\n', 'n', 'no', 'maybe' and 'yess'. `wants_setup` with a counting closure: not attended and no flag gives false with 0 asks; flag set (attended or not) gives true with 0 asks; attended with an answer of false gives false with 1 ask; attended with an answer of true gives true with 1 ask.

**Read first.**
- crates/vibe-bootstrap/src/optional_setup.rs (after_install, tests)
- crates/vibe-bootstrap/src/kit.rs (the `if args.configure_ai && let Some(component)` block directly after `report.stage("Installation complete")?;`, and the `io::{self, IsTerminal, Write}` import)
- campaigns/corporate-env-2026-11/FORMAT-CHANGES.md (title line and the section below it)

**Steps.**
1. In optional_setup.rs, add above `pub(crate) fn after_install`:
```rust
/// Whether an answer to the first-install question continues into optional
/// setup: only an explicit `y` or `yes` does; anything else skips.
pub(crate) fn accepted(answer: &str) -> bool {
    matches!(answer.trim().to_ascii_lowercase().as_str(), "y" | "yes")
}

/// Whether optional setup runs: always with `--configure-ai` (never asks),
/// otherwise only on an attended run whose single question is answered yes.
pub(crate) fn wants_setup(configure_ai: bool, attended: bool, ask: impl FnOnce() -> bool) -> bool {
    configure_ai || (attended && ask())
}

/// Ask once on the attended terminal (default skip); a read failure skips.
pub(crate) fn offer() -> bool {
    use std::io::Write;
    let mut stderr = std::io::stderr();
    let _ = write!(
        stderr,
        "Would you like to configure an AI agent now? You can run `vibe ai setup` later. [y/N] "
    );
    let _ = stderr.flush();
    let mut answer = String::new();
    std::io::stdin().read_line(&mut answer).is_ok() && accepted(&answer)
}
```
2. In the `#[cfg(all(test, unix))] mod tests` of optional_setup.rs, add:
```rust
    #[test]
    fn first_install_question_accepts_only_explicit_yes() {
        for answer in ["y", "Y", " yes \n", "YES"] {
            assert!(accepted(answer), "{answer:?}");
        }
        for answer in ["", "\n", "n", "no", "maybe", "yess"] {
            assert!(!accepted(answer), "{answer:?}");
        }
        let cases = [
            (false, false, true, false, 0),
            (true, false, false, true, 0),
            (true, true, false, true, 0),
            (false, true, false, false, 1),
            (false, true, true, true, 1),
        ];
        for (flag, attended, answer, expected, asks) in cases {
            let mut count = 0;
            let result = wants_setup(flag, attended, || {
                count += 1;
                answer
            });
            assert_eq!(result, expected, "{flag} {attended} {answer}");
            assert_eq!(count, asks, "{flag} {attended} {answer}");
        }
    }
```
3. In kit.rs, directly after `report.stage("Installation complete")?;`, add
```rust
    let attended = !args.unattended
        && io::stdin().is_terminal()
        && io::stdout().is_terminal()
        && io::stderr().is_terminal();
```
Then change the condition `if args.configure_ai
        && let Some(component) = ...` to `if crate::optional_setup::wants_setup(args.configure_ai, attended, crate::optional_setup::offer)
        && let Some(component) = ...`. The body stays the same. Confirm with `wc -l` that kit.rs stays at 600 lines or fewer (574 at planning time).
4. In campaigns/corporate-env-2026-11/FORMAT-CHANGES.md (newest section first), insert this section directly after the title line `# Corporate-env campaign: format and behaviour changes` and its blank line, above `## CORP-FMT-067 — site profile `[ai]` hint and default endpoint`. The block ends with one blank line, so a blank line separates it from the 067 heading:
```
## CORP-FMT-068 — first-install AI setup question

An attended native installation (stdin, stdout and stderr are terminals, no `--unattended`) without `--configure-ai` asks once on stderr after `Installation complete`: "Would you like to configure an AI agent now? You can run `vibe ai setup` later. [y/N]". Only `y` or `yes` (any case) continues into the same optional setup route and `vibe-installer-optional-setup/1` summary as `--configure-ai`; any other answer, an empty line or a read failure skips with the previous output. Nonterminal and unattended runs never ask, and `--configure-ai` never asks. This amends CORP-FMT-056's no-prompt rule for attended runs without the flag.

```
5. Run `cargo fmt --all`, `cargo clippy -p vibe-bootstrap --all-targets -- -D warnings` and `cargo xtask conform check`, then the check. Also run the existing bootstrap tests: `cargo test -p vibe-bootstrap`.

**Decisions already made.**
- The question is asked on every attended run that reaches `Installation complete` without --configure-ai, including reruns. Dry-run and --upgrade return earlier and never ask.
- The default is skip: only an explicit y/yes (trimmed, any case) continues. An empty line, any other text or a read error skips, and output and exit stay as before.
- The kit.rs condition goes through `optional_setup::wants_setup`, so the unit test covers the wiring rule: the flag never asks, non-attended never asks, and attended asks exactly once.
- A yes uses the existing optional_setup::after_install path unchanged, including its installed-identity verification and its `completed_unconfirmed` or `failed` summary.
- The prompt is written to stderr and the answer read with a plain line read. No new prompt dependency is added to vibe-bootstrap.
- Non-attended installs keep the current behaviour (no question, no summary unless --configure-ai).
- depends_on CORP-AISETUP-4 orders the two FORMAT-CHANGES sections: 068 sits above 067.
- Native Windows and macOS console runs of this question are owner-side checks; this entry is verified on Linux only.

**Not in this entry.** Not in this entry: telling a first install apart from a rerun, changing after_install or its summary statuses, adding agent install or connection logic to the installer, or native-host runs.

**Depends on.** CORP-AISETUP-4

**Spec edit (apply as a step of this entry).**

none

Entries that write the same file run one after another, never in parallel.
