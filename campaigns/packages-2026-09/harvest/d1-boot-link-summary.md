# D1 — boot-link rewrite: the twenty-line answer

- **Counts in `spec/boot/STATIC.md`:** 69 `](../flows/` links · 41 `spec/flows/`
  strings (38 are the links' own labels, 3 are bare prose) · **60 distinct
  documents** across **25 flow directories**. 62 links come from direct snippets,
  7 ride inside `flow-git-practices`'s compiled `STATIC.md`; 7 are duplicates.
- **Repair A (snippets):** 27 canonical files, 65 occurrences under
  `packages/org.vibevm.*/` (70 files / 184 occurrences counting vendored copies).
- **Repair B (compiler):** one function — `render_static`,
  `crates/vibe-workspace/src/boot_artifacts.rs:220`. It is the only renderer;
  the host lane and every per-unit slot both call it.
- **Is the information available there? Yes for 62 of 69, no for 7.**
  `BootEntry.path` is the snippet's workspace-relative path
  (`crates/vibe-workspace/src/boot.rs:131`) and `render_static` already reads it
  at `boot_artifacts.rs:245-256`; the target is `../../<slot>/spec/flows/…`. But
  for the `flow-git-practices` entry the path names an aggregator that has **no
  `spec/flows/` at all**, and its 7 links belong to four other packages — their
  provenance survives only as HTML comments inside the body text.
- **Today's transform is `body.trim_end()` and nothing else**
  (`boot_artifacts.rs:259`). No rewriting precedent exists anywhere: materialisation
  is `fs::copy`/`hard_link` (`vibedeps.rs:352-364`), and every `.replace(` in
  `crates/` + `xtask/` is separator normalisation, HTML escaping, or test data.
- **Sharpest fact:** the link is *correct in the installed slot* for 22 of 27
  packages — `vibedeps/flow-wal/0.2.0/spec/boot/` + `../flows/wal/…` hits a real
  file, and the `INDEX.md` dynamic lane reads it that way successfully.
  Concatenation into `spec/boot/STATIC.md` is what severs it. The other 5
  (`sync-from-code`, `git-atomic-commits`, `git-autonomy`,
  `git-conventional-commits`, `dev-runtime-docs`) declare their snippet at bare
  `boot/`, so their 12 links in the lane dangle *before* any compiler step.
- **What binds a decision:** PROP-009 §2.1 already obliges the packages —
  *"a package's internal cross-references must become package-relative or
  `spec://` URIs"* (`spec/modules/vibe-workspace/PROP-009-loading-model.md:41`),
  and the same section retires `spec/flows/` as a consumer path (`:40`;
  `vibe.lock` shows 36 × `files_written = []`). Against that, `STATIC.md` is
  specified twice as the **verbatim** concatenation (`:61`, `:99`) — so a
  compile-time rewrite is a spec amendment, not only a code change.
