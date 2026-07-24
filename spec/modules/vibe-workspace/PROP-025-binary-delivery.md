# PROP-025 — vibe-native binary delivery {#root}

<status stage="impl" state="done" comment="B0 2026-07-24: v1 IMPLEMENTED SS2-5 (deferrals-closeout campaign); S6-S7 specified v2 surface; fact grain 2026-07-24"/>

##status-line **Status: v1 IMPLEMENTED (§§2–5; the deferrals-closeout campaign). §6–§7 are
specified v2 surface.** Module: `vibe-workspace` / `vibe-install` / `vibe-cli`. @impl/done

## 1. Problem {#problem}

##req-problem `req r1` @impl/done

- ##packages-ship-tools Code-bearing packages (PROP-024) ship runnable tools — the discipline
  stacks alone ship the umbrella + gate binaries (`rust-ai-native`,
  `rust-ai-native-conform`, `rust-ai-native-specmap`, `typescript-ai-native`,
  `typescript-ai-native-conform`, `typescript-ai-native-specmap`) — but
  `vibe install` stops at materialising source
  into `vibedeps/`. @impl/done
- ##manual-path-step Getting from a slot to a tool on PATH is a manual,
  documented step (`cargo install --path vibedeps/<slot>/crates/<cli>`,
  GUIDE §13), repeated per machine and re-repeated per version bump: n
  stacks × m tools of PATH management that vibe already knows how to do for
  itself (PROP-019). @impl/done
- ##BINARY-MUST MUST: a package declares its binaries; vibe builds and
  dispatches them. @impl/done

## 2. Manifest surface {#manifest}

##req-manifest `req r1` @impl/done

##BINARY-TABLE A code-bearing package declares each shipped tool in its `vibe.toml`: @impl/done

```toml
[[binary]]
name = "rust-ai-native"             # the PATH-facing name (the family
                                    # prefix keeps it collision-safe)
crate = "crates/rust-ai-native-cli" # package-relative crate directory
```

- ##NAME-CONSTRAINTS Constraints: `name` MUST be unique within the package and SHOULD be
  globally collision-safe (the family-prefix convention, PROP-028 §2.4). @impl/done
- ##CRATE-CONSTRAINT `crate` MUST name a directory inside the shippable tree carrying a
  `[[bin]]`-bearing (or default-bin) Cargo package whose bin name equals
  `name`. @impl/done
- ##LINTER-VALIDATES The linter (`vibe check`) validates both. @impl/done
- ##OPT-IN Absent `[[binary]]`
  tables mean the package ships no tools — every field of this PROP is
  opt-in. @impl/done

## 3. Install-time build {#build}

##req-build `req r1` @impl/done

- ##BUILD-TIMING After materialising a slot whose manifest declares `[[binary]]` entries,
  `vibe install` MAY build them (v1: on `vibe bin sync`, see §4 — the
  install itself only RECORDS the declarations; an install-time
  `--build-bins` opt-in flag is v2 surface). @impl/done
- ##BUILD-CONSENT Building executes the
  package's build scripts and proc-macros — arbitrary code — so it is
  consent-gated exactly like install hooks (PROP-020): the `org.vibevm`
  group is allow-listed; any other group prompts (or requires
  `--allow-hooks`-equivalent consent) before the first build of a
  (package, version). @impl/done

- ##SLOT-RESIDENT Artifacts are **slot-resident**: `cargo build --release` runs with the
  slot's own workspace manifest and lands in `vibedeps/<slot>/<ver>/target/`. @impl/done
- ##HASHES-STABLE Build output sits outside the shippable tree (PROP-024 §2.2), so content
  hashes never move. @impl/done
- ##REFRESH-INVALIDATES A slot refresh (version bump, re-materialise)
  naturally invalidates its binaries — staleness handling costs nothing. @impl/done

## 4. The `vibe bin` family and dispatch {#dispatch}

##req-dispatch `req r1` @impl/done

- ##BIN-LIST `vibe bin list` — every `[[binary]]` declared by the project's
  installed packages, with build state (built / not built) and the
  artifact path. @impl/done
- ##BIN-BUILD `vibe bin build [<name>…]` — consent-gated release build of the named
  tools (default: all declared), in their slots. @impl/done
- ##BIN-PATH `vibe bin path <name>` — the artifact path (non-zero when not built);
  scripts compose with it. @impl/done
- ##BIN-EXEC `vibe bin exec <name> [--] <args…>` — resolve `name` through the
  CURRENT project's lockfile → its slot → the slot-resident artifact
  (building it, with consent, if absent), then execute with the exit
  code passed through. This is the rustup dispatch model: the project's
  pinned version is what runs, always. @impl/done

- ##SHIMS-SPEC Shims (`vibe bin sync`) are v1.5 surface, specified here: a per-user bin
  dir (reconciled with PROP-019's shim dir — one PATH entry, not two) of
  dumb launchers, each `exec`-ing `vibe bin exec <name> -- %*` / `"$@"`
  (Windows `.cmd` + POSIX sh pair; the `cmd /c` spawn lesson of PROP-015
  applies). @spec/done
- ##LAUNCHER-VERSION-FREE A launcher never encodes a version — dispatch stays per-CWD
  through the lockfile walk, so two projects pinning different stack
  versions get different binaries from the same PATH entry. @spec/done

## 5. Staleness and offline honesty {#staleness}

##req-staleness `req r1` @impl/done

- ##TRUST-CURRENT-SLOT The artifact is trusted iff it exists in the CURRENT slot (the slot
  version is the unit of staleness — PROP-011 slot replacement removes the
  `target/` with the slot). @impl/done
- ##WARM-NOOP `vibe bin build` on a warm artifact is a cargo
  no-op (~seconds). @impl/done
- ##OFFLINE-HONESTY Cargo needs crates.io for third-party deps unless the
  local cargo cache is warm: offline boxes get the same honest failure
  cargo gives, plus the hint that `cargo install --path <slot>/crates/…`
  (the documented degraded path, which stays valid indefinitely) has the
  same network shape — there is no offline shortcut to a first build. @impl/done

## 6. Cross-package path dependencies (v2, specified only) {#cross-package}

##req-cross-package `req r1` @spec/done

- ##CROSS-DEP-IMPOSSIBLE A stack crate cannot Cargo-`path`-dep on a core-ai-native crate across
  slots: the authored layout (`packages/org.vibevm/<name>/v<ver>/`) and the
  materialised layout (`vibedeps/<kind>-<name>/<ver>/`) disagree on both
  directory naming and version prefix, and each slot must stay a
  self-buildable workspace (PROP-024 §2.4). @impl/done
- ##V1-VENDOR-SYNC v1 answers this with
  vendor-sync (byte-identical copies under `crates/vendor/`, gated by
  `sync-engines --check`). @impl/done
- ##V2-REWRITE-ALTERNATIVE The v2 alternative — vibe REWRITING declared
  cross-package path-deps at materialise time (a `[binary.cross_paths]`
  manifest table mapping dep names to `<pkgref>:<crate-path>`) — interacts
  with shippable-tree hashing (the rewritten manifest must be excluded from
  the hash exactly like build output, or identity breaks) and with
  presence-trust. @spec/done
- ##V2-TRADE-VISIBLE It lands only with its own campaign and only if
  vendor-sync's duplication cost ever exceeds the rewrite machinery's
  complexity cost. Recorded so the trade stays visible. @spec/done

## 7. Uninstall and GC (v2, specified only) {#gc}

##req-gc `req r1` @spec/done

- ##UNINSTALL-REMOVES `vibe uninstall` of a package removes its slot and therefore its
  artifacts (nothing else to clean in v1 — artifacts are slot-resident and
  launchers are version-free). @impl/done
- ##V2-SHIM-GC v2 shim GC: `vibe bin sync` removes
  launchers whose names no installed package declares. @spec/done
- ##V2-VARS-ROW `vibe vars` learns
  the bin-dir row when shims land. @spec/done

## 8. Security posture {#security}

##req-security `req r1` @impl/done

- ##CONSENT-MODEL An install-time / exec-time build runs third-party build scripts:
  consent is per (group, package, version) with the PROP-020 recording
  convention, `org.vibevm` allow-listed, `--assume-yes` honoured in
  non-interactive runs. @impl/done
- ##NO-PATH-HIJACK `vibe bin exec` never searches PATH for the tool it
  dispatches (no hijack surface): resolution is lockfile → slot → artifact,
  all project-local. @impl/done
- ##SCOPE-UNCHANGED Scope discipline (PROP-002 §2.10) applies unchanged. @impl/done

## 9. The v1 cut {#v1-cut}

##req-v1-cut `req r1` @impl/done

- ##V1-IMPLEMENTED Implemented this campaign: §2 manifest parsing + linting, §3 slot
  builds with consent, §4 `vibe bin list/build/path/exec`, §5 staleness by
  slot-residency. @impl/done
- ##V1-DEFERRED Deferred with names: §4 shims (`vibe bin sync` — lands
  with a PROP-019 shim-dir reconciliation pass), §6 cross-package path
  rewriting, §7 GC + `vibe vars` row. @impl/done

## History {#history}

- ##HIST-AUTHORED 2026-07-07 — authored and v1-implemented in the deferrals-closeout
  campaign; supersedes the "future PROP" note the Self-Sufficiency
  campaign's §10 recorded. @impl/done
