# Native corporate delivery — 2026-11-18

## Delivered local result

The existing owner-local `~/corp` kit is updated with fresh native Vibe bundles and
ready-to-run Rust installers for Linux x64 GNU,Windows x64 MSVC and macOS Intel.
Every build binds to source `576e6a8623c10087c9ad5a0730754d6ba9d61be6`.
The installer is approximately1.4–1.5MB per platform; recipients need no Rust,
installed Vibe,shell script or external archive utility for native bootstrap.
No external source push,tag,release or corporate rollout was performed.

The requested offline-transfer delta is present at **`~/corp-upd-2`**. Its nine
payload files total **371,719,530bytes** (about354.5MiB),plus compact update metadata.
Unchanged third-party payloads were reused rather than downloaded or retransferred.

### Exact owned update scope

- Three existing `kit/drop/releases/v1.0.12/vibevm-1.0.12-<target>.zip` files replaced.
- Three new `kit/vibe-installer-<target>[.exe]` executables added.
- New `kit/NATIVE-ASSETS.json` with source,target,size and SHA256 bindings.
- New `kit/INSTALL-NATIVE.md` with direct native launch and fallback instructions.
- Existing `kit/SHA256SUMS.txt` updated only for changed/new owned delivery entries.

That is **four replacements and five additions**,with **no deletions**. The delta
contains only these changed required files under their relative paths,alongside
`UPDATE-MANIFEST.json`, `SHA256SUMS-UPDATE.txt` and `UPDATE.md`. Instructions require
checking each replacement's old SHA256 before copying and the new SHA256 afterward;
unexpected local changes must be reviewed,not overwritten.

## Preserved inputs and recovery boundary

Before mutation,the complete5758-file private baseline and all staged source hashes
were verified. Previous bytes of the four replaced files were retained separately.
After replacement,all **5754 unrelated baseline files** were hash-identical.
Private profiles,admin data,mirrors,independent `install.sh`/`install.cmd` and their
archive helper remain intact. All52unchanged third-party drop entries are reused.
The native installer does not require the fallback scripts,and those scripts were
not rewritten to require the native executable.

The update used staged replacement and retained backups. It does not claim atomic
rollback across nine files. The delta was replayed against the baseline hash model
and matched the updated main kit; every carried file and both checksum manifests
were independently rechecked. The full private baseline is an audit-local artifact,
not something recipients need to copy or something published in this repository.

## Verification

- Final coherent canonical host gate: **8/8PASS,exit0**. Workspace tests3432s;
  strict workspace/all-target Clippy364s;format14s;host check50s;1MiB startup;
  conform9s;codegen6s;wire diff28s. No waiver/baseline rewrite.
- All three final native platform bundles and installers were built on their own
  OS,with identical source identity. Manifest,target and every component/source
  archive size/hash were verified. Linux release50m26s,Windows17m07s,macOS34m15s.
- Seven actual native cases pass on each platform: preflight,fresh installation,
  repeat,explicit no-PATH,conflicting homes,invalid profile and corrupt ZIP.
  Tests check automatic user PATH,no-PATH state preservation,all three runtime
  components and source,foreign-file preservation,space-containing paths,unrelated
  cwd,private debug sentinel omission,and absence of external tools in PATH.
- Linux runs under ordinary UID1000;macOS under ordinary UID501.
- The **updated main kit itself**,not merely staging,completed an additional native
  installation into isolated scratch under UID1000. All components/source and
  automatic isolated user PATH were checked;normal host installation was not changed.

## Explicit remaining criterion

**Windows non-elevated runtime verification is still pending.** The SSH functional
run is elevated and is not presented as proof of ordinary-user operation. A prepared
single normal-terminal verification command uses the final ready assets; no build
is required from the owner. No UAC,security-policy,account,credential or desktop ACL
changes were made to manufacture this evidence. Do not claim that all three OS
ordinary-user proofs or whole-task owner acceptance have been obtained.

The earlier OOM-interrupted final gate has no verdict. The initial mixed-subject
canonical failures and repaired integration regressions remain documented. The
successful final gate uses reduced process-local parallelism and enabled incremental
compilation. A local sccache/incremental incompatibility was handled by a process-only
direct compiler invocation,not a source/configuration policy rewrite.

## Evidence pointers (machine-local)

- `/fast/tmp/corp-final-canonical-green-576-20261118.json`
- `/fast/tmp/corp-final-delivery-576e6a862/{linux,windows,macos}/ADMISSION.json`
- `/fast/tmp/corp-final-delivery-576e6a862/{linux,windows,macos}/NATIVE-FUNCTIONAL.json`
- `/fast/tmp/corp-final-delivery-576e6a862/OWNED-UPDATE-PLAN.json`
- `/fast/tmp/corp-owned-update-apply-576-20261118.log`
- `/fast/tmp/corp-owned-update-final-verification-576-20261118.json`
- `/fast/tmp/corp-upd-2-baseline-20261118.json`
- `/fast/tmp/corp-upd-2-owned-backup-20261118/`
- `~/corp-upd-2/UPDATE-MANIFEST.json`

No actual private profile contents,corporate endpoints or credentials are included
in this report. Paths identify local evidence rather than new public package inputs.
