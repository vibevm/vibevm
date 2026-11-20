# Optional setup recovery — existing rerun and constrained direct refusal

Architect requested a read-only direct preview after accepted adapter `7e064c26ea5ff8fc49e56201c9c703a0f0d3a231`, without mounting secret-value files, creating secrets, altering original receipts or rerunning suppliers. Report-only; no product change.

## Previously executed full-bootstrap route

Accepted `corp-setup-adapter-public-r5/steps.json` and its four terminal transcripts already establish fresh opt-in input_required, no-flag repeat, opt-in repeat and later actual PTY metadata application against the same synthetic settings. Component hashes, foreign file, secret index and terminal state were preserved. Optional failed after explicit fixture-owned default clearing preserved successful mandatory installation. Those prior results are reused with their exact scope; they are not new recovery runs.

The earlier standalone fresh binding read proves the applied metadata, not a later direct setup recovery write. Full-bootstrap repeat **after** optional_failed, and configure-only recovery **after** optional_failed with a new metadata write, were not established by those receipts. No missing transition is silently filled by generic exit0.

## Actual constrained direct probes

Two new isolated attempts copied only these three explicitly nonsecret metadata files from the retained synthetic r5 state into separate fresh private roots:

- keyring `index.toml`: SHA-256 `43d1a15afb91d4eca65d3e6b3ad0c9ddbd2fb6c174997ddd53592379c8c58b70`;
- AI `index.toml`: SHA-256 `6447b31e98a0811516667bf5c51d00109848cc43322b4fda128a4a4569a44c50`;
- selected connection revision `config.toml`: SHA-256 `7f45e64cf5649cefb596792c4cb7db16d3bcc92f1954605c285c2c2041ec6475`.

No value file, ordinary HOME/auth/config, socket, original world or installed runtime tree was mounted. Metadata copies were read-only in the namespace. Existing CLI SHA-256 `b3e5f2a0d0ace1db88e4e3fa866a6b487de1acde63593e888eca2b3ff1a9094b` invoked `ai setup --agent pi --mode connection --connection <existing-synthetic-id> --json --unattended`. Both attempts returned exit1, empty stdout and the static private-keyring protection refusal. R2 additionally mounted the private parent and enforced private directory modes; the same refusal remained. No preview was emitted and no metadata publication occurred.

First actual owner: `vibe-secrets/src/lib.rs::Store::load` validates each indexed secret revision and requires a protected `value` leaf before admitting the keyring inventory. `reference_snapshot` calls that owner even for reference-only metadata. This presence/protection validation is distinct from reading credential contents, but removing value leaves makes this constrained fixture inadmissible. Do not call it an ABI/provider failure or weaken the existing storage guard merely to make a recovery test pass.

## Preservation and boundary

Private evidence `/data/git/vibevm-keyring-storage-20261120T0100Z/corp-setup-recovery-preview-r{1,2}/receipt.json` records exits, diagnostics and metadata hashes. Original accepted r5 metadata was not edited; only named nonsecret metadata bytes were read/copied. Mandatory installation and unselected bindings were not mounted or touched. No new secret, placeholder value, token access, provider call, supplier install, fallback or host state mutation occurred.

**Disposition:** constrained direct preview is an executed protection refusal, not a successful preview. Direct recovery-write and full-bootstrap-after-optional_failed remain **UNRUN** within this no-value-mount contract. The accepted full rerun after input_required remains its prior positive receipt, not a substitute for those transitions. Further qualification needs an explicitly admitted fixture/runner that satisfies storage presence without expanding credential exposure, or a different independently executable NST cell. This report grants no such permission and closes no parent/platform/credential gate.
