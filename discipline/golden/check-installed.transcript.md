# golden flow: check-installed
vibe check over a freshly initialised + installed project — what a clean checkup looks like.

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
## $ vibe check --path .
exit: 0
### stdout
```
vibe check: clean — every check passed against `<SANDBOX>`
```
### stderr
```
```
## $ vibe check --path . --quiet
exit: 0
### stdout
```
vibe check: 0 errors, 0 warnings, 0 info
```
### stderr
```
```
