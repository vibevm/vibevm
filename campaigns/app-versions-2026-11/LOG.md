# App-versions campaign log

Append-only. Newest entry last. One entry per phase step, spike or decision
that a later session would otherwise have to re-derive.

## 2026-11-10 — campaign planned

`PLAN.md` and `FORMAT-CHANGES.md` written against `main` at `7d495d137`;
`spec://org.vibevm.core/vibevm/common/PROP-075#root` rewritten from the
owner's decided shape into a settled design with the open section replaced by
decisions. Nothing implemented.

Facts established while planning, which the plan's §2 records in full and
which cost a survey to find:

- `install -g` does not go through `vibe-resolver`; it reaches the same
  `MultiRegistryResolver` through `vibe-package-source`, so one template layer
  serves both paths (`crates/vibe-app/src/resolve.rs:181-232`).
- `crates/vibe-requirements/` is the PROP-054 spec-facts query library, **not**
  the dependency-requirement grammar; that is
  `PackageRef::parse` / `VersionSpec` in `crates/vibe-core/src/package_ref.rs`.
- `vibe check` does **not** validate spec-XML dialect well-formedness: a
  deliberately malformed probe spec under `vibevm/vibespecs/common/` left
  `vibe check` at `0 errors, 42 warnings`. Whoever edits a spec XML must
  validate it another way (the projector in `crates/vibe-specdoc`), and the
  campaign should consider adding that check — filed as a candidate, not done.
- The per-version index record is a **shared vocabulary** (`version_entry`), so
  one optional member reaches `primary.jsonl`, `by-name` and the journal at
  once (spike S3 confirms the golden bytes do not move).
- `formats/EPOCHS.toml` is `public = false`, `break_window_open = true`:
  breaking is free and no `formats/breaks/` note is required.
- nvm-sh is pinned at v0.41.0 (`4e8758a801eea9479b32a904f31093e410ddaba4`),
  whose surface is wider than the 0.40.x line (`--only=<categories>`,
  `install --offline`, `--save`/`-w`, install locks, `$NVM_AUTH_HEADER`); its
  own `test/fast/` suite is *near*-offline and has no suite-wide mirror
  override, so reuse is bounded by a recorded allow-list.
- nodejs.org publishes digests but no sizes, so the Node template omits `size`
  — the one deliberate loss against the retired generator
  (`AV-FMT-006`).

Open to the owner: `PROP-075#OWNER-QUESTIONS` Q1–Q5. Only Q1 (row-by-row
approval of the proposed nvm exclusions) blocks a phase, and only P7.
