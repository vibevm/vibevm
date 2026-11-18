# CORP-NATIVE-INSTALLER-01 — directly executable corporate bootstrap

Status: architect dispatch prepared2026-11-18; execute after APP-VERSIONS Windows
migration is accepted. Owner mandate: RUST-INSTALLER-FOLLOWUP.md. This is one finite
user workflow, not a new corporate deployment framework.

## Task and ownership

Builder implements and verifies in their own checkout. Architect owns design
clarifications and integrated acceptance. Base at dispatch preparation is
`d1267eb04` (resolve the exact synchronized head again when this task starts).
No worktree, external publication, host security-policy change or shared service
operation is implied. Existing APP work retains priority and source ownership.
Owner grant2026-11-18: if base or flash lacks build prerequisites such as the Rust
compiler, installing the prerequisites necessary for these native builds is authorized.
Inspect existing tools first, install only actual missing dependencies through normal
trusted mechanisms, and record versions/changes. This does not authorize unrelated
upgrades, disabling corporate/security policies, credential changes or destructive
replacement of existing toolchains. Keep build/test data in established isolated roots.

## Existing seams already identified

- Corporate private instance has separate `kit/` and `admin/`. The kit has
  install.sh/install.cmd, profile variants, checksums and installation guidance.
  Current scripts orchestrate selection, extraction, self import and self apply.
- Existing Rust CLI owns verified distribution/bootstrap, VersionStore, site profile
  preparation/application and platform PATH persistence. Inspect
  `crates/vibe-cli/src/commands/vvm/bundle.rs`, `vvm/site/apply.rs`,
  `xtask/src/dist/build.rs` and `distribution/install/` before choosing integration.
- A raw release bootstrap already contains Rust CLI behavior; do not mistake its
  existence for a complete corporate kit installer or create another resolver.
- Private `~/corp` contains actual internal addresses. Never copy profiles, private
  configuration, credentials or real endpoints into committed tests/docs/prompts.

## Design constraints

1. The corporate kit MUST contain already compiled, ready-to-run Rust installer
   executables for Windows, Linux and macOS. Source code, build instructions or a
   promise to download/build on first launch do not satisfy this requirement. Build
   Windows on the established base SSH host, macOS on flash, and Linux locally,
   using their established isolated test/build directories and existing toolchains.
   Bind each asset to its source revision, target and SHA256; test the actual native
   executable on the corresponding host. Direct Windows launch is first-class.
   A small CLI/TUI is sufficient; a GUI framework is not required. No Rust toolchain
   or script interpreter may be required on the recipient's machine for bootstrap.
2. No essential dependency on bash/cmd/PowerShell scripts, external tar/unzip or an
   already-installed vibe for the supported bootstrap path. Calling a verified native
   vibe executable with structured argv is allowed; do not shell-interpolate input.
3. Reuse existing Rust owners for digest validation, bounded safe extraction,
   import/activation, profile parsing/application and consent-aware PATH changes.
   Do not duplicate configuration precedence, release resolution or home selection.
   Choose a minimal executable front end plus shared bootstrap code, not a parallel
   independently evolving installer. Bring the concrete crate/API seam to architect
   after bounded source inspection, then implement without per-helper pauses.
4. Installer discovers bundled files relative to its own location, never the caller's
   cwd. Explicit kit/profile/destination options may override this with validation.
   Existing profiles remain the site configuration authority. Do not bake this owner's
   endpoints or absolute local paths into generic binaries.
5. Preflight validates the chosen profile, platform, required bundled artifacts and
   digest declarations before publication. Errors clearly distinguish no effects from
   already committed partial effects; do not promise unsupported atomic rollback.
6. No automatic privilege elevation, execution-policy bypass or consent bypass.
   Preserve default PATH consent and explicit no-PATH behavior. Explain that native
   launch avoids script dependence but cannot evade corporate executable restrictions.
7. Default is the kit's intended closed-network workflow. No implicit public network
   fallback when local artifacts are missing. Missing inputs name the required file.
   Existing configured corporate routes retain their ordinary policy; do not invent
   a broad mandatory offline mode that breaks supported configured proxies.
8. Retain existing script entry compatibility unless a concrete contract requires
   change. They may delegate to native binaries; native startup must not delegate
   essential installation back to them.

## User experience and diagnostics — owner clarification2026-11-18

- Provide `--verbose`: describe each action in useful detail, including what is
  checked, selected, unpacked, installed and applied, and whether it changed anything.
- Provide `--debug`: detailed diagnostic logging for corporate-environment failures,
  including stages, timings, safe resolved paths, platform/source/target identity,
  validation decisions, child outcome/exit status and causal error chains. It includes
  verbose progress but must never dump credentials, complete environment/configuration,
  authorization headers or private profile contents. State the log location clearly;
  diagnostic logging is not authority to collect secrets.
- Default UI should be pleasant and compact, borrowing vibe styling where useful:
  a few clear stage/status lines, restrained pseudographics and a spinner for long
  operations. Disable animations/control sequences when output is redirected or the
  terminal is unsuitable. Keep actionable errors and useful plain-text logs.
- Prioritize complete basic user scenarios. Do not add a GUI framework, generic
  observability platform or an exhaustive installer-feature/perfection campaign.
  Basic installation, safe refusal, clear diagnostics and repeat invocation suffice
  when their supported behavior is reliable. Existing correctness/privacy boundaries
  remain binding; optional polish does not become a prerequisite ladder.

## Acceptance

- Generic fresh fixture kit, path with spaces, unrelated cwd: native installer
  selects the matching platform/profile and installs into isolated settings/root.
- Run/re-run, profile validation refusal, missing/corrupt/wrong-target artifact,
  no-PATH and unattended consent cases; existing user files remain intact.
- Demonstrate the bootstrap with script interpreters/external archive tools absent
  from its process PATH. Native child binaries are permitted, shell wrappers are not.
- Actual Windows on base, Linux locally and macOS on flash using their established
  separate test roots/new run-specific children. Do not alter normal installations,
  global PATH or profiles for tests. Report unavailable architectures honestly.
- Existing focused self/bootstrap/site-profile tests plus format/strict/conform and
  applicable comprehensive gate on the coherent final subject. Reuse unchanged valid
  evidence rather than repeat full matrices after documentation-only synchronization.
- Packaging includes native assets and correct hashes; no real private configuration
  enters a release/test fixture. Review source/binary binding, not version string only.

## Rebuild the current distribution and update the owner's existing ~/corp

Owner clarification2026-11-18: completion requires a freshly rebuilt corporate
 distribution containing all latest reviewed improvements in vibe, not merely new
installer executables beside stale vibe bundles. Freeze the coherent final accepted
source revision, build matching native release payloads and installer binaries for
Windows/Linux/macOS using base/local/flash, and regenerate the applicable canonical
manifests/checksums with exact source/target/hash binding. Include the accepted
APP-VERSIONS fixes. Inspect and retain existing source/distribution version and
immutability rules; do not silently republish changed bytes as an already published
release. Local staging/build/update is authorized, external release/tag/push is not.
Validate the newly built payloads through the native installer on their actual
platforms before updating the local corporate instance. Do not substitute an older
bundled vibe just because its version string or profile parser is convenient.

Explicitly authorized by the owner as part of this task, after implementation review.
Inspect current layout and compute a bounded update manifest first. Preserve private
profiles, payload drops, admin data, local mirrors and unrelated custom files.
Update only installer assets, owned checksums and appropriate private installation
instructions; amend profile selection only if the actual new contract requires it.
Use staged/create-new outputs and ordinary recoverable replacement; never wipe the
whole directory or regenerate confidential profiles from generic examples.

Verify the resulting kit with native entry points in isolated scratch, then report
exact updated paths and hashes without exposing endpoint/profile contents. External
source push, releases, corporate rollout and signing remain separately authorized.
