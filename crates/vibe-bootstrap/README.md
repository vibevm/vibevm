# Native corporate kit installer

`vibe-installer` runs directly as an ordinary user. No Rust compiler, installed
Vibe, shell script or external archive utility is required on the recipient.
It discovers the kit beside its own executable, or accepts `--kit PATH`.

```text
vibe-installer --verbose
vibe-installer --mode manual --debug
vibe-installer --no-path --unattended
vibe-installer --dry-run
```

Default behavior activates **user-level PATH automatically**, including unattended
runs. `--no-path` explicitly preserves persistent PATH. It never requests elevation
or writes machine PATH. Explicit destination/settings options must agree with any
corresponding field in the selected profile; contradictory homes are refused.

The kit supplies `profile-<linux|windows|macos>-<auto|manual|pac>.toml`, a local
platform ZIP and `SHA256SUMS.txt`. Missing local inputs are errors, not implicit
public downloads. Existing third-party companion archives are reused when needed.

The complete release ZIP, its platform, component/source hashes and safe source
paths are verified before a bundled executable runs. Full site validation and
installation use the verified native `vibe self apply` owner, including its complete
unpacked-bundle import. Failure after an applied stage is not claimed to roll back.

`--verbose` reports actions; `--debug` adds a private log with stage timings,
verified public artifact provenance, child status and closed diagnostic causes.
Logs never dump profiles, credentials, arbitrary child output or environment.
Terminal progress is compact; redirected output contains no animation controls.

The existing `install.sh`, `install.cmd` and associated helpers remain independent
fallbacks. Neither native installation nor fallback scripts require the other.
Ready native installer assets are emitted by `cargo xtask dist build` beside each
platform release. Local delivery binds every asset to source, target and SHA256;
no external release or publication is implied by building them.
