# Attended prerequisite Cancel versus manual defer

Finite test/report follow-up authorized by architect to CORP-HERMES-INTERPRETER-QUALIFY-01. No production change, real npm/Node installation, supplier code, credential access or external network. This tests the existing missing-command consent owner, not native npm version compatibility or full agent onboarding.

`qualify-prerequisite-cancel.py` uses retained accepted CLI SHA-256 `2acac8641b59a4a811e583df9c029171c2be680d92397fb39ac66400b16be69e`, explicit proxy and synthetic local structured-install package. The package has one copy step guarded by an intentionally absent synthetic command. Each journey has its own filesystem/PID/network sandbox, HOME/settings and controlling PTY. Platform, architecture and installer are explicit so the actual missing-command menu is reached without unattended mode or parser bypass. Input is sent only after observing the missing-command prompt and Cancel entry with terminal echo disabled.

Actual r2 results, two independent worlds:

- **Cancel:** Enter chooses the visible default Cancel installation. Exit2 and `need_unmet`; no `prerequisite_deferred`. This is cancellation at the offered menu, not successful installation.
- **Manual defer:** Down then Enter selects Install the runtime yourself and rerun later. Exit2 and `prerequisite_deferred`, distinct from Cancel.
- Both restore full termios and leave app-work, application index, public-command directory and app payload directory absent. Package resolution cache/coordination are not claimed absent. No archive/network step is defined in this synthetic fixture; the earlier four-test regression supplies its independent zero-download-hit control.

Initial r1 was RED: the driver waited for the missing-command prompt while the earlier Platform picker was active; it timed out and stopped only its own bounded test child. Transcript and outer failure are retained. No cancellation evidence is credited to r1. R2 explicitly binds earlier choices and actually reaches both required outcomes.

Evidence root `/data/git/vibevm-keyring-storage-20261120T0100Z/`: `corp-prerequisite-pty-r1.log`, `corp-prerequisite-pty-r1/cancel/terminal.txt`, final `corp-prerequisite-pty-r2.log` and each r2 world's `terminal.txt`, `result.json`, `outer.json`. Exact script/report/runtime hashes are in the separate candidate manifest. Final proxy SHA-256 `a722f37dde364a7b54c7a9a2e8946223ad84e82de41d5fb8060c659d2480d24f`.

Independent exact-source acceptance pending. This does not convert the synthetic command into a native npm ABI test, complete the six-agent distribution matrix, resolve Hermes system-provenance/ABI/drift, close the parked guarded source-rebind RED or finish NST.
