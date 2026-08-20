# Building the Windows distributive (1.0.0 recipe)

The authored halves of the zip live in this directory (`install.ps1`,
`uninstall.ps1`, `README-INSTALL.md`); the rest of the archive is built
from the tree. The zip itself is a disk artifact (`dist/` is gitignored —
a binary archive is never committed).

Recipe, verified 2026-08-20 (the C9 landing):

```powershell
# 1. Release binary with static CRT — no VC++ Redistributable needed.
$env:RUSTFLAGS = '-C target-feature=+crt-static'
cargo build --locked --release -p vibe-cli --bin vibe
& .\target\release\vibe.exe --version           # must print: vibe 1.0.0

# 2. Prove the CRT verdict (imports must NOT contain VCRUNTIME140.dll):
#    dumpbin lives in the VS toolchain, not on PATH.
& "C:\Program Files\Microsoft Visual Studio\2022\Community\VC\Tools\MSVC\<ver>\bin\Hostx64\x64\dumpbin.exe" /DEPENDENTS .\target\release\vibe.exe

# 3. Stage the seven files.
#    dist/stage/: vibe.exe, install.ps1, uninstall.ps1, README-INSTALL.md,
#    LICENSE.md (repo root), ALPHA-NOTES.md (docs/), SHA256SUMS.txt
#    (sha256 of the six files; first line:
#    "# corruption check only, not authenticity").

# 4. Zip via python zipfile (NOT Compress-Archive): sorted entry names,
#    fixed entry timestamps, DEFLATE level 9 →
#    dist/vibe-<version>-windows-x86_64.zip; record the zip's SHA256.
```

Sandbox smoke before handing the zip to anyone: extract to a temp dir,
`install.ps1 -InstallBase <temp>\.vibe -NoPath`, verify
`<temp>\.vibe\opt\bin\vibe.cmd --version` / `self current` / `self ls`,
re-run install (must reuse, one instance), then
`uninstall.ps1 -InstallBase <temp>\.vibe` (store gone, settings kept).
The live `%USERPROFILE%\.vibe` and the user PATH are never touched by the
smoke; only the owner's real installation run exercises the PATH step
(`vibe self doctor --fix`).
