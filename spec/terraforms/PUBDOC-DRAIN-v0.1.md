# vibevm Pub-Doctest Drain v0.1 — teach every public vibe-core type by one compiled example
**status: EXECUTED TO COMPLETION 2026-06-14 · all 8 batches (B1–B8) landed · conform baseline drained to ZERO · vibevm-specific**

> **Execution record (2026-06-14).** All 55 `pub-doctest` entries drained
> across eight file-cohesive commits on `origin/main` (`f0067cc` B1 →
> `53021b6` B8), the panel green at every commit. Per-batch shrink:
> 55 → 50 → 37 → 29 → 22 → 17 → 12 → 5 → **0**. Since this debt was the whole
> residual conform baseline, `conform-baseline.json` now carries an empty
> `findings` array — `cargo xtask conform check` reports 0 frozen / 0 new
> for the first time in the project's history. vibe-core stays in
> `GATED_PUB_DOCTEST`: the gate stays armed against new undocumented types.
> No file crossed the 600 `file-length` budget (the heaviest, `package.rs`,
> landed at 582); specmap stayed clean throughout (454 / 448 / 459,
> 0 suspects); full `self-check.sh` green at close. The §5 predictions held:
> no test expectation moved, every drain was a deletions-only freeze diff,
> and the edge (b) `#[spec(documents)]` escape was used zero times — all 55
> landed as compiled doctests.

*Origin: CONVERT-PLAN v0.1 §1.4 landed the `pub-doctest` rule (Class G, commit `647ce68`,
`crates/conform-core/src/rules/diagnostics.rs:137`) and froze vibe-core's accumulated
type-doc debt as a shrink-only baseline. The plan predicted single digits (§0: "vibe-core
≥8"); the flip **falsified** that — **55** public types under `crates/vibe-core/src/` page
in with prose alone and no compiled example. That 55 is now the **entire** conform baseline
(`conform-baseline.json`): every other gate drained to zero across SHRINK v0.1/v0.2 and
CONVERT v0.1, the MCP `file-length` pair drained in Phase 7. So this drain does not merely
shrink a rule — it takes the whole panel to an **empty baseline** for the first time in the
project's history.*

*The rule's contract (`PubDoctest`, gated crate list `GATED_PUB_DOCTEST = ["vibe-core"]`,
`xtask/src/conform.rs:77`): every `pub struct|enum|trait|union` declared under `src/`
carries either (a) one compiled doctest in its **own** doc comment, or (b) a
`#[spec(documents = "spec://…")]` edge. `has_doctest` keys on the type's own `///` block —
a doctest on a method or on the module does not satisfy it (so `CapabilityNamespace` is in
debt though `CapabilityRef` and the `capability_ref` module both carry examples). The
foundation crate is the few-shot prompt a model copies first (guide §2, R3-006): every
type it pages in must teach by runnable capital, not WISH-prose (Discipline law 2).*

*This is drain-as-you-go, not a blocker. The gate is a downward ratchet — any batch is a
safe stopping point, the baseline only shrinks, and the work can interleave with any other
goal. vibe-core **stays** in `GATED_PUB_DOCTEST` after the drain: the gate stays armed so a
future undocumented type fails CI as `new`; only the standing debt goes to zero.*

*Rhythm: file-cohesive batches (one file's types share a TOML fixture and one commit), the
SHRINK/CONVERT cadence — build + `cargo test -p vibe-core --doc` + crate tests + `fmt` +
`conform check` (0 new) + a **shrink-only** `conform freeze` diff + `specmap --check`
(regen only if an edge (b) was added) → topic commit citing the batch → push. Edits land
only through the editor tools — PS5.1 corrupts UTF-8-no-BOM round-trips; recover via
`git restore`.*

---

## 0. Baseline survey and target arithmetic

Panel at plan time (2026-06-14, tree `13bb61c`, CONVERT-PLAN v0.1 complete): `conform check`
— **55 frozen / 0 new** (the residual 55 = this debt, the only frozen entries left);
`specmap --check` — clean, 454 units / 459 edges / 0 suspects / 0 warnings / 0 orphans /
0 dispositioned; `CONFORM_GATED` = 16; `vibe check` 0/0/0; full `self-check.sh` green.

The 55 by file (`conform-baseline.json`, every fingerprint `pub-doctest|<file>|<symbol>`),
with each file's current physical line count and its headroom under the `file-length` 600
budget — the budget is the only structural risk in this plan, called out per batch in §4:

| File (`crates/vibe-core/src/…`) | Types | Lines | Headroom |
|---|---:|---:|---:|
| `manifest/package.rs` | 13 | 442 | 158 |
| `manifest/project.rs` | 8 | 484 | 116 |
| `manifest/subskill.rs` | 7 | 372 | 228 |
| `manifest/lockfile.rs` | 5 | 401 | 199 |
| `manifest/package/deps.rs` | 4 | 424 | 176 |
| `manifest/document.rs` | 3 | 385 | 215 |
| `manifest/redirect.rs` | 3 | 367 | 233 |
| `user_config.rs` | 3 | 305 | 295 |
| `capability_ref.rs` | 2 | 346 | 254 |
| `manifest/purl.rs` | 2 | 239 | 361 |
| `error.rs` | 1 | 140 | 460 |
| `manifest/i18n.rs` | 1 | 281 | 319 |
| `manifest/package/when.rs` | 1 | 155 | 445 |
| `package_ref.rs` | 1 | 535 | **65** |
| `values.rs` | 1 | 61 | 539 |

Sum = 55 across 15 files. **No file is at the 600 mine**; even `package.rs` +13 doctests
(~78 lines) lands at ~520 and `project.rs` +8 at ~532. No file is forced to split. The
single tight case is `package_ref.rs` (535, headroom 65) — but it carries exactly one
target type (`PackageKind`, an enum), so a terse example clears it.

`toml` is a normal dependency of vibe-core (`Cargo.toml:18`) and `serde_json` a dev-dep
(`:27`) — both reachable from doctests, so the TOML round-trip idiom (§1) compiles. `semver`
is also a normal dep.

**Exit state of this plan:**

- `pub-doctest` baseline entries **55 → 0**.
- conform total frozen **55 → 0** — the panel reaches an **empty baseline**.
- `CONFORM_GATED` unchanged at 16; `GATED_PUB_DOCTEST` unchanged at `["vibe-core"]` (gate
  stays armed).
- specmap warnings/suspects/orphans stay 0 (doc-only edits move no tags; an edge (b), if
  used, adds a `documents` edge to an existing unit and is regenerated).
- ≈55 new compiled doctests on vibe-core's public type API; every type a reader lands on
  teaches by example.

## 1. The drain idiom — how each type earns its doctest

The canonical example is the type's **real use**, not a contrived constructor. Four shapes
cover the 55; a fifth is the escape valve.

- **serde sections** (the majority: `PackageMeta`, the `*Section` family, `Subskill*`,
  `Locked*`, `Redirect*`, `*List`, `Provides`/`Requires`/`RequiresAny`, deps, …) — a
  **TOML round-trip** on the type's own doc, which is exactly how the type is deserialized
  in production:

  ```rust
  /// ```
  /// use vibe_core::manifest::PackageMeta;
  /// let p: PackageMeta = toml::from_str(r#"
  ///     name = "wal"
  ///     group = "org.vibevm"
  ///     kind = "feat"
  ///     version = "0.1.0"
  /// "#).unwrap();
  /// assert_eq!(p.name, "wal");
  /// ```
  ```

- **newtypes** (`CapabilityNamespace`, `CapabilityName`) — a one-line parse on the type's
  own doc (the module/`CapabilityRef` examples do not count for the wrapped type):

  ```rust
  /// ```
  /// let ns = vibe_core::CapabilityNamespace::parse("ui").unwrap();
  /// assert_eq!(ns, "ui");
  /// ```
  ```

- **enums** (`LinkType`, `SourceKind`, `AuthKind`, `RefPolicy`, `GitRefKind`, `PackageKind`,
  `ValueTag`, `DeliveryMode`, `BootCategory`, `NamingConvention`, `PublishPosture`,
  `WhenCondition`, …) — the smallest meaningful witness: a `toml::from_str::<T>("\"…\"")`
  for a serde-tagged enum, or a direct variant + assert.

- **error enums** (`Error`, `PurlError`, `UserConfigError`) — construct one variant and
  assert on its `Display`; the Class-F message already cites the `spec://` REQ, so the
  example doubles as a navigability demonstration.

- **escape valve (b)** — `#[spec(documents = "spec://…#unit")]`, a single attribute line,
  used **only** when a compiled example would teach nothing, or when a file approaches the
  600 budget and a doctest body would push it over. The `documents` target must be a real
  resolvable spec/guide unit (e.g. the governing PROP section or
  `discipline://core/cards/scaffold-g-doctests`), else specmap raises a
  pin-into-unmarked-unit warning (CONVERT-PLAN Phase 0.3 lesson). Default to a doctest;
  reserve (b) for the genuinely untestable.

The exact `use` path and field set come from each type's own schema; the examples above are
schematic. Prefer an `assert_eq!`/`assert!` so the example is a live oracle, not just a
compile check.

## 2. The worklist — eight file-cohesive batches

B1 first: it exercises all four idioms (newtype, enum, error, serde) on small files, fixing
the house pattern before it is tiled across the heavy serde clusters. The strictly-one-file
variant (15 commits) is equally legal if finer granularity is wanted.

- **B1 — idiom warm-up (5 types, 4 files):** `values.rs` (`ValueTag`); `error.rs` (`Error`);
  `package_ref.rs` (`PackageKind` — mind the 65-line headroom, keep it terse);
  `capability_ref.rs` (`CapabilityNamespace`, `CapabilityName`).
- **B2 — `manifest/package.rs` (13):** `BootCategory`, `BootSnippet`, `Compatibility`,
  `ConditionalTarget`, `ConflictsList`, `LinkType`, `Obsoletes`, `PackageMeta`, `Provides`,
  `PublishPosture`, `Requires`, `RequiresAny`, `TargetOs`. Heaviest single commit; +13
  doctests → ~520 lines (safe). Share one `[package]` TOML fixture across the section types.
- **B3 — `manifest/project.rs` (8):** `ActiveSection`, `AuthKind`, `LlmSection`,
  `MirrorSection`, `NamingConvention`, `OverrideSection`, `ProjectSection`,
  `RegistrySection`. +8 → ~532 (safe).
- **B4 — `manifest/subskill.rs` (7):** `ActivationRules`, `DeliveryMode`,
  `SubskillConflicts`, `SubskillContent`, `SubskillManifest`, `SubskillMeta`,
  `SubskillRecommends`.
- **B5 — `manifest/lockfile.rs` (5):** `LockedPackage`, `LockedSubskill`, `LockfileMeta`,
  `SourceKind`, `VirtualCapabilityRecord`.
- **B6 — `manifest/package/` subtree (5):** `deps.rs` — `GitPackageDep`, `GitRefKind`,
  `PathPackageDep`, `VarRegistryDep`; `when.rs` — `WhenCondition`.
- **B7 — small manifest sections (7):** `document.rs` — `BootSection`, `OriginSection`,
  `WorkspaceSection`; `redirect.rs` — `RedirectFile`, `RedirectSection`, `RefPolicy`;
  `i18n.rs` — `I18nDecl`.
- **B8 — `purl.rs` + `user_config.rs` (5):** `Purl`, `PurlError`; `InstallConfig`,
  `SlotIntegrity`, `UserConfigError`.

Total 5 + 13 + 8 + 7 + 5 + 5 + 7 + 5 = 55.

## 3. Per-batch cadence

1. Add doctests to one batch's types (editor tools only; never PowerShell `Set-Content`).
2. `cargo test -p vibe-core --doc` — doctests **compile and run** — then `cargo test -p
   vibe-core`.
3. `cargo fmt --all`.
4. `cargo xtask conform check` — expect the count to shrink, a "baseline entry no longer
   fires — prune it" warning for each drained fingerprint, and **0 new**.
5. `cargo xtask conform freeze` — then **review the diff: it must contain only deletions of
   `pub-doctest|…` lines, nothing added**. `freeze` rewrites the whole-workspace baseline
   (`xtask/src/conform.rs:262`); on an otherwise-clean tree that is a pure shrink. Any added
   line means something else regressed — stop and investigate before committing.
6. `cargo xtask specmap --check` — doc edits move no tags; required only if a batch used an
   edge (b), which adds a `documents` edge to regenerate.
7. Commit `docs(core): <file>'s public types teach by doctest`, body citing
   `spec://vibevm/terraforms/PUBDOC-DRAIN-v0.1#b<N>` and the `pub-doctest` ratchet
   (CONVERT-PLAN v0.1 §1.4). Group by file per Rule 3.
8. Push — routine per Rule 4.

At plan close (after B8): a full `bash tools/self-check.sh` through **Git Bash, not WSL**
(`bash` in PowerShell resolves to WSL); check `$?`, not a `| tail` (a tail pipe masks the
real exit code). Then refresh `spec/WAL.md` — the standing-ratchet line drops to zero.

## 4. Risks and gotchas

- **`file-length` 600.** No file is forced over budget (§0), but keep doctests terse on the
  three tightest: `package_ref.rs` (535), `project.rs` (484 → ~532), `package.rs`
  (442 → ~520). If a body would cross 600, fall back to edge (b) or split the file with the
  tests-out idiom (the CONVERT/SHRINK Phase-4 recipe) — a file must never cross 600 to drain
  a doc-debt entry.
- **edge (b) targets a real unit.** A `#[spec(documents = "…")]` edge is scanned by specmap
  and must resolve to an existing, marked unit, or it raises a pin-into-unmarked-unit
  warning. Prefer doctests; this is the rare exception.
- **doctests execute by default.** For these pure value types that is the point (the
  example is a live oracle). Only an example that does real I/O needs `no_run` — none here
  should.
- **the freeze diff is the audit.** A non-shrink `conform freeze` diff = immediate stop. The
  legal moments for `freeze` are a new-rule landing (not this plan) and a reviewed shrink
  (this plan).
- **PS5.1 UTF-8.** PowerShell 5.1 corrupts UTF-8-no-BOM round-trips; edit only via the
  editor tools, recover via `git restore`. `git commit` via `-F - <<'MSG'` heredoc only.

## 5. Exit / definition of done

`conform-baseline.json` carries **zero** `pub-doctest|…` entries (and, since this debt is
the whole residual, the baseline `findings` array is empty); `cargo xtask conform check`
reports 0 frozen / 0 new; `bash tools/self-check.sh` green; vibe-core remains in
`GATED_PUB_DOCTEST` with the gate armed against future undocumented types. The plan's header
status flips to an execution record (the CONVERT/SHRINK convention: the plan file carries
its own outcome). `spec/WAL.md`'s standing-ratchet line reads zero.

*Prediction:* no test expectation moves anywhere (doc-only additions); the freeze diff is
deletions only; no file crosses 600; edge (b) is used for 0–2 types at most, the rest land
as compiled doctests.
