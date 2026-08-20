# vibe 1.0.0 for Windows x86_64

This zip is the Windows x86_64 distribution of `vibe` 1.0.0.

## Install

1. Extract the whole zip to a directory. Do not run the scripts from inside
   the zip viewer.
2. Open PowerShell in the extracted directory.
3. Run:

   ```powershell
   powershell -ExecutionPolicy Bypass -File .\install.ps1
   ```

The installer verifies `vibe.exe` against `SHA256SUMS.txt`, imports the
release into `%USERPROFILE%\.vibe\opt`, activates `tag:1.0.0`, writes stable
shims under `%USERPROFILE%\.vibe\opt\bin`, and safely ensures that shim
directory is at the front of the user `PATH`. Open a new terminal after the
install so it inherits the updated `PATH`.

For a sandboxed or portable install, override the base directory and leave
`PATH` unchanged:

```powershell
powershell -ExecutionPolicy Bypass -File .\install.ps1 `
  -InstallBase C:\path\to\sandbox\.vibe -NoPath
```

## Prerequisites

- 64-bit Windows 10 or later and PowerShell 5.1 or later.
- Git available on `PATH` for package and source-registry operations.
- Network access when a requested package must be fetched from a registry.

The shipped `vibe.exe` uses the static Microsoft C runtime. The Microsoft
Visual C++ Redistributable is therefore not required for this distribution.
Rust and Visual Studio Build Tools are not required to run the prebuilt
binary; they are required only for source builds.

## Verify

In a new PowerShell window:

```powershell
vibe --version
vibe self current
vibe self which
vibe self ls
```

The version command must print `vibe 1.0.0`, and the active line must begin
with `tag:1.0.0` (the following `#N` is the immutable instance number).

`SHA256SUMS.txt` detects accidental corruption only. It is not a signature or
proof of authenticity.

## Updates in this alpha

Automatic binary self-update is not available yet. `vibe self update` and
`vibe self install` still build from source and therefore require Git, Rust,
and the MSVC C++ build tools. A later zip can be installed by extracting it
and running its installer; immutable release tags prevent one payload from
silently replacing a different payload with the same version.

## Uninstall

Remove only the release installed by this zip. This leaves the user `PATH`,
other installed VVM versions, shims, and settings unchanged:

```powershell
powershell -ExecutionPolicy Bypass -File .\uninstall.ps1
```

Remove the complete VVM store under `%USERPROFILE%\.vibe\opt`, both `vibe`
shims, and one matching user-`PATH` entry:

```powershell
powershell -ExecutionPolicy Bypass -File .\uninstall.ps1 -All
```

`-All` does not remove settings or caches elsewhere under `%USERPROFILE%\.vibe`.
Both modes accept `-InstallBase` for an overridden installation base.
