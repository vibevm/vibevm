# golden flow: install-short-name
vibe init, then install by bare short name — exercises the PROP-008 Phase 5 short-name resolution boundary.

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
## $ vibe install wal --registry <REPO>/fixtures/registry --assume-yes
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
## file: vibe.toml
```
[project]
name = "golden-proj"
version = "0.0.1"
authors = []

[requires.packages]
"org.vibevm/wal" = "^0.1.0"

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
root_dependencies = ["org.vibevm/wal@^0.1.0"]

[[package]]
kind = "flow"
name = "wal"
group = "org.vibevm"
version = "0.1.0"
source_url = "file:///<REPO>/fixtures/registry/org.vibevm/wal/v0.1.0"
content_hash = "sha256:865d47fb41fb8590ef6f0780f7fe98c716b897dea494769dd37a0e5280bc55a5"
files_written = []
source_kind = "registry"
```
