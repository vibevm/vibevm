# Configure-only recovery after actual optional failure

2026-11-20 test-only candidate, product source `8d97d74ea020357cc77bf4ca7c9b244c7280c25a`. This closes a specific previously unrun transition from `SETUP-RECOVERY-QUALIFICATION-20261120.md`, not complete onboarding, native auth, provider compatibility or parent acceptance.

## Fresh authorized subject

A new private `/data` fixture was created from scratch. No older key/value file, ordinary HOME, native auth, user settings or corporate profile was mounted. Explicit read-only binaries and a synthetic self-contained distribution supplied mandatory installation through the real native installer. Artifact SHA-256: CLI `704dadddaa0177f6b54e69279467c4200ff5d838be7ebb49d33114776f6e55f1`; PATH proxy `e91f3f7841a12c6ec76a8b3ce09cb897bae1677c56135b7ee47457e811c1fb72`; installer `1e20ea3ff9861de08e172575a65174933434e3752c4426ac5520d73cda4bea11`. No foreign/native-agent executable or provider call was part of this proof.

The script `qualify-setup-recovery-fresh.py` runs inside an enforced cleared-environment filesystem/PID/network sandbox, with only a fresh private evidence world writable. It creates one random synthetic value in memory, passes it solely through the public CLI's explicit nonterminal stdin input, and creates one ordinary metadata connection referencing its stable ID. The value and its digest are never placed in argv, environment, reports or external requests. Captured child output is checked before any report; only boolean leakage disposition is retained. The ordinary sanctioned private-at-rest fixture keyring is retained, not copied into Git.

## Executed transition

Mandatory installation succeeds. With no selected default, an attended native-installer `--configure-ai` invocation selects Pi/Qwen inheritance and reaches a genuine optional failure. The installer exits0 with `installation:completed` and optional status `failed`; both keyring/AI indexes remain unchanged. This is **not input_required**, a fake status injection or a failed mandatory install.

From the same settings, the actual installed CLI performs an explicit Pi/Qwen connection-mode preview without mutation. After an explicit metadata-default selection, an attended setup default-No decline preserves both indexes. A fresh explicit preview followed by its exact `--apply-preview` writes the selected two bindings. Separate new installed-CLI processes observe both effective connection modes and the same connection ID. Unselected Claude's semantic mode/connection/resolution/capability remain unchanged; the observation's global store revision is intentionally not compared as an unselected binding value.

The secret's logical ID/current revision and keyring-index bytes remain identical. Active installation identity, installed CLI bytes and a fixture foreign file remain unchanged. Raw PTY transcripts and synthetic value/digest are not published. Parent terminal attributes are restored. No product source repair was required.

Terminal r2 exit0;16 labelled public-command outcomes plus the attended cancellation control passed. Receipt SHA-256 `0681d5a7b0a134c6d914633dccb5ee51c69be53abe98960fd065f8fa604d8258`; runtime log SHA-256 `85b50b0fb625ab5c6314f4b80422da6379a09e15cbd62c9e0c27193e251ebbc9`. Evidence: `/data/git/vibevm-keyring-storage-20261120T0100Z/setup-recovery-fresh-r2/` and `setup-recovery-artifact-binding-r2.json`.

First r1 is retained RED: it reached confirmed mutation but incorrectly compared the entire unselected binding observation, including the global store revision. The corrected test compares the five semantic fields; it does not weaken selected-write/index-preservation assertions. r1 is not credited as a complete pass.

## Scope

Successful configure-only **metadata** recovery after actual optional failure is now measured. Full-bootstrap rerun after that failure, new-agent installation, protected native credential delivery, provider/native execution, full CRUD/UI/platform protection and corporate distribution remain their separate criteria. Accepted package/installer/platform panels were not replayed. Independent review and aggregate prerequisites remain separate.
