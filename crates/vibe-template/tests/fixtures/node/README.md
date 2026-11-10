# Node template fixtures — what each file is, and where it came from

These are the inputs and the expected outputs of `tests/node_golden.rs`,
the correctness proof of the PROP-075 renderer: the template of
`org.vibevm.node/node` must render the manifests the retired generator
produced, byte for byte. The same files drive
`crates/vibe-cli/tests/cli_template_node.rs`, the channel and
`template_optional` proof of campaign phase P3.

**Every document here is upstream's own, frozen.** Phase P1 derived two of
them by hand; phase P3 replaced each with the real document, fetched once on
2026-11-10 from `https://nodejs.org/` and committed verbatim. Nothing in
either test suite connects to anything: the fixtures are the whole input.

- **`vibe.toml`** — the **template** manifest of `org.vibevm.node/node`,
  byte-identical to the one `/fast/git/vibe-pkg-node` carries at template
  revision `1.0.0` (campaign `AV-FMT-006`). It is the committed `vibe.toml`
  of tag `v24.22.2` with the generator's four provenance header lines
  removed, every occurrence of the produced version replaced by
  `{version}`, `[package].version` set to the template's own revision,
  every `sha256` replaced by a `{sha256:<archive>}` variable, every `size`
  line dropped (nodejs.org publishes digests but not sizes — PROP-075
  `#TEMPLATE-DIGEST-SOURCE`), a `[template]` declaration added, and
  `template_optional = true` on the **three** rows upstream does not carry
  for every version in the template's `min` window: `linux-musl-x64`,
  `windows-arm64` and `macos-arm64`. (P1 marked the first two; P3 measured
  the frozen index and found the third — Apple silicon archives start at
  `v16.0.0`, so without it every version below that refuses to render.
  Measured over the 406 produced versions of the frozen index:
  `linux-x64-musl` is absent from 389, `osx-arm64-tar` from 132,
  `win-arm64-zip` from 234, and all seven targets are present together only
  from `v26.8.0` up.)

- **`vibe-24.22.2.toml`**, **`vibe-22.23.4.toml`** — the manifests
  `/fast/git/vibe-pkg-node` carries at tags `v24.22.2` and `v22.23.4`,
  copied verbatim (`git show v24.22.2:vibe.toml`). They are the golden
  bytes; the test strips only the generator's provenance header and the
  `size` lines before comparing.

- **`SHASUMS256-24.22.2.txt`**, **`SHASUMS256-22.23.4.txt`** — the release
  checksum documents of those two versions, fetched verbatim from
  `https://nodejs.org/dist/v<V>/SHASUMS256.txt` (34 and 43 lines; the
  template names seven of them and the parser skips every line it does not
  need). Each one hashes to exactly the digest the committed manifest's own
  provenance header records as its input
  (`aec92ae945293821c22aed4c37b97741688254bac353b861fc9fb25b81ce4567` and
  `c90c3f1478fda8308b22cc1a1b0dc536169196444e67ee8b5c582fd351d02bce`), and
  `node_golden.rs` asserts that equality, so the golden comparison now runs
  from upstream's bytes end to end: the template and the retired generator
  are proved to render the same manifest *from the same document*. P1's
  seven-line hand-derived excerpts, which phase P3 verified against these
  (0 mismatches), are gone.

- **`SHASUMS256-26.13.0.txt`**, **`SHASUMS256-12.0.0.txt`** — the same
  documents for the two versions phase P3's channel test needs: the newest
  LTS release at freeze time (what `@lts` lowers to) and the oldest version
  in the template's `min` window (what `template_optional` is measured on —
  it carries neither the musl, nor the Apple-silicon, nor the Windows-ARM
  archive).

- **`index-excerpt.json`** — a frozen subset of
  `https://nodejs.org/dist/index.json`, fetched 2026-11-10
  (878 records, `sha256 =
  6b27fe5766e87929d16ccdde05c516cbdd12200eab62b28d4b2f31d34e21481a`).
  Upstream writes one record per line, newest first, so the subset is
  **line-selected and every kept record is byte-verbatim**:

  > keep the array's leading records down to and including `v12.0.0` — the
  > template's declared `min` — plus the next three records, which are
  > below `min` and must be filtered.

  That is 410 of 878 records (`sha256 =
  b2f5470ded88f524611f26fd1feae5ace717a503714bbe7752da121e4af61c0a`,
  175 KiB instead of 329 KiB), and because the document is sorted the subset
  is the template's **whole** version domain: every answer the tests check
  is upstream's real answer, not a doctored one. Of the 410: one prerelease
  (`v27.0.0-alpha.0.0.0`) the `prerelease` filter drops, three records below
  `min` (`v11.15.0`, `v11.14.0`, `v11.13.0`) the `min` filter drops, and
  **406 produced versions** from `v12.0.0` to `v26.13.0`. 177 of those carry
  a truthy `lts`, in eight codenames — `lts` (truthy) resolves to `v26.13.0`
  (Lithium) and `lts/jod` to `v22.23.4`.

  The one thing the hand-written P1 excerpt proved and this document cannot
  is that a record whose `version` is not semver is **skipped with a reason**
  rather than refused (PROP-075 `#TEMPLATE-SOURCE-KINDS`): upstream's real
  index has no such record. That behaviour is covered by
  `src/source/tests.rs::a_record_whose_version_is_not_semver_is_skipped_with_a_reason`
  and by the `version_records` doctest, and `node_golden.rs` asserts the
  frozen document skips nothing at all.

## Refreshing these fixtures

Nothing here is regenerated on a schedule: they are frozen evidence, and a
test that changes its own expectations when upstream publishes is not a
test. To move them deliberately, refetch the five documents above, re-apply
the one-line selection rule to `index.json`, and expect the channel
assertions of `cli_template_node.rs` to move with them — `@lts` names
whatever the newest LTS release is in the document you froze.
