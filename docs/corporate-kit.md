# Create a local corporate kit

`vibe kit` produces a kit from explicit local files. It does not install Vibe, download inputs, build source, publish a release or read your corporate credentials. You do not need an existing owner's kit directory or a Vibe project.

## Prerequisites and trust

The current producer runs on Linux. Its kit target is independent of that host: the existing distribution targets are Windows x64 MSVC, Linux x64 GNU/musl and macOS Intel/arm64. Linux-host production using actual historical Windows x64 and macOS Intel inputs is qualified separately; native producer execution and foreign consume/upgrade are not implied. Obtain reviewed, already built versioned distribution ZIP and native installer through your approved process. The distribution ZIP's closed manifest, components, source archive, target and version are verified. The separate installer is **trusted by the recipe author**: its hash identifies the selected bytes, not its publisher, role, version or platform. The producer never executes it to guess its identity.

Supply a site profile using the existing site-profile schema. Do not put keys, passwords, tokens or credential-bearing URLs into a recipe, profile, kit or checksums. Recipe input paths are literal and relative to the recipe's directory, with no absolute paths, parent traversal, symlinks or environment interpolation. This command is not a secret scanner: review explicitly supplied files before distributing them.

## 1. Initialize and edit

From a new ordinary working directory, create the recipe directory. Keep this shell in `recipe` through validation; return to its parent for build so recipe and output paths need no parent traversal:

```sh
mkdir recipe
cd recipe
vibe kit init
vibe kit validate --file vibe-kit.toml --structure-only
```

Init creates one commented `vibe-kit.toml`. It refuses any existing occupant and does not create missing parent directories. The template contains demonstration paths and all-zero placeholder digests; structure-only success does **not** mean those artifacts exist.

Place the three approved inputs under the recipe directory, then edit the paths, kit version and exact lowercase SHA-256 values. Compute each digest using your normal trusted checksum tool, for example:

```sh
sha256sum inputs/release.zip inputs/vibe-installer inputs/site-profile.toml
```

The format is:

```toml
schema_version = 1

[kit]
name = "example-kit"
target = "x86_64-unknown-linux-gnu"
version = "1.0.12" # Must match the supplied distribution, not a promised release.

[bundle]
path = "inputs/release.zip"
sha256 = "REPLACE_WITH_64_LOWERCASE_HEX_DIGITS"

[installer]
path = "inputs/vibe-installer"
sha256 = "REPLACE_WITH_64_LOWERCASE_HEX_DIGITS"

[site_profile]
path = "inputs/site-profile.toml"
sha256 = "REPLACE_WITH_64_LOWERCASE_HEX_DIGITS"
```

The uppercase replacement text is intentionally not a valid digest. Unknown fields and malformed values refuse. A minimal synthetic profile for learning the file structure is `schema = 1` with `name = "example"` and `[path] fix = "never"`; a real site's registries/proxy/release policy remain owned by the existing profile format, not duplicated in the recipe.

## 2. Validate actual inputs

```sh
vibe kit validate --file vibe-kit.toml
```

Default validation checks real file hashes and profile semantics. It does not construct a kit or certify that an arbitrary installer is suitable. Bundle target/version and full member verification occur during build. The kit commands are local-only and independent of unrelated HOME/settings profiles. No interactive wizard is required; the editable template and flags support unattended automation.

## 3. Build to a new private output

The **existing parent of the output** must be owned by the current user with mode0700. It is separate from the recipe directory; the producer does not chmod an unsuitable parent. Create a new parent deliberately if needed:

```sh
cd ..
mkdir -m 700 output-parent
vibe kit build --file recipe/vibe-kit.toml --out output-parent/example-kit
```

The output must be absent and outside the recipe/input scope. Existing output is never replaced. The producer stages verified bytes and publishes with the existing Linux checked no-replace owner, then rechecks directory identity, all expected names and content. It makes no atomic hostile same-user identity or crash-recovery promise. On uncertain failure, preserve the stage/output for inspection instead of blindly deleting or retrying over it.

The emitted kit contains the target-versioned bundle ZIP, named native installer (`.exe` for Windows), `profile-<platform>-auto.toml` (`linux`, `windows` or `macos`), `SHA256SUMS.txt` and `INSTALL.md`. The commands below illustrate a Linux-target kit; run a foreign installer only on its actual target OS, never on the Linux producer merely because build succeeded. Hashes describe the exact staged inputs. Optional registry mirrors/drop acquisition and a guided wizard are not added by this producer; use their existing owners separately where admitted.

## Consume or explicitly upgrade

Use the emitted native installer, not an executable found by an untrusted PATH lookup. Replace the example paths with your chosen isolated test destinations:

```sh
output-parent/example-kit/vibe-installer-x86_64-unknown-linux-gnu \
  --kit output-parent/example-kit --destination /chosen/install \
  --settings /chosen/settings --no-path --unattended --dry-run
```

After reviewing the dry-run, remove `--dry-run` for a fresh installation. To upgrade an existing installation, request it explicitly:

```sh
output-parent/example-kit/vibe-installer-x86_64-unknown-linux-gnu \
  --kit output-parent/example-kit --destination /chosen/install \
  --settings /chosen/settings --no-path --unattended --upgrade --dry-run
```

Then repeat without `--dry-run`. Upgrade requires an intact exact active instance and a strictly newer verified full distribution. It refuses pending activation, downgrade and same-version requests. It retains the previous immutable instance and verifies the actual new active payload before success. Ordinary rerun and `--unattended` alone are **not upgrade consent**. Dry-run does not allocate an instance or run activation recovery; a missing/busy validation lock or pending journal requires diagnosis through the ordinary existing owner.

## Verification limits

Accepted Linux proofs cover clean template/edit/validate/build, no overwrite or user-state discovery, malformed/private-looking diagnostic controls, no network, produced-kit dry-run and fresh install, and genuine compiled1.0.11→1.0.12 explicit upgrade on one installation with corrupt-input preservation and foreign-file retention. These are synthetic fixtures, not supplier redistribution approval or a public release.

Earlier native consumer receipts for Windows x64 and macOS Intel at source `576e6a862` remain historical evidence. Their exact retained bundle/installer bytes can qualify Linux-host production of foreign-target kits; they do not prove native Windows/macOS production or new upgrade on those systems. Windows ordinary-user/manual checks were owner-excepted, not executed PASS. Unavailable/manual-resource-dependent checks are SKIPPED / NOT RUN, never replaced by Linux/compile-only receipts. No runner, VibeTerm or provider session is required to create this local recipe. Final campaign integration/conformance remains separately governed.
