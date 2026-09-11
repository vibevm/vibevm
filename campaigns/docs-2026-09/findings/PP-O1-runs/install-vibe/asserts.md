# asserts — install-vibe

The prompt itself is **not** run (installing vibe changes this machine). Per the packet its
asserts are executed on this machine as it is: cwd `%USERPROFILE%`, the machine's own
`vibe` on `PATH` (`C:\Users\olegc\.vibe\opt\bin\vibe`), no sandbox `VIBE_SETTINGS`.
A tripwire over `%USERPROFILE%\.vibe` was taken immediately before and immediately after
the two commands; the diff is empty.

```
before: 46514 entries
===== ASSERT: vibe --version   (machine install on PATH)
vibe 1.0.0
----- exit: 0
===== ASSERT: vibe self doctor  (read-only without --fix)
vibe self doctor
  → ok   git 2.52.0
  → ok   cargo 1.93.1
  → ok   rustc 1.93.1
  → also MSVC Build Tools (Desktop development with C++) — https://visualstudio.microsoft.com/visual-cpp-build-tools/
  → ok   shim dir C:\Users\olegc\.vibe\opt\bin (on PATH)
  → ok   active branch:main #70
  → ok   embedded registry C:\Users\olegc\git\v\vibevm\vibevm\vibepacks (source install; precedence embedded-first)
all good.
----- exit: 0
after: 46514 entries
=== diff around the two commands ===
(identical)
```
