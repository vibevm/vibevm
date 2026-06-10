# golden flow: uninstall
install then uninstall — the slot, lockfile and manifest must come back clean.

## $ vibe init --path .
exit: 0
### stdout
```
Initializing project `golden-proj` in `.`
  ✓ created  spec/boot/00-core.md
  ✓ created  spec/boot/90-user.md
  ✓ created  vibe.toml
  ✓ created  vibe.lock
  ✓ created  .vibe/.gitignore
  ✓ created  .gitignore
  ✓ created  spec/boot/INDEX.md
  ✓ created  CLAUDE.md
  ✓ created  AGENTS.md
  ✓ created  GEMINI.md

Done. Project `golden-proj`: 10 files created, 0 kept.

Next:
  • edit spec/boot/00-core.md and spec/common/ as your project takes shape
  • install packages with `vibe install <kind>:<name>` (e.g. flow:wal)
```
### stderr
```
```
## $ vibe install org.vibevm/wal --registry <REPO>/fixtures/registry --assume-yes
exit: 0
### stdout
```
Resolving 1 root package…

Materialising 1 package into vibedeps/:
  org.vibevm/wal@0.1.0


Materialised 1 package into vibedeps/; regenerated boot artifacts for 1 node.
```
### stderr
```
```
## $ vibe uninstall org.vibevm/wal --assume-yes
exit: 0
### stdout
```

Uninstall org.vibevm/wal@0.1.0 — remove `vibedeps/flow-wal/0.1.0` and regenerate boot.
  - removed  vibedeps/flow-wal/0.1.0

Uninstalled org.vibevm/wal@0.1.0 — removed its vibedeps/ slot, regenerated boot.
```
### stderr
```
```
## final file tree
```
./.gitignore
./.vibe/.gitignore
./.vibe/cache/org.vibevm/wal/v0.1.0/README.md
./.vibe/cache/org.vibevm/wal/v0.1.0/boot/10-flow-wal.md
./.vibe/cache/org.vibevm/wal/v0.1.0/spec/flows/wal/WAL-PROTOCOL.md
./.vibe/cache/org.vibevm/wal/v0.1.0/spec/flows/wal/morning-routine.md
./.vibe/cache/org.vibevm/wal/v0.1.0/spec/flows/wal/session-end-hook.md
./.vibe/cache/org.vibevm/wal/v0.1.0/vibe.toml
./AGENTS.md
./CLAUDE.md
./GEMINI.md
./spec/boot/00-core.md
./spec/boot/90-user.md
./spec/boot/INDEX.md
./vibe.lock
./vibe.toml
```
## file: vibe.toml
```
[project]
name = "golden-proj"
version = "0.0.1"
authors = []

[[registry]]
name = "vibespecs"
url = "https://github.com/vibespecs"

[[registry]]
name = "vibespecs-gitverse"
url = "https://gitverse.ru/vibespecs"
naming = "name"
```
## file: vibe.lock
```
[meta]
generated_by = "vibe 0.1.0-dev"
generated_at = "<TIMESTAMP>"
schema_version = 5
```
