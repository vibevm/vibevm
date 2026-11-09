# Node template fixtures — what each file is, and where it came from

These are the inputs and the expected outputs of
`tests/node_golden.rs`, the correctness proof of the PROP-075 renderer:
the template of `org.vibevm.node/node` must render the manifests the
retired generator produced, byte for byte.

- **`vibe.toml`** — the **template** manifest of `org.vibevm.node/node`,
  authored here for PROP-075 phase P1 and the source of the template the
  package repository will carry in phase P3
  (`campaigns/app-versions-2026-11/PLAN.md`, AV-FMT-006). It is the
  committed `vibe.toml` of tag `v24.22.2` with the generator's four
  provenance header lines removed, every occurrence of the produced
  version replaced by `{version}`, `[package].version` set to the
  template's own revision, every `sha256` replaced by a
  `{sha256:<archive>}` variable, every `size` line dropped (nodejs.org
  publishes digests but not sizes — PROP-075
  `#TEMPLATE-DIGEST-SOURCE`), a `[template]` declaration added, and
  `template_optional = true` on the two rows upstream's `files[]` does not
  carry for every version (`linux-musl-x64`, `windows-arm64`).

- **`vibe-24.22.2.toml`**, **`vibe-22.23.4.toml`** — the manifests
  `/fast/git/vibe-pkg-node` carries at tags `v24.22.2` and `v22.23.4`,
  copied verbatim (`git show v24.22.2:vibe.toml`). They are the golden
  bytes; the test strips only the generator's provenance header and the
  `size` lines before comparing.

- **`SHASUMS256-24.22.2.txt`**, **`SHASUMS256-22.23.4.txt`** — the seven
  `<digest>  <archive>` pairs of each release that the template consumes,
  in the exact format `https://nodejs.org/dist/v<V>/SHASUMS256.txt`
  publishes. Derived mechanically from the `sha256` and `source` fields of
  the two committed manifests above, which is where those digests came
  from in the first place; the upstream document lists further files the
  template does not name, and the parser skips every line it does not
  need.

- **`index-excerpt.json`** — six records in the shape of
  `https://nodejs.org/dist/index.json` (a top-level array whose records
  carry `version`, `date`, `files`, `npm`, `v8`, `uv`, `zlib`, `openssl`,
  `modules`, `lts` as `false` or a codename, and `security`; verified
  against the live document on 2026-11-10, PROP-075
  `#TEMPLATE-SELECTOR`). It is **hand-written, not a copy of the live
  document**: the renderer's correctness depends on the document's shape
  and on the two records it must produce, not on the other 872. The last
  two records exist to be filtered or skipped — a prerelease below the
  template's `min`, and a record whose version string is not semver at
  all, which `#TEMPLATE-SOURCE-KINDS` requires to be skipped with a
  reason rather than refused. Phase P3 freezes a real copy for its
  channel test.
