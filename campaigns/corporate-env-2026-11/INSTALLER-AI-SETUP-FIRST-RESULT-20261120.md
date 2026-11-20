# Opt-in post-bootstrap metadata setup — implementation candidate

Contract `CORP-INSTALLER-AI-SETUP-FIRST-01`, architect `7431b16144e46f3d102a1c66f03384b87bfdf3bf`, integrated `d6443f9c0`, plus explicit narrow active-instance query amendment. This first slice configures existing connection metadata only; not six-agent onboarding, new keys, supplier installation, Node remediation, provider launch, recovery-only operation or a new corporate distribution.

## Existing owners and opt-in behavior

`vibe-installer --configure-ai` runs only after successful mandatory verified `self apply`. No flag keeps previous mandatory output/exit and no prompt. Dry-run and failed mandatory preflight/application never invoke setup. Unattended/nonterminal mode without a selection emits separate `installation:completed` and optional `input_required`; it never creates AI selection or inherits implicit approval.

Attended operation uses the existing atomic `vibe ai setup` planner with inherited stdin/stdout/stderr, without `--unattended`, raw output capture or bootstrap answer logging. Existing preview/default-No confirmation owns write consent. Current setup exit0 cannot distinguish cancel/no-op/applied, so bootstrap reports **completed_unconfirmed**, never applied. Errors become optional **failed** while mandatory installation remains successful. Opt-in summary protocol `vibe-installer-optional-setup/1` is documented in CORP-FMT-056; no persistence schema is migrated.

## Installed identity, not PATH

Existing read-only `self which vibe --active --json` queries the canonical version-store owner, ignoring running-source preference only under the new flag and refusing absent active identity. Without flag old running-source behavior remains. Bootstrap requires the reported absolute installed path and nonempty selector, refuses its own staging path, opens the regular non-link executable through the existing verifier and hashes complete bytes/size against the already authenticated bundle component. No store-layout parser, shim launch or PATH fallback is introduced.

The verified file handle remains held during process creation. **Limit:** standard `Command` still executes the pathname, not a platform-specific fd-exec capability; no race-free immunity to hostile concurrent path replacement is claimed. This bounded owner reuses existing path execution with immediate full verification, not a new secure process framework. Digest mismatch refuses before the child in the explicit negative test.

## Actual final isolated evidence

Final synthetic-kit runtime r5 uses actual new Vibe/installer/proxy binaries, network/PID/filesystem isolation, synthetic HOME/settings and no host credential/config/socket mounts. Kit is a synthetic verified distribution fixture: actual CLI/proxy, minimal source archive and proxy bytes honestly declared as an unexecuted index placeholder. It is not a deliverable release or matching full source distribution. No supplier agent or provider is executed. Synthetic secret creation occurs only to seed test reference metadata; setup code never reads/resolves that value. Sentinel is absent from command logs; secret index remains byte-identical across setup operations.

`corp-setup-adapter-public-r5/summary.json` and complete steps/terminals qualify:

- fresh opt-in unattended bootstrap succeeds with input_required; repeat without flag has no optional summary; repeat opt-in remains input_required and creates no AI indexes;
- actual source-shaped executable: ordinary self-which returns source path/null selector; active flag returns installed path/selector; empty active refuses;
- actual inherited-terminal menu Cancel and explicit default-No decline preserve AI metadata; explicit selection/yes updates metadata and a fresh process reads Pi inherit-default;
- missing default after explicit fixture-owned clear yields optional failed, bootstrap exit0, unchanged AI/secret indexes and mandatory component bytes;
- invalid mandatory profile produces failure with no optional summary or AI/component mutation;
- full terminal attributes restore in all four PTY journeys; foreign settings bytes and mandatory component hashes remain intact;
- no-path avoids PATH activation. Mandatory self-apply legitimately persists synthetic VIBE_SETTINGS in managed `.profile`; subsequent bytes are preserved, not mislabeled zero HOME writes.

Bootstrap unit5/5 (including digest mismatch/no child), integration4/4, identity unit1/1 (1330 filtered) passed in isolation. Bootstrap main artifact has zero tests and is not credited as coverage. Strict bootstrap all-target lint passed. Full CLI-bin strict lint remains **RED** solely on preexisting unchanged `cli/command.rs::Command` large_enum_variant; separate category-except check is not a strict PASS. Separate format/diff gates passed. No whole-host/platform gate is inferred.

Initial runtime r1 RED used an invalid empty-HOME oracle despite legitimate settings persistence; it passed dry/fresh mandatory operations before stopping. R2 passed after correction but predates final lint rebuild/stronger controls. R3 and r4 passed progressive final controls; r5 is final exact driver. Strict lint r1 found owned collapsible-if, corrected without suppression. All predecessor receipts remain retained.

Private evidence root `/data/git/vibevm-keyring-storage-20261120T0100Z/`: `corp-setup-adapter-public-r5/`, earlier runtime roots/logs, bootstrap test/lint receipts, identity runtime and CLI lint logs. Final exact-source manifest binds code, script/report, ledger and artifacts. Independent acceptance pending; NST remains open. No push, public release, credentialed trial, normal-user mutation or configure-only recovery claim.
