# Keyring CLI — bounded Unix candidate

Baseline: accepted library `3acd58e17`, finite CLI contract integrated as `5cf303d69`. Candidate only, pending independent review. No AI integration, distribution or Windows DACL acceptance.

## Implemented surface

`vibe secrets list`, `details ID`, `create --kind K --label L`, `edit ID --label L --expect-store N --expect-current R`, `rotate ID --expect-current R`, and `remove ID --expect-store N --expect-current R --yes` reuse the generic Store. Metadata reports carry observed store/value revisions and value presence only. No value accessor, export, file/FD input, environment-valued input or raw stdout exists. Public remove remains fail-closed with unknown reference inventory: no compatible protected roster witness currently exists, and neither this binary's compiled consumer set nor consent attests an empty roster. Public positive removal is outstanding, not PASS; admitted synthetic roster success belongs to the accepted library tests.

The effective settings root comes from the existing canonical settings selector and pure absolute-root admission, without user-project initialization or reports. Unsafe components refuse; persistence remains exclusively the safe-filesystem owner. Empty-root list/details do not initialize storage. No cwd fallback or native credential discovery exists.

Create/rotate accept explicit non-terminal `--value-stdin`, bounded to 1 MiB plus one detection byte, preserving final newline. Missing/empty/oversized input refuses before mutation. Otherwise attended TTY input is required; JSON, unattended and non-TTY modes never silently prompt. Hidden input uses the existing shared prompt theme and crossterm terminal owner with one session-scoped raw-mode guard, bounded Unicode accumulation and ordinary Ctrl-C/Escape/EOF cancellation. No process-global signal handler is installed. Installer prompts are unchanged.

Bare `vibe secrets` uses existing shared menu/text/confirm owners for create/select/details/edit/rotate/removal preview. Tokens are captured on item selection, not automatically refreshed on submit. No new TUI framework or complete boxed-form/resize qualification is claimed. Raw input is not echoed or rendered in a collapsed answer.

Malformed actual secrets invocations suppress clap's rejected-token interpolation. Root command classification uses the existing clap grammar, respecting global option operands; unrelated commands retain ordinary diagnostics. Runtime identifiers and bounded input failures have fixed messages without rejected raw values.

## Evidence and limitations

Private evidence root: `/data/git/vibevm-keyring-storage-20261120T0100Z`. Actual public CLI fixtures run inside an unshared network/process/filesystem namespace with cleared environment and synthetic HOME/settings. Final `cli-tests-r4.log` records three PASS cases covering lifecycle, negative/redaction and root-command parser controls. `cli-clippy-r4.log` records scoped strict lint PASS; format/diff checks passed. Exact final source and tested binary digests are bound by private `cli-candidate-r4.json`.

Controlling-PTY `cli-pty-r1.log` preserved a real RED: dialoguer Password Ctrl-C left echo disabled; that fixture aborted before its final index comparison, so r1 does not establish index preservation. The shared secret helper was replaced, not the installer prompt. Review then found the prompt appeared before noecho and empty Enter continued waiting. Final `cli-pty-r4.log` records five successful prompt-triggered journeys with no ECHO-off precondition: full-string paste plus Unicode backspace and Enter, Ctrl-C after partial entry, Escape, Ctrl-D and rejected empty Enter. Each restores the complete previous termios, emits no synthetic secret and preserves index bytes on cancellation/empty rejection. The guard is acquired before prompt output, covering early paste and prompt-write failure. No real key or user store is read. Terminal SIGKILL/external signal resilience is not claimed by these keyboard cancellation tests.

Remaining: independent final source review; public compatible reference roster and successful removal; complete polished form/resize/plain keyboard journeys; Windows protection; admitted local consumers; AI bindings/onboarding and distribution. This slice does not complete CORP-AI-CONNECTIONS or NST.
